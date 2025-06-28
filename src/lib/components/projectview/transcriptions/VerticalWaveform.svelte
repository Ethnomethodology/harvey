<script>
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js'; // For types if needed, or direct access

	export let audioBuffer = null;
	export let currentTime = 0;
	export let duration = 0;

	const TIMESCALE_WIDTH = 35; // For vertical timescale, increased for padding
	let waveformCanvas;
	let timescaleCanvas;
	let componentContainer; // Outermost container ref
	let waveformAreaContainerRef; // Ref for the actual waveform drawing area

	let visibleCanvasHeight = 0;
	let waveformCanvasWidth = 0;
	let webAudioApiSupported = true;
	let resizeObserverInstance;
	let isMounted = false;
	let animationFrameId = null;

	let lastDrawnTime = -1;
	let lastDrawnBufferOrPeaks = null; // Combined check for buffer or peaks
	let lastDrawnActualDuration = -1;
	const redrawTimeThreshold = 1 / 60; // 60 FPS

	const dispatch = createEventDispatcher();

	function formatTimescaleTimeVertical(sec, totalDuration) {
		if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '0:00';
		const tot = Math.floor(sec);
		const minutes = Math.floor(tot / 60);
		const seconds = tot % 60;
		// Display H:MM:SS if duration is very long, else M:SS
		if (totalDuration >= 3600) {
			const hours = Math.floor(minutes / 60);
			const remainingMinutes = minutes % 60;
			return `${hours}:${String(remainingMinutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
		}
		return `${String(minutes)}:${String(seconds).padStart(2, '0')}`;
	}

	function timeToLogicalPy(time, mediaDuration, logicalHeight) {
		if (!mediaDuration || mediaDuration <= 0 || !logicalHeight || logicalHeight <= 0) return 0;
		const proportion = Math.max(0, Math.min(1, time / mediaDuration));
		return proportion * logicalHeight;
	}

	function pyToTime(py, mediaDuration, logicalHeight) {
		if (!mediaDuration || mediaDuration <= 0 || !logicalHeight || logicalHeight <= 0) return 0;
		const proportion = Math.max(0, Math.min(1, py / logicalHeight));
		return proportion * mediaDuration;
	}

	function drawVerticalWaveform(ctx, buffer, peaksData, logicalHeight, canvasWidth, color) {
		if (!ctx || logicalHeight <= 0 || canvasWidth <= 0) return;
		if (!buffer && (!peaksData || peaksData.length === 0)) return;

		const midX = canvasWidth / 2;
		const isDark = document.documentElement.classList.contains('dark');
		// Consistent color with InteractiveWaveform.svelte's default waveform color
		ctx.strokeStyle = isDark ? '#9ca3af' : '#9ca3af'; // Tailwind gray-400 for both, or choose specific dark/light variants
		ctx.lineWidth = 1;

		const usePeaks = peaksData && peaksData.length > 0;

		if (usePeaks) {
			const numPeakBlocks = peaksData.length / 2;
			const peaksPerLogicalPixelY = numPeakBlocks / logicalHeight;

			// Draw Max Peaks (e.g., top line in horizontal, right line in vertical)
			ctx.beginPath();
			for (let yPx = 0; yPx < logicalHeight; yPx++) {
				const peakBlockStartIndex = Math.floor(yPx * peaksPerLogicalPixelY);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let maxPeak = 0.0;
				if (targetBlock * 2 + 1 < peaksData.length) {
					maxPeak = peaksData[targetBlock * 2 + 1];
				}
				const xVal = midX + maxPeak * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5);
				else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();

			// Draw Min Peaks (e.g., bottom line in horizontal, left line in vertical)
			ctx.beginPath();
			for (let yPx = 0; yPx < logicalHeight; yPx++) {
				const peakBlockStartIndex = Math.floor(yPx * peaksPerLogicalPixelY);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let minPeak = 0.0;
				if (targetBlock * 2 < peaksData.length) {
					minPeak = peaksData[targetBlock * 2];
				}
				const xVal = midX + minPeak * midX; // minPeak is negative
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5);
				else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();

		} else if (buffer) { // Fallback to raw data
			const data = buffer.getChannelData(0);
			const totalSamples = data.length;
			const samplesPerLogicalPixelY = totalSamples / logicalHeight;

			// Draw Max Envelope
			ctx.beginPath();
			for (let yPx = 0; yPx < logicalHeight; yPx++) {
				const sampleStartIndex = Math.floor(yPx * samplesPerLogicalPixelY);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((yPx + 1) * samplesPerLogicalPixelY));
				let maxVal = 0;
				if (sampleStartIndex < sampleEndIndex) {
					maxVal = data[sampleStartIndex];
					for (let i = sampleStartIndex + 1; i < sampleEndIndex; i++) {
						if (data[i] > maxVal) maxVal = data[i];
					}
				}
				const xVal = midX + maxVal * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5);
				else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();

			// Draw Min Envelope
			ctx.beginPath();
			for (let yPx = 0; yPx < logicalHeight; yPx++) {
				const sampleStartIndex = Math.floor(yPx * samplesPerLogicalPixelY);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((yPx + 1) * samplesPerLogicalPixelY));
				let minVal = 0;
				if (sampleStartIndex < sampleEndIndex) {
					minVal = data[sampleStartIndex];
					for (let i = sampleStartIndex + 1; i < sampleEndIndex; i++) {
						if (data[i] < minVal) minVal = data[i];
					}
				}
				const xVal = midX + minVal * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5);
				else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();
		}
	}


	function clearWaveformCanvases() {
		if (waveformCanvas) {
			const ctx = waveformCanvas.getContext('2d');
			if (ctx) ctx.clearRect(0, 0, waveformCanvas.width, waveformCanvas.height);
		}
		if (timescaleCanvas) {
			const ctx = timescaleCanvas.getContext('2d');
			if (ctx) ctx.clearRect(0, 0, timescaleCanvas.width, timescaleCanvas.height);
		}
		if (waveformCanvas && waveformCanvasWidth > 0 && visibleCanvasHeight > 0) {
			const ctx = waveformCanvas.getContext('2d');
			if (ctx) {
				const dpr = window.devicePixelRatio || 1;
				ctx.save();
				ctx.scale(dpr, dpr);
				ctx.fillStyle = '#6b7280'; // Tailwind gray-500
				ctx.font = `10px sans-serif`;
				ctx.textAlign = 'center';
				ctx.textBaseline = 'middle';
				let message = 'Vertical Waveform';
				if (!webAudioApiSupported) message = 'Web Audio API not supported.';
				else if (!audioBuffer && !$transcriptStore.audioBufferPeaks) message = 'Load media for waveform.';
				// Adjust text position for vertical canvas
				ctx.fillText(message, waveformCanvasWidth / 2 / dpr, visibleCanvasHeight / 2 / dpr); // Account for DPR
				ctx.restore();
			}
		}
	}

	function drawVerticalTimescale() {
		const mediaDur = duration;
		const bufOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
		const dpr = window.devicePixelRatio || 1;

		if (!timescaleCanvas || !bufOrPeaks || mediaDur <= 0 || visibleCanvasHeight <= 0 || TIMESCALE_WIDTH <= 0) {
			if (timescaleCanvas) { timescaleCanvas.width = 0; timescaleCanvas.height = 0;}
			return;
		}
		const ctx = timescaleCanvas.getContext('2d');
		if (!ctx) return;

		const reqW = Math.round(TIMESCALE_WIDTH * dpr);
		const reqH = Math.round(visibleCanvasHeight * dpr);
		if (timescaleCanvas.width !== reqW || timescaleCanvas.height !== reqH) {
			timescaleCanvas.width = reqW;
			timescaleCanvas.height = reqH;
		}
		ctx.save();
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, TIMESCALE_WIDTH, visibleCanvasHeight);

		const isDark = document.documentElement.classList.contains('dark');
		ctx.strokeStyle = '#d1d5db'; // Tailwind gray-300
		ctx.fillStyle = isDark ? '#ffffff' : '#6b7280'; // Tailwind gray-500 or white
		ctx.font = '10px sans-serif';
		ctx.textAlign = 'right';
		ctx.textBaseline = 'middle';

		const minPixelSpacingForLabel = 30; // Vertical spacing
		const intervals = [0.1, 0.5, 1, 5, 10, 30, 60, 300, 600, 1800, 3600];
		let interval = intervals[0];
		let intervalPy = timeToLogicalPy(interval, mediaDur, visibleCanvasHeight);

		for (let i = 0; i < intervals.length; i++) {
			const currentIntervalPy = timeToLogicalPy(intervals[i], mediaDur, visibleCanvasHeight);
			if (currentIntervalPy >= minPixelSpacingForLabel) {
				interval = intervals[i];
				intervalPy = currentIntervalPy;
				break;
			}
			if (i === intervals.length - 1) { // Fallback to largest if none meet criteria
				interval = intervals[i];
				intervalPy = currentIntervalPy;
			}
		}

		const firstMajorTickTime = 0; // Start from 0 for vertical

		for (let time = firstMajorTickTime; time <= mediaDur; time += interval) {
			if (time < 0) continue;
			const py = timeToLogicalPy(time, mediaDur, visibleCanvasHeight);
			if (py >= 0 && py <= visibleCanvasHeight) {
				ctx.beginPath();
				ctx.moveTo(TIMESCALE_WIDTH - (time % (interval * 5) < 0.0001 && interval >=1 ? 7 : 5), py + 0.5);
				ctx.lineTo(TIMESCALE_WIDTH, py + 0.5);
				ctx.stroke();

				const labelStr = formatTimescaleTimeVertical(time, mediaDur);
				const textPadding = 5;
				if (py - (ctx.measureText(labelStr).actualBoundingBoxAscent / 2) >= 0 &&
					py + (ctx.measureText(labelStr).actualBoundingBoxDescent / 2) <= visibleCanvasHeight) {
					ctx.fillText(labelStr, TIMESCALE_WIDTH - textPadding - 2, py);
				}
			}
		}
		ctx.beginPath();
		ctx.moveTo(TIMESCALE_WIDTH - 0.5, 0);
		ctx.lineTo(TIMESCALE_WIDTH - 0.5, visibleCanvasHeight);
		ctx.strokeStyle = '#d1d5db'; // Tailwind gray-300
		ctx.lineWidth = 1;
		ctx.stroke();
		ctx.restore();
	}

	function drawWaveformUI() {
		const buf = audioBuffer;
		const peaks = $transcriptStore.audioBufferPeaks; // Use peaks from store for vertical waveform
		const cur = currentTime;
		const mediaDur = duration;
		const dpr = window.devicePixelRatio || 1;

		if (!waveformCanvas || (!buf && !peaks) || mediaDur <= 0 || visibleCanvasHeight <= 0 || waveformCanvasWidth <= 0) {
			if (waveformCanvas) {
                const c = waveformCanvas.getContext('2d');
                if(c) c.clearRect(0, 0, waveformCanvas.width, waveformCanvas.height);
				waveformCanvas.width = 0; waveformCanvas.height = 0;
            }
			return;
		}

		const ctx = waveformCanvas.getContext('2d');
		if (!ctx) return;

		const reqW = Math.round(waveformCanvasWidth * dpr);
		const reqH = Math.round(visibleCanvasHeight * dpr);
		if (waveformCanvas.width !== reqW || waveformCanvas.height !== reqH) {
			waveformCanvas.width = reqW;
			waveformCanvas.height = reqH;
		}

		ctx.save();
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, waveformCanvasWidth, visibleCanvasHeight);

		if ((buf || peaks) && visibleCanvasHeight > 0) {
			// For vertical, logicalHeight is visibleCanvasHeight (no zoom)
			drawVerticalWaveform(ctx, buf, peaks, visibleCanvasHeight, waveformCanvasWidth, '#9ca3af'); // Tailwind gray-400
		}

		// Draw red seek bar
		const pyCur = timeToLogicalPy(cur, mediaDur, visibleCanvasHeight);
		if (pyCur >= -1 && pyCur <= visibleCanvasHeight + 1) {
			ctx.strokeStyle = '#ef4444'; // Tailwind red-500
			ctx.lineWidth = 1.5;
			ctx.beginPath();
			ctx.moveTo(0, pyCur + 0.5);
			ctx.lineTo(waveformCanvasWidth, pyCur + 0.5);
			ctx.stroke();
		}
		ctx.restore();

		lastDrawnTime = cur;
		lastDrawnBufferOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
		lastDrawnActualDuration = mediaDur;
	}

	let forceNextRedraw = false;
	function requestRedraw(force = false) {
		if (force) forceNextRedraw = true;
		if (isMounted) {
            // The animationLoop will call the draw functions.
            // We just need to ensure it's running if a redraw is requested.
            if (animationFrameId === null) {
                animationFrameId = requestAnimationFrame(animationLoop);
            }
		}
	}

	function animationLoop() {
        if (!isMounted) {
            animationFrameId = null; // Stop loop if unmounted
            return;
        }
		const cur = currentTime;
		const mediaDur = duration;
		const currentBufOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;

		let needsDraw = forceNextRedraw ||
			(currentBufOrPeaks !== lastDrawnBufferOrPeaks) ||
			(Math.abs(cur - lastDrawnTime) > redrawTimeThreshold) ||
			(mediaDur !== lastDrawnActualDuration);

		forceNextRedraw = false;

		if (needsDraw && visibleCanvasHeight > 0 && currentBufOrPeaks && mediaDur > 0) {
			drawVerticalTimescale(); // Timescale might depend on duration or height
			drawWaveformUI();
		} else if (needsDraw) { // Conditions for drawing not met (e.g. no buffer, no duration)
            clearWaveformCanvases();
            lastDrawnTime = cur;
            lastDrawnBufferOrPeaks = currentBufOrPeaks;
            lastDrawnActualDuration = mediaDur;
        }
		animationFrameId = requestAnimationFrame(animationLoop);
	}

	onMount(() => {
		isMounted = true;
		webAudioApiSupported = typeof window.AudioContext !== 'undefined' || typeof window.webkitAudioContext !== 'undefined';

		tick().then(() => {
			if (isMounted && waveformAreaContainerRef) { // Changed to waveformAreaContainerRef
				setupResizeObserver(); // Call setup
                // Initial size update now happens inside setupResizeObserver or its subsequent tick
			}
		});
        // No need to subscribe to transcriptStore if props are passed down
	});

	onDestroy(() => {
		isMounted = false;
		if (resizeObserverInstance) {
			resizeObserverInstance.disconnect();
			resizeObserverInstance = null;
		}
		if (animationFrameId) {
			cancelAnimationFrame(animationFrameId);
			animationFrameId = null;
		}
		lastDrawnTime = -1;
		lastDrawnBufferOrPeaks = null;
		lastDrawnActualDuration = -1;
	});

	function setupResizeObserver() {
		if (waveformAreaContainerRef && !resizeObserverInstance && isMounted && typeof window !== 'undefined' && window.ResizeObserver) {
			resizeObserverInstance = new ResizeObserver(entries => {
				for (const entry of entries) {
					if (entry.target === waveformAreaContainerRef) { // Observe waveformAreaContainerRef
						const newHeight = Math.max(0, entry.contentRect.height);
						const newWidth = Math.max(0, entry.contentRect.width);
						let changed = false;

						if (newHeight !== visibleCanvasHeight) { // Use newHeight directly
							visibleCanvasHeight = newHeight;
							changed = true;
						}

						const newWaveformCanvasWidth = Math.max(0, newWidth - TIMESCALE_WIDTH);
						if (newWaveformCanvasWidth !== waveformCanvasWidth) {
							waveformCanvasWidth = newWaveformCanvasWidth;
							changed = true;
						}

						if (changed) {
							requestRedraw(true); // Force redraw on resize
						}
					}
				}
			});
			resizeObserverInstance.observe(waveformAreaContainerRef); // Observe waveformAreaContainerRef

            // Initial size update more reliably after observer is set up and element is surely in DOM
            tick().then(() => {
                if(waveformAreaContainerRef) {
                    visibleCanvasHeight = Math.max(0, waveformAreaContainerRef.clientHeight);
                    waveformCanvasWidth = Math.max(0, waveformAreaContainerRef.clientWidth - TIMESCALE_WIDTH);
                    requestRedraw(true);
                }
            });
		}
	}

	function handleCanvasClick(e) {
		const mediaDur = duration;
		if (!waveformCanvas || !audioBuffer || mediaDur <= 0 || visibleCanvasHeight <= 0) return;

		const rect = waveformCanvas.getBoundingClientRect();
		const clickY = e.clientY - rect.top;

		const time = pyToTime(clickY, mediaDur, visibleCanvasHeight);
		dispatch('navigate', { time: time });
	}

    // Watch for prop changes to force redraw
    $: if (isMounted &&
        (
            (audioBuffer !== lastDrawnBufferOrPeaks && !$transcriptStore.audioBufferPeaks) || // audioBuffer changed
            ($transcriptStore.audioBufferPeaks && $transcriptStore.audioBufferPeaks !== lastDrawnBufferOrPeaks) || // peaks changed
            (audioBuffer && $transcriptStore.audioBufferPeaks && (audioBuffer !== lastDrawnBufferOrPeaks && $transcriptStore.audioBufferPeaks !== lastDrawnBufferOrPeaks)) || // both available and one changed
            duration !== lastDrawnActualDuration
        )
    ) {
        lastDrawnBufferOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
        requestRedraw(true);
    }

    // Separate watcher for currentTime to ensure smooth updates via animation loop
    $: if (isMounted && Math.abs(currentTime - lastDrawnTime) > redrawTimeThreshold / 2) { // A bit more sensitive for time
        // The animation loop handles drawing if lastDrawnTime is different enough
        // We just need to ensure the loop is running if not already.
        if (animationFrameId === null) {
             animationFrameId = requestAnimationFrame(animationLoop);
        }
    }

</script>

<div bind:this={componentContainer} class="vertical-waveform-panel flex flex-col w-full h-full bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded overflow-hidden">
	<!-- New Header Section -->
	<div class="h-[calc(1.75rem+1px)] flex-shrink-0 px-3 flex items-center border-b border-gray-300 dark:border-gray-600">
		<span class="text-xs font-medium text-gray-700 dark:text-gray-200 select-none">Waveform</span>
	</div>

	<!-- Existing Waveform and Timescale Area -->
	<div bind:this={waveformAreaContainerRef} class="flex flex-grow min-h-0">
		<canvas bind:this={timescaleCanvas} class="timescale-canvas-vertical shrink-0" style="width: {TIMESCALE_WIDTH}px; height: 100%;" aria-hidden="true"></canvas>
		<div class="waveform-canvas-container flex-grow h-full relative min-w-0">
			<canvas
				bind:this={waveformCanvas}
			class="waveform-canvas-vertical w-full h-full cursor-pointer"
			aria-label="Vertical waveform visualization. Click to seek audio."
			on:click={handleCanvasClick}
		></canvas>
		{#if !webAudioApiSupported}
			<div class="overlay-message"><p>Web Audio API not supported.</p></div>
		{:else if !audioBuffer && !$transcriptStore.audioBufferPeaks}
			<div class="overlay-message"><p>Load audio/video media for waveform.</p></div>
		{/if}
	</div>
	</div>
</div>

<style lang="postcss">
	.vertical-waveform-panel {
		/* Basic styling for the panel itself */
	}
	.timescale-canvas-vertical {
		display: block;
		/* background-color: #f0f0f0; dark mode? */
	}
	.waveform-canvas-container {
		/* Container for the waveform canvas if needed for layout */
	}
	.waveform-canvas-vertical {
		display: block;
	}
	.overlay-message {
		@apply absolute inset-0 flex items-center justify-center text-xs p-1 bg-white/80 dark:bg-gray-900/80 text-gray-600 dark:text-gray-300 pointer-events-none;
		text-align: center;
	}
</style>
