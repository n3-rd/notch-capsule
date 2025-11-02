#![cfg(target_os = "macos")]

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};
use tauri::{AppHandle, Emitter, Manager};
use std::sync::Mutex;
use std::os::raw::c_char;
use std::ffi::CString;
use crate::config;

// Wrapper to make id Send-safe (safe because we only access on main thread)
struct SendId(Option<id>);
unsafe impl Send for SendId {}

static NOTCH_MANAGER: Mutex<SendId> = Mutex::new(SendId(None));

/// Initialize the Swift NotchManager with configuration
pub fn init_notch_manager(
    app: &AppHandle,
    closed_w: f64,
    closed_h: f64,
    expanded_w: f64,
    expanded_h: f64,
    corner: f64,
) -> Result<(), String> {
    let app = app.clone();
    
    app.run_on_main_thread(move || {
        unsafe {
            // Load Swift framework
            load_swift_framework();
            
            // Get NotchManager shared instance
            let manager = get_notch_manager();
            if manager == nil {
                eprintln!("✗ Failed to get NotchManager shared instance");
                return;
            }
            
            // Get config as JSON string
            let cfg = config::NotchConfig::get();
            let config_json = serde_json::to_string(cfg)
                .ok()
                .and_then(|s| CString::new(s).ok());
            
            let json_ptr = config_json.as_ref().map(|cs| cs.as_ptr()).unwrap_or(std::ptr::null());
            let json_nsstring: id = if json_ptr.is_null() {
                nil
            } else {
                msg_send![class!(NSString), stringWithUTF8String:json_ptr]
            };
            
            // Call setup
            let _: () = msg_send![
                manager,
                setupWithClosedW: closed_w
                closedH: closed_h
                expandedW: expanded_w
                expandedH: expanded_h
                corner: corner
                configJson: json_nsstring
            ];
            
            // Store manager reference
            if let Ok(mut guard) = NOTCH_MANAGER.lock() {
                guard.0 = Some(manager);
            }
            
            println!("✓ Swift NotchManager initialized");
        }
    }).map_err(|e| format!("Failed to init NotchManager: {:?}", e))
}

/// Cleanup the NotchManager
pub fn cleanup_notch_manager() {
    if let Ok(guard) = NOTCH_MANAGER.lock() {
        if let Some(manager) = guard.0 {
            unsafe {
                let _: () = msg_send![manager, cleanup];
            }
        }
    }
}

/// Force expand (for testing/debugging)
pub fn force_expand() {
    if let Ok(guard) = NOTCH_MANAGER.lock() {
        if let Some(manager) = guard.0 {
            unsafe {
                let _: () = msg_send![manager, forceExpand];
            }
        }
    }
}

/// Force collapse (for testing/debugging)
pub fn force_collapse() {
    if let Ok(guard) = NOTCH_MANAGER.lock() {
        if let Some(manager) = guard.0 {
            unsafe {
                let _: () = msg_send![manager, forceCollapse];
            }
        }
    }
}

// Helper: Load Swift framework
unsafe fn load_swift_framework() {
    use std::env;
    use std::ffi::CString;
    
    extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut std::ffi::c_void;
        fn dlerror() -> *const c_char;
    }
    
    const RTLD_LAZY: i32 = 0x1;
    const RTLD_GLOBAL: i32 = 0x8;
    
    let exe_path = env::current_exe().ok();
    let exe_dir = exe_path.as_ref().and_then(|p| p.parent());
    
    let mut paths = vec![];
    
    if let Some(dir) = exe_dir {
        paths.push(dir.join("libNotchCapsuleKit.dylib"));
        paths.push(dir.join("../Resources/target/release/libNotchCapsuleKit.dylib"));
        if let Some(parent) = dir.parent() {
            paths.push(parent.join("libNotchCapsuleKit.dylib"));
        }
    }
    
    if let Ok(cwd) = env::current_dir() {
        paths.push(cwd.join("target/debug/libNotchCapsuleKit.dylib"));
        paths.push(cwd.join("target/release/libNotchCapsuleKit.dylib"));
    }
    
    for path in &paths {
        if path.exists() {
            if let Some(path_str) = path.to_str() {
                if let Ok(dylib_path) = CString::new(path_str) {
                    let handle = dlopen(dylib_path.as_ptr(), RTLD_LAZY | RTLD_GLOBAL);
                    if !handle.is_null() {
                        println!("✓ Loaded Swift dylib: {}", path_str);
                        return;
                    }
                }
            }
        }
    }
    
    eprintln!("⚠ Could not explicitly load Swift dylib, relying on linker");
}

// Helper: Get NotchManager shared instance
unsafe fn get_notch_manager() -> id {
    extern "C" {
        fn objc_getClass(name: *const c_char) -> id;
    }
    
    let class_names = vec![
        "NotchCapsuleKit.NotchManager",
        "NotchManager",
        "_TtC15NotchCapsuleKit12NotchManager",
    ];
    
    for name in &class_names {
        if let Ok(c_name) = CString::new(*name) {
            let cls = objc_getClass(c_name.as_ptr());
            if cls != nil {
                let manager: id = msg_send![cls, shared];
                if manager != nil {
                    return manager;
                }
            }
        }
    }
    
    nil
}

