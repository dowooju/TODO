//! 보내기 대기열 (Transactional Outbox)
//!
//! 할 일을 바꾸는 명령은 같은 트랜잭션 안에서 `enqueue_change` 를 불러 대기열에 쌓는다.
//! 그래서 "할 일은 바뀌었는데 구글로 보낼 기록은 없는" 상태가 생기지 않는다 (앱이 꺼져도 다음 실행 때 보냄).

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppResult;
use crate::models::Task;
use crate::repo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Upsert,
    Delete,
}

impl Op {
    fn as_str(self) -> &'static str {
        match self {
            Op::Upsert => "upsert",
            Op::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: i64,
    pub task_id: String,
    pub op: Op,
    pub attempts: i64,
}

pub fn enqueue(conn: &Connection, task_id: &str, op: Op) -> AppResult<()> {
    conn.execute("DELETE FROM outbox WHERE task_id = ?1", [task_id])?;
    let now = repo::now();
    conn.execute(
        "INSERT INTO outbox (task_id, op, created_at, next_attempt_at) VALUES (?1, ?2, ?3, ?3)",
        params![task_id, op.as_str(), now],
    )?;
    Ok(())
}

/// 할 일이 `before` → `after` 로 바뀌었을 때 구글에 무엇을 보내야 하는지 정한다.
/// - 날짜가 있으면 일정 생성/수정 (F-25, F-28 "날짜가 지정되는 순간 등록")
/// - 날짜가 없어졌거나 할 일이 삭제됐는데 예전에 날짜가 있었으면 일정 삭제 (F-28)
/// - 날짜 없는 할 일은 구글에 올리지 않는다 (F-28)
pub fn enqueue_change(conn: &Connection, task_id: &str, before: Option<&Task>, after: Option<&Task>) -> AppResult<()> {
    let had_date = before.is_some_and(|t| t.date.is_some());
    let in_google = has_etag(conn, task_id)?;
    match after {
        Some(t) if t.date.is_some() => enqueue(conn, task_id, Op::Upsert),
        _ if had_date || in_google => enqueue(conn, task_id, Op::Delete),
        _ => Ok(()),
    }
}

fn has_etag(conn: &Connection, task_id: &str) -> AppResult<bool> {
    Ok(conn
        .query_row("SELECT google_etag IS NOT NULL FROM tasks WHERE id = ?1", [task_id], |r| r.get(0))
        .optional()?
        .unwrap_or(false))
}

/// 지금 보낼 차례가 된 항목들 (오래된 순)
pub fn due(conn: &Connection, now: DateTime<Utc>) -> AppResult<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, op, attempts FROM outbox WHERE next_attempt_at <= ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)], |r| {
        let op: String = r.get(2)?;
        Ok(Item {
            id: r.get(0)?,
            task_id: r.get(1)?,
            op: if op == "delete" { Op::Delete } else { Op::Upsert },
            attempts: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// 가장 이른 재시도 시각
pub fn next_attempt(conn: &Connection) -> AppResult<Option<DateTime<Utc>>> {
    let v: Option<String> = conn.query_row("SELECT MIN(next_attempt_at) FROM outbox", [], |r| r.get(0))?;
    Ok(v.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&Utc)))
}

pub fn pending_count(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0))?)
}

pub fn has_pending(conn: &Connection, task_id: &str) -> AppResult<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM outbox WHERE task_id = ?1)", [task_id], |r| r.get(0))?)
}

/// 처리 완료. 처리하는 동안 같은 할 일에 새 변경이 쌓였으면 그 항목은 id 가 달라서 남는다.
pub fn done(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM outbox WHERE id = ?1", [id])?;
    Ok(())
}

pub fn drop_for_task(conn: &Connection, task_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM outbox WHERE task_id = ?1", [task_id])?;
    Ok(())
}

/// 재시도 간격: 5초, 10초, 20초 … 최대 30분 (지수 백오프)
pub fn backoff_secs(attempts: i64) -> i64 {
    (5_i64 << attempts.clamp(0, 20)).min(30 * 60)
}

pub fn retry_later(conn: &Connection, item: &Item, error: &str, now: DateTime<Utc>) -> AppResult<()> {
    let attempts = item.attempts + 1;
    let next = now + chrono::Duration::seconds(backoff_secs(attempts - 1));
    conn.execute(
        "UPDATE outbox SET attempts = ?2, next_attempt_at = ?3, last_error = ?4 WHERE id = ?1",
        params![item.id, attempts, next.to_rfc3339_opts(chrono::SecondsFormat::Millis, true), error],
    )?;
    Ok(())
}

/// 로그인 직후: 날짜가 있는 모든 할 일을 구글에 올리도록 쌓는다 (멱등: 일정 ID 가 할 일 ID 에서 정해짐)
pub fn enqueue_all_dated(conn: &Connection) -> AppResult<usize> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE date IS NOT NULL")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    for id in &ids {
        enqueue(conn, id, Op::Upsert)?;
    }
    Ok(ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::models::TaskInput;

    fn mk(conn: &Connection, date: Option<&str>) -> Task {
        repo::create_task(
            conn,
            TaskInput { title: "x".into(), description: String::new(), date: date.map(Into::into), due_time: None, tag_id: None },
        )
        .unwrap()
    }

    fn ops(conn: &Connection) -> Vec<(String, Op)> {
        due(conn, Utc::now() + chrono::Duration::days(1))
            .unwrap()
            .into_iter()
            .map(|i| (i.task_id, i.op))
            .collect()
    }

    #[test]
    fn someday_task_is_not_uploaded() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, None);
        enqueue_change(&c, &t.id, None, Some(&t)).unwrap();
        assert!(ops(&c).is_empty());
    }

    #[test]
    fn scheduling_uploads_and_unscheduling_deletes() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, None);
        let scheduled = repo::set_task_date(&c, &t.id, Some("2026-11-28".into())).unwrap();
        enqueue_change(&c, &t.id, Some(&t), Some(&scheduled)).unwrap();
        assert_eq!(ops(&c), vec![(t.id.clone(), Op::Upsert)]);

        let back = repo::set_task_date(&c, &t.id, None).unwrap();
        enqueue_change(&c, &t.id, Some(&scheduled), Some(&back)).unwrap();
        // 최신 상태만 남는다
        assert_eq!(ops(&c), vec![(t.id.clone(), Op::Delete)]);
    }

    #[test]
    fn deleting_dated_task_deletes_event() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"));
        repo::delete_task(&c, &t.id).unwrap();
        enqueue_change(&c, &t.id, Some(&t), None).unwrap();
        assert_eq!(ops(&c), vec![(t.id, Op::Delete)]);
    }

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_secs(0), 5);
        assert_eq!(backoff_secs(1), 10);
        assert_eq!(backoff_secs(3), 40);
        assert_eq!(backoff_secs(50), 30 * 60);
    }

    #[test]
    fn retry_postpones_item() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = mk(&c, Some("2026-11-24"));
        enqueue(&c, &t.id, Op::Upsert).unwrap();
        let now = Utc::now();
        let item = due(&c, now + chrono::Duration::seconds(1)).unwrap().remove(0);
        retry_later(&c, &item, "offline", now).unwrap();
        assert!(due(&c, now + chrono::Duration::seconds(1)).unwrap().is_empty());
        assert_eq!(due(&c, now + chrono::Duration::seconds(6)).unwrap()[0].attempts, 1);
    }
}
