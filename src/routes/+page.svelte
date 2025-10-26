<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { animate, type JSAnimation } from 'animejs';
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

  // Ultra-smooth Anime.js animations with spring physics
  let expandedAnime: JSAnimation | null = null;
  let capsuleAnime: JSAnimation | null = null;
  let capsuleHoverAnime: JSAnimation | null = null;

  function animateExpandIn(node: HTMLElement) {
    if (expandedAnime) {
      expandedAnime.pause();
      expandedAnime = null;
    }
    
    // Set initial state explicitly
    node.style.transform = 'scale(0.88) translate3d(0, -4px, 0)';
    node.style.opacity = '0';
    node.style.filter = 'blur(8px)';

    // Small delay to ensure styles are applied
    requestAnimationFrame(() => {
      expandedAnime = animate(node, {
        scale: [0.88, 1],
        translateY: ['-4px', '0px'],
        opacity: [0, 1],
        blur: ['8px', '0px'],
        duration: 320,
        ease: 'spring(1, 70, 8, 0)',
        composition: 'replace',
      });
    });
  }

  function animateExpandOut(node: HTMLElement) {
    if (expandedAnime) {
      expandedAnime.pause();
      expandedAnime = null;
    }
    
    requestAnimationFrame(() => {
      const currentOpacity = parseFloat(window.getComputedStyle(node).opacity) || 1;
      
      expandedAnime = animate(node, {
        scale: [1, 0.88],
        translateY: ['0px', '-8px'],
        opacity: [currentOpacity, 0],
        blur: ['0px', '10px'],
        duration: 260,
        delay: 50,
        ease: 'in(3)',
        composition: 'replace',
      });
    });
  }

  function animateCapsuleIn(node: HTMLElement) {
    if (capsuleAnime) {
      capsuleAnime.pause();
      capsuleAnime = null;
    }
    
    // Set initial state
    node.style.opacity = '0';
    node.style.transform = 'scale(0.94) translate3d(0, 0, 0)';

    requestAnimationFrame(() => {
      capsuleAnime = animate(node, {
        opacity: [0, 1],
        scale: [0.94, 1],
        duration: 160,
        delay: 310,
        ease: 'out(4)',
        composition: 'replace',
      });
    });
  }

  function animateCapsuleOut(node: HTMLElement) {
    if (capsuleAnime) {
      capsuleAnime.pause();
      capsuleAnime = null;
    }
    
    requestAnimationFrame(() => {
      const currentOpacity = parseFloat(window.getComputedStyle(node).opacity) || 1;
      
      capsuleAnime = animate(node, {
        opacity: [currentOpacity, 0],
        scale: [1, 1.02],
        duration: 80,
        ease: 'inOut(2)',
        composition: 'replace',
      });
    });
  }

  function animateCapsuleHoverIn(node: HTMLElement) {
    if (capsuleHoverAnime) {
      capsuleHoverAnime.pause();
      capsuleHoverAnime = null;
    }
    
    capsuleHoverAnime = animate(node, {
      scale: [1, 1.04],
      duration: 200,
      ease: 'out(3)',
      composition: 'blend',
    });
  }

  function animateCapsuleHoverOut(node: HTMLElement) {
    if (capsuleHoverAnime) {
      capsuleHoverAnime.pause();
      capsuleHoverAnime = null;
    }
    
    capsuleHoverAnime = animate(node, {
      scale: [1.04, 1],
      duration: 300,
      ease: 'out(4)',
      composition: 'blend',
    });
  }
  
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
    let expandedEl = $state<HTMLDivElement | null>(null);
    let capsuleEl = $state<HTMLDivElement | null>(null);
    let windowInstance: TauriWindow | null = null;
    let cancelWindowResize: (() => void) | null = null;

    // Trigger animations when notchExpanded changes
    $effect(() => {
      if (notchExpanded && expandedEl) {
        animateExpandIn(expandedEl);
      }
    });

    $effect(() => {
      if (!notchExpanded && capsuleEl) {
        animateCapsuleIn(capsuleEl);
      }
    });

    const EXPANDED_WIDTH = notchExpandedWidth;
    const EXPANDED_HEIGHT = notchExpandedHeight;

    function syncNativeExpanded(expanded: boolean) {
      invoke('set_notch_expanded', { expanded }).catch(() => {});
    }

    const easeOutCubic = (t: number) => {
      const inv = 1 - t;
      return 1 - inv * inv * inv;
    };

    const round = (value: number) => Math.round(value * 100) / 100;
    const toPx = (value: number) => `${round(value)}px`;
    // More pronounced notch ears with extended top curves
    const notchPathD = 'M120 4C120 1.79086 121.791 0 124 0H127V0H0V0H3C5.20914 0 7 1.79086 7 4V14C7 17.3137 9.68629 20 13 20H114C117.314 20 120 17.3137 120 14V4Z';
    const notchMaskSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 127 20" preserveAspectRatio="none"><path d="${notchPathD}" fill="white"/></svg>`;
    const notchMaskUri = `url("data:image/svg+xml,${encodeURIComponent(notchMaskSvg)}")`;

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
      resizeWindow(true);
      syncNativeExpanded(true);
    }
  }

  function closeNotch() {
    if (notchExpanded && !DEV_KEEP_NOTCH_EXPANDED) {
      // Trigger exit animation first
      if (expandedEl) {
        animateExpandOut(expandedEl);
      }
      
      // Delay state change to allow animation to complete (delay 50ms + duration 260ms = 310ms)
      setTimeout(() => {
        notchExpanded = false;
        pointerInExpanded = false;
      }, 330);
      
      // Resize window slightly later
      setTimeout(() => {
        resizeWindow(false);
        syncNativeExpanded(false);
      }, 350);
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
    
    // Cleanup animations
    if (expandedAnime) {
      expandedAnime.pause();
      expandedAnime = null;
    }
    if (capsuleAnime) {
      capsuleAnime.pause();
      capsuleAnime = null;
    }
    if (capsuleHoverAnime) {
      capsuleHoverAnime.pause();
      capsuleHoverAnime = null;
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
        bind:this={expandedEl}
        style={`--notch-mask:${notchMaskUri};`}
        onmouseenter={() => {
          manualHold = true;
          pointerInExpanded = true;
          openNotch();
        }}
        onmouseleave={() => {
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
        class="capsule rounded-tab"
        bind:this={capsuleEl}
        style={`width:${toPx(notchWidth)}; height:${toPx(notchHeight)}; --notch-mask:${notchMaskUri};`}
        onpointerenter={(e) => {
          manualHold = true;
          if (capsuleEl) animateCapsuleHoverIn(capsuleEl);
          openNotch();
        }}
        onpointerleave={() => {
          manualHold = false;
          pointerInExpanded = false;
          if (capsuleEl) animateCapsuleHoverOut(capsuleEl);
          if (!DEV_KEEP_NOTCH_EXPANDED) {
            closeNotch();
          }
        }}
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
      /* Hardware acceleration and layout containment */
      transform: translateZ(0);
      contain: layout style paint;
    }

    /* Make the whole window draggable (great for a tiny notch window) */
    .capsule {
      display: flex;
      align-items: center;
      justify-content: center;
      margin-left: 2px;
      -webkit-app-region: no-drag; /* critical: hover target should NOT be draggable */
      pointer-events: auto;
      /* Maximum hardware acceleration for 60fps+ transitions */
      transform: translate3d(0, 0, 0);
      backface-visibility: hidden;
      perspective: 1000px;
      will-change: transform, opacity;
      contain: layout style paint;
      /* Force GPU layer */
      isolation: isolate;
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
      padding: 2.25rem 2.5rem 2rem;
      background: #000;
      border: none !important;
      transform-origin: top center;
      overflow: hidden;
      box-shadow:
        0 28px 90px -40px rgba(0, 0, 0, 0.85),
        0 16px 48px -28px rgba(0, 0, 0, 0.55);
      -webkit-mask-image: var(--notch-mask);
      mask-image: var(--notch-mask);
      -webkit-mask-repeat: no-repeat;
      mask-repeat: no-repeat;
      -webkit-mask-size: 100% 100%;
      mask-size: 100% 100%;
      -webkit-mask-position: center;
      mask-position: center;
      
      /* Allow anime.js to control all animations */
      will-change: transform, opacity, filter;
      
      /* Maximum hardware acceleration for 120fps on ProMotion displays */
      transform: translate3d(0, 0, 0);
      backface-visibility: hidden;
      perspective: 1000px;
      contain: layout style paint;
      isolation: isolate;
      /* Optimize compositing and rendering */
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
      image-rendering: -webkit-optimize-contrast;
    }

    .expanded-wrapper:hover {
      background: rgba(12, 12, 12, 0.92);
      box-shadow:
        0 36px 120px -36px rgba(0, 0, 0, 0.92),
        0 18px 52px -28px rgba(0, 0, 0, 0.65);
      transition: background-color 180ms ease-out, box-shadow 180ms ease-out;
    }
    
  
    /* === Notch styling driven by the supplied SVG === */
    .rounded-tab {
      position: relative;
      box-sizing: border-box;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0 16px;
      background: #000;
      color: #f5f5f5;
      overflow: hidden;
      box-shadow:
        0 12px 32px rgba(0, 0, 0, 0.28),
        0 4px 12px rgba(0, 0, 0, 0.22);
      -webkit-mask-image: var(--notch-mask);
      mask-image: var(--notch-mask);
      -webkit-mask-repeat: no-repeat;
      mask-repeat: no-repeat;
      -webkit-mask-size: 100% 100%;
      mask-size: 100% 100%;
      -webkit-mask-position: center;
      mask-position: center;
      /* Hardware acceleration */
      transform: translate3d(0, 0, 0);
      backface-visibility: hidden;
      contain: layout style paint;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
    }
  
    /* Label/text inside */
    .rounded-tab .label {
      font: 500 11px/1.1 ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Inter, "Helvetica Neue", Arial, "Apple Color Emoji", "Segoe UI Emoji";
      color: #e5e5e5;
      user-select: none;
      pointer-events: none;
      opacity: 0.9;
    }
  
  
  </style>
  
