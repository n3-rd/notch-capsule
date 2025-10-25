<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    getCurrentWindow,
    Window as TauriWindow,
    LogicalSize,
  } from '@tauri-apps/api/window';
  import { moveWindow, Position } from '@tauri-apps/plugin-positioner';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NotchExpanded from '$lib/notch-expanded.svelte';
  
    type NotchDimensions = {
      width_pts: number;
      top_inset_pts: number;
      width_px: number;
      top_inset_px: number;
      scale: number;
    };
  
    let notchWidth = $state(320);   // fallback
    let notchHeight = $state(34);
    let notchExpanded = $state(false)
    let windowInstance: TauriWindow | null = null;

    async function resizeWindow(expanded: boolean) {
      if (!windowInstance) return;

      if (expanded) {
        // Make window resizable and expand it
        await windowInstance.setResizable(true);
        // Set expanded size (24rem = 384px, h-80 = 320px)
        const expandedWidth = 384;
        const expandedHeight = 320;
        await windowInstance.setSize(new LogicalSize(expandedWidth, expandedHeight));

        // Reposition to center the expanded content at top
        await moveWindow(Position.TopCenter);
      } else {
        // Return to capsule size and position
        await windowInstance.setSize(new LogicalSize(notchWidth, notchHeight));
        await moveWindow(Position.TopCenter);
        await windowInstance.setResizable(false);
      }
    }

  let unlisten: (() => void) | null = null;

  onMount(async () => {
    const win = (await TauriWindow.getByLabel('notch-capsule')) ?? getCurrentWindow();
    windowInstance = win;

    // Note: Mouse events don't require Accessibility permission (only key events do)

    // Dimensions
    let dims: NotchDimensions | null = null;
    try { dims = await invoke('get_notch_dimensions') as NotchDimensions | null; } catch {}
    if (dims && dims.width_pts > 0 && dims.top_inset_pts > 0) {
      notchWidth  = Math.round(dims.width_pts);
      notchHeight = Math.max(28, Math.round(dims.top_inset_pts));
    }
    await win.setSize(new LogicalSize(notchWidth, notchHeight));
    await moveWindow(Position.TopCenter);

    // Listen for native hover (works even when window not focused)
    unlisten = await listen<{ inside: boolean }>('notch-hover', ({ payload }) => {
      notchExpanded = !!payload?.inside;
      resizeWindow(notchExpanded);
    });
  });

  onDestroy(() => { if (unlisten) unlisten(); });
  </script>
  <div class="drag-strip"></div>

{#if notchExpanded}
<NotchExpanded/>
{/if}
  
  <!-- Window content -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="capsule rounded-tab bg-black"
    style="width:{notchWidth}px; height:{notchHeight}px;"
  >
    <span class="label no-drag">Notch Capsule</span>
  </div>


  <style>
    :global(html, body) {
      background: transparent;           /* transparent Tauri window */
    }
  
    /* Make the whole window draggable (great for a tiny notch window) */
    .capsule {
      display: flex;
      align-items: center;
      justify-content: center;
      margin-left: 2px;
      -webkit-app-region: no-drag; /* critical: hover target should NOT be draggable */
    }
    .drag-strip {
      position: fixed; top: -6px; left: 0; width: 100%; height: 8px;
      -webkit-app-region: drag; pointer-events: auto;
    }
    .no-drag { -webkit-app-region: no-drag; }

    
  
    /* === Notch styling for your .rounded-tab container === */
    .rounded-tab {
      /* Tunables (derived from your logical size) */
      --notch-radius: 18px;     /* main corner radius */
      --ear-size: 18px;         /* size of outer “ears” */
      --bg: #000000;            /* notch fill */
      --edge: #0009;            /* subtle border edge */
      /* --shadow: 0 2px 12px #0008, 0 2px 6px #0005; */
  
      position: relative;
      box-sizing: border-box;
      background: var(--bg);
      border: 1px solid var(--edge);
      box-shadow: var(--shadow);
  
      /* Elliptical radii to mimic macOS notch curvature */
      border-radius:
        var(--notch-radius) var(--notch-radius) calc(var(--notch-radius) * 1.9) calc(var(--notch-radius) * 1.9)
        / var(--notch-radius) var(--notch-radius) calc(var(--notch-radius) * 2.6) calc(var(--notch-radius) * 2.6);
  
      overflow: visible;
      padding: 0 10px;
    }
  
    /* “Ears” that soften the outer top corners (looks closer to real notch) */

    .rounded-tab::after {
      content: "";
      position: absolute;
      top: -1px; /* align with border */
      width: var(--ear-size);
      height: var(--ear-size);
      background: var(--bg);
      border-top: 1px solid var(--edge);
    }
    .rounded-tab::before {
      left: calc(-1 * var(--ear-size));
      border-radius: var(--ear-size) 0 0 0;
      box-shadow: -1px 0 0 0 var(--edge);
    }
    .rounded-tab::after {
      right: calc(-1 * var(--ear-size));
      border-radius: 0 var(--ear-size) 0 0;
      box-shadow: 1px 0 0 0 var(--edge);
    }
  
    /* Label/text inside */
    .rounded-tab .label {
      font: 500 11px/1.1 ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Inter, "Helvetica Neue", Arial, "Apple Color Emoji", "Segoe UI Emoji";
      color: #e5e5e5;
      user-select: none;
      pointer-events: none;
      opacity: 0.9;
    }
  
    /* Optional: Glassy “liquid” look—toggle the class on .rounded-tab if you like */
    /* .rounded-tab.glass {
      background: rgba(18, 18, 18, 0.6);
      border-color: rgba(0, 0, 0, 0.35);
      backdrop-filter: blur(10px) saturate(120%);
    } */
  
    /* Optional: Light-mode variant */
    /* .rounded-tab.light {
      --bg: #ffffff;
      --edge: #00000020;
      --shadow: 0 2px 12px #0002, 0 2px 6px #0001;
      color: #222;
    } */
  </style>
  