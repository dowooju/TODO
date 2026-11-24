//! SQLite 연결과 스키마 마이그레이션.
//!
//! 마이그레이션은 `PRAGMA user_version` 으로 몇 번째까지 적용됐는지 기록한다.
//! 새 스키마 변경은 `MIGRATIONS` 끝에 추가만 하고, 이미 배포된 항목은 절대 고치지 않는다.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// v1: 태그, 할 일, 설정
const V1_CORE: &str = r#"
CREATE TABLE tags (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    sort_order  INTEGER NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE tasks (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    date         TEXT,              -- YYYY-MM-DD, NULL 이면 "언젠가 할 일"
    due_time     TEXT,              -- HH:MM, date 가 있을 때만 의미 있음
    tag_id       TEXT REFERENCES tags(id) ON DELETE SET NULL,
    done         INTEGER NOT NULL DEFAULT 0,
    sort_order   INTEGER NOT NULL,  -- 생성 순서. 완료해도 바뀌지 않는다 (F-10)
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX idx_tasks_date ON tasks(date);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

-- 시안의 기본 태그 (PRD 4. 디자인 방향)
INSERT INTO tags (id, name, color, sort_order, created_at) VALUES
    ('tag-study',  '공부', 'purple', 1, datetime('now')),
    ('tag-life',   '생활', 'yellow', 2, datetime('now')),
    ('tag-job',    '취업', 'sky',    3, datetime('now')),
    ('tag-hw',     '과제', 'orange', 4, datetime('now')),
    ('tag-health', '운동', 'green',  5, datetime('now'));
"#;

/// v2: 보낸 마감 알림 기록. 같은 마감 시각에 대해 알림을 두 번 보내지 않는다.
/// 마감 시각이 바뀌면(due_at 이 달라지면) 새 알림으로 본다.
const V2_NOTIFICATIONS: &str = r#"
CREATE TABLE sent_notifications (
    task_id  TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    due_at   TEXT NOT NULL,     -- YYYY-MM-DDTHH:MM (로컬 시각)
    sent_at  TEXT NOT NULL,
    PRIMARY KEY (task_id, due_at)
);
"#;

/// v3: 구글 캘린더 동기화
/// - tasks.google_etag: 마지막으로 구글과 주고받은 일정 etag. NULL 이면 구글에 올라가 있지 않음
/// - outbox: 구글로 보낼 변경 대기열 (Transactional Outbox). 할 일 변경과 같은 트랜잭션에서 쌓인다.
///   할 일 하나당 최신 상태만 의미 있으므로 새로 쌓을 때 같은 task_id 의 이전 항목은 지운다.
const V3_GOOGLE_SYNC: &str = r#"
ALTER TABLE tasks ADD COLUMN google_etag TEXT;

CREATE TABLE outbox (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id          TEXT NOT NULL,
    op               TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    created_at       TEXT NOT NULL,
    attempts         INTEGER NOT NULL DEFAULT 0,
    next_attempt_at  TEXT NOT NULL,   -- RFC3339 UTC, 재시도 시각 (지수 백오프)
    last_error       TEXT
);
CREATE INDEX idx_outbox_task ON outbox(task_id);
"#;

const MIGRATIONS: &[&str] = &[V1_CORE, V2_NOTIFICATIONS, V3_GOOGLE_SYNC];

/// 백그라운드 작업(알림, 동기화)과 명령 처리기가 함께 쓰는 DB 핸들
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AppError::Other(e.to_string()))?;
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> AppResult<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        migrate(&mut conn)?;
        Ok(Db(Arc::new(Mutex::new(conn))))
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // 다른 스레드가 패닉으로 잠금을 오염시켜도 데이터는 SQLite 에 있으므로 계속 진행한다
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current = conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = Db::open(&path).unwrap();
            let n: i64 = db.conn().query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0)).unwrap();
            assert_eq!(n, 5);
        }
        // 다시 열어도 마이그레이션이 중복 적용되지 않는다
        let db = Db::open(&path).unwrap();
        let v: i64 = db.conn().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
    }
}
