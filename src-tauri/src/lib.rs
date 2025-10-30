// macOS-only imports
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
mod macos;

#[cfg(all(desktop, target_os = "macos"))]
use tauri::Manager;

use block2::{Block, RcBlock};
use objc2_app_kit::{NSEvent, NSEventMask, NSScreen};
use objc2_foundation::MainThreadMarker; // for NSScreen::screens(mtm)
use objc2_foundation::NSPoint;
#[cfg(target_os = "macos")]
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
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

// Media info structure
#[derive(serde::Serialize, Clone, Debug)]
struct MediaInfo {
    title: String,
    artist: String,
    album: String,
    artwork_url: Option<String>,
    duration: f64,
    elapsed: f64,
    is_playing: bool,
}

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
#[tauri::command]
fn set_capsule_focus(focus: bool, app: tauri::AppHandle) -> Result<(), String> {
    use core::ffi::c_void;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSApp, NSWindow, NSWindowStyleMask};
    use std::sync::mpsc::channel;

    let app_for_closure = app.clone();
    let (tx, rx) = channel();
    if let Err(err) = app.run_on_main_thread(move || {
        let result = (|| -> Result<(), String> {
            let Some(win) = app_for_closure.get_webview_window("notch-capsule") else {
                return Err("notch window not found".into());
            };

            let raw: *mut c_void = win
                .ns_window()
                .map_err(|_| "unable to access native window".to_string())?;
            if raw.is_null() {
                return Err("native window pointer was null".into());
            }

            let obj: *mut AnyObject = raw.cast();
            let ns_win: &NSWindow = unsafe { &*(obj as *mut NSWindow) };
            let mut mask = ns_win.styleMask();

            if focus {
                mask.remove(NSWindowStyleMask::NonactivatingPanel);
                ns_win.setStyleMask(mask);
                if let Some(mtm) = MainThreadMarker::new() {
                    let ns_app = NSApp(mtm);
                    ns_app.activate();
                }
                ns_win.makeKeyAndOrderFront(None);
            } else {
                mask.insert(NSWindowStyleMask::NonactivatingPanel);
                ns_win.setStyleMask(mask);
                if let Some(mtm) = MainThreadMarker::new() {
                    let ns_app = NSApp(mtm);
                    ns_app.deactivate();
                }
                ns_win.orderBack(None);
            }

            Ok(())
        })();

        let _ = tx.send(result);
    }) {
        return Err(format!("failed to schedule focus change: {err:?}"));
    }

    match rx
        .recv()
        .map_err(|_| "focus change channel dropped".to_string())?
    {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn set_capsule_focus(_focus: bool, _app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

// Get currently playing media from macOS Now Playing (works with ALL apps)
#[cfg(target_os = "macos")]
#[tauri::command]
fn get_current_media() -> Option<MediaInfo> {
    use std::process::Command;

    // Use AppleScript to get Now Playing info from macOS system-wide
    // This works with Music, Spotify, Chrome, Safari, VLC, and ANY app that reports to Now Playing
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            use framework "Foundation"
            use framework "MediaPlayer"
            use scripting additions
            
            try
                -- Get Now Playing info from system
                set infoCenter to current application's MPNowPlayingInfoCenter's defaultCenter()
                set nowPlayingInfo to infoCenter's nowPlayingInfo()
                
                if nowPlayingInfo is not missing value then
                    set titleKey to current application's MPMediaItemPropertyTitle
                    set artistKey to current application's MPMediaItemPropertyArtist
                    set albumKey to current application's MPMediaItemPropertyAlbumTitle
                    set durationKey to current application's MPMediaItemPropertyPlaybackDuration
                    set elapsedKey to current application's MPNowPlayingInfoPropertyElapsedPlaybackTime
                    set rateKey to current application's MPNowPlayingInfoPropertyPlaybackRate
                    
                    set trackTitle to (nowPlayingInfo's objectForKey:titleKey) as text
                    set trackArtist to (nowPlayingInfo's objectForKey:artistKey) as text
                    set trackAlbum to (nowPlayingInfo's objectForKey:albumKey) as text
                    set trackDuration to (nowPlayingInfo's objectForKey:durationKey) as real
                    set trackElapsed to (nowPlayingInfo's objectForKey:elapsedKey) as real
                    set playbackRate to (nowPlayingInfo's objectForKey:rateKey) as real
                    
                    set isPlaying to (playbackRate > 0)
                    
                    return trackTitle & "|||" & trackArtist & "|||" & trackAlbum & "|||" & trackDuration & "|||" & trackElapsed & "|||" & isPlaying & "|||" & (current date)
                end if
            end try
            
            -- Fallback: Try common music apps if Now Playing API fails
            try
                tell application "System Events"
                    set musicRunning to (name of processes) contains "Music"
                end tell
                
                if musicRunning then
                    tell application "Music"
                        if player state is not stopped then
                            set trackName to name of current track
                            set artistName to artist of current track
                            set albumName to album of current track
                            set trackDuration to duration of current track
                            set trackPosition to player position
                            set isPlaying to (player state is playing)
                            return trackName & "|||" & artistName & "|||" & albumName & "|||" & trackDuration & "|||" & trackPosition & "|||" & isPlaying & "|||" & (current date)
                        end if
                    end tell
                end if
            end try
            
            -- Try Spotify
            try
                tell application "System Events"
                    set spotifyRunning to (name of processes) contains "Spotify"
                end tell
                
                if spotifyRunning then
                    tell application "Spotify"
                        if player state is not stopped then
                            set trackName to name of current track
                            set artistName to artist of current track
                            set albumName to album of current track
                            set trackDuration to duration of current track / 1000
                            set trackPosition to player position
                            set isPlaying to (player state is playing)
                            return trackName & "|||" & artistName & "|||" & albumName & "|||" & trackDuration & "|||" & trackPosition & "|||" & isPlaying & "|||" & (current date)
                        end if
                    end tell
                end if
            end try
            
            return ""
        "#)
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !result.is_empty() {
            let parts: Vec<&str> = result.split("|||").collect();
            if parts.len() >= 6 {
                return Some(MediaInfo {
                    title: parts[0].to_string(),
                    artist: parts[1].to_string(),
                    album: parts[2].to_string(),
                    artwork_url: None, // Will fetch separately
                    duration: parts[3].parse().unwrap_or(0.0),
                    elapsed: parts[4].parse().unwrap_or(0.0),
                    is_playing: parts[5] == "true",
                });
            }
        }
    }

    None
}

// Get album artwork as base64 data URL
#[cfg(target_os = "macos")]
#[tauri::command]
fn get_media_artwork() -> Option<String> {
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            try
                tell application "System Events"
                    set musicRunning to (name of processes) contains "Music"
                end tell
                
                if musicRunning then
                    tell application "Music"
                        if player state is not stopped then
                            set artworkData to data of artwork 1 of current track
                            return artworkData as «class PNGf»
                        end if
                    end tell
                end if
            end try
            
            try
                tell application "System Events"
                    set spotifyRunning to (name of processes) contains "Spotify"
                end tell
                
                if spotifyRunning then
                    tell application "Spotify"
                        if player state is not stopped then
                            return artwork url of current track
                        end if
                    end tell
                end if
            end try
            
            return ""
        "#)
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = output.stdout;
        if stdout.is_empty() {
            return None;
        }

        match std::str::from_utf8(&stdout) {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.starts_with("http") && !trimmed.is_empty() && trimmed != "missing value" {
                    return Some(trimmed.to_string());
                }
                return None;
            }
            Err(_) => {
                let data = if stdout.last() == Some(&b'\n') {
                    &stdout[..stdout.len().saturating_sub(1)]
                } else {
                    &stdout[..]
                };
                if !data.is_empty() {
                    let encoded = BASE64_STANDARD.encode(data);
                    if !encoded.is_empty() {
                        return Some(format!("data:image/png;base64,{}", encoded));
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn get_current_media() -> Option<MediaInfo> {
    None
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn get_media_artwork() -> Option<String> {
    None
}

// Control media playback
#[cfg(target_os = "macos")]
#[tauri::command]
fn media_play_pause() -> bool {
    use std::process::Command;

    // Try Music app
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            tell application "System Events"
                set musicRunning to (name of processes) contains "Music"
            end tell
            
            if musicRunning then
                tell application "Music"
                    playpause
                    return true
                end tell
            end if
            
            -- Try Spotify
            tell application "System Events"
                set spotifyRunning to (name of processes) contains "Spotify"
            end tell
            
            if spotifyRunning then
                tell application "Spotify"
                    playpause
                    return true
                end tell
            end if
            
            return false
        "#)
        .output()
        .ok();

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn media_next_track() -> bool {
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            tell application "System Events"
                set musicRunning to (name of processes) contains "Music"
            end tell
            
            if musicRunning then
                tell application "Music"
                    next track
                    return true
                end tell
            end if
            
            tell application "System Events"
                set spotifyRunning to (name of processes) contains "Spotify"
            end tell
            
            if spotifyRunning then
                tell application "Spotify"
                    next track
                    return true
                end tell
            end if
            
            return false
        "#)
        .output()
        .ok();

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn media_previous_track() -> bool {
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            tell application "System Events"
                set musicRunning to (name of processes) contains "Music"
            end tell
            
            if musicRunning then
                tell application "Music"
                    previous track
                    return true
                end tell
            end if
            
            tell application "System Events"
                set spotifyRunning to (name of processes) contains "Spotify"
            end tell
            
            if spotifyRunning then
                tell application "Spotify"
                    previous track
                    return true
                end tell
            end if
            
            return false
        "#)
        .output()
        .ok();

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn media_play_pause() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn media_next_track() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn media_previous_track() -> bool {
    false
}

// Seek to a specific position in the track
#[cfg(target_os = "macos")]
#[tauri::command]
fn media_seek(position: f64) -> bool {
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"
            tell application "System Events"
                set musicRunning to (name of processes) contains "Music"
            end tell
            
            if musicRunning then
                tell application "Music"
                    set player position to {}
                    return true
                end tell
            end if
            
            tell application "System Events"
                set spotifyRunning to (name of processes) contains "Spotify"
            end tell
            
            if spotifyRunning then
                tell application "Spotify"
                    set player position to {}
                    return true
                end tell
            end if
            
            return false
            "#,
            position, position
        ))
        .output()
        .ok();

    output.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn media_seek(_position: f64) -> bool {
    false
}

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
            (700.0, 200.0)
        } else {
            (460.0, 50.0) // Wider hover zone to match expanded capsule
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
    if let Some(monitor) = unsafe {
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mouse_moved, global_handler)
    } {
        std::mem::forget(monitor);
    }

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
    if let Some(monitor) =
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mouse_moved, local_handler) }
    {
        std::mem::forget(monitor);
    }

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

// Native mask animation commands
#[tauri::command]
async fn notch_attach(
    window: tauri::Window,
    app: tauri::AppHandle,
    _label: String,
    closed_w: f64,
    closed_h: f64,
    expanded_w: f64,
    expanded_h: f64,
    corner: f64,
) {
    #[cfg(target_os = "macos")]
    {
        macos::native_mask::attach_animator(&app, &window, closed_w, closed_h, expanded_w, expanded_h, corner);
    }
}

#[tauri::command]
async fn notch_expand(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        macos::native_mask::expand(&app);
    }
}

#[tauri::command]
async fn notch_collapse(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        macos::native_mask::collapse(&app);
    }
}

#[tauri::command]
async fn notch_set_progress(progress: f64) {
    #[cfg(target_os = "macos")]
    {
        macos::native_mask::set_progress(progress);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_notch_dimensions,
            ensure_accessibility,
            set_notch_expanded,
            set_capsule_focus,
            get_current_media,
            get_media_artwork,
            media_play_pause,
            media_next_track,
            media_previous_track,
            media_seek,
            notch_attach,
            notch_expand,
            notch_collapse,
            notch_set_progress
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
