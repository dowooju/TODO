//! 구글 캘린더 연동 (F-23 ~ F-28)

pub mod auth;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::repo;
use auth::{http, GoogleAuth};

pub const AUTH_CHANGED: &str = "google-auth-changed";
pub const CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3/calendars/primary";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleStatus {
    /// .env 에 OAuth 클라이언트 ID 가 들어간 채로 빌드됐는지
    pub configured: bool,
    pub logged_in: bool,
    /// 기본 캘린더 이름 (보통 구글 계정 이메일)
    pub account: Option<String>,
}

pub fn status(db: &Db, auth: &GoogleAuth) -> AppResult<GoogleStatus> {
    let logged_in = auth.is_logged_in();
    Ok(GoogleStatus {
        configured: auth::client_config().is_some(),
        logged_in,
        account: if logged_in { repo::get_setting(&db.conn(), "google_account")? } else { None },
    })
}

/// 기본 캘린더의 이름(= 계정 이메일)을 가져온다. calendar.events 범위만으로 읽을 수 있는
/// events.list 응답의 summary 필드를 쓴다.
fn fetch_account(auth: &GoogleAuth) -> AppResult<String> {
    #[derive(serde::Deserialize)]
    struct Res {
        summary: Option<String>,
    }
    let res = http()
        .get(format!("{CALENDAR_API}/events"))
        .bearer_auth(auth.access_token()?)
        .query(&[("maxResults", "1")])
        .send()?;
    if !res.status().is_success() {
        return Err(AppError::Google { status: res.status().as_u16(), message: res.text().unwrap_or_default() });
    }
    Ok(res.json::<Res>()?.summary.unwrap_or_else(|| "구글 캘린더".into()))
}

#[tauri::command]
pub fn google_status(db: State<'_, Db>, auth: State<'_, GoogleAuth>) -> AppResult<GoogleStatus> {
    status(&db, &auth)
}

/// 브라우저 로그인이 끝날 때까지 기다려야 하므로 블로킹 스레드에서 실행한다
#[tauri::command]
pub async fn google_login(app: AppHandle) -> AppResult<GoogleStatus> {
    use tauri::Manager;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let auth = handle.state::<GoogleAuth>();
        let db = handle.state::<Db>();
        auth.login(&handle)?;
        match fetch_account(&auth) {
            Ok(account) => repo::put_setting(&db.conn(), "google_account", &account)?,
            Err(e) => log::warn!("계정 이름을 가져오지 못함: {e}"),
        }
        let _ = handle.emit(AUTH_CHANGED, ());
        status(&db, &auth)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

#[tauri::command]
pub async fn google_logout(app: AppHandle) -> AppResult<GoogleStatus> {
    use tauri::Manager;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let auth = handle.state::<GoogleAuth>();
        let db = handle.state::<Db>();
        auth.logout()?;
        repo::put_setting(&db.conn(), "google_account", "")?;
        let _ = handle.emit(AUTH_CHANGED, ());
        status(&db, &auth)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}
