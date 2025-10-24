#[cfg(target_os = "macos")]
use window_vibrancy::apply_vibrancy;
#[cfg(all(desktop, target_os = "macos"))]
use tauri::Manager;

#[cfg(target_os = "macos")]
fn elevate_to_status_bar(win: &tauri::WebviewWindow) -> tauri::Result<()> {
  use core::ffi::c_void;
  use objc2::runtime::AnyObject;
  use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};

  let raw: *mut c_void = win.ns_window()?;
  if raw.is_null() {
    return Err(tauri::Error::InvalidWindowHandle);
  }

  let obj: *mut AnyObject = raw.cast();
  let ns_win: &NSWindow = unsafe { &*(obj.cast()) };

  ns_win.setLevel(NSStatusWindowLevel);

  let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
    | NSWindowCollectionBehavior::FullScreenAuxiliary
    | NSWindowCollectionBehavior::IgnoresCycle;
  ns_win.setCollectionBehavior(behavior);

  Ok(())
}

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
        app.handle().plugin(tauri_plugin_positioner::init())?;

        tauri::tray::TrayIconBuilder::new()
          .on_tray_icon_event(|tray, evt| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &evt);
          })
          .build(app)?;
      }
      #[cfg(all(desktop, target_os = "macos"))]
      {
        if let Some(win) = app.get_webview_window("notch-capsule") {
          elevate_to_status_bar(&win)?;
          let _ = apply_vibrancy(
            &win,
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
