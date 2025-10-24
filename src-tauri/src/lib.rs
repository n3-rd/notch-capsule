// macOS-only imports
#[cfg(target_os = "macos")]
use window_vibrancy::apply_vibrancy;

#[cfg(all(desktop, target_os = "macos"))]
use tauri::Manager;


// ---------- Notch metrics (accurate; macOS 12+) ----------
#[cfg(target_os = "macos")]
#[derive(serde::Serialize)]
struct NotchDimensions {
  // Notch span (gap made by the camera housing) in layout points
  width_pts: f64,
  // Top safe-area inset in points (non-zero => notched screen)
  top_inset_pts: f64,
  // Same values in device pixels
  width_px: f64,
  top_inset_px: f64,
  // Backing scale used for conversion
  scale: f64,
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn get_notch_dimensions(window: tauri::Window) -> Option<NotchDimensions> {
  use core::ffi::c_void;
  use objc2::runtime::AnyObject;
  use objc2_foundation::MainThreadMarker;              // correct crate for MTM
  use objc2_app_kit::{NSScreen, NSWindow};

  // NSWindow pointer
  let raw: *mut c_void = window.ns_window().ok()?;
  if raw.is_null() {
    return None;
  }
  let obj: *mut AnyObject = raw.cast();
  let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };

  // Prefer the window's current screen; fall back to mainScreen(mtm)
  let screen = unsafe { ns_win.screen() }.or_else(|| {
    let mtm = MainThreadMarker::new()?;               // <- use ? on Option
    unsafe { NSScreen::mainScreen(mtm) }
  })?;

  // Notch presence: top safe-area inset > 0
  // Apple: safeAreaInsets reflects area obscured by the camera housing on some Macs.
  let insets = unsafe { screen.safeAreaInsets() };
  if insets.top <= 0.0 {
    return None;
  }

  // True notch width = full width - (left visible area + right visible area)
  // Apple exposes those “auxiliary” rects precisely for notch-aware layout.
  let frame = unsafe { screen.frame() };                 // CGRect (points)
  let left  = unsafe { screen.auxiliaryTopLeftArea() };  // CGRect
  let right = unsafe { screen.auxiliaryTopRightArea() }; // CGRect

  let width_pts = (frame.size.width - left.size.width - right.size.width).max(0.0);

  let scale = unsafe { screen.backingScaleFactor() } as f64;
  Some(NotchDimensions {
    width_pts,
    top_inset_pts: insets.top,
    width_px: width_pts * scale,
    top_inset_px: insets.top * scale,
    scale,
  })
}

#[cfg(target_os = "macos")]
fn elevate_to_status_bar(win: &tauri::WebviewWindow) -> tauri::Result<()> {
  use core::ffi::c_void;
  use objc2::runtime::AnyObject;
  use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSStatusWindowLevel};

  let raw: *mut c_void = win.ns_window()?;
  if raw.is_null() { return Err(tauri::Error::InvalidWindowHandle); }
  let obj: *mut AnyObject = raw.cast();
  let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };

  unsafe {
    // Ride above menu bar & behave nicely across Spaces/fullscreen
    ns_win.setLevel(NSStatusWindowLevel);
    ns_win.setCollectionBehavior(
      NSWindowCollectionBehavior::CanJoinAllSpaces
      | NSWindowCollectionBehavior::FullScreenAuxiliary
      | NSWindowCollectionBehavior::IgnoresCycle,
    );

    // 💡 Critical for hover when inactive
    ns_win.setAcceptsMouseMovedEvents(true);   // opt-in for mouse moved/enter/leave
    ns_win.setIgnoresMouseEvents(false);       // ensure we don't ignore events
  }
  Ok(())
}




#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    // expose the command to frontend
    .invoke_handler(tauri::generate_handler![get_notch_dimensions])
    .setup(|app| {
      #[cfg(desktop)]
      {
        // Positioner plugin used by the Svelte side to snap to TopCenter
        app.handle().plugin(tauri_plugin_positioner::init())?;
      }
      #[cfg(all(desktop, target_os = "macos"))]
      {
        if let Some(win) = app.get_webview_window("notch-capsule") {
          // elevate so the window rides at status-bar level (covers the notch)
          elevate_to_status_bar(&win)?;
          // optional nice blur
          
        }
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
