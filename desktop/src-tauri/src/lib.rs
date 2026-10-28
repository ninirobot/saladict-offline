mod config;
mod dict;
mod history;
mod logging;
mod net;
mod state;
mod tray;
mod win;

pub use state::AppState;

use tauri::{Manager, RunEvent, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        crate::win::popup::show_main(app);
    }));

    builder
        .manage(AppState::new())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            state.init(&handle)?;
            logging::init(&state.dir());

            crate::tray::build(&handle)?;
            crate::win::hook::install(handle.clone());
            crate::win::hotkey::sync(&handle);

            if state.config_snapshot().autostart {
                let _ = crate::win::autostart::set(true);
            }

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
            log::info!("saladict desktop started, version {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                // 关主窗口只是收进托盘
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            state::cmd_bootstrap,
            state::cmd_lookup,
            state::cmd_dict_add,
            state::cmd_dict_remove,
            state::cmd_dict_move,
            state::cmd_dict_toggle,
            state::cmd_dict_refresh,
            state::cmd_config_set,
            state::cmd_autostart_set,
            state::cmd_history_clear,
            state::cmd_history_delete,
            state::cmd_fetch_url,
            state::cmd_popup_hide,
            state::cmd_popup_pin,
            state::cmd_popup_resize,
            state::cmd_popup_take,
            state::cmd_open_entry,
            state::cmd_open_main,
            state::cmd_pick_mdx,
            state::cmd_open_url,
            state::cmd_read_asset,
            state::cmd_log,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    // 托盘常驻：所有窗口关闭后也不退出进程
                    api.prevent_exit();
                }
            }
        });
}
