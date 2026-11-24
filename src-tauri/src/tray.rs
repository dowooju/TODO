//! 시스템 트레이와 메인 창 수명 관리 (F-21)
//!
//! 창을 닫으면 숨기지 않고 WebView 까지 완전히 없앤다. 숨기기만 하면 WebView2 가 살아 있어
//! 60~120MB 를 계속 쓰지만, 없애면 Rust 프로세스만 남아 10~25MB 정도만 쓴다.
//! 데이터와 상태는 모두 Rust(SQLite)에 있으므로 창을 다시 만들어도 잃는 것이 없다.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindowBuilder};

pub const MAIN_WINDOW: &str = "main";

/// 메인 창을 앞으로 띄운다. 없으면 tauri.conf.json 의 창 설정으로 새로 만든다.
pub fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == MAIN_WINDOW).cloned() else {
        log::error!("메인 창 설정을 찾을 수 없습니다");
        return;
    };
    match WebviewWindowBuilder::from_config(app, &config).and_then(|b| b.visible(true).build()) {
        Ok(win) => {
            let _ = win.set_focus();
        }
        Err(e) => log::error!("메인 창을 만들지 못했습니다: {e}"),
    }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "내 달력 열기", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("내 달력")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => {
                log::info!("트레이 메뉴에서 종료");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 트레이 아이콘을 왼쪽 클릭하면 창을 연다
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
