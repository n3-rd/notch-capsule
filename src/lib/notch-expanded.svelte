<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut, quintOut } from 'svelte/easing';
  import type { TransitionConfig } from 'svelte/transition';
  
  type Shortcut = {
    title: string;
    description: string;
    shortcut: string;
  };

  function liquidTransition(node: Element, options: { direction: 'in' | 'out' }): TransitionConfig {
    const { direction } = options;
    const isIn = direction === 'in';

    return {
      duration: isIn ? 280 : 80, // Very fast content fade-out
      easing: isIn ? quintOut : cubicOut,
      css: (t) => {
        const eased = isIn ? 1 - t : t;
        const blurAmount = eased * 2;
        const translateY = isIn ? (1 - t) * 12 : t * 8;
        const scale = isIn ? 0.95 + (t * 0.05) : 1 - (eased * 0.02);
        
        // Hide content completely first - fade out in first 30% of transition
        const contentOpacity = isIn ? t : Math.max(0, 1 - (t / 0.3));
        
        return `
          transform: translateY(${translateY}px) scale(${scale});
          filter: blur(${blurAmount}px);
          opacity: ${contentOpacity};
        `;
      }
    };
  }
</script>

<style>
  :global(body) {
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  }

</style>