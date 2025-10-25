<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import {
    getCurrentWindow,
    Window as TauriWindow,
    LogicalSize,
  } from '@tauri-apps/api/window';
  import { moveWindow, Position } from '@tauri-apps/plugin-positioner';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import NotchExpanded from '$lib/notch-expanded.svelte';
	import { notchExpandedHeight, notchExpandedWidth, DEV_KEEP_NOTCH_EXPANDED } from '$lib';
  
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
    let manualHold = $state(false);
    let pointerInExpanded = $state(false);
    let showBouncyAnimation = $state(false);
    let expandedEl: HTMLDivElement | null = null;
    let windowInstance: TauriWindow | null = null;
    let cancelWindowResize: (() => void) | null = null;

    const EXPANDED_WIDTH = notchExpandedWidth;
    const EXPANDED_HEIGHT = notchExpandedHeight;

    function syncNativeExpanded(expanded: boolean) {
      invoke('set_notch_expanded', { expanded }).catch(() => {});
    }

    const easeOutCubic = (t: number) => {
      const inv = 1 - t;
      return 1 - inv * inv * inv;
    };

    async function animateWindowSize(targetWidth: number, targetHeight: number, duration = 280) {
      if (!windowInstance) return;

      if (cancelWindowResize) {
        cancelWindowResize();
        cancelWindowResize = null;
      }

      const [{ width: physicalWidth, height: physicalHeight }, scale] = await Promise.all([
        windowInstance.innerSize(),
        windowInstance.scaleFactor(),
      ]);

      const startWidth = physicalWidth / scale;
      const startHeight = physicalHeight / scale;
      const deltaWidth = targetWidth - startWidth;
      const deltaHeight = targetHeight - startHeight;

      if (Math.abs(deltaWidth) < 0.5 && Math.abs(deltaHeight) < 0.5) {
        await windowInstance.setSize(new LogicalSize(Math.round(targetWidth), Math.round(targetHeight)));
        return;
      }

      await new Promise<void>((resolve) => {
        const start = performance.now();
        let frame = 0;
        let cancelled = false;
        let repositionTick = 0;

        const step = (ts: number) => {
          if (!windowInstance || cancelled) {
            resolve();
            return;
          }

          const elapsed = ts - start;
          const t = Math.min(1, elapsed / duration);
          const eased = easeOutCubic(t);
          const nextWidth = Math.round(startWidth + deltaWidth * eased);
          const nextHeight = Math.round(startHeight + deltaHeight * eased);

          void windowInstance.setSize(new LogicalSize(nextWidth, nextHeight));
          void moveWindow(Position.TopCenter).catch(() => {});

          if (t < 1) {
            frame = requestAnimationFrame(step);
          } else {
            resolve();
          }
        };

        cancelWindowResize = () => {
          cancelled = true;
          if (frame) cancelAnimationFrame(frame);
          resolve();
        };

        frame = requestAnimationFrame(step);
      });

      cancelWindowResize = null;
    }

    async function resizeWindow(expanded: boolean) {
      if (!windowInstance) return;

      await windowInstance.setResizable(true);

      if (expanded) {
        await animateWindowSize(EXPANDED_WIDTH, EXPANDED_HEIGHT, 280);
        await moveWindow(Position.TopCenter);
      } else {
        await animateWindowSize(notchWidth, notchHeight, 220);
        await moveWindow(Position.TopCenter);
        await windowInstance.setResizable(false);
      }
    }

  let unlisten: (() => void) | null = null;

  function updatePointerState(event: PointerEvent) {
    if (!expandedEl || !notchExpanded) {
      pointerInExpanded = false;
      return;
    }
    const rect = expandedEl.getBoundingClientRect();
    const { clientX, clientY } = event;
    pointerInExpanded =
      clientX >= rect.left &&
      clientX <= rect.right &&
      clientY >= rect.top &&
      clientY <= rect.bottom;
  }

  function handlePointerLeave() {
    pointerInExpanded = false;
  }

  function openNotch() {
    if (!notchExpanded) {
      notchExpanded = true;
      showBouncyAnimation = true;
      resizeWindow(true);
      syncNativeExpanded(true);
      
      // Remove bouncy animation class after animation completes
      setTimeout(() => {
        showBouncyAnimation = false;
      }, 250); // Match --dur-fast duration
    }
  }

  function closeNotch() {
    if (notchExpanded && !DEV_KEEP_NOTCH_EXPANDED) {
      notchExpanded = false;
      pointerInExpanded = false;
      resizeWindow(false);
      syncNativeExpanded(false);
    }
  }

  onMount(async () => {
    const win = (await TauriWindow.getByLabel('notch-capsule')) ?? getCurrentWindow();
    windowInstance = win;

    window.addEventListener('pointermove', updatePointerState, { passive: true });
    window.addEventListener('pointerleave', handlePointerLeave, { passive: true });

    // Note: Mouse events don't require Accessibility permission (only key events do)

    // Dimensions
    let dims: NotchDimensions | null = null;
    try { dims = await invoke('get_notch_dimensions') as NotchDimensions | null; } catch {}
    if (dims && dims.width_pts > 0 && dims.top_inset_pts > 0) {
      notchWidth  = Math.round(dims.width_pts);
      notchHeight = Math.max(28, Math.round(dims.top_inset_pts));
    }
    if (DEV_KEEP_NOTCH_EXPANDED) {
      notchExpanded = true;
      await win.setSize(new LogicalSize(EXPANDED_WIDTH, EXPANDED_HEIGHT));
      await moveWindow(Position.TopCenter);
      syncNativeExpanded(true);
    } else {
      await win.setSize(new LogicalSize(notchWidth, notchHeight));
      await moveWindow(Position.TopCenter);
      syncNativeExpanded(false);
    }

    // Listen for native hover (works even when window not focused)
    unlisten = await listen<{ inside: boolean }>('notch-hover', ({ payload }) => {
      const inside = !!payload?.inside;
      if (inside) {
        manualHold = false;
        pointerInExpanded = false;
        openNotch();
      } else if (!DEV_KEEP_NOTCH_EXPANDED) {
        requestAnimationFrame(() => {
          if (!(manualHold || pointerInExpanded)) {
            closeNotch();
          }
        });
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    window.removeEventListener('pointermove', updatePointerState);
    window.removeEventListener('pointerleave', handlePointerLeave);
    if (cancelWindowResize) {
      cancelWindowResize();
      cancelWindowResize = null;
    }
    if (!DEV_KEEP_NOTCH_EXPANDED) {
      syncNativeExpanded(false);
    }
  });
  </script>
  <div class="surface">
    <div class="drag-strip"></div>

    {#if notchExpanded}
      <div
        class="expanded-wrapper"
        class:animate-bouncy-open={showBouncyAnimation}
        bind:this={expandedEl}
       
        in:scale={{ duration: 250, easing: cubicOut, start: 0.88 }}
        out:scale={{ duration: 250, easing: cubicOut, start: 0.96, delay: 100 }}
        on:mouseenter={() => {
          manualHold = true;
          pointerInExpanded = true;
          openNotch();
        }}
        on:mouseleave={() => {
          manualHold = false;
          pointerInExpanded = false;
          closeNotch();
        }}
      >
        <NotchExpanded />
      </div>
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="capsule rounded-tab bg-black"
        style="width:{notchWidth}px; height:{notchHeight}px;"
        in:fade={{ duration: 100 }}
        out:fade={{ duration: 80 }}
      >
        <span class="label no-drag">Notch Capsule</span>
      </div>
    {/if}
  </div>


  <style>
    :root {
      /* Dimensions tuned to resemble the app; adjust as needed */
      --closed-w: 180px;
      --closed-h: 28px;
      --open-w: 460px;
      --open-h: 80px;

      /* Timings and easings approximating SwiftUI .smooth/.spring/.bouncy */
      --dur-base: 300ms;
      --speed-boost: 1.2;
      --dur-fast: calc(var(--dur-base) / var(--speed-boost));
      --ease-smooth: cubic-bezier(0.4, 0, 0.2, 1);
      --ease-spring: cubic-bezier(0.2, 0.8, 0.2, 1);
      --ease-bouncy: cubic-bezier(0.34, 1.56, 0.64, 1);
    }

    :global(html, body) {
      background: transparent;           /* transparent Tauri window */
      width: 100%;
      height: 100%;
    }

    .surface {
      position: relative;
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      pointer-events: none;
    }

    /* Make the whole window draggable (great for a tiny notch window) */
    .capsule {
      display: flex;
      align-items: center;
      justify-content: center;
      margin-left: 2px;
      -webkit-app-region: no-drag; /* critical: hover target should NOT be draggable */
      pointer-events: auto;
      /* Hardware acceleration for smooth transitions */
      transform: translateZ(0);
      backface-visibility: hidden;
      will-change: transform, opacity;
      opacity: 0; /* Invisible but still interactive */
    }
    .drag-strip {
      position: fixed; top: -6px; left: 0; width: 100%; height: 8px;
      -webkit-app-region: drag; pointer-events: auto;
    }
    .no-drag { -webkit-app-region: no-drag; }

    .expanded-wrapper {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      pointer-events: auto;
      padding: 1.5rem;
      background: black;
      border-radius: 0 0 26px 26px;
      border: none!important;
      transform-origin: top center;
      
      /* SwiftUI-style transitions */
      transition-property: transform, opacity, background-color, box-shadow;
      transition-duration: var(--dur-fast);
      transition-timing-function: var(--ease-spring);
      will-change: transform, opacity, background-color, box-shadow;
      
      /* Hardware acceleration and smooth rendering */
      transform: translateZ(0);
      backface-visibility: hidden;
    }

    .expanded-wrapper:hover {
      background: rgba(12, 12, 12, 0.9);
      box-shadow:
        0 32px 72px -38px rgba(0, 0, 0, 0.85),
        0 10px 28px -16px rgba(0, 0, 0, 0.6);
    }
    
  
    /* === Notch styling for your .rounded-tab container === */
    .rounded-tab {
      /* Tunables (derived from your logical size) */
      --notch-radius: 18px;     /* main corner radius */
      --ear-size: 18px;         /* size of outer "ears" */
      --bg: #000000;            /* notch fill */
      --edge: #0009;            /* subtle border edge */
      /* --shadow: 0 2px 12px #0008, 0 2px 6px #0005; */
  
      position: relative;
      box-sizing: border-box;
      background: var(--bg);
      box-shadow: var(--shadow);
  
      /* Elliptical radii to mimic macOS notch curvature */
      border-radius:
        var(--notch-radius) var(--notch-radius) calc(var(--notch-radius) * 1.9) calc(var(--notch-radius) * 1.9)
        / var(--notch-radius) var(--notch-radius) calc(var(--notch-radius) * 2.6) calc(var(--notch-radius) * 2.6);
  
      overflow: visible;
      padding: 0 10px;
    }
  
    /* "Ears" that soften the outer top corners (looks closer to real notch) */

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

    /* Optional: use this class when changing to open to add a subtle bounce */
    @keyframes bouncy-open {
      0%   { transform: scale(0.98); }
      40%  { transform: scale(1.03); }
      60%  { transform: scale(0.995); }
      100% { transform: scale(1); }
    }
    .expanded-wrapper.animate-bouncy-open {
      animation: bouncy-open var(--dur-fast) var(--ease-bouncy);
    }
  
  
  </style>
  
