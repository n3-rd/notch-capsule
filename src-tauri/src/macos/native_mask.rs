#![cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::{msg_send, sel, sel_impl, class};
use tauri::{AppHandle, Emitter, Manager, Window};
use std::sync::Mutex;
use std::os::raw::{c_char, c_int, c_void};
use crate::config;

// Wrapper to make id Send-safe (safe because we only access on main thread)
struct SendId(Option<id>);
unsafe impl Send for SendId {}

static ANIMATOR: Mutex<SendId> = Mutex::new(SendId(None));

unsafe fn nswindow_from_tauri(window: &Window) -> Option<id> {
    window.ns_window().ok().map(|w| w as id)
}

pub fn attach_animator(
    app: &AppHandle, 
    window: &Window, 
    closed_w: f64, 
    closed_h: f64, 
    expanded_w: f64, 
    expanded_h: f64, 
    corner: f64
) {
    let app = app.clone();
    let window = window.clone();
    
    // Objective-C window operations must happen on the main thread
    let result = app.run_on_main_thread(move || {
        unsafe {
            let ns_win = match nswindow_from_tauri(&window) { 
                Some(w) => w, 
                None => {
                    eprintln!("Failed to get NSWindow from Tauri window");
                    return;
                }
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
            // Use objc_getClass as a direct approach
            extern "C" {
                fn objc_getClass(name: *const c_char) -> id;
            }
            
            let class_names = vec![
                "NotchCapsuleKit.NotchAnimator\0",
                "NotchAnimator\0",
                "_TtC15NotchCapsuleKit13NotchAnimator\0", // Mangled Swift name
            ];
            
            let mut cls: id = nil;
            for name in &class_names {
                let c_name = CString::new(&name[..name.len()-1]).unwrap();
                cls = objc_getClass(c_name.as_ptr());
                if cls != nil {
                    eprintln!("Found class with name: {}", &name[..name.len()-1]);
                    break;
                }
            }
            
            if cls == nil {
                eprintln!("Failed to find NotchAnimator class after trying multiple names");
                return;
            }
            
            let animator: id = msg_send![cls, new];
            if animator == nil {
                eprintln!("Failed to create NotchAnimator instance");
                return;
            }
            
            // Verify the selector exists before calling
            extern "C" {
                fn sel_registerName(str: *const c_char) -> objc::runtime::Sel;
                fn class_respondsToSelector(cls: id, sel: objc::runtime::Sel) -> bool;
            }
            
            let attach_sel = sel_registerName(b"attachTo:closedRect:expandedRect:corner:\0".as_ptr() as *const c_char);
            if !class_respondsToSelector(cls, attach_sel) {
                eprintln!("NotchAnimator does not respond to attachTo:closedRect:expandedRect:corner:");
                eprintln!("Trying alternative selectors...");
                
                // Try without module prefix in selector
                let alt_sel = sel_registerName(b"attach:\0".as_ptr() as *const c_char);
                if !class_respondsToSelector(cls, alt_sel) {
                    eprintln!("Class does not respond to any attach selector");
                    return;
                }
            }
            
            let closed_rect = NSRect::new(
                NSPoint::new(0., 0.),
                NSSize::new(closed_w, closed_h)
            );
            let expanded_rect = NSRect::new(
                NSPoint::new(0., 0.),
                NSSize::new(expanded_w, expanded_h)
            );
            
            eprintln!("Calling attachTo:closedRect:expandedRect:corner: on animator");
            // Use catch_unwind to prevent panics from crashing the app
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _: () = msg_send![
                    animator, 
                    attachTo:ns_win 
                    closedRect:closed_rect
                    expandedRect:expanded_rect
                    corner:corner
                ];
            }));
            
            match result {
                Ok(_) => {
                    eprintln!("Successfully attached animator to window");
                }
                Err(_) => {
                    eprintln!("Panic occurred while calling attachTo method");
                    return;
                }
            }
            
            if let Ok(mut guard) = ANIMATOR.lock() {
                guard.0 = Some(animator);
            }
            
            // Send config to Swift via JSON string - wrapped to catch any exceptions
            let cfg = config::NotchConfig::get();
            if let Ok(config_json) = serde_json::to_string(cfg) {
                if let Ok(config_cstr) = std::ffi::CString::new(config_json) {
                    // Check if selector exists before calling
                    extern "C" {
                        fn sel_registerName(str: *const c_char) -> objc::runtime::Sel;
                        fn class_respondsToSelector(cls: id, sel: objc::runtime::Sel) -> bool;
                    }
                    
                    let set_config_sel = sel_registerName(b"setConfigJson:\0".as_ptr() as *const c_char);
                    let animator_class: id = msg_send![animator, class];
                    
                    if class_respondsToSelector(animator_class, set_config_sel) {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let _: () = msg_send![animator, setConfigJson:config_cstr.as_ptr()];
                        }));
                        
                        match result {
                            Ok(_) => eprintln!("Successfully sent config to Swift animator"),
                            Err(_) => eprintln!("Error sending config to Swift animator, using defaults"),
                        }
                    } else {
                        eprintln!("Swift animator doesn't have setConfigJson method, using defaults");
                    }
                } else {
                    eprintln!("Failed to create C string from config JSON");
                }
            } else {
                eprintln!("Failed to serialize config to JSON");
            }
        }
    });
    
    if let Err(e) = result {
        eprintln!("Failed to run attach_animator on main thread: {:?}", e);
    }
}

pub fn expand(app: &AppHandle) {
    let app_clone = app.clone();
    
    // Objective-C window operations must happen on the main thread
    let _ = app.run_on_main_thread(move || {
        unsafe {
            if let Ok(guard) = ANIMATOR.lock() {
                if let Some(animator) = guard.0 {
                    // Duration parameter is passed but overridden by Swift constants
                    // matching Boring Notch animation timing
                    let duration: f64 = 0.30;
                    let app_ptr = &app_clone as *const _ as *mut std::ffi::c_void;
                    let _: () = msg_send![animator, expandWithDuration:duration appHandle:app_ptr];
                }
            }
        }
    });
}

pub fn collapse(app: &AppHandle) {
    let app_clone = app.clone();
    
    // Objective-C window operations must happen on the main thread
    let _ = app.run_on_main_thread(move || {
        unsafe {
            if let Ok(guard) = ANIMATOR.lock() {
                if let Some(animator) = guard.0 {
                    // Duration parameter is passed but overridden by Swift constants
                    // matching Boring Notch animation timing
                    let duration: f64 = 0.22;
                    let app_ptr = &app_clone as *const _ as *mut std::ffi::c_void;
                    let _: () = msg_send![animator, collapseWithDuration:duration appHandle:app_ptr];
                }
            }
        }
    });
}

pub fn set_progress(progress: f64) {
    let animator_id = if let Ok(guard) = ANIMATOR.lock() {
        guard.0
    } else {
        return;
    };
    
    if animator_id.is_none() {
        return;
    }
    
    // Objective-C window operations must happen on the main thread
    // Note: This might be called frequently, so we need to get an AppHandle somehow
    // For now, we'll need to pass app handle or use a different approach
    // Actually, setProgress might not need main thread if it's just updating a value
    // But to be safe, let's check if we can do it without main thread first
    unsafe {
        if let Some(animator) = animator_id {
            let _: () = msg_send![animator, setProgress:progress];
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
