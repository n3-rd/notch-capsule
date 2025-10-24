<!-- src/routes/+page.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { getCurrentWindow, Window as TauriWindow } from '@tauri-apps/api/window';
    import { moveWindow, Position } from '@tauri-apps/plugin-positioner';
  
    onMount(async () => {
      // access the predeclared 'notch' window
      const win = (await TauriWindow.getByLabel('notch')) ?? getCurrentWindow();
  
      // hug the top-center (works across displays)
      await moveWindow(Position.TopCenter);
  
      // nudge it down a few pixels to avoid overlapping the menubar baseline
      const pos = await win.outerPosition();
      await win.setPosition({ x: pos.x, y: pos.y + 6 });
  
      // make it click-through if you just want a decorative overlay
      await getCurrentWindow().setIgnoreCursorEvents(true);
    });
  </script>
  
  <div class="capsule">
    <!-- put tiny status icons, media info, clock, etc. here -->
    <span class="dot"></span>
    <span class="label">Notch Capsule</span>
  </div>
  
  <style>
    :global(html, body) { background: transparent; }
    .capsule {
      height: 34px; width: 320px;
      border-radius: 17px;
      backdrop-filter: saturate(180%) blur(20px);
      -webkit-backdrop-filter: saturate(180%) blur(20px);
      display: flex; align-items: center; justify-content: center;
    }
    .dot { width: 8px; height: 8px; border-radius: 9999px; margin-right: .5rem; }
  </style>
  
