//! 화면(React)에서 `invoke` 로 부르는 Tauri 명령.
//!
//! 데이터가 바뀌면 `data-changed` 이벤트를 보내서, 열려 있는 창이 목록을 다시 불러오게 한다.
//! (백그라운드 동기화가 데이터를 바꿀 때도 같은 이벤트를 쓴다.)

use tauri::{AppHandle, Emitter, State};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::{Settings, Tag, TagInput, Task, TaskInput};
use crate::repo;

pub const DATA_CHANGED: &str = "data-changed";

pub fn notify_changed(app: &AppHandle) {
    let _ = app.emit(DATA_CHANGED, ());
}

#[tauri::command]
pub fn list_tasks(db: State<'_, Db>) -> AppResult<Vec<Task>> {
    repo::list_tasks(&db.conn())
}

#[tauri::command]
pub fn create_task(app: AppHandle, db: State<'_, Db>, input: TaskInput) -> AppResult<Task> {
    let task = repo::create_task(&db.conn(), input)?;
    notify_changed(&app);
    Ok(task)
}

#[tauri::command]
pub fn update_task(app: AppHandle, db: State<'_, Db>, id: String, input: TaskInput) -> AppResult<Task> {
    let task = repo::update_task(&db.conn(), &id, input)?;
    notify_changed(&app);
    Ok(task)
}

#[tauri::command]
pub fn set_task_done(app: AppHandle, db: State<'_, Db>, id: String, done: bool) -> AppResult<Task> {
    let task = repo::set_task_done(&db.conn(), &id, done)?;
    notify_changed(&app);
    Ok(task)
}

#[tauri::command]
pub fn set_task_date(app: AppHandle, db: State<'_, Db>, id: String, date: Option<String>) -> AppResult<Task> {
    let task = repo::set_task_date(&db.conn(), &id, date)?;
    notify_changed(&app);
    Ok(task)
}

#[tauri::command]
pub fn delete_task(app: AppHandle, db: State<'_, Db>, id: String) -> AppResult<()> {
    repo::delete_task(&db.conn(), &id)?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_tags(db: State<'_, Db>) -> AppResult<Vec<Tag>> {
    repo::list_tags(&db.conn())
}

#[tauri::command]
pub fn create_tag(app: AppHandle, db: State<'_, Db>, input: TagInput) -> AppResult<Tag> {
    let tag = repo::create_tag(&db.conn(), input)?;
    notify_changed(&app);
    Ok(tag)
}

#[tauri::command]
pub fn update_tag(app: AppHandle, db: State<'_, Db>, id: String, input: TagInput) -> AppResult<Tag> {
    let tag = repo::update_tag(&db.conn(), &id, input)?;
    notify_changed(&app);
    Ok(tag)
}

#[tauri::command]
pub fn delete_tag(app: AppHandle, db: State<'_, Db>, id: String) -> AppResult<()> {
    repo::delete_tag(&db.conn(), &id)?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_settings(db: State<'_, Db>) -> AppResult<Settings> {
    repo::get_settings(&db.conn())
}

#[tauri::command]
pub fn save_settings(db: State<'_, Db>, settings: Settings) -> AppResult<Settings> {
    let conn = db.conn();
    repo::save_settings(&conn, &settings)?;
    repo::get_settings(&conn)
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> AppResult<bool> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| AppError::Other(format!("자동 실행 상태 확인 실패: {e}")))
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> AppResult<bool> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| AppError::Other(format!("자동 실행 설정 실패: {e}")))?;
    log::info!("Windows 시작 시 자동 실행: {enabled}");
    launcher.is_enabled().map_err(|e| AppError::Other(e.to_string()))
}
