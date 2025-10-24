#[cfg(target_os = "macos")]
use window_vibrancy::apply_vibrancy;
#[cfg(all(desktop, target_os = "macos"))]
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      #[cfg(desktop)]
      {
        app.handle().plugin(tauri_plugin_positioner::init());

        tauri::tray::TrayIconBuilder::new()
          .on_tray_icon_event(|tray, evt| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &evt);
          })
          .build(app)?;
      }
      #[cfg(all(desktop, target_os = "macos"))]
      {
        if let Some(win) = app.get_webview_window("notch") {
          // Apply macOS vibrancy to the notch window
          let _ = apply_vibrancy(
            win,
            window_vibrancy::NSVisualEffectMaterial::HudWindow,
            None,
            None,
          );
        }
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
