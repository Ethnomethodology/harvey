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
	let waveformScrollDiv; // Ref for the scrollable div

	let visibleCanvasHeight = 0;
	let waveformCanvasWidth = 0;
	let webAudioApiSupported = true;
	let resizeObserverInstance;
	let isMounted = false;
	let animationFrameId = null;

	let scrollOffsetPy = 0;

	let zoomLevel = 1;
	const minZoomLevel = 1;
	const maxZoomLevel = 10;
	const zoomStep = 1.2;

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

	function timeToLogicalPy(time, mediaDuration, viewHeight) { // viewHeight is visibleCanvasHeight
		if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
		const contentLogicalHeight = viewHeight * zoomLevel;
		const proportion = Math.max(0, Math.min(1, time / mediaDuration));
		return proportion * contentLogicalHeight;
	}

	// pyToTime updated to include scrollOffsetPy
	function pyToTime(py, mediaDuration, viewHeight, currentScrollOffsetPy = 0) { // py is screen-relative
		if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
		const contentLogicalHeight = viewHeight * zoomLevel;
		const logicalPy = py + currentScrollOffsetPy; // Add scroll to screen-relative py
		const proportion = Math.max(0, Math.min(1, logicalPy / contentLogicalHeight));
		return proportion * mediaDuration;
	}

	function drawVerticalWaveform(ctx, buffer, peaksData, canvasClientHeight, canvasWidth, color) {
		// canvasClientHeight is visibleCanvasHeight
		if (!ctx || canvasClientHeight <= 0 || canvasWidth <= 0) return;
		if (!buffer && (!peaksData || peaksData.length === 0)) return;

		const midX = canvasWidth / 2;
		const isDark = document.documentElement.classList.contains('dark');
		// Consistent color with InteractiveWaveform.svelte's default waveform color
		ctx.strokeStyle = isDark ? '#9ca3af' : '#9ca3af';
		ctx.lineWidth = 1;

		const usePeaks = peaksData && peaksData.length > 0;
		// contentLogicalHeight is the total height the waveform data would occupy if laid out at current zoom.
		// We are drawing into canvasClientHeight.
		const contentLogicalHeight = canvasClientHeight * zoomLevel;

		if (usePeaks) {
			const numPeakBlocks = peaksData.length / 2;
			// peaksPerLogicalUnit: how many peak blocks fit into one unit of the contentLogicalHeight.
			// Or rather, how many data units (peak blocks) correspond to each logical pixel unit if contentLogicalHeight was 1.
			// This should be: data units per logical pixel on the zoomed canvas.
			const peaksPerLogicalUnitOfZoomedCanvas = numPeakBlocks / contentLogicalHeight;

			ctx.beginPath(); // Max Peaks
			for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen++) { // Iterate over screen pixels
				const logicalY_content = yPx_screen + scrollOffsetPy;
				const peakBlockStartIndex = Math.floor(logicalY_content * peaksPerLogicalUnitOfZoomedCanvas);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let maxPeak = 0.0;
				if (targetBlock >= 0 && targetBlock * 2 + 1 < peaksData.length) {
					maxPeak = peaksData[targetBlock * 2 + 1];
				}
				const xVal = midX + maxPeak * midX;
				if (yPx_screen === 0) ctx.moveTo(xVal, yPx_screen + 0.5);
				else ctx.lineTo(xVal, yPx_screen + 0.5);
			}
			ctx.stroke();

			ctx.beginPath(); // Min Peaks
			for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen++) {
				const logicalY_content = yPx_screen + scrollOffsetPy;
				const peakBlockStartIndex = Math.floor(logicalY_content * peaksPerLogicalUnitOfZoomedCanvas);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let minPeak = 0.0;
				if (targetBlock >= 0 && targetBlock * 2 < peaksData.length) {
					minPeak = peaksData[targetBlock * 2];
				}
				const xVal = midX + minPeak * midX;
				if (yPx_screen === 0) ctx.moveTo(xVal, yPx_screen + 0.5);
				else ctx.lineTo(xVal, yPx_screen + 0.5);
			}
			ctx.stroke();

		} else if (buffer) {
			const data = buffer.getChannelData(0);
			const totalSamples = data.length;
			const samplesPerLogicalUnitOfZoomedCanvas = totalSamples / contentLogicalHeight;

			ctx.beginPath(); // Max Envelope
			for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen++) {
				const logicalY_content = yPx_screen + scrollOffsetPy;
				const sampleStartIndex = Math.floor(logicalY_content * samplesPerLogicalUnitOfZoomedCanvas);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((logicalY_content + 1) * samplesPerLogicalUnitOfZoomedCanvas));

				let maxVal = 0;
				if (sampleStartIndex < sampleEndIndex && sampleStartIndex < totalSamples) {
					maxVal = data[sampleStartIndex];
					for (let i = sampleStartIndex + 1; i < sampleEndIndex; i++) {
						if (data[i] > maxVal) maxVal = data[i];
					}
				} else if (sampleStartIndex < totalSamples) {
					maxVal = data[sampleStartIndex];
				}
				const xVal = midX + maxVal * midX;
				if (yPx_screen === 0) ctx.moveTo(xVal, yPx_screen + 0.5);
				else ctx.lineTo(xVal, yPx_screen + 0.5);
			}
			ctx.stroke();

			ctx.beginPath(); // Min Envelope
			for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen++) {
				const logicalY_content = yPx_screen + scrollOffsetPy;
				const sampleStartIndex = Math.floor(logicalY_content * samplesPerLogicalUnitOfZoomedCanvas);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((logicalY_content + 1) * samplesPerLogicalUnitOfZoomedCanvas));
				let minVal = 0;
				if (sampleStartIndex < sampleEndIndex && sampleStartIndex < totalSamples) {
					minVal = data[sampleStartIndex];
					for (let i = sampleStartIndex + 1; i < sampleEndIndex; i++) {
						if (data[i] < minVal) minVal = data[i];
					}
				} else if (sampleStartIndex < totalSamples) {
					minVal = data[sampleStartIndex];
				}
				const xVal = midX + minVal * midX;
				if (yPx_screen === 0) ctx.moveTo(xVal, yPx_screen + 0.5);
				else ctx.lineTo(xVal, yPx_screen + 0.5);
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
		const pyCur_logical = timeToLogicalPy(cur, mediaDur, visibleCanvasHeight); // Position on the full logical (zoomed) canvas
		const pyCurOnScreen = pyCur_logical - scrollOffsetPy; // Subtract scroll offset to get screen position

		if (pyCurOnScreen >= -1 && pyCurOnScreen <= visibleCanvasHeight + 1) { // Check if visible on screen
			ctx.save();
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0); // Reset transform, apply only DPR for crisp line

			ctx.strokeStyle = '#ef4444';
			ctx.lineWidth = 1; // 1 CSS pixel line

			const finalLineY = Math.round(pyCurOnScreen) + 0.5; // Use the on-screen Y

			ctx.beginPath();
			ctx.moveTo(0, finalLineY);
			ctx.lineTo(waveformCanvasWidth, finalLineY); // waveformCanvasWidth is in CSS pixels
			ctx.stroke();

			ctx.restore();
		}
		ctx.restore(); // Outer restore for initial dpr scaling

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

	function handleScrollDivClick(event) {
		const mediaDur = duration;
		if (!waveformScrollDiv || (!audioBuffer && !$transcriptStore.audioBufferPeaks) || mediaDur <= 0) return;

		const rect = event.currentTarget.getBoundingClientRect(); // event.currentTarget is waveformScrollDiv
		const clickY_in_viewport = event.clientY - rect.top;

		const logicalClickY = waveformScrollDiv.scrollTop + clickY_in_viewport;
		const contentTotalScrollHeight = waveformScrollDiv.scrollHeight;

		if (contentTotalScrollHeight <= 0) return;

		const proportion = Math.max(0, Math.min(1, logicalClickY / contentTotalScrollHeight));
		const time = proportion * mediaDur;

		dispatch('navigate', { time: time });
	}

	function handleZoom(direction) {
		if (!audioBuffer && !$transcriptStore.audioBufferPeaks) return;
		let newZoomLevel = zoomLevel;
		if (direction === 'in') {
			newZoomLevel = zoomLevel * zoomStep;
		} else {
			newZoomLevel = zoomLevel / zoomStep;
		}
		newZoomLevel = Math.max(minZoomLevel, Math.min(maxZoomLevel, newZoomLevel));
		if (Math.abs(newZoomLevel - zoomLevel) > 0.001) {
			zoomLevel = newZoomLevel;
			resetScrollAndZoom(false); // Don't reset zoomLevel again, just scroll
		}
	}

	function zoomIn() { handleZoom('in'); }
	function zoomOut() { handleZoom('out'); }

	function handleWaveformScroll(event) {
		if (event.target) {
			const newScrollOffsetPy = Math.round(event.target.scrollTop);
			if (Math.abs(newScrollOffsetPy - scrollOffsetPy) > 0) {
				scrollOffsetPy = newScrollOffsetPy;
				requestRedraw();
			}
		}
	}

	function resetScrollAndZoom(resetZoomToo = true) {
		if (resetZoomToo) {
			zoomLevel = 1;
		}
		scrollOffsetPy = 0;
		if (waveformScrollDiv) {
			waveformScrollDiv.scrollTop = 0;
		}
		requestRedraw(true);
	}

    // Watch for prop changes to force redraw
    let prevAudioBuffer = audioBuffer;
    let prevDuration = duration;
    let prevStorePeaks = $transcriptStore.audioBufferPeaks; // For store-based changes

    $: if (isMounted) {
        let resetNeeded = false;
        if (audioBuffer !== prevAudioBuffer) {
            resetNeeded = true;
            prevAudioBuffer = audioBuffer;
        }
        if (duration !== prevDuration) {
            resetNeeded = true;
            prevDuration = duration;
        }
        if (!audioBuffer && $transcriptStore.audioBufferPeaks !== prevStorePeaks) {
            resetNeeded = true;
            prevStorePeaks = $transcriptStore.audioBufferPeaks;
        }

        if (resetNeeded) {
            resetScrollAndZoom(true); // Full reset including zoom
        } else {
            const currentEffectiveBufferOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
            if (currentEffectiveBufferOrPeaks !== lastDrawnBufferOrPeaks) {
                 lastDrawnBufferOrPeaks = currentEffectiveBufferOrPeaks;
                 requestRedraw(true);
            } else {
                 requestRedraw(false);
            }
        }
    }


    // Separate watcher for currentTime to ensure smooth updates via animation loop
    $: if (isMounted && Math.abs(currentTime - lastDrawnTime) > redrawTimeThreshold / 2) {
        if (animationFrameId === null) {
             animationFrameId = requestAnimationFrame(animationLoop);
        }
    }

	$: canZoomIn = isMounted && zoomLevel < maxZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);
	$: canZoomOut = isMounted && zoomLevel > minZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);

</script>

<div bind:this={componentContainer} class="vertical-waveform-panel flex flex-col w-full h-full bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded overflow-hidden">
	<div class="flex-shrink-0 px-2 py-1.5 flex items-center justify-end space-x-1.5 border-b border-gray-300 dark:border-gray-600 w-full">
		<button class="ui-button-icon-panelheader" title="Zoom In Waveform" aria-label="Zoom In Waveform" on:click={zoomIn} disabled={!canZoomIn}>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM10.5 7.5v6m3-3h-6" />
			</svg>
		</button>
		<button class="ui-button-icon-panelheader" title="Zoom Out Waveform" aria-label="Zoom Out Waveform" on:click={zoomOut} disabled={!canZoomOut}>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM13.5 10.5h-6" />
			</svg>
		</button>
	</div>

	<!-- Existing Waveform and Timescale Area -->
	<div bind:this={waveformAreaContainerRef} class="flex flex-grow min-h-0 relative">
		<canvas bind:this={timescaleCanvas} class="timescale-canvas-vertical shrink-0" style="width: {TIMESCALE_WIDTH}px; height: 100%;" aria-hidden="true"></canvas>
		<div
			bind:this={waveformScrollDiv}
			class="waveform-scroll-container flex-grow h-full relative min-w-0 overflow-y-auto"
			on:scroll={handleWaveformScroll}
			on:click={handleScrollDivClick}
		>
			<canvas
				bind:this={waveformCanvas}
				class="waveform-canvas-vertical w-full cursor-pointer"
				aria-label="Vertical waveform visualization. Click to seek audio."
				<!-- on:click removed -->
				style="height: {visibleCanvasHeight * zoomLevel}px;"
			></canvas>
			{#if !webAudioApiSupported && isMounted}
				<div class="overlay-message"><p>Web Audio API not supported.</p></div>
			{:else if !audioBuffer && !$transcriptStore.audioBufferPeaks && isMounted}
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
	.ui-button-icon-panelheader { /* Standardized button style for panel headers */
		@apply p-1 rounded text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-gray-100 dark:disabled:hover:bg-gray-700;
	}

	.waveform-scroll-container {
		scrollbar-width: thin; /* Firefox */
		scrollbar-color: transparent transparent; /* Firefox */
	}
	.waveform-scroll-container:hover {
		scrollbar-color: #a0aec0 #e2e8f0; /* Firefox on hover */
	}
	.dark .waveform-scroll-container:hover {
		scrollbar-color: #6b7280 #3c3c3c; /* Firefox dark on hover */
	}
	.waveform-scroll-container::-webkit-scrollbar {
		width: 8px;
		height: 8px;
	}
	.waveform-scroll-container::-webkit-scrollbar-track {
		background: transparent;
	}
	.waveform-scroll-container::-webkit-scrollbar-thumb {
		background-color: transparent;
		border-radius: 4px;
	}
	.waveform-scroll-container:hover::-webkit-scrollbar-thumb {
		background-color: #a0aec0; /* Tailwind gray-400 */
	}
	.dark .waveform-scroll-container:hover::-webkit-scrollbar-thumb {
		background-color: #4a5568; /* Tailwind gray-600 dark */
	}
	.waveform-scroll-container:hover::-webkit-scrollbar-track {
		background: #e2e8f0; /* Tailwind gray-200 */
	}
	.dark .waveform-scroll-container:hover::-webkit-scrollbar-track {
		background: #3c3c3c;
	}
</style>
