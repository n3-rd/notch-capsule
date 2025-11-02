# Swift-Based Notch Integration

The notch expansion functionality has been rewritten in Swift for better performance and smoother animations, following the architecture of Boring Notch.

## Architecture

### Swift Side (`NotchManager.swift`)
- **NotchManager**: Main controller that handles everything
  - Window creation and management
  - Hover detection via event monitors and polling
  - State management (expanded/collapsed)
  - Animation coordination
  - Automatic expansion/collapse with configurable delays

- **NotchWindow**: Custom NSWindow subclass
  - Transparent, borderless window
  - Status bar level
  - Full-screen auxiliary behavior
  
- **HoverMonitor**: Mouse tracking system
  - Global event monitors
  - Polling fallback for reliability
  - Configurable hover zones

- **NotchAnimator**: Mask animation handler
  - Spring physics on macOS 14+
  - Smooth path interpolation
  - GPU-accelerated animations

### Rust Side (`swift_bridge.rs`)
- Minimal bridge to Swift
- Initialization only
- Optional force expand/collapse for testing

## Usage

### 1. Initialize on App Startup

In your Svelte `onMount` or during app initialization:

```typescript
import { invoke } from '@tauri-apps/api/core';

onMount(async () => {
  try {
    await invoke('init_swift_notch', {
      closedW: 240,
      closedH: 37,
      expandedW: 800,
      expandedH: 600,
      corner: 12
    });
    
    console.log('✓ Swift notch manager initialized');
  } catch (error) {
    console.error('Failed to initialize notch:', error);
  }
});
```

### 2. That's It!

The Swift NotchManager handles everything automatically:
- ✅ Hover detection
- ✅ Expansion timing
- ✅ Collapse timing  
- ✅ Window positioning
- ✅ Focus management
- ✅ Smooth animations

No need to:
- ❌ Listen for hover events
- ❌ Manage window resize
- ❌ Handle focus states
- ❌ Coordinate animations

### 3. Optional: Manual Control

For testing or special cases:

```typescript
// Force expand
await invoke('notch_force_expand');

// Force collapse
await invoke('notch_force_collapse');
```

## Configuration

The NotchManager reads from `notch-config.json`:

```json
{
  "hover": {
    "collapsed_zone_width": { "value": 300 },
    "collapsed_zone_height": { "value": 60 },
    "expanded_zone_width": { "value": 850 },
    "expanded_zone_height": { "value": 650 },
    "expand_delay_ms": { "value": 250 },
    "collapse_delay_ms": { "value": 150 }
  },
  "animation": {
    "expand_duration": { "value": 0.4 },
    "collapse_duration": { "value": 0.3 }
  }
}
```

## Benefits

1. **Native Performance**: All window/event management in Swift
2. **Smoother Animations**: Spring physics, GPU-accelerated
3. **Simpler Frontend**: No complex state management
4. **Better Integration**: Uses macOS APIs directly
5. **Boring Notch Architecture**: Proven, battle-tested approach

## Comparison: Old vs New

### Old Architecture (Rust-heavy)
```
Rust ←→ Tauri Events ←→ Svelte
↓
- Hover detection in Rust
- Window resize in Rust  
- State management in Svelte
- Animation triggers in Svelte
- Focus handling in Rust
```

### New Architecture (Swift-native)
```
Rust → Swift (init only)
         ↓
    NotchManager
    (handles everything)
```

## Migration Guide

If you have existing code using the old approach:

1. **Remove** Rust hover listeners
2. **Remove** Svelte hover state management
3. **Remove** manual window resize calls
4. **Remove** focus management code
5. **Add** single `init_swift_notch` call on mount
6. **Keep** your notch content UI

That's it! The Swift manager handles all the mechanics.

