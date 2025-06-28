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

	let zoomLevel = 1; // Initial zoom level
	const minZoomLevel = 1; // Most zoomed-out
	const maxZoomLevel = 10; // Most zoomed-in
	const zoomStep = 1.2; // Factor for zooming in/out

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

	// Updated timeToLogicalPy to consider zoom
	function timeToLogicalPy(time, mediaDuration, viewHeight) {
		if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
		const logicalHeight = viewHeight * zoomLevel;
		const proportion = Math.max(0, Math.min(1, time / mediaDuration));
		return proportion * logicalHeight;
	}

	// Updated pyToTime to consider zoom (py is within visible canvas, needs to be mapped to logical)
	function pyToTime(py, mediaDuration, viewHeight, scrollOffsetPy = 0) {
		if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
		const logicalHeight = viewHeight * zoomLevel;
		const logicalPy = py + scrollOffsetPy; // Assuming scrollOffsetPy is 0 for vertical scroll for now
		const proportion = Math.max(0, Math.min(1, logicalPy / logicalHeight));
		return proportion * mediaDuration;
	}

	// drawVerticalWaveform now uses visibleCanvasHeight (passed as canvasClientHeight) and zoomLevel
	function drawVerticalWaveform(ctx, buffer, peaksData, canvasClientHeight, canvasWidth, color) {
		if (!ctx || canvasClientHeight <= 0 || canvasWidth <= 0) return;
		if (!buffer && (!peaksData || peaksData.length === 0)) return;

		// logicalHeight is the total height the waveform data would occupy if drawn at zoomLevel 1,
		// then scaled by current zoomLevel.
		// However, for drawing, we iterate over visible pixels (canvasClientHeight)
		// and map them to the source data.
		// The source data (buffer or peaks) represents the entire duration.
		// We want to draw this entire duration stretched or compressed into logicalHeight,
		// and then view a 'window' of this logicalHeight that is `canvasClientHeight` tall.
		// For now, no scrolling, so the window is the entire logicalHeight scaled to fit canvasClientHeight.

		const midX = canvasWidth / 2;
		const isDark = document.documentElement.classList.contains('dark');
		ctx.strokeStyle = isDark ? '#9ca3af' : '#9ca3af';
		ctx.lineWidth = 1;

		const usePeaks = peaksData && peaksData.length > 0;

		// The loop iterates over each vertical pixel line on the canvas.
		for (let yPx = 0; yPx < canvasClientHeight; yPx++) {
			// Calculate what proportion of the total duration this pixel represents.
			// If zoomLevel is 1, this pixel yPx corresponds to data at (yPx / canvasClientHeight) of total duration.
			// If zoomLevel > 1, we are "zoomed in". The visible part of the waveform is still canvasClientHeight,
			// but it represents a smaller portion of the total duration.
			// Let effectiveLogicalHeight be the height of the data segment we want to display in canvasClientHeight.
			// If no vertical scrolling of zoom, effectiveLogicalHeight = total_duration_logical_height / zoomLevel.
			// This means we are showing the whole waveform but squashed more if zoom level is higher. This is opposite of horizontal.
			// For vertical zoom: zoomLevel > 1 means we see LESS of the waveform, magnified.
			// So, the logical "total height" of the data is fixed by its duration.
			// `zoomLevel` determines how much of this logical height is shown in `canvasClientHeight`.
			// A scroll offset `scrollOffsetPy` would determine *which part* is shown.
			// For now, assume scrollOffsetPy = 0 (top of waveform is shown).

			// `sampleProportion` is the point in the normalized data (0 to 1) that corresponds to yPx on canvas.
			// It needs to account for zoom and a (currently zero) scroll offset.
			// The total logical height of the waveform data is `canvasClientHeight * zoomLevel`.
			// We are viewing a window of size `canvasClientHeight` from this total logical height.
			// Let `currentLogicalY` be the y-coordinate in the full logical waveform.
			// currentLogicalY = yPx + scrollOffsetPy (where scrollOffsetPy is 0 for now).
			// `sampleProportion` is currentLogicalY / (canvasClientHeight * zoomLevel)
			// This means yPx on screen maps to (yPx / (canvasClientHeight * zoomLevel)) if no scroll.
			// If zoomLevel = 1, sampleProportion = yPx / canvasClientHeight. (Shows full waveform)
			// If zoomLevel = 2, sampleProportion = yPx / (canvasClientHeight * 2). (Shows top half of waveform, magnified 2x)

			// Correct mapping:
			// The visible portion of the logical waveform spans from scrollOffsetPy to scrollOffsetPy + canvasClientHeight.
			// Let logicalCanvasHeight = canvasClientHeight * zoomLevel (this is the full height of the zoomable content)
			// The `yPx` on screen corresponds to a logical coordinate `logicalY = yPx_on_visible_window + scrollOffsetPy`.
			// For now, `scrollOffsetPy` is 0 and `yPx_on_visible_window` is just `yPx`.
			// The proportion of the total *duration* this `logicalY` represents is `logicalY / logicalCanvasHeight`.
			// This `sampleProportion` should then be used to index into `peaksData` or `audioBuffer`.

			// Let's simplify: the `drawVerticalWaveform` function is always given `canvasClientHeight` which is `visibleCanvasHeight`.
			// The `zoomLevel` applies to this. So, the total "logical extent" of the data is `visibleCanvasHeight * zoomLevel`.
			// We are currently *not* implementing scrolling within this zoomed logical extent.
			// So, we are always viewing the segment from `0` to `visibleCanvasHeight` of this logical extent.
			// This means we effectively scale the *entire* waveform into `visibleCanvasHeight / zoomLevel` and then draw that.
			// This is the opposite of typical zoom and needs to be re-thought if standard zoom behavior is expected.

			// Sticking to the current interpretation: `zoomLevel=1` shows the whole waveform.
			// `zoomLevel > 1` means we are "zoomed out" further (seeing more, smaller). This is not standard.
			// Let's assume standard: `zoomLevel > 1` means "zoomed in" (seeing less, magnified).
			// If `zoomLevel = 1`, logicalHeight = canvasClientHeight.
			// If `zoomLevel = 2`, logicalHeight = canvasClientHeight * 2. We only see half of it unless we scroll.
			// For now, let's draw as if we are always looking at the top part of the (potentially taller) logical waveform.

			const currentLogicalSampleY = yPx; // This is the y in the visible part of the (potentially larger) logical waveform.
			                               // (Assumes scrollOffsetPy = 0)
			const totalLogicalWaveformHeight = canvasClientHeight * zoomLevel;


			if (usePeaks) {
				const numPeakBlocks = peaksData.length / 2;
				// Map currentLogicalSampleY (from 0 to canvasClientHeight-1) to an index in peaksData.
				// The peaksData covers the entire duration.
				// The logical waveform has `totalLogicalWaveformHeight`.
				// `currentLogicalSampleY` is a point on this logical waveform.
				const peakBlockIndex = Math.floor((currentLogicalSampleY / totalLogicalWaveformHeight) * numPeakBlocks);

				if (peakBlockIndex < 0 || peakBlockIndex >= numPeakBlocks) continue; // Should not happen if logic is correct

				const minPeak = peaksData[peakBlockIndex * 2];
				const maxPeak = peaksData[peakBlockIndex * 2 + 1];

				ctx.beginPath();
				ctx.moveTo(midX + minPeak * midX, yPx + 0.5);
				ctx.lineTo(midX + maxPeak * midX, yPx + 0.5);
				ctx.stroke();

			} else if (buffer) {
				const data = buffer.getChannelData(0);
				const totalSamples = data.length;
				// Map currentLogicalSampleY to an index in raw audio data.
				const sampleIndex = Math.floor((currentLogicalSampleY / totalLogicalWaveformHeight) * totalSamples);

				if (sampleIndex < 0 || sampleIndex >= totalSamples) continue;

				const sampleValue = data[sampleIndex];
				const xVal = midX + sampleValue * midX;

				// This draws a single line from center to sample value for each yPx.
				// For a typical waveform, we'd draw from min to max envelope, or a line connecting samples.
				// The original code drew two lines (min envelope, max envelope).
				// Let's try to replicate the min/max approach but simplified for the current yPx.
				// This requires finding min/max over a range of samples corresponding to this yPx.

				// Simplified: draw a line from center to the sample value, for now.
				// This will look like a scatter plot if not dense enough.
				// To draw lines like original:
				// We need to draw from (midX + prev_min*midX, yPx-1 + 0.5) to (midX + minPeak*midX, yPx + 0.5)
				// This means we need to process samples/peaks in blocks for each yPx.

				// Reverting to a simplified version of the original's block processing for a single yPx:
				// What range of actual data samples map to this yPx?
				const samplesPerLogicalUnit = totalSamples / totalLogicalWaveformHeight;
				const startDataSample = Math.floor(currentLogicalSampleY * samplesPerLogicalUnit);
				const endDataSample = Math.floor((currentLogicalSampleY + 1) * samplesPerLogicalUnit);

				let minVal = 0, maxVal = 0;
				if (startDataSample < endDataSample && startDataSample < totalSamples) {
					minVal = data[startDataSample];
					maxVal = data[startDataSample];
					for (let i = startDataSample + 1; i < endDataSample; i++) {
						if (i < totalSamples) {
							if (data[i] < minVal) minVal = data[i];
							if (data[i] > maxVal) maxVal = data[i];
						}
					}
				} else if (startDataSample < totalSamples) { // Single sample for this pixel
					minVal = data[startDataSample];
					maxVal = data[startDataSample];
				}

				ctx.beginPath();
				ctx.moveTo(midX + minVal * midX, yPx + 0.5);
				ctx.lineTo(midX + maxVal * midX, yPx + 0.5); // This creates a horizontal line for the range at yPx
				ctx.stroke();
				// This is still not quite right for a vertical waveform.
				// The original drew two paths, one for min envelope, one for max.

				// Let's use the original logic but ensure loops run over canvasClientHeight (visible pixels)
				// and map to data based on totalLogicalWaveformHeight.
			}
		}
		// Fallback to original structure if the above is too complex to get right quickly
		// The key change is that `logicalHeight` in original becomes `totalLogicalWaveformHeight`
		// and loops for drawing run `canvasClientHeight` times, correctly sampling from the logical space.

		// Corrected structure based on original, adapted for zoom:
		const dataLogicalHeight = canvasClientHeight * zoomLevel; // This is the extent of data we're trying to show
		                                                       // If zoomLevel=1, it's canvasClientHeight.
                                                               // If zoomLevel=2, data is magnified, so dataLogicalHeight refers
                                                               // to the source data portion that would be twice canvasClientHeight.
                                                               // We are viewing the top `canvasClientHeight` of this.

		if (usePeaks) {
			const numPeakBlocks = peaksData.length / 2;
			const peaksPerLogicalSample = numPeakBlocks / dataLogicalHeight;

			ctx.beginPath(); // Max peaks path
			for (let yPx = 0; yPx < canvasClientHeight; yPx++) { // Iterate onscreen pixels
				// yPx is the screen pixel. It corresponds to a logical Y in the (potentially zoomed) data.
				// If no scroll, this logical Y is simply yPx.
				const logicalY = yPx; // Assuming we are viewing the top part of the zoomed waveform
				const peakBlockStartIndex = Math.floor(logicalY * peaksPerLogicalSample);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let maxPeak = 0.0;
				if (targetBlock >= 0 && targetBlock * 2 + 1 < peaksData.length) { // Ensure targetBlock is valid
					maxPeak = peaksData[targetBlock * 2 + 1];
				}
				const xVal = midX + maxPeak * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5); else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();

			ctx.beginPath(); // Min peaks path
			for (let yPx = 0; yPx < canvasClientHeight; yPx++) {
				const logicalY = yPx;
				const peakBlockStartIndex = Math.floor(logicalY * peaksPerLogicalSample);
				const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
				let minPeak = 0.0;
				if (targetBlock >=0 && targetBlock * 2 < peaksData.length) { // Ensure targetBlock is valid
					minPeak = peaksData[targetBlock * 2];
				}
				const xVal = midX + minPeak * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5); else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();
		} else if (buffer) {
			const data = buffer.getChannelData(0);
			const totalSamples = data.length;
			const samplesPerLogicalSample = totalSamples / dataLogicalHeight;

			ctx.beginPath(); // Max envelope
			for (let yPx = 0; yPx < canvasClientHeight; yPx++) {
				const logicalY = yPx;
				const sampleStartIndex = Math.floor(logicalY * samplesPerLogicalSample);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((logicalY + 1) * samplesPerLogicalSample));
				let maxVal = 0;
				if (sampleStartIndex < sampleEndIndex && sampleStartIndex < totalSamples) {
					maxVal = data[sampleStartIndex];
					for (let i = sampleStartIndex + 1; i < sampleEndIndex; i++) {
						if (data[i] > maxVal) maxVal = data[i];
					}
				} else if (sampleStartIndex < totalSamples) { // Single sample case
                    maxVal = data[sampleStartIndex];
                }
				const xVal = midX + maxVal * midX;
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5); else ctx.lineTo(xVal, yPx + 0.5);
			}
			ctx.stroke();

			ctx.beginPath(); // Min envelope
			for (let yPx = 0; yPx < canvasClientHeight; yPx++) {
				const logicalY = yPx;
				const sampleStartIndex = Math.floor(logicalY * samplesPerLogicalSample);
				const sampleEndIndex = Math.min(totalSamples, Math.floor((logicalY + 1) * samplesPerLogicalSample));
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
				if (yPx === 0) ctx.moveTo(xVal, yPx + 0.5); else ctx.lineTo(xVal, yPx + 0.5);
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
			// canvasClientHeight for drawVerticalWaveform is visibleCanvasHeight
			drawVerticalWaveform(ctx, buf, peaks, visibleCanvasHeight, waveformCanvasWidth, '#9ca3af');
		}

		// Draw red seek bar - timeToLogicalPy uses zoomLevel internally
		const pyCur = timeToLogicalPy(cur, mediaDur, visibleCanvasHeight);
		// The position of pyCur can be outside the visible range if zoomed.
		// We only draw it if it's within the visible canvas area.
		// If we had scrolling, pyCur would be (timeToLogicalPy(...) - scrollOffsetPy)
		const pyCurOnScreen = pyCur; // Assuming no scroll offset for now.

		if (pyCurOnScreen >= -1 && pyCurOnScreen <= visibleCanvasHeight + 1) {
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
			requestRedraw(true);
		}
	}

	function zoomIn() {
		handleZoom('in');
	}
	function zoomOut() {
		handleZoom('out');
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

	$: canZoomIn = zoomLevel < maxZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);
	$: canZoomOut = zoomLevel > minZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);

</script>

<div bind:this={componentContainer} class="vertical-waveform-panel flex flex-col w-full h-full bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded overflow-hidden">
	<!-- Header Section with Zoom Buttons -->
	<div class="flex-shrink-0 px-2 py-1 flex items-center justify-end space-x-1 border-b border-gray-300 dark:border-gray-600">
		<button class="ui-button-icon-sm" title="Zoom In Waveform" aria-label="Zoom In Waveform" on:click={zoomIn} disabled={!canZoomIn}>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM10.5 7.5v6m3-3h-6" />
			</svg>
		</button>
		<button class="ui-button-icon-sm" title="Zoom Out Waveform" aria-label="Zoom Out Waveform" on:click={zoomOut} disabled={!canZoomOut}>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM13.5 10.5h-6" />
			</svg>
		</button>
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
	.ui-button-icon-sm {
		@apply p-0.5 rounded text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-700 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-transparent dark:disabled:hover:bg-transparent;
	}
</style>
