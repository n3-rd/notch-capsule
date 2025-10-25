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
<div class="expanded-root">
  <section
    class="panel"
    in:liquidTransition={{ direction: 'in' }}
    out:liquidTransition={{ direction: 'out' }}
  >
    <header class="panel__header">
      <h2>Notch Expanded</h2>
      <p>This is the expanded content that can now stretch beyond the capsule bounds.</p>
    </header>
  </section>
</div>
<style>
  :global(body) {
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  }
  .expanded-root {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: stretch;
    justify-content: center;
    color: #f5f5f5;
  }
  .panel {
    flex: 1;

    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    border-radius: 22px;
   background: black;
    transform: translateZ(0);
    backface-visibility: hidden;
    will-change: transform, box-shadow, background;
    transition:
      transform 280ms cubic-bezier(0.22, 1, 0.36, 1),
      box-shadow 280ms cubic-bezier(0.22, 1, 0.36, 1),
      background 280ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .panel:hover {
    transform: translateY(-4px);
    box-shadow:
      0 32px 60px -36px rgba(0, 0, 0, 0.86),
      0 18px 36px -18px rgba(0, 0, 0, 0.6);
    /* background: radial-gradient(circle at top, rgba(42, 42, 42, 0.92), rgba(10, 10, 10, 0.92)); */
  }
  .panel__header > h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    letter-spacing: 0.01em;
  }
  .panel__header > p {
    margin: 0.35rem 0 0;
    font-size: 0.875rem;
    line-height: 1.5;
    color: rgba(230, 230, 230, 0.78);
  }
  @media (max-width: 360px) {
    .panel {
      padding: 1.5rem 1.25rem;
    }
 
  }
</style>