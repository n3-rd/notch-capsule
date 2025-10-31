#![cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::{msg_send, sel, sel_impl, class};
use tauri::{AppHandle, Emitter, Manager, Window};
use std::sync::Mutex;
use std::os::raw::{c_char, c_int, c_void};

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
        
        // Explicitly load the Swift framework dylib
        use std::ffi::CString;
        use std::env;
        
        // Declare dlopen/dlerror from libSystem
        extern "C" {
            fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
            fn dlerror() -> *const c_char;
        }
        const RTLD_LAZY: c_int = 0x1;
        const RTLD_GLOBAL: c_int = 0x8;
        
        // Get the actual executable path
        let exe_path = env::current_exe().ok();
        let exe_dir = exe_path.as_ref().and_then(|p| p.parent());
        
        let mut paths = vec![];
        
        // Add paths relative to executable directory
        if let Some(dir) = exe_dir {
            paths.push(dir.join("libNotchCapsuleKit.dylib"));
            // In dev mode, the dylib is in target/debug
            if let Some(parent) = dir.parent() {
                paths.push(parent.join("libNotchCapsuleKit.dylib"));
            }
        }
        
        // Try current working directory
        if let Ok(cwd) = env::current_dir() {
            paths.push(cwd.join("target/debug/libNotchCapsuleKit.dylib"));
            paths.push(cwd.join("target/release/libNotchCapsuleKit.dylib"));
        }
        
        let mut loaded = false;
        for path in &paths {
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    if let Ok(dylib_path) = CString::new(path_str) {
                        let handle = dlopen(dylib_path.as_ptr(), RTLD_LAZY | RTLD_GLOBAL);
                        if !handle.is_null() {
                            eprintln!("Successfully loaded Swift dylib from: {}", path_str);
                            loaded = true;
                            break;
                        } else {
                            let error = dlerror();
                            if !error.is_null() {
                                let error_str = std::ffi::CStr::from_ptr(error).to_string_lossy();
                                eprintln!("Failed to load {}: {}", path_str, error_str);
                            }
                        }
                    }
                }
            }
        }
        
        if !loaded {
            eprintln!("Warning: Could not explicitly load Swift dylib, relying on linker");
            eprintln!("Tried paths: {:?}", paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>());
        }
        
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
                // Duration parameter is passed but overridden by Swift constants
                // matching Boring Notch animation timing
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
                // Duration parameter is passed but overridden by Swift constants
                // matching Boring Notch animation timing
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
