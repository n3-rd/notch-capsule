# Native Swift Mask Animator

This implementation adds native macOS animation for the notch expand/collapse using Swift CAShapeLayer.

## Architecture

### Swift Layer (`src-tauri/swift/`)

- **NotchAnimator.swift**: Core animation engine using CAShapeLayer mask
- **HitTestView**: Custom NSView that rejects clicks outside the mask path
- Event callback system to notify Rust when animations complete

### Rust Bindings (`src-tauri/src/macos/native_mask.rs`)

- Objective-C bridge to Swift NotchAnimator class
- Tauri command implementations for attach, expand, collapse, set_progress
- Event emitter for animation completion callbacks

### Frontend (`src/routes/+page.svelte`)

- Attaches native animator on mount with window dimensions
- Calls native commands when expanding/collapsing on macOS
- Listens for `notch-native-anim-end` events to sync UI state
- Falls back to window resize on non-macOS platforms

## Benefits

- Eliminates jank by using GPU-accelerated CAShapeLayer path animation
- No window resizing during animation - window stays at fixed expanded size
- Smooth 0.30s expand / 0.22s collapse with cubic bezier timing
- Hit-testing follows the mask so clicks outside are ignored
- Svelte UI remains unchanged, just hidden/revealed by the mask
