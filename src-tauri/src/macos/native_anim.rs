#![cfg(target_os = "macos")]
use tauri::{AppHandle, Emitter, Window};
use core::ffi::c_void;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSWindow;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::thread;
use std::time::Duration;

/// Animate a window to a target size with native macOS NSWindow animation
///
/// Uses NSWindow's setFrame:display:animate: for GPU-accelerated animations.
pub fn animate_window_to(
    app: &AppHandle,
    window: &Window,
    target_w: f64,
    target_h: f64,
    duration: f64,
    phase: &'static str,
) {
    let app_clone = app.clone();
    let window_clone = window.clone();
    
    // Must run on main thread for AppKit
    let _ = app.run_on_main_thread(move || {
        animate_impl(&app_clone, &window_clone, target_w, target_h, duration, phase);
    });
}

fn animate_impl(
    app: &AppHandle,
    window: &Window,
    target_w: f64,
    target_h: f64,
    duration: f64,
    phase: &'static str,
) {
    let Ok(raw_ptr) = window.ns_window() else {
        fallback_animate(app, window, target_w, target_h, phase);
        return;
    };
    
    let raw: *mut c_void = raw_ptr;
    if raw.is_null() {
        fallback_animate(app, window, target_w, target_h, phase);
        return;
    }

    let obj: *mut AnyObject = raw.cast();
    let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };
    
    // Get current frame
    let current_frame = ns_win.frame();
    let current_x = current_frame.origin.x;
    let current_y = current_frame.origin.y;
    let current_w = current_frame.size.width;
    let current_h = current_frame.size.height;
    
    // Calculate new position to keep window centered horizontally and top edge fixed
    let width_diff = target_w - current_w;
    let height_diff = target_h - current_h;
    let new_x = current_x - (width_diff / 2.0);
    // Adjust Y to keep top edge in same position (macOS uses bottom-left origin)
    let new_y = current_y - height_diff;
    
    let target_frame = NSRect::new(
        NSPoint::new(new_x, new_y),
        NSSize::new(target_w, target_h),
    );
    
    // Use NSWindow's built-in animation (GPU-accelerated)
    ns_win.setFrame_display_animate(target_frame, true, true);
    
    // Emit completion event after animation duration
    let app_for_completion = app.clone();
    let phase_str = phase.to_string();
    thread::spawn(move || {
        // Wait for animation to complete
        thread::sleep(Duration::from_secs_f64(duration));
        let _ = app_for_completion.emit("notch-native-anim-end", serde_json::json!({
            "phase": phase_str
        }));
    });
}

/// Fallback to simple resize if native animation fails
fn fallback_animate(app: &AppHandle, window: &Window, target_w: f64, target_h: f64, phase: &'static str) {
    let _ = window.set_size(tauri::LogicalSize {
        width: target_w,
        height: target_h,
    });
    
    let _ = app.emit("notch-native-anim-end", serde_json::json!({
        "phase": phase
    }));
}
