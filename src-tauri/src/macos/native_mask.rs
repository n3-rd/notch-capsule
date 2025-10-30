#![cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::{msg_send, sel, sel_impl, class};
use tauri::{AppHandle, Emitter, Manager, Window};
use std::sync::Mutex;

// Wrapper to make id Send-safe (safe because we only access on main thread)
struct SendId(Option<id>);
unsafe impl Send for SendId {}

static ANIMATOR: Mutex<SendId> = Mutex::new(SendId(None));

unsafe fn nswindow_from_tauri(window: &Window) -> Option<id> {
    window.ns_window().ok().map(|w| w as id)
}

pub fn attach_animator(
    _app: &AppHandle, 
    window: &Window, 
    closed_w: f64, 
    closed_h: f64, 
    expanded_w: f64, 
    expanded_h: f64, 
    corner: f64
) {
    unsafe {
        let ns_win = match nswindow_from_tauri(window) { 
            Some(w) => w, 
            None => return 
        };
        
        // Load NotchAnimator class from the Swift framework
        let cls = class!(NotchAnimator);
        
        let animator: id = msg_send![cls, new];
        if animator == nil {
            eprintln!("Failed to create NotchAnimator instance");
            return;
        }
        
        let closed_rect = NSRect::new(
            NSPoint::new(0., 0.),
            NSSize::new(closed_w, closed_h)
        );
        let expanded_rect = NSRect::new(
            NSPoint::new(0., 0.),
            NSSize::new(expanded_w, expanded_h)
        );
        
        let _: () = msg_send![
            animator, 
            attachTo:ns_win 
            closedRect:closed_rect
            expandedRect:expanded_rect
            corner:corner
        ];
        
        if let Ok(mut guard) = ANIMATOR.lock() {
            guard.0 = Some(animator);
        }
    }
}

pub fn expand(app: &AppHandle) {
    if let Ok(guard) = ANIMATOR.lock() {
        if let Some(animator) = guard.0 {
            unsafe {
                let duration: f64 = 0.30;
                let app_ptr = app as *const _ as *mut std::ffi::c_void;
                let _: () = msg_send![animator, expandWithDuration:duration appHandle:app_ptr];
            }
        }
    }
}

pub fn collapse(app: &AppHandle) {
    if let Ok(guard) = ANIMATOR.lock() {
        if let Some(animator) = guard.0 {
            unsafe {
                let duration: f64 = 0.22;
                let app_ptr = app as *const _ as *mut std::ffi::c_void;
                let _: () = msg_send![animator, collapseWithDuration:duration appHandle:app_ptr];
            }
        }
    }
}

pub fn set_progress(progress: f64) {
    if let Ok(guard) = ANIMATOR.lock() {
        if let Some(animator) = guard.0 {
            unsafe {
                let _: () = msg_send![animator, setProgress:progress];
            }
        }
    }
}

// Called by Swift stub via dlsym; we emit the event here.
#[no_mangle]
pub extern "C" fn _notch_notify_anim_end(app_ptr: *mut std::ffi::c_void, phase: i32) {
    if app_ptr.is_null() { 
        return; 
    }
    
    let app: &AppHandle = unsafe { &*(app_ptr as *const AppHandle) };
    let payload = if phase == 0 { "expand" } else { "collapse" };
    let _ = app.emit("notch-native-anim-end", serde_json::json!({"phase": payload}));
}
