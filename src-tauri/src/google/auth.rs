//! 구글 로그인 (F-23)
//!
//! OAuth 2.0 "데스크톱 앱" 클라이언트 + 루프백 리디렉트(127.0.0.1 임의 포트) + PKCE.
//! 로그인 화면은 사용자의 기본 브라우저로 연다. 별도 서버가 필요 없다.
//!
//! - 리프레시 토큰은 Windows 자격 증명 관리자에 보관한다 (PRD 6. 보안)
//! - 액세스 토큰은 메모리에만 두고, 만료 1분 전에 리프레시 토큰으로 새로 받는다

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
pub const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

const KEYRING_SERVICE: &str = "com.todocalendar.app";
const KEYRING_USER: &str = "google-refresh-token";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// .env 에서 빌드 시점에 넣은 OAuth 클라이언트 정보 (build.rs 참고)
pub struct ClientConfig {
    pub id: &'static str,
    pub secret: Option<&'static str>,
}

pub fn client_config() -> Option<ClientConfig> {
    let id = option_env!("GOOGLE_CLIENT_ID")?;
    Some(ClientConfig { id, secret: option_env!("GOOGLE_CLIENT_SECRET") })
}

pub fn http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("todo-calendar/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP 클라이언트 생성 실패")
}

// ───────────────────────── 토큰 보관 ─────────────────────────

fn keyring_entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| AppError::Other(format!("자격 증명 관리자 오류: {e}")))
}

fn load_refresh_token() -> AppResult<Option<String>> {
    match keyring_entry()?.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Other(format!("자격 증명 관리자 읽기 실패: {e}"))),
    }
}

fn save_refresh_token(token: &str) -> AppResult<()> {
    keyring_entry()?
        .set_password(token)
        .map_err(|e| AppError::Other(format!("자격 증명 관리자 저장 실패: {e}")))
}

fn delete_refresh_token() -> AppResult<()> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Other(format!("자격 증명 관리자 삭제 실패: {e}"))),
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

// ───────────────────────── 인증 상태 ─────────────────────────

/// Tauri state 로 등록되는 인증 관리자. 동기화 스레드와 명령 처리기가 함께 쓴다.
#[derive(Default)]
pub struct GoogleAuth {
    access: Mutex<Option<(String, Instant)>>,
}

impl GoogleAuth {
    pub fn is_logged_in(&self) -> bool {
        matches!(load_refresh_token(), Ok(Some(_)))
    }

    /// 유효한 액세스 토큰. 필요하면 리프레시 토큰으로 새로 받는다.
    pub fn access_token(&self) -> AppResult<String> {
        if let Some((token, expires)) = self.access.lock().unwrap().as_ref() {
            if Instant::now() + Duration::from_secs(60) < *expires {
                return Ok(token.clone());
            }
        }
        let refresh = load_refresh_token()?.ok_or(AppError::AuthRequired)?;
        let cfg = client_config().ok_or(AppError::AuthRequired)?;
        let mut form = vec![
            ("client_id", cfg.id),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = cfg.secret {
            form.push(("client_secret", secret));
        }
        let res = http().post(TOKEN_URL).form(&form).send()?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let err: Option<TokenError> = res.json().ok();
            // 사용자가 권한을 취소했거나 리프레시 토큰이 만료됨 → 다시 로그인해야 한다
            if err.as_ref().is_some_and(|e| e.error == "invalid_grant") {
                log::warn!("리프레시 토큰이 더 이상 유효하지 않음 → 로그아웃 처리");
                let _ = delete_refresh_token();
                return Err(AppError::AuthRequired);
            }
            let message = err.map(|e| e.error_description.unwrap_or(e.error)).unwrap_or_default();
            return Err(AppError::Google { status, message });
        }
        let tok: TokenResponse = res.json()?;
        self.store_access(&tok);
        Ok(tok.access_token)
    }

    fn store_access(&self, tok: &TokenResponse) {
        *self.access.lock().unwrap() =
            Some((tok.access_token.clone(), Instant::now() + Duration::from_secs(tok.expires_in)));
    }

    /// 401 을 받았을 때 캐시된 액세스 토큰을 버린다
    pub fn invalidate_access(&self) {
        *self.access.lock().unwrap() = None;
    }

    /// 기본 브라우저로 구글 로그인 → 루프백으로 인가 코드 수신 → 토큰 교환 → 저장.
    /// 사용자가 브라우저에서 끝낼 때까지 막히므로 별도 스레드에서 불러야 한다.
    pub fn login(&self, app: &AppHandle) -> AppResult<()> {
        let cfg = client_config()
            .ok_or_else(|| AppError::Other("구글 OAuth 클라이언트 ID 가 설정되지 않았습니다. README 를 참고하세요.".into()))?;

        let verifier = random_token();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_token();

        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| AppError::Other(e.to_string()))?;
        let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr().map_err(|e| AppError::Other(e.to_string()))?.port());

        let mut url = Url::parse(AUTH_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", cfg.id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", SCOPE)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            // 리프레시 토큰을 받기 위해 오프라인 접근 + 매번 동의 화면
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| AppError::Other(format!("브라우저를 열지 못했습니다: {e}")))?;

        let code = wait_for_code(&listener, &state)?;

        let mut form = vec![
            ("client_id", cfg.id),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ];
        if let Some(secret) = cfg.secret {
            form.push(("client_secret", secret));
        }
        let res = http().post(TOKEN_URL).form(&form).send()?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let message = res
                .json::<TokenError>()
                .map(|e| e.error_description.unwrap_or(e.error))
                .unwrap_or_default();
            return Err(AppError::Google { status, message });
        }
        let tok: TokenResponse = res.json()?;
        let refresh = tok
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::Other("구글이 리프레시 토큰을 주지 않았습니다. 다시 시도해 주세요.".into()))?;
        save_refresh_token(&refresh)?;
        self.store_access(&tok);
        log::info!("구글 로그인 완료");
        Ok(())
    }

    pub fn logout(&self) -> AppResult<()> {
        if let Ok(Some(token)) = load_refresh_token() {
            // 구글 쪽 권한도 취소한다 (실패해도 로컬 로그아웃은 진행)
            if let Err(e) = http().post(REVOKE_URL).form(&[("token", token.as_str())]).send() {
                log::warn!("토큰 취소 요청 실패: {e}");
            }
        }
        self.invalidate_access();
        delete_refresh_token()?;
        log::info!("구글 로그아웃");
        Ok(())
    }
}

/// 32바이트 무작위 값(UUID v4 두 개, OS 난수 사용)을 base64url 로 → 43자. PKCE verifier 규격(43~128자) 충족
fn random_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 브라우저가 http://127.0.0.1:<port>/?code=...&state=... 로 돌아올 때까지 기다린다
fn wait_for_code(listener: &TcpListener, expected_state: &str) -> AppResult<String> {
    listener.set_nonblocking(true).map_err(|e| AppError::Other(e.to_string()))?;
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        if Instant::now() > deadline {
            return Err(AppError::Other("로그인 시간이 초과됐습니다. 다시 시도해 주세요.".into()));
        }
        let mut stream = match listener.accept() {
            Ok((s, _)) => s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(e) => return Err(AppError::Other(e.to_string())),
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            continue;
        }
        // "GET /?code=...&state=... HTTP/1.1"
        let Some(path) = line.split_whitespace().nth(1) else { continue };
        let Ok(url) = Url::parse(&format!("http://127.0.0.1{path}")) else { continue };
        let param = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());

        if let Some(err) = param("error") {
            respond(&mut stream, "로그인이 취소됐어요", "앱으로 돌아가 다시 시도해 주세요.");
            return Err(AppError::Other(format!("구글 로그인이 취소됐습니다 ({err})")));
        }
        let Some(code) = param("code") else {
            // favicon 요청 등은 무시
            respond(&mut stream, "", "");
            continue;
        };
        if param("state").as_deref() != Some(expected_state) {
            respond(&mut stream, "잘못된 요청이에요", "앱에서 다시 로그인해 주세요.");
            return Err(AppError::Other("로그인 응답의 state 값이 일치하지 않습니다.".into()));
        }
        respond(&mut stream, "로그인 완료!", "이 창을 닫고 내 달력 앱으로 돌아가세요.");
        return Ok(code);
    }
}

fn respond(stream: &mut TcpStream, title: &str, body: &str) {
    let html = format!(
        "<!doctype html><html lang=\"ko\"><meta charset=\"utf-8\"><title>내 달력</title>\
         <body style=\"font-family:'Malgun Gothic',sans-serif;text-align:center;padding-top:80px;color:#333\">\
         <h2 style=\"color:#d1608a\">{title}</h2><p>{body}</p></body></html>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.flush();
}
