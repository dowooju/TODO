//! 할 일 / 태그 / 설정 저장소. 모든 함수는 `&Connection` 을 받으므로
//! 트랜잭션(`Transaction` 은 `Connection` 으로 역참조됨) 안에서도 그대로 쓸 수 있다.

use chrono::{NaiveDate, NaiveTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{Settings, Tag, TagInput, Task, TaskInput};

pub const TAG_COLORS: &[&str] = &["purple", "yellow", "sky", "orange", "green", "pink", "teal", "gray"];

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

// ───────────────────────── 검증 ─────────────────────────

pub fn normalize_date(date: Option<String>) -> AppResult<Option<String>> {
    match date.map(|d| d.trim().to_string()).filter(|d| !d.is_empty()) {
        None => Ok(None),
        Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map(|v| Some(v.format("%Y-%m-%d").to_string()))
            .map_err(|_| AppError::Invalid(format!("날짜 형식이 올바르지 않습니다: {d}"))),
    }
}

pub fn normalize_time(time: Option<String>) -> AppResult<Option<String>> {
    match time.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(t) => NaiveTime::parse_from_str(&t, "%H:%M")
            .map(|v| Some(v.format("%H:%M").to_string()))
            .map_err(|_| AppError::Invalid(format!("시간 형식이 올바르지 않습니다: {t}"))),
    }
}

fn normalize_title(title: &str) -> AppResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(AppError::Invalid("제목을 입력해 주세요.".into()));
    }
    Ok(t.to_string())
}

fn normalize_tag_id(conn: &Connection, tag_id: Option<String>) -> AppResult<Option<String>> {
    match tag_id.filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(id) => {
            let exists: bool =
                conn.query_row("SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)", [&id], |r| r.get(0))?;
            if exists {
                Ok(Some(id))
            } else {
                Err(AppError::NotFound(format!("태그 {id}")))
            }
        }
    }
}

// ───────────────────────── 할 일 ─────────────────────────

const TASK_COLUMNS: &str =
    "id, title, description, date, due_time, tag_id, done, sort_order, created_at, updated_at";

fn row_to_task(r: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        title: r.get(1)?,
        description: r.get(2)?,
        date: r.get(3)?,
        due_time: r.get(4)?,
        tag_id: r.get(5)?,
        done: r.get(6)?,
        sort_order: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

pub fn list_tasks(conn: &Connection) -> AppResult<Vec<Task>> {
    let mut stmt = conn.prepare(&format!("SELECT {TASK_COLUMNS} FROM tasks ORDER BY sort_order"))?;
    let rows = stmt.query_map([], row_to_task)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn find_task(conn: &Connection, id: &str) -> AppResult<Option<Task>> {
    Ok(conn
        .query_row(&format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1"), [id], row_to_task)
        .optional()?)
}

pub fn get_task(conn: &Connection, id: &str) -> AppResult<Task> {
    find_task(conn, id)?.ok_or_else(|| AppError::NotFound(format!("할 일 {id}")))
}

pub fn create_task(conn: &Connection, input: TaskInput) -> AppResult<Task> {
    let title = normalize_title(&input.title)?;
    let date = normalize_date(input.date)?;
    // 날짜 없는 할 일에는 마감 시간이 의미가 없다
    let due_time = if date.is_some() { normalize_time(input.due_time)? } else { None };
    let tag_id = normalize_tag_id(conn, input.tag_id)?;
    let id = Uuid::new_v4().to_string();
    let ts = now();
    let next_order: i64 =
        conn.query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tasks", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO tasks (id, title, description, date, due_time, tag_id, done, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8, ?8)",
        params![id, title, input.description, date, due_time, tag_id, next_order, ts],
    )?;
    get_task(conn, &id)
}

pub fn update_task(conn: &Connection, id: &str, input: TaskInput) -> AppResult<Task> {
    get_task(conn, id)?;
    let title = normalize_title(&input.title)?;
    let date = normalize_date(input.date)?;
    let due_time = if date.is_some() { normalize_time(input.due_time)? } else { None };
    let tag_id = normalize_tag_id(conn, input.tag_id)?;
    conn.execute(
        "UPDATE tasks SET title = ?2, description = ?3, date = ?4, due_time = ?5, tag_id = ?6, updated_at = ?7
         WHERE id = ?1",
        params![id, title, input.description, date, due_time, tag_id, now()],
    )?;
    get_task(conn, id)
}

pub fn set_task_done(conn: &Connection, id: &str, done: bool) -> AppResult<Task> {
    let n = conn.execute(
        "UPDATE tasks SET done = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, done, now()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound(format!("할 일 {id}")));
    }
    get_task(conn, id)
}

/// 날짜만 바꾼다. 날짜를 지우면 "언젠가 할 일"로 돌아가고 마감 시간도 지운다 (F-11, F-12~F-14)
pub fn set_task_date(conn: &Connection, id: &str, date: Option<String>) -> AppResult<Task> {
    let date = normalize_date(date)?;
    let n = if date.is_some() {
        conn.execute("UPDATE tasks SET date = ?2, updated_at = ?3 WHERE id = ?1", params![id, date, now()])?
    } else {
        conn.execute(
            "UPDATE tasks SET date = NULL, due_time = NULL, updated_at = ?2 WHERE id = ?1",
            params![id, now()],
        )?
    };
    if n == 0 {
        return Err(AppError::NotFound(format!("할 일 {id}")));
    }
    get_task(conn, id)
}

pub fn delete_task(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    Ok(())
}

// ───────────────────────── 태그 ─────────────────────────

fn validate_tag(input: &TagInput) -> AppResult<(String, String)> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("태그 이름을 입력해 주세요.".into()));
    }
    if !TAG_COLORS.contains(&input.color.as_str()) {
        return Err(AppError::Invalid(format!("지원하지 않는 색상입니다: {}", input.color)));
    }
    Ok((name.to_string(), input.color.clone()))
}

fn row_to_tag(r: &Row) -> rusqlite::Result<Tag> {
    Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, sort_order: r.get(3)? })
}

pub fn list_tags(conn: &Connection) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, name, color, sort_order FROM tags ORDER BY sort_order")?;
    let rows = stmt.query_map([], row_to_tag)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn get_tag(conn: &Connection, id: &str) -> AppResult<Tag> {
    conn.query_row("SELECT id, name, color, sort_order FROM tags WHERE id = ?1", [id], row_to_tag)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("태그 {id}")))
}

pub fn create_tag(conn: &Connection, input: TagInput) -> AppResult<Tag> {
    let (name, color) = validate_tag(&input)?;
    let id = Uuid::new_v4().to_string();
    let next_order: i64 =
        conn.query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tags", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO tags (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, color, next_order, now()],
    )?;
    get_tag(conn, &id)
}

pub fn update_tag(conn: &Connection, id: &str, input: TagInput) -> AppResult<Tag> {
    let (name, color) = validate_tag(&input)?;
    let n = conn.execute("UPDATE tags SET name = ?2, color = ?3 WHERE id = ?1", params![id, name, color])?;
    if n == 0 {
        return Err(AppError::NotFound(format!("태그 {id}")));
    }
    get_tag(conn, id)
}

/// 태그를 지우면 그 태그가 붙은 할 일은 태그 없음이 된다 (ON DELETE SET NULL)
pub fn delete_tag(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}

// ───────────────────────── 설정 ─────────────────────────

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
}

pub fn put_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_settings(conn: &Connection) -> AppResult<Settings> {
    let mut s = Settings::default();
    if let Some(v) = get_setting(conn, "notifications_enabled")? {
        s.notifications_enabled = v == "1";
    }
    Ok(s)
}

pub fn save_settings(conn: &Connection, s: &Settings) -> AppResult<()> {
    put_setting(conn, "notifications_enabled", if s.notifications_enabled { "1" } else { "0" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn input(title: &str, date: Option<&str>) -> TaskInput {
        TaskInput {
            title: title.into(),
            description: String::new(),
            date: date.map(Into::into),
            due_time: None,
            tag_id: None,
        }
    }

    #[test]
    fn create_requires_title() {
        let db = Db::open_in_memory().unwrap();
        let err = create_task(&db.conn(), input("   ", None)).unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[test]
    fn someday_task_drops_due_time() {
        let db = Db::open_in_memory().unwrap();
        let mut i = input("언젠가 동생이랑 게임하기", None);
        i.due_time = Some("10:00".into());
        let t = create_task(&db.conn(), i).unwrap();
        assert_eq!(t.date, None);
        assert_eq!(t.due_time, None);
    }

    #[test]
    fn done_keeps_order() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let a = create_task(&c, input("a", Some("2026-11-24"))).unwrap();
        let b = create_task(&c, input("b", Some("2026-11-24"))).unwrap();
        set_task_done(&c, &a.id, true).unwrap();
        let ids: Vec<_> = list_tasks(&c).unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[test]
    fn clearing_date_returns_to_someday() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let mut i = input("과제", Some("2026-11-24"));
        i.due_time = Some("23:59".into());
        let t = create_task(&c, i).unwrap();
        let t = set_task_date(&c, &t.id, None).unwrap();
        assert_eq!((t.date, t.due_time), (None, None));
    }

    #[test]
    fn rejects_bad_date_and_time() {
        let db = Db::open_in_memory().unwrap();
        assert!(create_task(&db.conn(), input("x", Some("2026-13-01"))).is_err());
        let mut i = input("x", Some("2026-11-24"));
        i.due_time = Some("25:00".into());
        assert!(create_task(&db.conn(), i).is_err());
    }

    #[test]
    fn deleting_tag_untags_tasks() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let tag = create_tag(&c, TagInput { name: "독서".into(), color: "pink".into() }).unwrap();
        let mut i = input("책 읽기", None);
        i.tag_id = Some(tag.id.clone());
        let t = create_task(&c, i).unwrap();
        delete_tag(&c, &tag.id).unwrap();
        assert_eq!(get_task(&c, &t.id).unwrap().tag_id, None);
    }

    #[test]
    fn settings_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        assert!(get_settings(&c).unwrap().notifications_enabled);
        save_settings(&c, &Settings { notifications_enabled: false }).unwrap();
        assert!(!get_settings(&c).unwrap().notifications_enabled);
    }
}
