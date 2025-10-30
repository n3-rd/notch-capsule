<script lang="ts">
	import { onMount, onDestroy, tick } from 'svelte';
	import { animate, type JSAnimation } from 'animejs';
	import { getCurrentWindow, Window as TauriWindow, LogicalSize } from '@tauri-apps/api/window';
	import { moveWindow, Position } from '@tauri-apps/plugin-positioner';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import NotchExpanded from '$lib/notch-expanded.svelte';
	import { notchExpandedHeight, notchExpandedWidth, DEV_KEEP_NOTCH_EXPANDED } from '$lib';
	import Waveform from '$lib/components/music/waveform.svelte';

	// Media info for capsule display
	interface MediaInfo {
		title: string;
		artist: string;
		album: string;
		artwork_url?: string;
		duration: number;
		elapsed: number;
		is_playing: boolean;
	}

	let capsuleMedia = $state<MediaInfo | null>(null);
	let capsuleArtwork = $state<string | null>(null);
	let mediaPollInterval: ReturnType<typeof setTimeout> | null = null;
	let capsuleFadingOut = $state(false);
	let showCapsuleContent = $state(true); // Control visibility of artwork/waveform
	let capsuleRenderKey = $state(0); // Force re-render on collapse
	const DEFAULT_WAVE_COLOR = '#ffffff';
	let capsuleWaveColor = $state(DEFAULT_WAVE_COLOR);
	const artworkColorCache = new Map<string, string>();
	let artworkColorJob = 0;
	let isFetchingMedia = false;
	let openIntentToken = 0;
	let hasPendingOpen = false;
	let capsuleHasFocus = false;
	const MEDIA_POLL_ACTIVE_MS = 1200;
	const MEDIA_POLL_IDLE_MS = 4000;

	// Hover timing constants matching Boring Notch behavior
	const MIN_HOVER_DURATION = 300; // ms - delay before opening on hover
	const LEAVE_DEBOUNCE = 100; // ms - debounce before closing on leave

	// Hover scheduling state
	let hoverOpenTimer: ReturnType<typeof setTimeout> | null = null;
	let hoverCloseTimer: ReturnType<typeof setTimeout> | null = null;

	// Track if native mask animator is attached (macOS only)
	let nativeAnimatorAttached = $state(false);

	// Ultra-smooth Anime.js animations with spring physics
	let expandedAnime: JSAnimation | null = null;
	let capsuleAnime: JSAnimation | null = null;
	let capsuleHoverAnime: JSAnimation | null = null;

	// Animation duration constants for synchronization
	const EXPAND_IN_DURATION = 320;
	const EXPAND_OUT_DURATION = 220;
	const CAPSULE_IN_DURATION = 240;
	const CAPSULE_OUT_DURATION = 120;

	function animateExpandIn(node: HTMLElement) {
		if (expandedAnime) {
			expandedAnime.pause();
			expandedAnime = null;
		}

		// Set will-change for GPU optimization
		node.style.willChange = 'transform, opacity';

		// Set initial state for morph
		node.style.opacity = '0';
		node.style.transform = 'scale(0.92)';

		// Small delay to ensure styles are applied
		requestAnimationFrame(() => {
			expandedAnime = animate(node, {
				scale: [0.92, 1],
				translateY: ['-2px', '0px'],
				opacity: [0, 1],
				duration: EXPAND_IN_DURATION,
				delay: 0,
				ease: 'spring(1, 90, 10, 0)',
				composition: 'replace',
				complete: () => {
					// Clear will-change after animation completes
					node.style.willChange = 'auto';
				}
			});
		});
	}

	function animateExpandOut(node: HTMLElement) {
		if (expandedAnime) {
			expandedAnime.pause();
			expandedAnime = null;
		}

		// Set will-change for GPU optimization
		node.style.willChange = 'transform, opacity';

		requestAnimationFrame(() => {
			const currentOpacity = parseFloat(window.getComputedStyle(node).opacity) || 1;

			expandedAnime = animate(node, {
				scale: [1, 0.92],
				translateY: ['0px', '-4px'],
				opacity: [currentOpacity, 0],
				duration: EXPAND_OUT_DURATION,
				delay: 0,
				ease: 'in(2.5)',
				composition: 'replace',
				complete: () => {
					// Clear will-change after animation completes
					node.style.willChange = 'auto';
				}
			});
		});
	}

	function animateCapsuleIn(node: HTMLElement) {
		if (capsuleAnime) {
			capsuleAnime.pause();
			capsuleAnime = null;
		}

		// Set will-change for GPU optimization
		node.style.willChange = 'transform, opacity';

		// Set initial state
		node.style.opacity = '0';

		requestAnimationFrame(() => {
			capsuleAnime = animate(node, {
				opacity: [0, 1],
				scale: [0.96, 1],
				duration: CAPSULE_IN_DURATION,
				delay: 180,
				ease: 'spring(1, 70, 10, 0)',
				composition: 'replace',
				complete: () => {
					// Clear will-change after animation completes
					node.style.willChange = 'auto';
				}
			});
		});
	}

	function animateCapsuleOut(node: HTMLElement) {
		if (capsuleAnime) {
			capsuleAnime.pause();
			capsuleAnime = null;
		}

		// Set will-change for GPU optimization (opacity-only since no transform)
		node.style.willChange = 'opacity';

		requestAnimationFrame(() => {
			const currentOpacity = parseFloat(window.getComputedStyle(node).opacity) || 1;

			capsuleAnime = animate(node, {
				opacity: [currentOpacity, 0],
				// Note: scale animation removed for GPU optimization (opacity-only fade)
				duration: CAPSULE_OUT_DURATION,
				ease: 'inOut(2)',
				composition: 'replace',
				complete: () => {
					// Clear will-change after animation completes
					node.style.willChange = 'auto';
				}
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
			composition: 'blend'
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
			composition: 'blend'
		});
	}

	type NotchDimensions = {
		width_pts: number;
		top_inset_pts: number;
		width_px: number;
		top_inset_px: number;
		scale: number;
	};

	let notchWidth = $state(420); // wider fallback for better visibility
	let notchWidthNormal = $state(240); // normal width when not playing
	let notchHeight = $state(37); // slightly taller to match macOS notch
	let notchExpanded = $state(false);
	let manualHold = $state(false);
	let pointerInExpanded = $state(false);
	let expandedEl = $state<HTMLDivElement | null>(null);
	let capsuleEl = $state<HTMLDivElement | null>(null);
	let windowInstance: TauriWindow | null = null;
	let cancelWindowResize: (() => void) | null = null;
	let closingNotch = false;

	$effect(() => {
		const artwork = capsuleArtwork;
		const jobId = ++artworkColorJob;

		if (!artwork) {
			capsuleWaveColor = DEFAULT_WAVE_COLOR;
			return;
		}

		const cached = artworkColorCache.get(artwork);
		if (cached) {
			capsuleWaveColor = cached;
			return;
		}

		capsuleWaveColor = DEFAULT_WAVE_COLOR;

		extractDominantColor(artwork)
			.then((color) => {
				if (artworkColorJob !== jobId) return;

				if (color) {
					artworkColorCache.set(artwork, color);
					capsuleWaveColor = color;
				} else {
					artworkColorCache.set(artwork, DEFAULT_WAVE_COLOR);
					capsuleWaveColor = DEFAULT_WAVE_COLOR;
				}
			})
			.catch(() => {
				if (artworkColorJob === jobId) {
					artworkColorCache.set(artwork, DEFAULT_WAVE_COLOR);
					capsuleWaveColor = DEFAULT_WAVE_COLOR;
				}
			});
	});

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

	const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
	const round = (value: number) => Math.round(value * 100) / 100;
	const toPx = (value: number) => `${round(value)}px`;
	// More pronounced notch ears with extended top curves
	const notchPathD =
		'M120 4C120 1.79086 121.791 0 124 0H127V0H0V0H3C5.20914 0 7 1.79086 7 4V14C7 17.3137 9.68629 20 13 20H114C117.314 20 120 17.3137 120 14V4Z';
	const notchMaskSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 127 20" preserveAspectRatio="none"><path d="${notchPathD}" fill="white"/></svg>`;
	const notchMaskUri = `url("data:image/svg+xml,${encodeURIComponent(notchMaskSvg)}")`;
	const HOVER_HIT_SLOP = 3;

	async function animateWindowSize(targetWidth: number, targetHeight: number, duration = 280) {
		if (!windowInstance) return;

		if (cancelWindowResize) {
			cancelWindowResize();
			cancelWindowResize = null;
		}

		const [{ width: physicalWidth, height: physicalHeight }, scale] = await Promise.all([
			windowInstance.innerSize(),
			windowInstance.scaleFactor()
		]);

		const startWidth = physicalWidth / scale;
		const startHeight = physicalHeight / scale;
		const deltaWidth = targetWidth - startWidth;
		const deltaHeight = targetHeight - startHeight;

		if (Math.abs(deltaWidth) < 0.5 && Math.abs(deltaHeight) < 0.5) {
			await windowInstance
				.setSize(new LogicalSize(Math.round(targetWidth), Math.round(targetHeight)))
				.catch(() => {});
			await moveWindow(Position.TopCenter).catch(() => {});
			return;
		}

		await new Promise<void>((resolve) => {
			const start = performance.now();
			let frame = 0;
			let cancelled = false;

			const step = async (ts: number) => {
				if (!windowInstance || cancelled) {
					resolve();
					return;
				}

				const elapsed = ts - start;
				const t = Math.min(1, elapsed / duration);
				const eased = easeOutCubic(t);
				const nextWidth = Math.round(startWidth + deltaWidth * eased);
				const nextHeight = Math.round(startHeight + deltaHeight * eased);

				await windowInstance.setSize(new LogicalSize(nextWidth, nextHeight)).catch(() => {});
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

		await moveWindow(Position.TopCenter).catch(() => {});
		cancelWindowResize = null;
	}

	async function resizeWindow(expanded: boolean) {
		if (!windowInstance) return;

		await windowInstance.setResizable(true);

		if (expanded) {
			await animateWindowSize(EXPANDED_WIDTH, EXPANDED_HEIGHT, 280);
			await moveWindow(Position.TopCenter);
		} else {
			// Use sync resize for collapse
			const targetWidth = capsuleMedia?.is_playing ? notchWidth : notchWidthNormal;
			await windowInstance.setSize(new LogicalSize(targetWidth, notchHeight));
			await moveWindow(Position.TopCenter);
			await windowInstance.setResizable(false);
		}
	}

	let unlisten: (() => void) | null = null;
	let unlistenNative: (() => void) | null = null;

	function isWithinRect(rect: DOMRect, x: number, y: number, padding = 0) {
		return (
			x >= rect.left - padding &&
			x <= rect.right + padding &&
			y >= rect.top - padding &&
			y <= rect.bottom + padding
		);
	}

	function cancelScheduledOpen() {
		if (hoverOpenTimer) {
			clearTimeout(hoverOpenTimer);
			hoverOpenTimer = null;
		}
		if (hasPendingOpen) {
			hasPendingOpen = false;
			capsuleFadingOut = false;
			showCapsuleContent = true;
		}
		openIntentToken++;
	}

	function cancelScheduledClose() {
		if (hoverCloseTimer) {
			clearTimeout(hoverCloseTimer);
			hoverCloseTimer = null;
		}
	}

	function scheduleOpenOnHover() {
		if (DEV_KEEP_NOTCH_EXPANDED) return;

		cancelScheduledOpen();
		hoverOpenTimer = setTimeout(() => {
			hoverOpenTimer = null;
			if (manualHold && !notchExpanded) {
				void openNotch();
			}
		}, MIN_HOVER_DURATION);
	}

	function scheduleCloseOnLeave() {
		if (DEV_KEEP_NOTCH_EXPANDED) return;

		cancelScheduledClose();
		hoverCloseTimer = setTimeout(() => {
			hoverCloseTimer = null;
			if (!manualHold && !pointerInExpanded) {
				void closeNotch();
			}
		}, LEAVE_DEBOUNCE);
	}

	async function updateCapsuleFocus(focused: boolean) {
		if (capsuleHasFocus === focused) {
			return;
		}

		try {
			await invoke('set_capsule_focus', { focus: focused });
			if (focused && windowInstance) {
				await windowInstance.setFocusable(true).catch(() => {});
				await windowInstance.setFocus().catch(() => {});
			}
			capsuleHasFocus = focused;
		} catch (error) {
			console.warn('Failed to update capsule focus state:', error);
		}
	}

	function clearMediaPoll() {
		if (mediaPollInterval) {
			clearTimeout(mediaPollInterval);
			mediaPollInterval = null;
		}
	}

	function scheduleMediaPoll(delay: number) {
		clearMediaPoll();
		const safeDelay = Math.max(250, delay);
		mediaPollInterval = setTimeout(async () => {
			const playing = await fetchCapsuleMedia();
			scheduleMediaPoll(playing ? MEDIA_POLL_ACTIVE_MS : MEDIA_POLL_IDLE_MS);
		}, safeDelay);
	}

	function updatePointerState(event: PointerEvent) {
		const { clientX, clientY } = event;

		if (expandedEl && notchExpanded) {
			pointerInExpanded = isWithinRect(
				expandedEl.getBoundingClientRect(),
				clientX,
				clientY,
				HOVER_HIT_SLOP
			);
			return;
		}

		pointerInExpanded = false;

		if (!capsuleEl || notchExpanded) {
			return;
		}

		const rect = capsuleEl.getBoundingClientRect();
		const withinHitArea = isWithinRect(rect, clientX, clientY, HOVER_HIT_SLOP);

		if (withinHitArea) {
			if (!manualHold) {
				manualHold = true;
				if (capsuleEl) animateCapsuleHoverIn(capsuleEl);
				void openNotch();
			}
		} else if (manualHold) {
			manualHold = false;
			if (capsuleEl) animateCapsuleHoverOut(capsuleEl);
			cancelScheduledOpen();
		}
	}

	function handlePointerLeave() {
		manualHold = false;
		pointerInExpanded = false;
		cancelScheduledOpen();
	}

	async function openNotch() {
		if (notchExpanded || DEV_KEEP_NOTCH_EXPANDED) {
			hasPendingOpen = false;
			capsuleFadingOut = false;
			showCapsuleContent = true;
			return;
		}

		const token = ++openIntentToken;
		hasPendingOpen = true;
		showCapsuleContent = false;
		capsuleFadingOut = true;

		await wait(150);

		if (token !== openIntentToken || notchExpanded) {
			hasPendingOpen = false;
			return;
		}

		hasPendingOpen = false;
		notchExpanded = true;
		capsuleFadingOut = false;

		// Use native animator if available (macOS), otherwise fallback to window resize
		if (nativeAnimatorAttached) {
			// Native animation - just call the command, no window resizing
			// Don't call updateCapsuleFocus - keep window non-activating for hover to work
			try {
				await invoke('notch_expand');
				// Show content immediately since native animation handles the mask
				showCapsuleContent = true;
			} catch (err) {
				console.warn('Native expand error:', err);
				// Fallback to showing content immediately
				showCapsuleContent = true;
			}
		} else {
			// Fallback: Fire-and-forget window resize - don't block DOM animation
			resizeWindow(true)
				.then(() => updateCapsuleFocus(true))
				.catch((err) => console.warn('Window resize error:', err))
				.finally(() => syncNativeExpanded(true));

			// DOM animation starts immediately without waiting for native resize
			await tick();
		}
	}

	async function closeNotch() {
		cancelScheduledOpen();

		if (!notchExpanded || DEV_KEEP_NOTCH_EXPANDED || closingNotch) {
			return;
		}

		closingNotch = true;

		try {
			// Hide capsule content during collapse
			showCapsuleContent = false;

			// Start DOM collapse animation first
			if (expandedEl) {
				animateExpandOut(expandedEl);
			}

			// Wait for DOM animation to complete
			await wait(EXPAND_OUT_DURATION);

			if (DEV_KEEP_NOTCH_EXPANDED || manualHold || pointerInExpanded) {
				if (expandedEl) {
					animateExpandIn(expandedEl);
				}
				showCapsuleContent = true;
				return;
			}

			// Change state first
			notchExpanded = false;
			pointerInExpanded = false;

			// Use native animator if available (macOS), otherwise fallback to window resize
			if (nativeAnimatorAttached) {
				// Native animation - just call the command, no window resizing
				// Don't call updateCapsuleFocus - keep window non-activating for hover to work
				try {
					await invoke('notch_collapse');
					// Content will be shown when native animation completes (notch-native-anim-end event)
				} catch (err) {
					console.warn('Native collapse error:', err);
					// Fallback
					capsuleRenderKey++;
					await tick();
					showCapsuleContent = true;
				}
			} else {
				// Fallback: resize window synchronously to prevent gaps
				const targetWidth = capsuleMedia?.is_playing ? notchWidth : notchWidthNormal;
				await resizeWindowSync(targetWidth, notchHeight);

				syncNativeExpanded(false);
				await updateCapsuleFocus(false);

				// Force complete re-render with new key
				await tick();
				capsuleRenderKey++;

				// Show capsule content after re-render
				await tick();
				showCapsuleContent = true;
			}
		} finally {
			capsuleFadingOut = false;
			closingNotch = false;
		}
	}

	// Synchronous resize for collapse to prevent gaps
	async function resizeWindowSync(width: number, height: number) {
		if (!windowInstance) return;

		try {
			await windowInstance.setResizable(true);
			const logicalWidth = Math.round(width);
			const logicalHeight = Math.round(height);
			await windowInstance.setSize(new LogicalSize(logicalWidth, logicalHeight));
			await moveWindow(Position.TopCenter);
		} catch (error) {
			console.error('Error resizing window:', error);
		} finally {
			try {
				await windowInstance.setResizable(false);
			} catch (lockError) {
				console.error('Error locking window size:', lockError);
			}
		}
	}

	function channelToHex(channel: number) {
		return Math.min(255, Math.max(0, Math.round(channel)))
			.toString(16)
			.padStart(2, '0');
	}

	function rgbToHex(r: number, g: number, b: number) {
		return `#${channelToHex(r)}${channelToHex(g)}${channelToHex(b)}`;
	}

	function adjustForContrast(r: number, g: number, b: number) {
		const luminance = 0.299 * r + 0.587 * g + 0.114 * b;
		if (luminance >= 96) {
			return { r, g, b };
		}

		const boost = (value: number) => Math.round(value + (255 - value) * 0.45);
		return {
			r: boost(r),
			g: boost(g),
			b: boost(b)
		};
	}

	function loadArtworkImage(url: string): Promise<HTMLImageElement> {
		return new Promise((resolve, reject) => {
			const img = new Image();
			const cleanup = () => {
				img.onload = null;
				img.onerror = null;
			};

			img.crossOrigin = 'anonymous';
			img.decoding = 'async';
			img.onload = () => {
				cleanup();
				resolve(img);
			};
			img.onerror = () => {
				cleanup();
				reject(new Error(`Failed to load artwork image: ${url}`));
			};
			img.src = url;

			if (img.complete && img.naturalWidth > 0) {
				cleanup();
				resolve(img);
			}
		});
	}

	async function extractDominantColor(url: string): Promise<string | null> {
		try {
			const image = await loadArtworkImage(url);
			const size = 16;
			const canvas = document.createElement('canvas');
			canvas.width = size;
			canvas.height = size;
			const context = canvas.getContext('2d', { willReadFrequently: true });

			if (!context) {
				return null;
			}

			context.imageSmoothingEnabled = true;
			context.drawImage(image, 0, 0, size, size);

			let rTotal = 0;
			let gTotal = 0;
			let bTotal = 0;
			let count = 0;

			try {
				const { data } = context.getImageData(0, 0, size, size);

				for (let i = 0; i < data.length; i += 4) {
					const alpha = data[i + 3];
					if (alpha < 64) continue;
					rTotal += data[i];
					gTotal += data[i + 1];
					bTotal += data[i + 2];
					count += 1;
				}
			} catch (error) {
				console.warn('Unable to sample artwork color:', error);
				return null;
			} finally {
				canvas.width = 0;
				canvas.height = 0;
			}

			if (!count) {
				return null;
			}

			const r = Math.round(rTotal / count);
			const g = Math.round(gTotal / count);
			const b = Math.round(bTotal / count);
			const adjusted = adjustForContrast(r, g, b);

			return rgbToHex(adjusted.r, adjusted.g, adjusted.b);
		} catch (error) {
			console.warn('Failed to compute dominant artwork color:', error);
			return null;
		}
	}

	// Track ID for artwork caching
	let lastCapsuleTrackId = $state<string>('');

	function getCapsuleTrackId(media: MediaInfo | null): string {
		if (!media) return '';
		return `${media.title}_${media.artist}_${media.duration}`;
	}

	// Fetch media for capsule display
	async function fetchCapsuleMedia(): Promise<boolean> {
		if (isFetchingMedia) {
			return capsuleMedia?.is_playing ?? false;
		}
		isFetchingMedia = true;
		let isPlaying = capsuleMedia?.is_playing ?? false;
		try {
			const media = await invoke<MediaInfo | null>('get_current_media');
			capsuleMedia = media;
			isPlaying = !!media?.is_playing;

			if (media) {
				const currentTrackId = getCapsuleTrackId(media);
				const isNewTrack = currentTrackId !== lastCapsuleTrackId;

				// Fetch artwork if new track or no artwork yet
				if (isNewTrack || !capsuleArtwork) {
					lastCapsuleTrackId = currentTrackId;
					const artwork = await invoke<string | null>('get_media_artwork');
					if (artwork && (artwork.startsWith('http') || artwork.startsWith('data:image'))) {
						capsuleArtwork = artwork;
					} else {
						capsuleArtwork = null;
					}
				}
			} else {
				capsuleArtwork = null;
				lastCapsuleTrackId = '';
			}
			return isPlaying;
		} catch (error) {
			capsuleMedia = null;
			capsuleArtwork = null;
			lastCapsuleTrackId = '';
			return false;
		} finally {
			isFetchingMedia = false;
		}
	}

	async function ensureAccessibilityPermissions() {
		try {
			const trusted = await invoke<boolean>('ensure_accessibility', { prompt: false });
			if (!trusted) {
				await invoke<boolean>('ensure_accessibility', { prompt: true });
			}
		} catch (error) {
			console.warn('Accessibility permission check failed:', error);
		}
	}

	onMount(async () => {
		void ensureAccessibilityPermissions();

		const win = (await TauriWindow.getByLabel('notch-capsule')) ?? getCurrentWindow();
		windowInstance = win;

		window.addEventListener('pointermove', updatePointerState, { passive: true });
		window.addEventListener('pointerleave', handlePointerLeave, { passive: true });

		// Note: Mouse events don't require Accessibility permission (only key events do)

		// Dimensions
		let dims: NotchDimensions | null = null;
		try {
			dims = (await invoke('get_notch_dimensions')) as NotchDimensions | null;
		} catch {}
		if (dims && dims.width_pts > 0 && dims.top_inset_pts > 0) {
			// Extend the capsule width beyond the actual notch for better visibility
			notchWidth = Math.round(dims.width_pts * 1.5); // 50% wider when playing
			notchWidthNormal = Math.round(dims.width_pts * 0.9); // Normal width when not playing
			notchHeight = Math.round(dims.top_inset_pts); // Match notch height + small buffer
		} else {
			// Fallback to wider default
			notchWidth = 420;
			notchWidthNormal = 240;
			notchHeight = 37;
		}
		if (DEV_KEEP_NOTCH_EXPANDED) {
			notchExpanded = true;
			await win.setSize(new LogicalSize(EXPANDED_WIDTH, EXPANDED_HEIGHT));
			await moveWindow(Position.TopCenter);
			syncNativeExpanded(true);
			await updateCapsuleFocus(true);
		} else {
			await win.setSize(new LogicalSize(notchWidth, notchHeight));
			await moveWindow(Position.TopCenter);
			syncNativeExpanded(false);
			await updateCapsuleFocus(false);
		}

		// Try to attach native mask animator (macOS only)
		try {
			const targetWidth = capsuleMedia?.is_playing ? notchWidth : notchWidthNormal;
			await invoke('notch_attach', {
				label: 'notch-capsule',
				closedW: targetWidth,
				closedH: notchHeight,
				expandedW: EXPANDED_WIDTH,
				expandedH: EXPANDED_HEIGHT,
				corner: 14
			});
			nativeAnimatorAttached = true;
			console.log('Native mask animator attached');
		} catch (err) {
			console.log('Native animator not available (non-macOS or error):', err);
			nativeAnimatorAttached = false;
		}

		// Listen for native animation completion events
		unlistenNative = await listen<{ phase: string }>('notch-native-anim-end', ({ payload }) => {
			console.log('Native animation ended:', payload.phase);
			if (payload.phase === 'expand') {
				// Animation complete, ensure content is visible
				showCapsuleContent = true;
			} else if (payload.phase === 'collapse') {
				// Animation complete, re-render capsule and show content
				capsuleRenderKey++;
				tick().then(() => {
					showCapsuleContent = true;
				});
			}
		});

		// Listen for native hover (works even when window not focused)
		unlisten = await listen<{ inside: boolean }>('notch-hover', ({ payload }) => {
			const inside = !!payload?.inside;
			if (inside) {
				manualHold = false;
				pointerInExpanded = false;
				void openNotch();
			} else if (!DEV_KEEP_NOTCH_EXPANDED) {
				cancelScheduledOpen();
				requestAnimationFrame(() => {
					if (!(manualHold || pointerInExpanded)) {
						void closeNotch();
					}
				});
			}
		});

		// Fetch media for capsule
		const initiallyPlaying = await fetchCapsuleMedia();
		scheduleMediaPoll(initiallyPlaying ? MEDIA_POLL_ACTIVE_MS : MEDIA_POLL_IDLE_MS);
	});

	onDestroy(() => {
		if (unlisten) unlisten();
		if (unlistenNative) unlistenNative();
		clearMediaPoll();
		cancelScheduledOpen();
		cancelScheduledClose();
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
		if (capsuleHasFocus) {
			invoke('set_capsule_focus', { focus: false }).catch(() => {});
			capsuleHasFocus = false;
		}
	});
</script>

<div class="surface">
	<div class="drag-strip"></div>

	{#if notchExpanded}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="expanded-wrapper"
			bind:this={expandedEl}
			style={`--notch-mask:${notchMaskUri};`}
			onmouseenter={() => {
				if (DEV_KEEP_NOTCH_EXPANDED) return;
				manualHold = true;
				pointerInExpanded = true;
				cancelScheduledClose();
				void openNotch();
			}}
			onmouseleave={() => {
				if (DEV_KEEP_NOTCH_EXPANDED) return;
				manualHold = false;
				pointerInExpanded = false;
				cancelScheduledOpen();
				scheduleCloseOnLeave();
			}}
		>
			<NotchExpanded />
		</div>
	{:else}
		{#key capsuleRenderKey}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="capsule rounded-tab"
				bind:this={capsuleEl}
				style={`width:${toPx(capsuleMedia?.is_playing ? notchWidth : notchWidthNormal)}; height:${toPx(notchHeight)}; --notch-mask:${notchMaskUri};`}
				onpointerenter={() => {
					if (DEV_KEEP_NOTCH_EXPANDED) return;
					manualHold = true;
					if (capsuleEl) animateCapsuleHoverIn(capsuleEl);
					scheduleOpenOnHover();
				}}
				onpointerleave={() => {
					if (DEV_KEEP_NOTCH_EXPANDED) return;
					manualHold = false;
					pointerInExpanded = false;
					if (capsuleEl) animateCapsuleHoverOut(capsuleEl);
					cancelScheduledOpen();
					scheduleCloseOnLeave();
				}}
			>
				{#if capsuleMedia?.is_playing}
					{#if showCapsuleContent}
						<div class="capsule-content" class:morphing-out={capsuleFadingOut}>
							<!-- Artwork on the left -->
							<div class="capsule-artwork slide-in-left">
								{#if capsuleArtwork}
									<img src={capsuleArtwork} alt={capsuleMedia.title} class="artwork-image" />
								{:else}
									<div class="artwork-placeholder">🎵</div>
								{/if}
							</div>

							<!-- Waveform on the right -->
							<div class="capsule-letter slide-in-right">
								<Waveform color={capsuleWaveColor} />
							</div>
						</div>
					{/if}
				{:else if showCapsuleContent}
					<span class="label no-drag" class:morphing-out={capsuleFadingOut}>Notch Capsule</span>
				{/if}
			</div>
		{/key}
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
		background: transparent !important; /* transparent Tauri window */
		width: 100%;
		height: 100%;
		margin: 0;
		padding: 0;
		border: none !important;
		outline: none !important;
		overflow: hidden;
	}

	.surface {
		position: fixed; /* Fixed positioning to prevent shifts */
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: flex-start; /* Align to top to remove space */
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
		margin: 0; /* No margins */
		padding: 0;
		-webkit-app-region: no-drag; /* critical: hover target should NOT be draggable */
		pointer-events: auto;
		/* Smooth width transition for morphing */
		transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
		/* Maximum hardware acceleration for 60fps+ transitions */
		transform: translate3d(0, 0, 0);
		backface-visibility: hidden;
		perspective: 1000px;
		contain: layout style paint;
		/* Force GPU layer */
		isolation: isolate;
	}
	.drag-strip {
		position: fixed;
		top: -1px; /* Closer to top edge */
		left: 0;
		width: 100%;
		height: 4px; /* Thinner drag area */
		-webkit-app-region: drag;
		pointer-events: auto;
	}
	.no-drag {
		-webkit-app-region: no-drag;
	}

	.expanded-wrapper {
		width: 100%;
		height: 100%;
		display: flex;
		pointer-events: auto;
		padding: 2rem 2.8rem 2rem;
		padding-bottom: 0;
		background: #000000; /* True black */
		border: none !important;
		outline: none !important;
		box-shadow: none !important;
		transform-origin: top center;
		overflow: hidden;
		-webkit-mask-image: var(--notch-mask);
		mask-image: var(--notch-mask);
		-webkit-mask-repeat: no-repeat;
		mask-repeat: no-repeat;
		-webkit-mask-size: 100% 100%;
		mask-size: 100% 100%;
		-webkit-mask-position: center;
		mask-position: center;

		/* GPU-optimized properties only - no blur/filter */
		will-change: transform, opacity;

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
		transition:
			background-color 180ms ease-out,
			box-shadow 180ms ease-out;
	}

	/* === Notch styling driven by the supplied SVG === */
	.rounded-tab {
		position: relative;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 16px;
		background: #000000; /* True black */
		color: #f5f5f5;
		overflow: hidden;
		border: none !important;
		outline: none !important;
		box-shadow: none !important;
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
		font:
			500 11px/1.1 ui-sans-serif,
			system-ui,
			-apple-system,
			Segoe UI,
			Roboto,
			Inter,
			'Helvetica Neue',
			Arial,
			'Apple Color Emoji',
			'Segoe UI Emoji';
		color: #e5e5e5;
		user-select: none;
		pointer-events: none;
		opacity: 0.9;
		transition:
			opacity 0.15s ease-out,
			transform 0.15s ease-out;
	}

	/* Capsule content wrapper with flex */
	.capsule-content {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		height: 100%;
		padding: 0 2px;
		pointer-events: none;
		opacity: 1;
		transform: scale(1);
		transition:
			opacity 0.15s ease-out,
			transform 0.15s ease-out;
	}

	/* Morphing out animation */
	.capsule-content.morphing-out,
	.label.morphing-out {
		opacity: 0;
		transform: scale(0.96);
	}

	/* Capsule artwork on the left */
	.capsule-artwork {
		width: 22px;
		height: 22px;
		border-radius: 5px;
		overflow: hidden;
		background: rgba(255, 255, 255, 0.05);
		flex-shrink: 0;
	}

	.artwork-image {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.artwork-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 11px;
		opacity: 0.3;
	}

	/* Waveform container on the right */
	.capsule-letter {
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		user-select: none;
		flex-shrink: 0;
	}

	/* Make waveform smaller to fit in capsule */
	.capsule-letter :global(#wave) {
		width: 20px !important;
		height: 16px !important;
	}

	/* Sliding animations with bounce */
	.slide-in-left {
		animation: slideInLeft 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
	}

	.slide-in-right {
		animation: slideInRight 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
		animation-delay: 0.08s;
		animation-fill-mode: both;
	}

	@keyframes slideInLeft {
		from {
			opacity: 0;
			transform: translateX(-15px) scale(0.88);
		}
		to {
			opacity: 1;
			transform: translateX(0) scale(1);
		}
	}

	@keyframes slideInRight {
		from {
			opacity: 0;
			transform: translateX(15px) scale(0.88);
		}
		to {
			opacity: 1;
			transform: translateX(0) scale(1);
		}
	}
</style>
