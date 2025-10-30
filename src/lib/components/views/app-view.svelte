<script lang="ts">
	import { SkipBack, Play, Pause, SkipForward } from '@lucide/svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';

	interface MediaInfo {
		title: string;
		artist: string;
		album: string;
		artwork_url?: string;
		duration: number;
		elapsed: number;
		is_playing: boolean;
	}

	// State
	let currentMedia = $state<MediaInfo | null>(null);
	let artworkUrl = $state<string | null>(null);
	let predictedElapsed = $state(0);
	let lastFetchTime = $state(0);
	let lastTrackId = $state<string>('');

	// Derived values
	let progressPercent = $derived(
		currentMedia && currentMedia.duration > 0 ? (predictedElapsed / currentMedia.duration) * 100 : 0
	);

	// Intervals
	let pollInterval: number | undefined;
	let tickInterval: number | undefined;

	// Generate unique track ID for comparison
	function getTrackId(media: MediaInfo): string {
		return `${media.title}_${media.artist}_${media.duration}`;
	}

	// Fetch media info from backend
	async function fetchCurrentMedia(forceArtworkRefresh = false) {
		try {
			const media = await invoke<MediaInfo | null>('get_current_media');

			if (media) {
				const newTrackId = getTrackId(media);
				const isNewTrack = newTrackId !== lastTrackId;

				// Update media
				currentMedia = media;
				predictedElapsed = media.elapsed;
				lastFetchTime = Date.now();

				// Fetch artwork if new track or forced refresh
				if (isNewTrack || forceArtworkRefresh) {
					lastTrackId = newTrackId;
					// Fetch artwork immediately in parallel
					fetchArtwork();
				}
			} else {
				// No media playing
				currentMedia = null;
				artworkUrl = null;
				predictedElapsed = 0;
				lastTrackId = '';
			}
		} catch (error) {
			console.error('Error fetching media:', error);
			currentMedia = null;
			artworkUrl = null;
		}
	}

	// Fetch artwork separately (async, non-blocking)
	async function fetchArtwork() {
		try {
			const artwork = await invoke<string | null>('get_media_artwork');
			if (artwork && (artwork.startsWith('http') || artwork.startsWith('data:image'))) {
				artworkUrl = artwork;
			} else {
				artworkUrl = null;
			}
		} catch (error) {
			console.error('Error fetching artwork:', error);
			artworkUrl = null;
		}
	}

	// Local time prediction (smooth, no backend calls)
	function updatePredictedTime() {
		if (!currentMedia) return;

		if (currentMedia.is_playing) {
			// Predict elapsed time locally
			const timeSinceLastFetch = (Date.now() - lastFetchTime) / 1000;
			predictedElapsed = Math.min(currentMedia.elapsed + timeSinceLastFetch, currentMedia.duration);
		} else {
			// Paused - use last known elapsed
			predictedElapsed = currentMedia.elapsed;
		}
	}

	function formatTime(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = Math.floor(seconds % 60);
		return `${mins}:${secs.toString().padStart(2, '0')}`;
	}

	// Playback control handlers
	async function handlePlayPause() {
		try {
			await invoke('media_play_pause');
			// Immediately fetch updated state
			setTimeout(fetchCurrentMedia, 100);
		} catch (error) {
			console.error('Error toggling playback:', error);
		}
	}

	async function handleNextTrack() {
		try {
			await invoke('media_next_track');
			// Reset track ID to force artwork refresh
			lastTrackId = '';
			// Fetch new track info immediately with forced artwork refresh
			setTimeout(() => fetchCurrentMedia(true), 300);
		} catch (error) {
			console.error('Error skipping to next track:', error);
		}
	}

	async function handlePreviousTrack() {
		try {
			await invoke('media_previous_track');
			// Reset track ID to force artwork refresh
			lastTrackId = '';
			// Fetch new track info immediately with forced artwork refresh
			setTimeout(() => fetchCurrentMedia(true), 300);
		} catch (error) {
			console.error('Error going to previous track:', error);
		}
	}

	// Seek to a specific position when clicking the seekbar
	async function handleSeek(event: MouseEvent) {
		if (!currentMedia) return;

		const seekBar = event.currentTarget as HTMLElement;
		const rect = seekBar.getBoundingClientRect();
		const clickX = event.clientX - rect.left;
		const percentage = clickX / rect.width;
		const newPosition = percentage * currentMedia.duration;

		try {
			await invoke('media_seek', { position: newPosition });
			// Update predicted elapsed immediately for instant feedback
			predictedElapsed = newPosition;
			lastFetchTime = Date.now();
			// Update actual state from backend
			setTimeout(fetchCurrentMedia, 100);
		} catch (error) {
			console.error('Error seeking:', error);
		}
	}

	onMount(() => {
		// Initial fetch
		fetchCurrentMedia();

		// Poll backend every 5 seconds (less frequent = memory efficient)
		pollInterval = setInterval(fetchCurrentMedia, 5000) as unknown as number;

		// Update predicted time every 100ms (smooth progress bar)
		tickInterval = setInterval(updatePredictedTime, 100) as unknown as number;
	});

	onDestroy(() => {
		if (pollInterval) clearInterval(pollInterval);
		if (tickInterval) clearInterval(tickInterval);
	});
</script>

{#if currentMedia}
	<div class="flex h-full w-full items-center overflow-hidden px-4 py-3">
		<div class="flex w-full max-w-full items-center gap-4">
			<!-- Album Art -->
			<div class="h-24 w-24 shrink-0 overflow-hidden rounded-2xl bg-white/5 shadow-lg">
				{#if artworkUrl}
					<img
						src={artworkUrl}
						alt={currentMedia.title}
						class="h-full w-full object-cover"
						loading="lazy"
					/>
				{:else}
					<div class="flex h-full w-full items-center justify-center text-2xl text-white/30">
						🎵
					</div>
				{/if}
			</div>

			<!-- Song Info & Controls -->
			<div class="flex min-w-0 flex-1 flex-col gap-2">
				<!-- Text Info -->
				<div class="flex flex-col gap-0.5">
					<h2 class="truncate text-xl leading-tight font-bold text-white">{currentMedia.title}</h2>
					<p class="truncate text-sm text-white/60">
						{currentMedia.artist}{currentMedia.album ? ` - ${currentMedia.album}` : ''}
					</p>
				</div>

				<!-- Progress Bar with Predicted Time -->
				<div class="flex items-center gap-2">
					<span class="shrink-0 text-xs text-white/50 tabular-nums"
						>{formatTime(predictedElapsed)}</span
					>
					<div
						class="group h-1.5 w-full cursor-pointer overflow-hidden rounded-full bg-white/10"
						onclick={handleSeek}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') handleSeek(e as unknown as MouseEvent);
						}}
						role="slider"
						tabindex="0"
						aria-label="Seek position"
						aria-valuenow={predictedElapsed}
						aria-valuemin={0}
						aria-valuemax={currentMedia.duration}
					>
						<div
							class="h-full rounded-full bg-white transition-all duration-100 ease-linear group-hover:bg-white/90"
							style="width: {Math.min(progressPercent, 100)}%"
						></div>
					</div>
					<span class="shrink-0 text-xs text-white/50 tabular-nums"
						>{formatTime(currentMedia.duration)}</span
					>
				</div>

				<!-- Controls -->
				<div class="flex items-center gap-1">
					<button
						class="flex cursor-pointer items-center justify-center rounded-full border-none bg-transparent p-1.5 text-white transition-all duration-200 hover:bg-white/10 active:scale-95"
						aria-label="Previous"
						onclick={handlePreviousTrack}
					>
						<SkipBack fill="white" size={16} />
					</button>
					<button
						class="flex cursor-pointer items-center justify-center rounded-full border-none bg-transparent p-2 text-white transition-all duration-200 hover:bg-white/10 active:scale-95"
						aria-label={currentMedia.is_playing ? 'Pause' : 'Play'}
						onclick={handlePlayPause}
					>
						{#if currentMedia.is_playing}
							<Pause fill="white" size={18} />
						{:else}
							<Play fill="white" size={18} />
						{/if}
					</button>
					<button
						class="flex cursor-pointer items-center justify-center rounded-full border-none bg-transparent p-1.5 text-white transition-all duration-200 hover:bg-white/10 active:scale-95"
						aria-label="Next"
						onclick={handleNextTrack}
					>
						<SkipForward fill="white" size={16} />
					</button>
				</div>
			</div>
		</div>
	</div>
{:else}
	<div class="flex h-full w-full items-center justify-center px-4 py-3">
		<div class="text-center text-white/50">
			<div class="mb-2 text-3xl">🎵</div>
			<p class="text-sm">No media playing</p>
			<p class="mt-1 text-xs">Play something in any app</p>
		</div>
	</div>
{/if}
