mod commands;
mod db;
mod error;
mod models;
mod repo;

use tauri::Manager;

use crate::db::Db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // %APPDATA%\<앱 ID>\todo.db — 업데이트·재설치해도 유지된다 (F-29)
            let db_path = app.path().app_data_dir()?.join("todo.db");
            let db = Db::open(&db_path)?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::set_task_done,
            commands::set_task_date,
            commands::delete_task,
            commands::list_tags,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("앱 실행 중 오류가 발생했습니다");
}
