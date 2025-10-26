<script lang="ts">
  import { SkipBack, Play, Pause, SkipForward } from '@lucide/svelte';
  
  let isPlaying = $state(false);
  let currentSong = $state({
    title: "Sucre / Saga Ship",
    artist: "Cruel Santino",
    album: "Subaru boys: Final heaven",
    artwork: "https://thenativemag.com/wp-content/uploads/2022/03/pjimage-3.jpg"
  });

  function togglePlay() {
    isPlaying = !isPlaying;
  }
</script>

<div class="w-full h-full flex items-center px-4 py-3 overflow-hidden">
  <div class="flex gap-4 items-center w-full max-w-full">
    <!-- Album Art -->
    <div class="w-24 h-20 rounded-2xl overflow-hidden shrink-0 shadow-lg">
      <img src={currentSong.artwork} alt={currentSong.title} class="w-full h-full object-cover" />
    </div>
    
    <!-- Song Info & Controls -->
    <div class="flex flex-col gap-2 min-w-0 flex-1">
      <!-- Text Info -->
      <div class="flex flex-col gap-0.5">
        <h2 class="text-xl font-bold text-white leading-tight truncate">{currentSong.title}</h2>
        <p class="text-sm text-white/60 truncate">{currentSong.artist} - {currentSong.album}</p>
      </div>

      <div class="progress-bar w-full h-2 bg-white/10 rounded-full">
        <div class="progress-bar-fill w-1/2 h-full bg-white rounded-full"></div>
      </div>
      
      <!-- Controls -->
      <div class="flex gap-1 items-center">
        <button 
          class="bg-transparent border-none text-white cursor-pointer p-1.5 flex items-center justify-center rounded-full transition-all duration-200 hover:bg-white/10 active:scale-95" 
          aria-label="Previous"
        >
          <SkipBack fill="white" size={16} />
        </button>
        <button 
          class="bg-transparent border-none text-white cursor-pointer p-2 flex items-center justify-center rounded-full transition-all duration-200 hover:bg-white/10 active:scale-95" 
          onclick={togglePlay} 
          aria-label={isPlaying ? 'Pause' : 'Play'}
        >
          {#if isPlaying}
            <Pause fill="white" size={18} />
          {:else}
            <Play fill="white" size={18} />
          {/if}
        </button>
        <button 
          class="bg-transparent border-none text-white cursor-pointer p-1.5 flex items-center justify-center rounded-full transition-all duration-200 hover:bg-white/10 active:scale-95" 
          aria-label="Next"
        >
          <SkipForward fill="white" size={16} />
        </button>
      </div>
    </div>
  </div>
</div>
