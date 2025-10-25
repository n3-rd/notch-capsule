// macOS-only imports
#[cfg(target_os = "macos")]
#[allow(unused_imports)]


#[cfg(all(desktop, target_os = "macos"))]
use tauri::Manager;

use block2::{Block, RcBlock};
use objc2_app_kit::{NSEvent, NSEventMask, NSScreen};
use objc2_foundation::MainThreadMarker; // for NSScreen::screens(mtm)
use objc2_foundation::NSPoint;
#[cfg(target_os = "macos")]
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::Emitter; // needed for AppHandle.emit(...)
#[cfg(target_os = "macos")]
use tauri::State;

// ---------- Notch metrics (accurate; macOS 12+) ----------
#[cfg(target_os = "macos")]
#[derive(serde::Serialize)]
struct NotchDimensions {
    width_pts: f64,
    top_inset_pts: f64,
    width_px: f64,
    top_inset_px: f64,
    scale: f64,
}

#[cfg(target_os = "macos")]
struct HoverState {
    expanded: Arc<AtomicBool>,
}

#[cfg(target_os = "macos")]
impl Default for HoverState {
    fn default() -> Self {
        Self {
            expanded: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn get_notch_dimensions(window: tauri::Window) -> Option<NotchDimensions> {
    use core::ffi::c_void;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSWindow;

    // NSWindow pointer
    let raw: *mut c_void = window.ns_window().ok()?;
    if raw.is_null() {
        return None;
    }
    let obj: *mut AnyObject = raw.cast();
    let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };

    // Screen
    let screen = ns_win.screen().or_else(|| {
        let mtm = MainThreadMarker::new()?;
        NSScreen::mainScreen(mtm)
    })?;

    // Notch presence
    let insets = screen.safeAreaInsets();
    if insets.top <= 0.0 {
        return None;
    }

    // True notch width
    let frame = screen.frame();
    let left = screen.auxiliaryTopLeftArea();
    let right = screen.auxiliaryTopRightArea();
    let width_pts = (frame.size.width - left.size.width - right.size.width).max(0.0);

    let scale = screen.backingScaleFactor() as f64;
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
    use objc2_app_kit::{
        NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    };

    let raw: *mut c_void = win.ns_window()?;
    if raw.is_null() {
        return Err(tauri::Error::InvalidWindowHandle);
    }
    let obj: *mut AnyObject = raw.cast();
    let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };

    ns_win.setLevel(NSStatusWindowLevel);
    ns_win.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    // Keep mouse events flowing
    ns_win.setAcceptsMouseMovedEvents(true);
    ns_win.setIgnoresMouseEvents(false);

    // Make it non-activating (panel-like) so it doesn't steal focus.
    // This toggles the style mask bit; doing it at setup time is important.
    let mut mask = ns_win.styleMask();
    mask.insert(NSWindowStyleMask::NonactivatingPanel); // non-activating panel
    ns_win.setStyleMask(mask);
    Ok(())
}

/* ===================== Accessibility (AX) ===================== */
#[cfg(target_os = "macos")]
#[tauri::command]
fn ensure_accessibility(prompt: bool) -> bool {
    use core_foundation::{
        base::{Boolean, CFRelease, TCFTypeRef}, // <-- TCFTypeRef fixes `as_void_ptr()`
        boolean::CFBooleanRef,
        dictionary::{CFDictionaryCreate, CFDictionaryRef},
        string::CFStringRef,
    };

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> Boolean;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
    }

    unsafe {
        if AXIsProcessTrusted() != 0 {
            return true;
        }
        if !prompt {
            return false;
        }
        // { kAXTrustedCheckOptionPrompt: true }
        let keys: [CFStringRef; 1] = [kAXTrustedCheckOptionPrompt];
        let true_val: CFBooleanRef = core_foundation::boolean::kCFBooleanTrue as CFBooleanRef;
        let vals: [CFBooleanRef; 1] = [true_val];

        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr() as *const *const _,
            vals.as_ptr() as *const *const _,
            1,
            &core_foundation::dictionary::kCFTypeDictionaryKeyCallBacks,
            &core_foundation::dictionary::kCFTypeDictionaryValueCallBacks,
        );

        let ok = AXIsProcessTrustedWithOptions(dict) != 0;
        CFRelease(dict.as_void_ptr());
        ok
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn set_notch_expanded(expanded: bool, state: State<HoverState>) {
    state.expanded.store(expanded, Ordering::Relaxed);
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn set_notch_expanded(_expanded: bool) {}

#[cfg(target_os = "macos")]
fn handle_mouse_move<F>(
    st: &Arc<Mutex<(bool, Instant)>>,
    app_handle: &tauri::AppHandle,
    hover_zone: &F,
) where
    F: Fn(NSPoint) -> Option<(f64, f64, f64, f64)>,
{
    let p: NSPoint = NSEvent::mouseLocation();
    let inside = if let Some((x, y, w, h)) = hover_zone(p) {
        p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
    } else {
        false
    };

    let mut lock = st.lock().unwrap();
    let (was_inside, ts) = *lock;
    let now = Instant::now();
    let open_delay = Duration::from_millis(40);
    let close_delay = Duration::from_millis(120);

    let mut changed = None;
    if inside && !was_inside && now.duration_since(ts) >= open_delay {
        *lock = (true, now);
        changed = Some(true);
    } else if !inside && was_inside && now.duration_since(ts) >= close_delay {
        *lock = (false, now);
        changed = Some(false);
    } else if inside != was_inside {
        *lock = (was_inside, now); // not enough time yet
    }

    if let Some(v) = changed {
        let _ = app_handle.emit("notch-hover", serde_json::json!({ "inside": v }));
    }
}

#[cfg(target_os = "macos")]
fn start_hover_monitors(app: &tauri::AppHandle, expanded_flag: Arc<AtomicBool>) {
    // --- shared debounce state ---
    let st = Arc::new(Mutex::new((false, Instant::now())));

    // Helper: top-center hover zone on the screen where the mouse is
    let expanded_flag_for_zone = expanded_flag.clone();
    let hover_zone = Arc::new(move |mouse: NSPoint| -> Option<(f64, f64, f64, f64)> {
        let mtm = MainThreadMarker::new()?;
        let screens = NSScreen::screens(mtm);
        let mut frame = None;
        for i in 0..screens.len() {
            let s = screens.objectAtIndex(i as _);
            let f = s.frame();
            let inside = mouse.x >= f.origin.x
                && mouse.x <= f.origin.x + f.size.width
                && mouse.y >= f.origin.y
                && mouse.y <= f.origin.y + f.size.height;
            if inside {
                frame = Some(f);
                break;
            }
        }
        let f = frame?;
        // tune these to your capsule size
        let expanded = expanded_flag_for_zone.load(Ordering::Relaxed);
        let (zone_w, zone_h) = if expanded {
            (700.0, 220.0)
        } else {
            (320.0, 40.0)
        };
        let x = f.origin.x + (f.size.width - zone_w) * 0.5;
        let y = f.origin.y + f.size.height - zone_h;
        Some((x, y, zone_w, zone_h))
    });

    // --- Global monitor: events targeted at other apps (works when you are NOT key) ---
    let st_global = st.clone();
    let app_handle_global = app.clone();
    let hover_zone_global = hover_zone.clone();
    let global_closure = move |_: NonNull<NSEvent>| {
        handle_mouse_move(&st_global, &app_handle_global, &*hover_zone_global);
    };

    let global_handler: &'static Block<dyn Fn(NonNull<NSEvent>)> =
        Box::leak(Box::new(RcBlock::new(global_closure)));
    let mouse_moved = NSEventMask::from_bits_truncate(1 << 6); // mouseMoved
    let _global =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mouse_moved, global_handler);

    // --- Local monitor: events targeted at YOUR app (fires when you ARE key) ---
    let st_local = st.clone();
    let app_handle_local = app.clone();
    let hover_zone_local = hover_zone.clone();
    let local_closure = move |evt: NonNull<NSEvent>| -> *mut NSEvent {
        handle_mouse_move(&st_local, &app_handle_local, &*hover_zone_local);
        evt.as_ptr()
    };
    let local_handler: &'static Block<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent> =
        Box::leak(Box::new(RcBlock::new(local_closure)));
    let _local = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(mouse_moved, local_handler)
    };

    // --- Poller: periodic fallback in case event monitors are blocked (e.g. missing accessibility permission) ---
    let st_poll = st.clone();
    let app_handle_poll = app.clone();
    let hover_zone_poll = hover_zone.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(50));
        let st_for_call = st_poll.clone();
        let hover_zone_for_call = hover_zone_poll.clone();
        let app_for_call = app_handle_poll.clone();
        let _ = app_handle_poll.run_on_main_thread(move || {
            handle_mouse_move(&st_for_call, &app_for_call, &*hover_zone_for_call);
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_notch_dimensions,
            ensure_accessibility,
            set_notch_expanded
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_positioner::init())?;
            }
            #[cfg(all(desktop, target_os = "macos"))]
            {
                if let Some(win) = app.get_webview_window("notch-capsule") {
                    elevate_to_status_bar(&win)?;
                }
                app.manage(HoverState::default());
                let handle = app.handle();
                let expanded_flag = app.state::<HoverState>().expanded.clone();
                start_hover_monitors(&handle, expanded_flag);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
