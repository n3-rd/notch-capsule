# Notch Capsule Configuration

The `notch-config.json` file contains all adjustable parameters for animation timing, sizes, and behavior. This config is **universal** - it's read by Rust (backend), Swift (native animations), and JavaScript (frontend).

## Location

- **Root**: `/notch-config.json` - Main config file (used by Rust and Swift)
- **Static**: `/static/notch-config.json` - Copy for web frontend (automatically copied)

## Quick Reference: Which Language Uses What

| Config Section | Rust | Swift | JavaScript |
|---------------|------|-------|------------|
| `animation.*` | ❌ | ✅ | ✅ (UI animations) |
| `dimensions.*` | ✅ | ✅ | ✅ |
| `hover.*` | ✅ | ❌ | ✅ (expand_delay_ms, collapse_delay_ms) |
| `window.level_offset` | ❌ | ✅ | ❌ |

- **Rust**: Hover detection zones, polling intervals
- **Swift**: Native mask animations (expand/collapse timing curves), window level, corner radius
- **JavaScript**: UI animations, dimension calculations, layout

## Configuration Structure

### Animation Settings

Controls the expand/collapse animation behavior:

```json
"animation": {
  "expand_duration": {
    "value": 0.50,
    "description": "Duration in seconds for the expand animation - slower gives more fluid spring feel"
  },
  "collapse_duration": {
    "value": 0.35,
    "description": "Duration in seconds for the collapse animation - smooth and gentle"
  },
  "expand_timing": {
    "value": [0.16, 1.0, 0.3, 1.0],
    "description": "Cubic bezier control points for expand animation (fluid spring curve)"
  },
  "collapse_timing": {
    "value": [0.25, 0.1, 0.25, 1.0],
    "description": "Cubic bezier control points for collapse animation (smooth ease out)"
  }
}
```

**Timing Functions**: Values are cubic bezier control points `[x1, y1, x2, y2]`
- `x1, y1`: First control point (0-1 range)
- `x2, y2`: Second control point (0-1 range)
- Try [cubic-bezier.com](https://cubic-bezier.com) to visualize curves

### Dimension Settings

Controls the size of the notch capsule:

```json
"dimensions": {
  "corner_radius": {
    "value": 12,
    "description": "Corner radius in points for the notch capsule rounded corners"
  },
  "collapsed_width": {
    "value": 460.0,
    "description": "Width in points when the notch is collapsed (matches notch size)"
  },
  "collapsed_height": {
    "value": 50.0,
    "description": "Height in points when the notch is collapsed"
  },
  "expanded_width": {
    "value": 700.0,
    "description": "Width in points when the notch is fully expanded"
  },
  "expanded_height": {
    "value": 200.0,
    "description": "Height in points when the notch is fully expanded"
  }
}
```

### Hover Detection Settings

Controls the mouse hover zones and polling:

```json
"hover": {
  "collapsed_zone_width": {
    "value": 460.0,
    "description": "Width in points of the hover detection zone when collapsed"
  },
  "collapsed_zone_height": {
    "value": 50.0,
    "description": "Height in points of the hover detection zone when collapsed"
  },
  "expanded_zone_width": {
    "value": 700.0,
    "description": "Width in points of the hover detection zone when expanded"
  },
  "expanded_zone_height": {
    "value": 200.0,
    "description": "Height in points of the hover detection zone when expanded"
  },
    "expand_delay_ms": {
      "value": 250,
      "description": "Milliseconds to wait before expanding when hovering over the notch area"
    },
    "collapse_delay_ms": {
      "value": 150,
      "description": "Milliseconds to wait before collapsing when leaving the hover area"
    },
    "poll_interval_ms": {
      "value": 50,
      "description": "How often in milliseconds to check mouse position (polling fallback)"
    }
}
```

**Note**: Hover zones should match or slightly exceed the actual dimensions for smooth detection.

### Window Settings

Controls window layering:

```json
"window": {
  "level_offset": {
    "value": 3,
    "description": "How many levels above main menu to place the window (higher = more on top)"
  }
}
```

## How to Adjust

1. **Edit the config file**: Open `notch-config.json` in the project root
2. **Modify values**: Change only the `"value"` fields, keep descriptions for reference
3. **Copy to static**: Run `cp notch-config.json static/notch-config.json`
4. **Restart**: Restart the dev server (`pnpm dev`) or rebuild the app

**Important**: The config is loaded at startup. After editing:
- In **development**: Stop and restart `pnpm dev`
- In **production**: Restart the built app

## Tips

### Making animations faster/slower
- Decrease `expand_duration` and `collapse_duration` for snappier feel
- Increase for more dramatic, fluid motion
- Current values (0.50/0.35) match the "Boring Notch" feel

### Adjusting animation curves
- First two values control initial acceleration
- Last two values control final deceleration
- Higher second value = more "spring" bounce
- Try: `[0.34, 1.56, 0.64, 1]` for elastic bounce

### Changing size
- Keep hover zones matching or slightly larger than dimensions
- Recommended ratios:
  - Collapsed: ~460x50 (notch-like)
  - Expanded: ~700x200 (compact but readable)

### Hover timing customization
- Reduce `expand_delay_ms` for faster expansion (50-100ms for snappy feel)
- Increase `expand_delay_ms` for more deliberate expansion (500+ms to prevent accidental triggers)
- `collapse_delay_ms` should be shorter than `expand_delay_ms` for smooth UX
- Lower values feel more responsive but may trigger accidentally

### Performance tuning
- Lower `poll_interval_ms` for more responsive hover (uses more CPU)
- Raise to 100ms+ for better battery life
- Default 50ms is a good balance

## Validation

The config includes fallback defaults in case the file is missing or invalid. Check console logs for:
- "Loading config from: /path/to/notch-config.json" (Rust - confirms file found)
- "✓ Successfully loaded animation config from JSON" (Swift - config applied)
- "Failed to load config, using defaults" (Rust - using fallback defaults)

If you see errors:
- Verify `notch-config.json` exists in project root
- Check JSON syntax is valid (no trailing commas, proper quotes)
- Ensure you copied to `static/` for frontend access

## Example Customizations

### Snappy & Quick
```json
"expand_duration": { "value": 0.25 },
"collapse_duration": { "value": 0.18 },
"expand_delay_ms": { "value": 100 },
"collapse_delay_ms": { "value": 80 }
```

### More Deliberate (Prevents Accidental Triggers)
```json
"expand_delay_ms": { "value": 500 },
"collapse_delay_ms": { "value": 200 }
```

### Dramatic & Bouncy
```json
"expand_timing": { "value": [0.34, 1.56, 0.64, 1] },
"collapse_timing": { "value": [0.68, -0.55, 0.265, 1.55] }
```

### Larger Capsule
```json
"expanded_width": { "value": 900.0 },
"expanded_height": { "value": 300.0 }
```

