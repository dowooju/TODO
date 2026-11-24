//! 구글 캘린더 양방향 동기화 워커 (F-25 ~ F-28, F-30)
//!
//! - 보내기: 할 일이 바뀌면 명령이 outbox 에 쌓고 워커를 깨운다 → 즉시 전송.
//!   오프라인이거나 5xx·429 이면 지수 백오프로 다시 시도한다.
//! - 받기: syncToken 증분 동기화를 앱 실행 시와 5분마다. 410 Gone 이면 전체 동기화를 다시 한다.
//! - 충돌: updated 기준 Last-Write-Wins, 수정 요청에는 If-Match: etag (mapping.rs 참고)
//!
//! 실시간 푸시(watch 채널)는 외부에서 접근 가능한 HTTPS 웹훅이 필요해 서버 없는 앱에서는 쓰지 않는다.

pub mod mapping;
pub mod outbox;

use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::notify_changed;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::google::auth::{client_config, GoogleAuth};
use crate::google::calendar::{is_retryable, Calendar, Reply};
use crate::google::AUTH_CHANGED;
use crate::repo;
use mapping::{apply_remote, event_body, event_id, set_task_etag, task_etag};
use outbox::{Item, Op};

const PULL_INTERVAL: Duration = Duration::from_secs(5 * 60);
const PULL_RETRY: Duration = Duration::from_secs(30);
const IDLE_WAIT: Duration = Duration::from_secs(60);
/// 다시 시도해도 소용없는 오류(400 등)는 이만큼 실패하면 버린다
const MAX_ATTEMPTS: i64 = 8;

pub const SYNC_STATUS: &str = "sync-status";
const KEY_SYNC_TOKEN: &str = "google_sync_token";
pub const KEY_LAST_SYNC: &str = "google_last_sync_at";
pub const KEY_LAST_ERROR: &str = "google_last_error";

/// 워커를 깨우는 손잡이 (Tauri state). `true` 를 보내면 받기도 바로 한다.
pub struct SyncHandle(Mutex<Sender<bool>>);

impl SyncHandle {
    pub fn wake(&self) {
        let _ = self.0.lock().unwrap().send(false);
    }
    pub fn sync_now(&self) {
        let _ = self.0.lock().unwrap().send(true);
    }
}

pub fn spawn(app: AppHandle, db: Db) -> SyncHandle {
    let (tx, rx) = channel::<bool>();
    thread::Builder::new()
        .name("google-sync".into())
        .spawn(move || {
            let mut next_pull = Instant::now();
            loop {
                let auth = app.state::<GoogleAuth>();
                let mut wait = IDLE_WAIT;
                if client_config().is_some() && auth.is_logged_in() {
                    let due_pull = Instant::now() >= next_pull;
                    match cycle(&app, &db, &auth, due_pull) {
                        Ok(()) => {
                            if due_pull {
                                next_pull = Instant::now() + PULL_INTERVAL;
                            }
                            record(&db, None);
                        }
                        Err(AppError::AuthRequired) => {
                            record(&db, Some("구글 계정으로 다시 로그인해 주세요."));
                            let _ = app.emit(AUTH_CHANGED, ());
                        }
                        Err(e) => {
                            log::warn!("동기화 실패: {e}");
                            if due_pull {
                                next_pull = Instant::now() + PULL_RETRY;
                            }
                            record(&db, Some(&e.to_string()));
                        }
                    }
                    let _ = app.emit(SYNC_STATUS, ());
                    wait = time_until_next(&db, next_pull);
                }
                match rx.recv_timeout(wait) {
                    Ok(force) => {
                        // 연달아 온 신호는 한 번으로 합친다
                        let force = force | rx.try_iter().fold(false, |a, b| a | b);
                        if force {
                            next_pull = Instant::now();
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .expect("동기화 스레드를 시작하지 못했습니다");
    SyncHandle(Mutex::new(tx))
}

fn time_until_next(db: &Db, next_pull: Instant) -> Duration {
    let until_pull = next_pull.saturating_duration_since(Instant::now());
    let until_push = outbox::next_attempt(&db.conn())
        .ok()
        .flatten()
        .map(|t| (t - Utc::now()).to_std().unwrap_or(Duration::ZERO));
    until_push.map_or(until_pull, |p| p.min(until_pull)).max(Duration::from_millis(500))
}

fn record(db: &Db, error: Option<&str>) {
    let conn = db.conn();
    if error.is_none() {
        let _ = repo::put_setting(&conn, KEY_LAST_SYNC, &repo::now());
    }
    let _ = repo::put_setting(&conn, KEY_LAST_ERROR, error.unwrap_or(""));
}

fn cycle(app: &AppHandle, db: &Db, auth: &GoogleAuth, pull_now: bool) -> AppResult<()> {
    let cal = Calendar::new(auth);
    let mut changed = push(db, &cal)?;
    if pull_now {
        changed |= pull(db, &cal)?;
        changed |= refresh_display_events(db, &cal)?;
        // 받는 동안 쌓인 변경도 바로 보낸다
        changed |= push(db, &cal)?;
    }
    if changed {
        notify_changed(app);
    }
    Ok(())
}

// ───────────────────────── 보내기 ─────────────────────────

/// 보낼 차례가 된 항목을 모두 보낸다. 로컬 데이터가 바뀌었으면(충돌로 구글 쪽을 반영) true.
fn push(db: &Db, cal: &Calendar) -> AppResult<bool> {
    let items = outbox::due(&db.conn(), Utc::now())?;
    let mut changed = false;
    for item in items {
        match push_item(db, cal, &item) {
            Ok(c) => {
                changed |= c;
                outbox::done(&db.conn(), item.id)?;
            }
            Err(AppError::AuthRequired) => return Err(AppError::AuthRequired),
            Err(e) => {
                let conn = db.conn();
                if !is_retryable(&e) && item.attempts + 1 >= MAX_ATTEMPTS {
                    log::error!("동기화 포기 ({} {:?}): {e}", item.task_id, item.op);
                    outbox::done(&conn, item.id)?;
                } else {
                    log::warn!("동기화 재시도 예정 ({} {:?}): {e}", item.task_id, item.op);
                    outbox::retry_later(&conn, &item, &e.to_string(), Utc::now())?;
                }
                if is_retryable(&e) {
                    // 오프라인이면 남은 항목도 어차피 실패하니 이번 차례는 여기서 멈춘다
                    return Err(e);
                }
            }
        }
    }
    Ok(changed)
}

fn unexpected(status: u16) -> AppError {
    AppError::Google { status, message: "예상하지 못한 응답".into() }
}

fn push_item(db: &Db, cal: &Calendar, item: &Item) -> AppResult<bool> {
    let (task, etag) = {
        let conn = db.conn();
        (repo::find_task(&conn, &item.task_id)?, task_etag(&conn, &item.task_id)?)
    };
    let eid = event_id(&item.task_id);

    // 날짜가 없어졌거나 할 일이 사라졌으면 일정 삭제 (F-28)
    let task = match (item.op, task) {
        (Op::Upsert, Some(t)) if t.date.is_some() => t,
        _ => {
            match cal.delete(&eid)? {
                Reply::Ok(()) | Reply::Status(404) | Reply::Status(410) => {}
                Reply::Status(s) => return Err(unexpected(s)),
            }
            set_task_etag(&db.conn(), &item.task_id, None)?;
            return Ok(false);
        }
    };

    let body = event_body(&task);
    let saved = match etag {
        // 아직 구글에 없음 → 생성. 같은 ID 가 이미 있으면(재시도, 삭제됐던 일정) 409 → 덮어쓰기
        None => match cal.insert(&body)? {
            Reply::Ok(ev) => ev,
            Reply::Status(409) => match cal.update(&eid, &body, None)? {
                Reply::Ok(ev) => ev,
                Reply::Status(s) => return Err(unexpected(s)),
            },
            Reply::Status(s) => return Err(unexpected(s)),
        },
        Some(tag) => match cal.update(&eid, &body, Some(&tag))? {
            Reply::Ok(ev) => ev,
            // 그사이 구글에서도 바뀜 → Last-Write-Wins
            Reply::Status(412) => {
                let Reply::Ok(remote) = cal.get(&eid)? else { return Err(unexpected(412)) };
                let remote_newer = match (remote.updated_at(), chrono::DateTime::parse_from_rfc3339(&task.updated_at)) {
                    (Some(r), Ok(l)) => r > l,
                    _ => false,
                };
                if remote_newer {
                    log::info!("충돌: 구글 쪽이 더 최신 → 로컬에 반영 ({})", task.id);
                    return apply_remote(&db.conn(), &remote);
                }
                match cal.update(&eid, &body, None)? {
                    Reply::Ok(ev) => ev,
                    Reply::Status(s) => return Err(unexpected(s)),
                }
            }
            Reply::Status(404) | Reply::Status(410) => match cal.insert(&body)? {
                Reply::Ok(ev) => ev,
                Reply::Status(s) => return Err(unexpected(s)),
            },
            Reply::Status(s) => return Err(unexpected(s)),
        },
    };
    // 이 etag 로 돌아오는 변경은 내가 보낸 것(에코)이므로 받을 때 무시된다
    set_task_etag(&db.conn(), &task.id, saved.etag.as_deref())?;
    Ok(false)
}

// ───────────────────────── 받기 ─────────────────────────

fn pull(db: &Db, cal: &Calendar) -> AppResult<bool> {
    let token = repo::get_setting(&db.conn(), KEY_SYNC_TOKEN)?.filter(|t| !t.is_empty());
    let full = token.is_none();
    let mut page: Option<String> = None;
    let mut changed = false;
    loop {
        match cal.list(token.as_deref(), page.as_deref())? {
            // 동기화 토큰 만료 → 처음부터 전체 동기화
            Reply::Status(410) if !full => {
                log::info!("syncToken 만료(410) → 전체 동기화");
                repo::put_setting(&db.conn(), KEY_SYNC_TOKEN, "")?;
                return pull(db, cal);
            }
            Reply::Status(s) => return Err(unexpected(s)),
            Reply::Ok(p) => {
                for ev in &p.items {
                    changed |= apply_remote(&db.conn(), ev)?;
                }
                match p.next_page_token {
                    Some(next) => page = Some(next),
                    None => {
                        if let Some(sync) = p.next_sync_token {
                            repo::put_setting(&db.conn(), KEY_SYNC_TOKEN, &sync)?;
                        }
                        break;
                    }
                }
            }
        }
    }
    if full {
        log::info!("구글 캘린더 전체 동기화 완료");
    }
    Ok(changed)
}

/// 달력에 함께 보여줄 구글 일정(앱이 만든 일정 제외)을 3개월 전 ~ 12개월 뒤 범위로 새로 받는다 (F-24)
fn refresh_display_events(db: &Db, cal: &Calendar) -> AppResult<bool> {
    let now = Utc::now();
    let fmt = |d: chrono::DateTime<Utc>| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let (min, max) = (fmt(now - chrono::Duration::days(92)), fmt(now + chrono::Duration::days(366)));
    let mut events = Vec::new();
    let mut page: Option<String> = None;
    loop {
        match cal.list_range(&min, &max, page.as_deref())? {
            Reply::Status(s) => return Err(unexpected(s)),
            Reply::Ok(p) => {
                events.extend(
                    p.items.iter().filter(|e| e.app_task_id().is_none() && !e.is_cancelled()).filter_map(|e| e.to_display()),
                );
                match p.next_page_token {
                    Some(next) => page = Some(next),
                    None => break,
                }
            }
        }
    }
    repo::replace_google_events(&db.conn(), &events)
}

// ───────────────────────── 로그인 / 로그아웃 ─────────────────────────

/// 로그인 직후: 처음부터 다시 받고, 날짜 있는 할 일을 모두 올린다
pub fn on_login(db: &Db) -> AppResult<()> {
    let conn = db.conn();
    repo::put_setting(&conn, KEY_SYNC_TOKEN, "")?;
    let n = outbox::enqueue_all_dated(&conn)?;
    log::info!("로그인 후 올릴 할 일 {n}개");
    Ok(())
}

/// 로그아웃: 동기화 상태를 모두 지운다 (할 일은 그대로)
pub fn on_logout(db: &Db) -> AppResult<()> {
    let conn = db.conn();
    conn.execute_batch("DELETE FROM outbox; DELETE FROM google_events; UPDATE tasks SET google_etag = NULL;")?;
    for key in [KEY_SYNC_TOKEN, KEY_LAST_SYNC, KEY_LAST_ERROR] {
        repo::put_setting(&conn, key, "")?;
    }
    Ok(())
}
