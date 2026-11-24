//! 마감 알림 스케줄러 (F-19 ~ F-22)
//!
//! `sleep` 으로 "마감 10분 전까지" 기다리는 방식은 쓰지 않는다. 그런 타이머는 단조 시계(monotonic
//! clock) 기준이라 PC 가 절전 모드에 들어가면 멈춰서 알림이 밀리거나 빠진다.
//! 대신 30초마다 현재 벽시계 시각과 "마감 − 10분" 을 비교하고, 보낸 알림은 DB 에 기록한다.

use std::thread;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Local, NaiveDateTime};
use rusqlite::{params, Connection};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::db::Db;
use crate::error::AppResult;
use crate::models::Task;
use crate::repo;

/// MVP 는 마감 10분 전 고정 (PRD 8. TODO: 사용자 지정은 이후)
pub const LEAD_MINUTES: i64 = 10;
const TICK: Duration = Duration::from_secs(30);

fn due_at(task: &Task) -> Option<NaiveDateTime> {
    let date = task.date.as_deref()?;
    let time = task.due_time.as_deref()?;
    NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M").ok()
}

/// 지금 알림을 보내야 하는 할 일. `[마감 − 10분, 마감)` 구간 안에 있고, 완료되지 않았고,
/// 이 마감 시각으로 아직 알림을 보내지 않은 것. 마감이 이미 지났으면 보내지 않는다.
pub fn pending(conn: &Connection, now: NaiveDateTime) -> AppResult<Vec<(Task, String)>> {
    let mut out = Vec::new();
    for task in repo::list_tasks(conn)? {
        if task.done {
            continue; // F-22
        }
        let Some(due) = due_at(&task) else { continue };
        if now < due - ChronoDuration::minutes(LEAD_MINUTES) || now >= due {
            continue;
        }
        let key = due.format("%Y-%m-%dT%H:%M").to_string();
        let sent: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sent_notifications WHERE task_id = ?1 AND due_at = ?2)",
            params![task.id, key],
            |r| r.get(0),
        )?;
        if !sent {
            out.push((task, key));
        }
    }
    Ok(out)
}

pub fn mark_sent(conn: &Connection, task_id: &str, due_key: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sent_notifications (task_id, due_at, sent_at) VALUES (?1, ?2, ?3)",
        params![task_id, due_key, repo::now()],
    )?;
    Ok(())
}

fn tick(app: &AppHandle, db: &Db) -> AppResult<()> {
    let due = {
        let conn = db.conn();
        if !repo::get_settings(&conn)?.notifications_enabled {
            return Ok(()); // F-20
        }
        pending(&conn, Local::now().naive_local())?
    };
    for (task, key) in due {
        let body = format!("{} 마감 {}분 전이에요", task.due_time.as_deref().unwrap_or(""), LEAD_MINUTES);
        match app.notification().builder().title(&task.title).body(body).show() {
            Ok(()) => mark_sent(&db.conn(), &task.id, &key)?,
            Err(e) => log::warn!("알림 표시 실패 ({}): {e}", task.id),
        }
    }
    Ok(())
}

/// 앱이 살아 있는 동안(창을 닫아 트레이에 있어도) 계속 도는 백그라운드 스레드
pub fn spawn(app: AppHandle, db: Db) {
    thread::Builder::new()
        .name("notifier".into())
        .spawn(move || loop {
            if let Err(e) = tick(&app, &db) {
                log::error!("알림 확인 실패: {e}");
            }
            thread::sleep(TICK);
        })
        .expect("알림 스레드를 시작하지 못했습니다");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskInput;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn add(conn: &Connection, title: &str, date: &str, time: Option<&str>) -> Task {
        repo::create_task(
            conn,
            TaskInput {
                title: title.into(),
                description: String::new(),
                date: Some(date.into()),
                due_time: time.map(Into::into),
                tag_id: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn fires_only_inside_window_and_once() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = add(&c, "알고리즘 과제 제출", "2026-11-24", Some("23:59"));
        add(&c, "시간 없는 할 일", "2026-11-24", None);

        assert!(pending(&c, at("2026-11-24 23:48")).unwrap().is_empty());
        let due = pending(&c, at("2026-11-24 23:49")).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0.id, t.id);

        mark_sent(&c, &t.id, &due[0].1).unwrap();
        assert!(pending(&c, at("2026-11-24 23:55")).unwrap().is_empty());
        // 마감이 지나면 보내지 않는다 (절전에서 늦게 깨어난 경우)
        assert!(pending(&c, at("2026-11-24 23:59")).unwrap().is_empty());
    }

    #[test]
    fn done_tasks_are_skipped() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = add(&c, "운동", "2026-11-24", Some("18:00"));
        repo::set_task_done(&c, &t.id, true).unwrap();
        assert!(pending(&c, at("2026-11-24 17:55")).unwrap().is_empty());
    }

    #[test]
    fn changing_due_time_rearms_notification() {
        let db = Db::open_in_memory().unwrap();
        let c = db.conn();
        let t = add(&c, "면접", "2026-11-24", Some("10:00"));
        let (_, key) = pending(&c, at("2026-11-24 09:55")).unwrap().remove(0);
        mark_sent(&c, &t.id, &key).unwrap();

        repo::update_task(
            &c,
            &t.id,
            TaskInput {
                title: "면접".into(),
                description: String::new(),
                date: Some("2026-11-24".into()),
                due_time: Some("11:00".into()),
                tag_id: None,
            },
        )
        .unwrap();
        assert_eq!(pending(&c, at("2026-11-24 10:55")).unwrap().len(), 1);
    }
}
