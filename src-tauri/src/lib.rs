mod commands;
mod db;
mod error;
mod models;
mod notifier;
mod repo;
mod tray;

use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use crate::db::Db;

/// Windows 시작 시 자동 실행될 때 붙는 인자. 이때는 창 없이 트레이에서만 시작한다.
const AUTOSTART_ARG: &str = "--autostart";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // 중복 실행 방지: 두 번째로 실행하면 새 프로세스는 바로 끝나고 기존 창을 앞으로 띄운다.
        // (동기화·알림 워커가 둘이 되어 같은 DB 에 동시에 쓰는 것을 막는다. 가장 먼저 등록해야 한다.)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
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
        // Windows 시작 시 자동 실행 (레지스트리 HKCU\...\Run 에 등록, 관리자 권한 불필요)
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![AUTOSTART_ARG])))
        .setup(|app| {
            // %APPDATA%\<앱 ID>\todo.db — 업데이트·재설치해도 유지된다 (F-29)
            let db_path = app.path().app_data_dir()?.join("todo.db");
            let db = Db::open(&db_path)?;
            log::info!("DB 열림: {}", db_path.display());
            app.manage(db.clone());

            init_autostart(app.handle(), &db);
            tray::setup_tray(app.handle())?;
            notifier::spawn(app.handle().clone(), db);

            // 부팅 시 자동 실행이면 트레이에서 조용히 시작, 직접 실행이면 창을 띄운다
            if !std::env::args().any(|a| a == AUTOSTART_ARG) {
                tray::show_main_window(app.handle());
            } else {
                log::info!("자동 실행: 트레이에서 시작");
            }
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
            commands::get_autostart,
            commands::set_autostart,
        ])
        .build(tauri::generate_context!())
        .expect("앱을 초기화하지 못했습니다");

    app.run(|_app, event| {
        // 창을 모두 닫아도(code 없음) 프로세스는 트레이에 남는다. 트레이 "종료" 는 exit(0) 이라 code 가 있다.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

/// 자동 실행은 기본으로 켠다. 첫 실행 때 한 번만 켜고, 이후에는 사용자가 설정에서 바꾼 값을 존중한다.
fn init_autostart(app: &tauri::AppHandle, db: &Db) {
    // 개발 빌드(target\debug)가 부팅 때마다 뜨면 곤란하므로 설치본에서만 자동 등록한다
    if cfg!(debug_assertions) {
        return;
    }
    let conn = db.conn();
    if matches!(repo::get_setting(&conn, "autostart_initialized"), Ok(Some(_))) {
        return;
    }
    match app.autolaunch().enable() {
        Ok(()) => log::info!("Windows 시작 시 자동 실행 등록"),
        Err(e) => log::warn!("자동 실행 등록 실패: {e}"),
    }
    let _ = repo::put_setting(&conn, "autostart_initialized", "1");
}
