//! Google Calendar API v3 호출 (기본 캘린더 `primary` 고정, F-23)

use reqwest::blocking::{RequestBuilder, Response};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;

use super::auth::{http, GoogleAuth};
use super::CALENDAR_API;
use crate::error::{AppError, AppResult};
use crate::sync::mapping::Event;

/// 호출 결과. 호출한 쪽이 상태 코드별로 처리해야 하는 4xx 는 `Status` 로 돌려준다.
/// 네트워크 오류·5xx·429 처럼 나중에 다시 시도하면 되는 오류는 `Err` 로 돌려준다.
pub enum Reply<T> {
    Ok(T),
    Status(u16),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    #[serde(default)]
    pub items: Vec<Event>,
    pub next_page_token: Option<String>,
    pub next_sync_token: Option<String>,
}

pub struct Calendar<'a> {
    auth: &'a GoogleAuth,
}

impl<'a> Calendar<'a> {
    pub fn new(auth: &'a GoogleAuth) -> Self {
        Self { auth }
    }

    /// 401 이면 액세스 토큰을 새로 받아 한 번 더 시도한다
    fn send(&self, build: impl Fn(&reqwest::blocking::Client) -> RequestBuilder) -> AppResult<Response> {
        let client = http();
        for attempt in 0..2 {
            let res = build(&client).bearer_auth(self.auth.access_token()?).send()?;
            if res.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                self.auth.invalidate_access();
                continue;
            }
            if res.status() == StatusCode::UNAUTHORIZED {
                return Err(AppError::AuthRequired);
            }
            let s = res.status();
            if s.is_server_error() || s == StatusCode::TOO_MANY_REQUESTS {
                return Err(AppError::Google { status: s.as_u16(), message: res.text().unwrap_or_default() });
            }
            return Ok(res);
        }
        unreachable!()
    }

    fn json<T: for<'de> Deserialize<'de>>(res: Response) -> AppResult<Reply<T>> {
        let status = res.status();
        if status.is_success() {
            Ok(Reply::Ok(res.json()?))
        } else {
            log::debug!("구글 API {status}: {}", res.text().unwrap_or_default());
            Ok(Reply::Status(status.as_u16()))
        }
    }

    pub fn insert(&self, body: &Value) -> AppResult<Reply<Event>> {
        Self::json(self.send(|c| c.post(format!("{CALENDAR_API}/events")).json(body))?)
    }

    /// 전체 교체(PUT). `if_match` 를 주면 그 etag 일 때만 바꾼다 (다르면 412)
    pub fn update(&self, id: &str, body: &Value, if_match: Option<&str>) -> AppResult<Reply<Event>> {
        Self::json(self.send(|c| {
            let req = c.put(format!("{CALENDAR_API}/events/{id}")).json(body);
            match if_match {
                Some(etag) => req.header("If-Match", etag),
                None => req,
            }
        })?)
    }

    pub fn get(&self, id: &str) -> AppResult<Reply<Event>> {
        Self::json(self.send(|c| c.get(format!("{CALENDAR_API}/events/{id}")))?)
    }

    pub fn delete(&self, id: &str) -> AppResult<Reply<()>> {
        let res = self.send(|c| c.delete(format!("{CALENDAR_API}/events/{id}")))?;
        Ok(if res.status().is_success() { Reply::Ok(()) } else { Reply::Status(res.status().as_u16()) })
    }

    /// 일정 목록 한 페이지. `sync_token` 이 있으면 그 뒤로 바뀐 것만(삭제 포함) 받는다.
    pub fn list(&self, sync_token: Option<&str>, page_token: Option<&str>) -> AppResult<Reply<EventsPage>> {
        Self::json(self.send(|c| {
            let mut q: Vec<(&str, &str)> = vec![("maxResults", "250")];
            if let Some(t) = sync_token {
                q.push(("syncToken", t));
            }
            if let Some(p) = page_token {
                q.push(("pageToken", p));
            }
            c.get(format!("{CALENDAR_API}/events")).query(&q)
        })?)
    }
}

/// 다시 시도하면 될 수 있는 오류인지 (오프라인, 5xx, 429)
pub fn is_retryable(e: &AppError) -> bool {
    match e {
        AppError::Network(_) => true,
        AppError::Google { status, .. } => *status >= 500 || *status == 429,
        _ => false,
    }
}
