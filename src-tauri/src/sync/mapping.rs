//! 앱 할 일 ↔ 구글 일정 변환과, 구글에서 받은 변경을 로컬에 반영하는 규칙

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::models::Task;
use crate::repo;
use crate::sync::outbox;

/// 완료한 할 일은 구글 일정 제목 앞에 붙여 표시하고, 받을 때 다시 완료 상태로 읽는다
pub const DONE_PREFIX: &str = "✅ ";
/// 마감 시간이 있는 할 일은 "마감 30분 전 ~ 마감" 일정으로 올린다
const TIMED_EVENT_MINUTES: i64 = 30;
const TIME_ZONE: &str = "Asia/Seoul";

/// 할 일 UUID(16바이트)를 base32hex(0-9a-v) 소문자로 바꿔 구글 일정 ID 로 쓴다 (26자).
/// 일정 ID 를 우리가 정하므로 같은 생성 요청이 재시도로 두 번 가도 409 로 끝나 중복 일정이 생기지 않는다.
pub fn event_id(task_id: &str) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";
    let bytes: Vec<u8> = match uuid::Uuid::parse_str(task_id) {
        Ok(u) => u.as_bytes().to_vec(),
        Err(_) => task_id.as_bytes().to_vec(), // UUID 가 아닌 ID (이론상 없음)
    };
    let mut out = String::new();
    let (mut buffer, mut bits) = (0u32, 0u32);
    for b in bytes {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn local_rfc3339(naive: NaiveDateTime) -> String {
    Local
        .from_local_datetime(&naive)
        .earliest()
        .unwrap_or_else(|| Local.from_utc_datetime(&naive))
        .to_rfc3339()
}

/// 구글 Calendar API 일정 본문
pub fn event_body(task: &Task) -> Value {
    let date = task.date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let (start, end) = match (date, task.due_time.as_deref().and_then(|t| NaiveTime::parse_from_str(t, "%H:%M").ok())) {
        (Some(d), Some(t)) => {
            let due = d.and_time(t);
            (
                json!({ "dateTime": local_rfc3339(due - Duration::minutes(TIMED_EVENT_MINUTES)), "timeZone": TIME_ZONE }),
                json!({ "dateTime": local_rfc3339(due), "timeZone": TIME_ZONE }),
            )
        }
        // 날짜만 있으면 종일 일정. 끝 날짜는 포함되지 않으므로 다음 날 ([start, end))
        (Some(d), None) => (
            json!({ "date": d.format("%Y-%m-%d").to_string() }),
            json!({ "date": (d + Duration::days(1)).format("%Y-%m-%d").to_string() }),
        ),
        (None, _) => (Value::Null, Value::Null),
    };
    let summary = if task.done { format!("{DONE_PREFIX}{}", task.title) } else { task.title.clone() };
    json!({
        "id": event_id(&task.id),
        "summary": summary,
        "description": task.description,
        "start": start,
        "end": end,
        "status": "confirmed",
        "extendedProperties": { "private": { "appTaskId": task.id } },
    })
}

// ───────────────────────── 구글 → 앱 ─────────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EventTime {
    pub date: Option<String>,
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Private {
    #[serde(rename = "appTaskId")]
    pub app_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtendedProperties {
    pub private: Option<Private>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Event {
    pub id: String,
    pub etag: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: Option<EventTime>,
    pub end: Option<EventTime>,
    pub updated: Option<String>,
    #[serde(rename = "extendedProperties")]
    pub extended_properties: Option<ExtendedProperties>,
    #[serde(rename = "htmlLink")]
    pub html_link: Option<String>,
}

impl Event {
    pub fn app_task_id(&self) -> Option<&str> {
        self.extended_properties.as_ref()?.private.as_ref()?.app_task_id.as_deref()
    }

    pub fn is_cancelled(&self) -> bool {
        self.status.as_deref() == Some("cancelled")
    }

    pub fn updated_at(&self) -> Option<DateTime<chrono::Utc>> {
        DateTime::parse_from_rfc3339(self.updated.as_deref()?).ok().map(|d| d.with_timezone(&chrono::Utc))
    }

    /// 달력 표시용으로 바꾼다. 종일 일정의 end.date 는 포함되지 않으므로 하루 뺀다.
    pub fn to_display(&self) -> Option<crate::models::GoogleEvent> {
        let (start, end) = (self.start.as_ref()?, self.end.as_ref()?);
        let local = |s: &str| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Local));
        let (start_date, end_date, start_time, end_time) = match (&start.date, &end.date) {
            (Some(s), Some(e)) => {
                let last = NaiveDate::parse_from_str(e, "%Y-%m-%d").ok()? - Duration::days(1);
                (s.clone(), last.format("%Y-%m-%d").to_string().max(s.clone()), None, None)
            }
            _ => {
                let (s, e) = (local(start.date_time.as_deref()?)?, local(end.date_time.as_deref()?)?);
                // 자정에 끝나는 일정은 전날까지로 본다
                let e_day = if e.format("%H:%M").to_string() == "00:00" && e > s { e - Duration::minutes(1) } else { e };
                (
                    s.format("%Y-%m-%d").to_string(),
                    e_day.format("%Y-%m-%d").to_string(),
                    Some(s.format("%H:%M").to_string()),
                    Some(e.format("%H:%M").to_string()),
                )
            }
        };
        Some(crate::models::GoogleEvent {
            id: self.id.clone(),
            summary: self.summary.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "(제목 없음)".into()),
            start_date,
            end_date,
            start_time,
            end_time,
            html_link: self.html_link.clone(),
        })
    }

    /// (날짜, 마감 시간). 시간 일정은 끝 시각을 마감으로 본다.
    pub fn date_and_time(&self) -> Option<(String, Option<String>)> {
        let start = self.start.as_ref()?;
        if let Some(d) = &start.date {
            return Some((d.clone(), None));
        }
        let end = self.end.as_ref().and_then(|e| e.date_time.as_deref()).or(start.date_time.as_deref())?;
        let local = DateTime::parse_from_rfc3339(end).ok()?.with_timezone(&Local);
        Some((local.format("%Y-%m-%d").to_string(), Some(local.format("%H:%M").to_string())))
    }
}

pub fn task_etag(conn: &Connection, task_id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT google_etag FROM tasks WHERE id = ?1", [task_id], |r| r.get::<_, Option<String>>(0))
        .optional()?
        .flatten())
}

pub fn set_task_etag(conn: &Connection, task_id: &str, etag: Option<&str>) -> AppResult<()> {
    conn.execute("UPDATE tasks SET google_etag = ?2 WHERE id = ?1", params![task_id, etag])?;
    Ok(())
}

fn parse_utc(s: &str) -> Option<DateTime<chrono::Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&chrono::Utc))
}

/// 구글에서 받은 일정 하나를 로컬 할 일에 반영한다. 바뀐 것이 있으면 true.
///
/// - 앱이 만든 일정(extendedProperties.private.appTaskId)만 다룬다. 나머지는 표시용 일정.
/// - 내가 보낸 변경이 그대로 돌아온 것(etag 같음)은 무시한다 (에코)
/// - 구글에서 삭제됨: 로컬 할 일도 삭제 (F-27). 단, 날짜를 지워 우리가 일정을 지운 경우와
///   아직 보내지 않은 로컬 변경이 있는 경우는 제외
/// - 수정됨: Last-Write-Wins. 구글 쪽 updated 가 로컬 updated_at 보다 늦을 때만 반영
pub fn apply_remote(conn: &Connection, ev: &Event) -> AppResult<bool> {
    let Some(task_id) = ev.app_task_id() else { return Ok(false) };
    let Some(task) = repo::find_task(conn, task_id)? else { return Ok(false) };
    let etag = task_etag(conn, task_id)?;
    if ev.etag.is_some() && ev.etag == etag {
        return Ok(false);
    }
    let pending = outbox::has_pending(conn, task_id)?;

    if ev.is_cancelled() {
        if task.date.is_none() {
            // 날짜를 해제해서 우리가 지운 일정이 돌아온 것
            set_task_etag(conn, task_id, None)?;
            return Ok(false);
        }
        if pending {
            return Ok(false); // 로컬 변경이 곧 일정을 되살린다
        }
        log::info!("구글에서 삭제된 일정 → 할 일 삭제: {task_id}");
        repo::delete_task(conn, task_id)?;
        return Ok(true);
    }

    let remote_newer = match (ev.updated_at(), parse_utc(&task.updated_at)) {
        (Some(r), Some(l)) => r > l,
        _ => true,
    };
    if pending && !remote_newer {
        return Ok(false); // 로컬이 더 최신. 곧 보내서 덮어쓴다
    }

    let raw_title = ev.summary.clone().unwrap_or_default();
    let (done, title) = match raw_title.strip_prefix(DONE_PREFIX) {
        Some(rest) => (true, rest.to_string()),
        None => (false, raw_title),
    };
    let title = if title.trim().is_empty() { task.title.clone() } else { title };
    let (date, due_time) = match ev.date_and_time() {
        Some((d, t)) => (Some(d), t),
        None => (task.date.clone(), task.due_time.clone()),
    };
    conn.execute(
        "UPDATE tasks SET title = ?2, description = ?3, date = ?4, due_time = ?5, done = ?6,
                          updated_at = ?7, google_etag = ?8
         WHERE id = ?1",
        params![
            task_id,
            title,
            ev.description.clone().unwrap_or_default(),
            date,
            due_time,
            done,
            ev.updated.clone().unwrap_or_else(repo::now),
            ev.etag,
        ],
    )?;
    outbox::drop_for_task(conn, task_id)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::models::TaskInput;

    fn mk(conn: &Connection, date: Option<&str>, time: Option<&str>) -> Task {
        repo::create_task(
            conn,
            TaskInput {
                title: "알고리즘 과제 제출하기".into(),
                description: "백준 3문제".into(),
                date: date.map(Into::into),
                due_time: time.map(Into::into),
                tag_id: None,
            },
        )
        .unwrap()
    }

    fn remote(task: &Task, etag: &str, updated: &str) -> Event {
        Event {
            id: event_id(&task.id),
            etag: Some(etag.into()),
            status: Some("confirmed".into()),
            summary: Some(task.title.clone()),
            description: Some(task.description.clone()),
            start: Some(EventTime { date: task.date.clone(), date_time: None }),
            end: None,
            updated: Some(updated.into()),
            extended_properties: Some(ExtendedProperties {
                private: Some(Private { app_task_id: Some(task.id.clone()) }),
            }),
            html_link: None,
        }
    }

    #[test]
    fn event_id_is_valid_base32hex() {
        let id = event_id("7f1c1d2e-3b4a-4c5d-8e9f-0a1b2c3d4e5f");
        assert_eq!(id.len(), 26);
        assert!(id.chars().all(|c| c.is_ascii_digit() || ('a'..='v').contains(&c)));
        assert_eq!(id, event_id("7f1c1d2e-3b4a-4c5d-8e9f-0a1b2c3d4e5f"));
    }

    #[test]
    fn all_day_event_ends_next_day() {
        let db = Db::open_in_memory().unwrap();
        let t = mk(&db.conn(), Some("2026-11-30"), None);
        let body = event_body(&t);
        assert_eq!(body["start"]["date"], "2026-11-30");
        assert_eq!(body["end"]["date"], "2026-12-01");
        assert_eq!(body["extendedProperties"]["private"]["appTaskId"], t.id);
    }

    #[test]
    fn timed_event_ends_at_due_time_and_roundtrips() {
        let db = Db::open_in_memory().unwrap();
        let t = mk(&db.conn(), Some("2026-11-24"), Some("23:59"));
        let body = event_body(&t);
        assert_eq!(body["end"]["timeZone"], "Asia/Seoul");
        let ev: Event = serde_json::from_value(body).unwrap();
        assert_eq!(ev.date_and_time(), Some(("2026-11-24".into(), Some("23:59".into()))));
    }

    #[test]
    fn all_day_display_range_is_inclusive() {
        let ev = Event {
            id: "g1".into(),
            summary: Some("제주 여행".into()),
            start: Some(EventTime { date: Some("2026-11-27".into()), date_time: None }),
            end: Some(EventTime { date: Some("2026-11-30".into()), date_time: None }),
            ..Default::default()
        };
        let d = ev.to_display().unwrap();
        assert_eq!((d.start_date.as_str(), d.end_date.as_str(), d.start_time), ("2026-11-27", "2026-11-29", None));
    }

    #[test]
    fn echo_is_ignored() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"), None);
        set_task_etag(&c, &t.id, Some("\"e1\"")).unwrap();
        let mut ev = remote(&t, "\"e1\"", "2099-01-01T00:00:00Z");
        ev.summary = Some("바뀐 제목".into());
        assert!(!apply_remote(&c, &ev).unwrap());
        assert_eq!(repo::get_task(&c, &t.id).unwrap().title, t.title);
    }

    #[test]
    fn newer_remote_edit_wins_and_done_prefix_is_read() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"), None);
        outbox::enqueue(&c, &t.id, outbox::Op::Upsert).unwrap();
        let mut ev = remote(&t, "\"e2\"", "2099-01-01T00:00:00Z");
        ev.summary = Some(format!("{DONE_PREFIX}과제 제출 (수정)"));
        ev.start = Some(EventTime { date: Some("2026-11-26".into()), date_time: None });
        assert!(apply_remote(&c, &ev).unwrap());

        let after = repo::get_task(&c, &t.id).unwrap();
        assert_eq!((after.title.as_str(), after.done, after.date.as_deref()), ("과제 제출 (수정)", true, Some("2026-11-26")));
        assert!(!outbox::has_pending(&c, &t.id).unwrap());
    }

    #[test]
    fn older_remote_edit_loses_to_pending_local_change() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"), None);
        outbox::enqueue(&c, &t.id, outbox::Op::Upsert).unwrap();
        let mut ev = remote(&t, "\"e3\"", "2000-01-01T00:00:00Z");
        ev.summary = Some("오래된 제목".into());
        assert!(!apply_remote(&c, &ev).unwrap());
        assert_eq!(repo::get_task(&c, &t.id).unwrap().title, t.title);
    }

    #[test]
    fn remote_delete_removes_task() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"), None);
        let mut ev = remote(&t, "\"e4\"", "2099-01-01T00:00:00Z");
        ev.status = Some("cancelled".into());
        assert!(apply_remote(&c, &ev).unwrap());
        assert!(repo::find_task(&c, &t.id).unwrap().is_none());
    }

    #[test]
    fn our_own_unschedule_delete_keeps_someday_task() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, None, None);
        set_task_etag(&c, &t.id, Some("\"old\"")).unwrap();
        let mut ev = remote(&t, "\"e5\"", "2099-01-01T00:00:00Z");
        ev.status = Some("cancelled".into());
        assert!(!apply_remote(&c, &ev).unwrap());
        assert!(repo::find_task(&c, &t.id).unwrap().is_some());
        assert_eq!(task_etag(&c, &t.id).unwrap(), None);
    }
}
