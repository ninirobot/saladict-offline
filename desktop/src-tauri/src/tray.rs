use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter};

pub const TRAY_ID: &str = "saladict-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "show-main", "打开主窗口", true, None::<&str>)?;
    let clipboard = MenuItem::with_id(app, "clipboard-lookup", "剪贴板取词", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(
        app,
        "pause",
        "暂停划词",
        true,
        crate::win::hook::is_paused(),
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "open-settings", "偏好设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出沙拉划词", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open, &clipboard, &pause, &separator, &settings, &quit],
    )?;

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(menu))?;
    let _ = tray.set_show_menu_on_left_click(true);

    let pause_item = pause.clone();
    tray.on_menu_event(move |app, event| match event.id().0.as_str() {
        "show-main" => crate::win::popup::show_main(app),
        "clipboard-lookup" => {
            if let Some(text) = crate::win::capture::clipboard_text() {
                crate::win::popup::show_for(app, &crate::state::clean_word(&text), "manual", true);
            } else {
                crate::win::popup::show_main(app);
            }
        }
        "pause" => {
            let paused = !crate::win::hook::is_paused();
            crate::win::hook::set_paused(paused);
            let _ = pause_item.set_checked(paused);
            let _ = app.emit("pause-changed", paused);
        }
        "open-settings" => {
            crate::win::popup::show_main(app);
            let _ = app.emit("open-settings", ());
        }
        "quit" => app.exit(0),
        _ => {}
    });

    Ok(())
}
