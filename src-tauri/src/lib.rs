mod commands;
mod db;
mod error;
mod models;
mod notifier;
mod repo;

use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use crate::db::Db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 로그: %LOCALAPPDATA%\<앱 ID>\logs, 1MB 씩 최대 5개 (약 5MB)
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([Target::new(TargetKind::LogDir { file_name: None }), Target::new(TargetKind::Stdout)])
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(RotationStrategy::KeepSome(4))
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // %APPDATA%\<앱 ID>\todo.db — 업데이트·재설치해도 유지된다 (F-29)
            let db_path = app.path().app_data_dir()?.join("todo.db");
            let db = Db::open(&db_path)?;
            log::info!("DB 열림: {}", db_path.display());
            app.manage(db.clone());

            notifier::spawn(app.handle().clone(), db);
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
