#![cfg(target_os = "macos")]
use cocoa::appkit::NSWindow;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::rc::autoreleasepool;
use objc::{class, msg_send, sel, sel_impl};
use tauri::{AppHandle, Manager, Window};

fn window_mid_x(frame: NSRect) -> f64 {
    frame.origin.x + frame.size.width / 2.0
}

unsafe fn nswindow_from_tauri(window: &Window) -> Option<id> {
    // Requires the WindowExtMacOS feature in tauri (enabled by default on macOS)
    #[allow(deprecated)]
    let ns_win = window.ns_window()?;
    Some(ns_win as id)
}

pub fn animate_window_to(
    app: &AppHandle,
    window: &Window,
    target_w: f64,
    target_h: f64,
    duration: f64,
    phase: &'static str,
) {
    let app_handle = app.clone();
    let win_clone = window.clone();

    // Drive native animation on the main thread
    win_clone
        .with_webview(move |_| {
            autoreleasepool(|| unsafe {
                if let Some(ns_win) = nswindow_from_tauri(&win_clone) {
                    let current_frame: NSRect = msg_send![ns_win, frame];
                    let mid_x = window_mid_x(current_frame);
                    let target_origin_x = mid_x - (target_w / 2.0);
                    // Keep the top edge anchored by adjusting y so the window grows downwards
                    let top_y = current_frame.origin.y + current_frame.size.height;
                    let target_origin_y = top_y - target_h;
                    let target = NSRect::new(
                        NSPoint::new(target_origin_x, target_origin_y),
                        NSSize::new(target_w, target_h),
                    );

                    let _: () = msg_send![class!(NSAnimationContext), beginGrouping];
                    let _: () = msg_send![class!(NSAnimationContext), setDuration: duration];
                    let animator: id = msg_send![ns_win, animator];
                    let _: () = msg_send![animator, setFrame: target display: true];
                    let _: () = msg_send![class!(NSAnimationContext), endGrouping];

                    // Schedule completion event after duration
                    // This is a simple approach; for tighter sync, use CA transactions' completion handler.
                    let app_for_timer = app_handle.clone();
                    let phase_str = phase.to_string();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(
                            (duration * 1000.0) as u64,
                        ));
                        let _ = app_for_timer.emit_all(
                            "notch-native-anim-end",
                            serde_json::json!({"phase": phase_str}),
                        );
                    });
                }
            });
        })
        .ok();
}

pub fn current_frame(window: &Window) -> Option<NSRect> {
    autoreleasepool(|| unsafe {
        nswindow_from_tauri(window).map(|ns_win| {
            let frame: NSRect = msg_send![ns_win, frame];
            frame
        })
    })
}
