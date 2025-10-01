<!-- src/lib/components/projectview/transcriptions/InteractiveWaveform.svelte -->
<script>
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js';
	import { transcriptStore, updatePlayerTime } from '$lib/stores/transcriptStore.js';
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';

	export let isTrimming = false;
	export let trimStartTime = 0;
	export let trimEndTime = 0;
	export let isEditingSegment = false;
	export let showTrimUI = true; // New prop, defaults to true for backward compatibility
	export let fixedHeightPx = 0; // New prop for setting a fixed height in pixels
export let compactMode = false; // New prop, defaults to false
	export let editSegmentStartTime = 0;
	export let editSegmentEndTime = 0;

	export let externalAudioBuffer = null;
	export let externalPeaks = null;
	export let externalCurrentTime = null;
	export let externalDuration = null;
	export let externalIsPlaying = null;
	export let externalSegments = null;
	export let externalCurrentSegmentIndex = null;

	const TIMESCALE_HEIGHT = 20;
	const BAR_THICKNESS_PX = 2; // Adapted from VerticalWaveform, will be bar height
	const BAR_SPACING_PX = 1;   // Adapted from VerticalWaveform, vertical spacing if multiple rows, or conceptual
	const RMS_GAIN_FACTOR = 4.5;  // Increased from 2.5 to make bars taller
	const MIN_BAR_LENGTH_PX = 1; // Adapted from MIN_BAR_HALF_LENGTH_PX, represents min bar height/length from center

	let actualMediaDuration = 0;
	let prevExternalAudioBufferForDuration = null;

	let timescaleCanvas;
	let segmentWaveformCanvas;
	let waveformScrollContainerRef;
	let componentRootRef;

	let visibleCanvasWidth = 0;
	let waveformCanvasHeight = 40; // Default, will be adjusted
	let webAudioApiSupported = true;
	let resizeObserverInstance;
	let unsubscribePlayer;
	let unsubscribeSegments;
	let unsubscribeAudioBuffer;
	let animationFrameId = null;
	let isObserverSetup = false;
	let isMounted = false;
	let zoomLevel = 1;
	const minZoomLevel = 1;
	const maxZoomLevel = 10;
	const zoomStep = 1.2;
	let scrollOffsetPx = 0;
	let isScrolling = false;
	let debounceScrollTimer = null;
	let autoScrollEnabled = true;
	let autoScrollEnableTimer = null;
	let lastDrawnTime = -1;
	let lastDrawnScrollOffset = -1;
	let lastDrawnZoomLevel = -1;
	let lastDrawnSegmentIndex = -1;
	let lastDrawnBuffer = null;
	let lastDrawnActualDuration = -1;
	let lastDrawnIsEditing = false;
	let lastDrawnEditStart = -1;
	let lastDrawnEditEnd = -1;
	const redrawTimeThreshold = 1/60;

	let isPanning = false;
	let panStartX = 0;
	let panInitialScrollOffsetPx = 0;

	let autoScrollRafId = null;
	let autoScrollDirection = ''; // 'left' or 'right'

	$: currentAudioBuffer = externalAudioBuffer ?? $transcriptStore.audioBuffer;
	$: currentAudioPeaks = externalPeaks ?? $transcriptStore.audioBufferPeaks;
	$: currentPlayTime = externalCurrentTime ?? $transcriptStore.player.currentTime;
	$: currentIsPlaying = externalIsPlaying ?? $transcriptStore.player.isPlaying;
	$: currentSegmentsToDisplay = externalSegments ?? $transcriptStore.segments;
	$: activeSegmentIndexForDisplay = externalCurrentSegmentIndex ?? $transcriptStore.player.currentSegmentIndex;

	$: {
		if (currentAudioBuffer && currentAudioBuffer !== prevExternalAudioBufferForDuration) {
			if (currentAudioBuffer.duration > 0) {
				actualMediaDuration = currentAudioBuffer.duration;
				zoomLevel = maxZoomLevel;
				scrollOffsetPx = 0;
				requestRedraw(true);
			} else {
				actualMediaDuration = 0;
			}
			prevExternalAudioBufferForDuration = currentAudioBuffer;
		} else if (!currentAudioBuffer && prevExternalAudioBufferForDuration) {
			actualMediaDuration = 0;
			prevExternalAudioBufferForDuration = null;
			requestRedraw(true);
		}
	}

	$: totalLogicalWidth = actualMediaDuration > 0 && visibleCanvasWidth > 0 ? visibleCanvasWidth * zoomLevel : 0;
	$: maxScrollPx = Math.max(0, totalLogicalWidth - visibleCanvasWidth);
	$: canZoomIn = zoomLevel < maxZoomLevel;
	$: canZoomOut = zoomLevel > minZoomLevel;
	let draggingHandle = null;
	const dispatch = createEventDispatcher();

	function formatTimestamp(sec) { if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '00:00.000'; const totalMs = Math.round(sec * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const tot = Math.floor(sec); return `${String(Math.floor(tot / 60)).padStart(2, '0')}:${String(tot % 60).padStart(2, '0')}.${ms}`; }
	function formatTimescaleTime(sec, totalDuration) { if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '0:00'; const tot = Math.floor(sec); const minutes = Math.floor(tot / 60); const seconds = tot % 60; const totalMinutes = Math.floor(totalDuration / 60); if (totalMinutes >= 60) { const hours = Math.floor(minutes / 60); const remainingMinutes = minutes % 60; return `${hours}:${String(remainingMinutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`; } else { return `${String(minutes).padStart(1, '0')}:${String(seconds).padStart(2, '0')}`; } }
	function timeToLogicalPx(time, duration, logicalWidth) { if (!duration || duration <= 0 || !logicalWidth || logicalWidth <= 0) return 0; const proportion = Math.max(0, Math.min(1, time / duration)); return proportion * logicalWidth; }
	function timeToVisiblePx(time, duration, logicalWidth, scrollOffset) { const logicalPx = timeToLogicalPx(time, duration, logicalWidth); return logicalPx - scrollOffset; }
	function pxToTime(px, duration, logicalWidth, visibleWidth, scrollOffset) { if (!duration || duration <= 0 || !logicalWidth || logicalWidth <= 0 || !visibleWidth || visibleWidth <= 0) return 0; const logicalPx = px + scrollOffset; const proportion = Math.max(0, Math.min(1, logicalPx / logicalWidth)); return proportion * duration; }

	function drawVisibleWaveform(ctx, buffer, peaks, logicalWidth, visibleWidth, scrollOffset, height, color) {
        if (!ctx || logicalWidth <= 0 || visibleWidth <= 0 || height <= 0) return;
        if (!buffer && !peaks) return;

        const mid = height / 2;
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;

        const samplesPerVisiblePixel = buffer ? (buffer.length / visibleWidth / zoomLevel) : Infinity;
        const PEAK_USAGE_THRESHOLD_SAMPLES_PER_PIXEL = 20;

        let usePeaks = peaks && peaks.length > 0 && (samplesPerVisiblePixel > PEAK_USAGE_THRESHOLD_SAMPLES_PER_PIXEL || !buffer);
        if (buffer && zoomLevel > (maxZoomLevel / 2) ) {
            usePeaks = false;
        }
        if (!buffer && !peaks) return;
        if (!buffer && peaks) usePeaks = true;

        ctx.beginPath();

        if (usePeaks) {
            const peakBlockSize = 512;
            const numPeakBlocks = peaks.length / 2;
            const peaksPerLogicalPixel = numPeakBlocks / logicalWidth;

            for (let x = 0; x < visibleWidth; x++) {
                const logicalX = x + scrollOffset;
                const peakBlockStartIndex = Math.floor(logicalX * peaksPerLogicalPixel);
                const peakBlockEndIndex = Math.floor((logicalX + 1) * peaksPerLogicalPixel);

                let minPeak = 0.0;
                let maxPeak = 0.0;

                if (peakBlockStartIndex < peakBlockEndIndex) {
                     minPeak = peaks[peakBlockStartIndex * 2];
                     maxPeak = peaks[peakBlockStartIndex * 2 + 1];
                    for (let i = peakBlockStartIndex + 1; i < peakBlockEndIndex; i++) {
                        if (peaks[i*2] < minPeak) minPeak = peaks[i*2];
                        if (peaks[i*2+1] > maxPeak) maxPeak = peaks[i*2+1];
                    }
                } else {
                    const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
                     if (targetBlock * 2 + 1 < peaks.length) {
                        minPeak = peaks[targetBlock * 2];
                        maxPeak = peaks[targetBlock * 2 + 1];
                    }
                }
                const yTop = mid + maxPeak * mid;
                if (x === 0) ctx.moveTo(x + 0.5, yTop);
                else ctx.lineTo(x + 0.5, yTop);
            }
            ctx.stroke();
            ctx.beginPath();
            for (let x = visibleWidth - 1; x >= 0; x--) {
                const logicalX = x + scrollOffset;
                const peakBlockStartIndex = Math.floor(logicalX * peaksPerLogicalPixel);
                const peakBlockEndIndex = Math.floor((logicalX + 1) * peaksPerLogicalPixel);
                let minPeak = 0.0;

                if (peakBlockStartIndex < peakBlockEndIndex) {
                    minPeak = peaks[peakBlockStartIndex * 2];
                    for (let i = peakBlockStartIndex + 1; i < peakBlockEndIndex; i++) {
                        if (peaks[i*2] < minPeak) minPeak = peaks[i*2];
                    }
                } else {
                     const targetBlock = Math.min(numPeakBlocks - 1, peakBlockStartIndex);
                     if (targetBlock * 2 < peaks.length) {
                       minPeak = peaks[targetBlock * 2];
                     }
                }
                const yBottom = mid + minPeak * mid;
                if (x === visibleWidth - 1) ctx.moveTo(x + 0.5, yBottom);
                else ctx.lineTo(x + 0.5, yBottom);
            }
        } else if (buffer) {
            const data = buffer.getChannelData(0);
            const totalSamples = data.length;
            const samplesPerLogicalPixelRaw = totalSamples / logicalWidth;
            const visibleStartTime = pxToTime(0, actualMediaDuration, logicalWidth, visibleWidth, scrollOffset);
            const visibleEndTime = pxToTime(visibleWidth, actualMediaDuration, logicalWidth, visibleWidth, scrollOffset);
            const startSampleIndexRaw = Math.max(0, Math.floor(visibleStartTime * buffer.sampleRate));
            const endSampleIndexRaw = Math.min(totalSamples, Math.ceil(visibleEndTime * buffer.sampleRate));

            if (startSampleIndexRaw >= endSampleIndexRaw) { ctx.stroke(); return; }

            for (let x = 0; x < visibleWidth; x++) {
                const logicalX = x + scrollOffset;
                const sampleStartIndexForPixel = Math.max(startSampleIndexRaw, Math.floor(logicalX * samplesPerLogicalPixelRaw));
                const sampleEndIndexForPixel = Math.min(endSampleIndexRaw, Math.floor((logicalX + 1) * samplesPerLogicalPixelRaw));

                if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue;
                let min = 0, max = 0;
                min = data[sampleStartIndexForPixel];
                max = data[sampleStartIndexForPixel];
                for (let i = sampleStartIndexForPixel + 1; i < sampleEndIndexForPixel; i++) {
                    const v = data[i];
                    if (v > max) max = v;
                    if (v < min) min = v;
                }
                const yTop = mid + max * mid;
                if (x === 0) ctx.moveTo(x + 0.5, yTop);
                else ctx.lineTo(x + 0.5, yTop);
            }
            ctx.stroke();
            ctx.beginPath();
            for (let x = visibleWidth - 1; x >= 0; x--) {
                const logicalX = x + scrollOffset;
                const sampleStartIndexForPixel = Math.max(startSampleIndexRaw, Math.floor(logicalX * samplesPerLogicalPixelRaw));
                const sampleEndIndexForPixel = Math.min(endSampleIndexRaw, Math.floor((logicalX + 1) * samplesPerLogicalPixelRaw));
                if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue;
                let min = data[sampleStartIndexForPixel];
                for (let i = sampleStartIndexForPixel + 1; i < sampleEndIndexForPixel; i++) {
                    const v = data[i];
                    if (v < min) min = v;
                }
                const yBottom = mid + min * mid;
                if (x === visibleWidth - 1) ctx.moveTo(x + 0.5, yBottom);
                else ctx.lineTo(x + 0.5, yBottom);
            }
        }
        ctx.stroke();
    }

	function drawHorizontalRmsWaveform(ctx, buffer, logicalWidth, visibleWidth, scrollOffset, canvasHeight, color) {
		if (!ctx || logicalWidth <= 0 || visibleWidth <= 0 || canvasHeight <= 0 || !buffer) return;

		const midY = canvasHeight / 2;
		ctx.fillStyle = color;

		const data = buffer.getChannelData(0);
		const totalSamples = data.length;
		if (totalSamples === 0) return;

		const samplesPerLogicalPixel = totalSamples / logicalWidth;
		const barUnitWidthPx = BAR_THICKNESS_PX + BAR_SPACING_PX; // How much horizontal space each bar unit takes

		for (let xPx = 0; xPx < visibleWidth; xPx += barUnitWidthPx) {
			const logicalX = xPx + scrollOffset;
			const startSample = Math.floor(logicalX * samplesPerLogicalPixel);
			const endSample = Math.ceil((logicalX + BAR_THICKNESS_PX) * samplesPerLogicalPixel);

			if (startSample >= totalSamples) break;

			let sumOfSquares = 0;
			const effectiveEndSample = Math.min(endSample, totalSamples);
			for (let i = startSample; i < effectiveEndSample; i++) {
				sumOfSquares += data[i] * data[i];
			}

			const numSamples = effectiveEndSample - startSample;
			const rms = numSamples > 0 ? Math.sqrt(sumOfSquares / numSamples) : 0;
			const cappedRms = Math.min(1.0, rms * RMS_GAIN_FACTOR);
			const displayHalfHeight = Math.max(MIN_BAR_LENGTH_PX, cappedRms * midY);

			ctx.fillRect(xPx, midY - displayHalfHeight, BAR_THICKNESS_PX, displayHalfHeight * 2);
		}
	}

	function clearWaveformCanvases() {
		if (segmentWaveformCanvas) {
			const c = segmentWaveformCanvas.getContext('2d');
			if (c) c.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height);
		}
		if (timescaleCanvas) {
			const c = timescaleCanvas.getContext('2d');
			if (c) c.clearRect(0, 0, timescaleCanvas.width, timescaleCanvas.height);
		}
		if (segmentWaveformCanvas && visibleCanvasWidth > 0 && waveformCanvasHeight > 0) {
			const c = segmentWaveformCanvas.getContext('2d');
			if (c) {
				const dpr = window.devicePixelRatio || 1;
				c.save();
				c.scale(dpr, dpr);
				c.fillStyle = document.documentElement.classList.contains('dark') ? '#A3A3A3' : '#6b7280';
				c.font = `10px sans-serif`;
				c.textAlign = 'center';
				c.textBaseline = 'middle';
				let message = 'Waveform';
				if (!webAudioApiSupported) message = 'Web Audio API not supported.';
				else if (!currentAudioBuffer && !currentAudioPeaks)
					message = 'Load media to see waveform.';
				c.fillText(message, visibleCanvasWidth / 2, waveformCanvasHeight / 2);
				c.restore();
			}
		}
	}
	function drawTimescale() {
		const dur = actualMediaDuration;
		const bufOrPeaks = currentAudioBuffer || currentAudioPeaks;
		const dpr = window.devicePixelRatio || 1;
		if (!timescaleCanvas || !bufOrPeaks || dur <= 0 || visibleCanvasWidth <= 0 || TIMESCALE_HEIGHT <= 0 || totalLogicalWidth <= 0) {
			if (timescaleCanvas) {
				timescaleCanvas.width = 0;
				timescaleCanvas.height = 0;
			}
			return;
		}
		const ctx = timescaleCanvas.getContext('2d');
		if (!ctx) return;
		const reqW = Math.round(visibleCanvasWidth * dpr);
		const reqH = Math.round(TIMESCALE_HEIGHT * dpr);
		if (timescaleCanvas.width !== reqW || timescaleCanvas.height !== reqH) {
			timescaleCanvas.width = reqW;
			timescaleCanvas.height = reqH;
		}
		ctx.save();
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, visibleCanvasWidth, TIMESCALE_HEIGHT);
		const isDark = document.documentElement.classList.contains('dark');
		ctx.fillStyle = isDark ? '#171717' : '#F3F4F6';
		ctx.fillRect(0, 0, visibleCanvasWidth, TIMESCALE_HEIGHT);
		ctx.strokeStyle = isDark ? '#404040' : '#d1d5db';
		ctx.fillStyle = isDark ? '#E5E5E5' : '#000000';
		ctx.font = '10px sans-serif';
		ctx.textBaseline = 'top';
		const minPixelSpacingForLabel = 50;
		const minPixelSpacingForMinorTick = 8;
		const intervals = [0.1, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];
		let interval = intervals[0];
		let intervalPx = timeToLogicalPx(interval, dur, totalLogicalWidth);
		for (let i = 0; i < intervals.length; i++) {
			const currentIntervalPx = timeToLogicalPx(intervals[i], dur, totalLogicalWidth);
			if (currentIntervalPx >= minPixelSpacingForLabel) {
				interval = intervals[i];
				intervalPx = currentIntervalPx;
				break;
			}
			if (i === intervals.length - 1) {
				interval = intervals[i];
				intervalPx = currentIntervalPx;
			}
		}
		let minorInterval = interval / 5;
		if (interval === 2 || interval === 15) minorInterval = interval / 2;
		else if (interval === 10 && intervalPx < 100) minorInterval = interval / 2;
		let minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth);
		while (minorIntervalPx < minPixelSpacingForMinorTick && minorInterval < interval / 2) {
			minorInterval *= 2;
			minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth);
		}
		const firstVisibleTime = pxToTime(0, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const lastVisibleTime = pxToTime(visibleCanvasWidth, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const firstMajorTickTime = Math.ceil(firstVisibleTime / interval) * interval;
		for (let t = firstMajorTickTime; t <= lastVisibleTime; t += interval) {
			const x = timeToVisiblePx(t, dur, totalLogicalWidth, scrollOffsetPx);
			ctx.beginPath();
			ctx.moveTo(x, 0);
			ctx.lineTo(x, TIMESCALE_HEIGHT);
			ctx.stroke();
			ctx.fillText(formatTimescaleTime(t, dur), x + 3, 2);
		}
		if (minorIntervalPx > 0) {
			const firstMinorTickTime = Math.ceil(firstVisibleTime / minorInterval) * minorInterval;
			for (let t = firstMinorTickTime; t <= lastVisibleTime; t += minorInterval) {
				if (t % interval !== 0) {
					const x = timeToVisiblePx(t, dur, totalLogicalWidth, scrollOffsetPx);
					ctx.beginPath();
					ctx.moveTo(x, 0);
					ctx.lineTo(x, TIMESCALE_HEIGHT * 0.4);
					ctx.stroke();
				}
			}
		}
		ctx.restore();
	}
	function drawSegmentWaveformUI() {
		const buf = currentAudioBuffer;
		const peaks = currentAudioPeaks;
		const cur = currentPlayTime || 0;
		const dur = actualMediaDuration;
		const segments = currentSegmentsToDisplay || [];
		const currentActiveIndex = activeSegmentIndexForDisplay ?? -1;
		const seg = segments[currentActiveIndex];
		const dpr = window.devicePixelRatio || 1;

		if (
			!segmentWaveformCanvas ||
			(!buf && !peaks) ||
			dur <= 0 ||
			visibleCanvasWidth <= 0 ||
			waveformCanvasHeight <= 0 ||
			totalLogicalWidth <= 0
		) {
			if (segmentWaveformCanvas) {
				const c = segmentWaveformCanvas.getContext('2d');
				if (c) c.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height);
				segmentWaveformCanvas.width = 0;
				segmentWaveformCanvas.height = 0;
			}
			return;
		}

		const ctx = segmentWaveformCanvas.getContext('2d');
		if (!ctx) return;

		const reqW = Math.round(visibleCanvasWidth * dpr);
		const reqH = Math.round(waveformCanvasHeight * dpr);
		if (segmentWaveformCanvas.width !== reqW || segmentWaveformCanvas.height !== reqH) {
			segmentWaveformCanvas.width = reqW;
			segmentWaveformCanvas.height = reqH;
		}

		ctx.save();
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, visibleCanvasWidth, waveformCanvasHeight);
		const isDark = document.documentElement.classList.contains('dark');

		if (buf && totalLogicalWidth > 0) {
			// RMS drawing currently only supports AudioBuffer
			drawHorizontalRmsWaveform(
				ctx,
				buf,
				totalLogicalWidth,
				visibleCanvasWidth,
				scrollOffsetPx,
				waveformCanvasHeight,
				isDark ? '#737373' : '#9ca3af'
			);
		} else if (peaks && totalLogicalWidth > 0) {
			// Fallback to path drawing if only peaks available
			drawVisibleWaveform(
				ctx,
				null,
				peaks,
				totalLogicalWidth,
				visibleCanvasWidth,
				scrollOffsetPx,
				waveformCanvasHeight,
				isDark ? '#737373' : '#9ca3af'
			);
		}

		let highlightStartTime = -1;
		let highlightEndTime = -1;
		let highlightColor = isDark ? 'rgba(59, 130, 246, 0.3)' : 'rgba(147, 197, 253, 0.4)';
		let waveColor = isDark ? '#60a5fa' : '#2563eb';

		if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
			highlightStartTime = editSegmentStartTime;
			highlightEndTime = editSegmentEndTime;
		} else if (currentActiveIndex >= 0 && currentActiveIndex < segments.length && seg) {
			const segStartTime = Number(seg.start_time);
			const segEndTime = Number(seg.end_time);
			if (!isNaN(segStartTime) && !isNaN(segEndTime) && segEndTime >= segStartTime) {
				highlightStartTime = segStartTime;
				highlightEndTime = segEndTime;
			}
		}

		if (highlightStartTime >= 0 && highlightEndTime >= highlightStartTime) {
			const pxS_logical = timeToLogicalPx(highlightStartTime, dur, totalLogicalWidth);
			const pxE_logical = timeToLogicalPx(highlightEndTime, dur, totalLogicalWidth);
			const pxS_visible = pxS_logical - scrollOffsetPx;
			const pxE_visible = pxE_logical - scrollOffsetPx;
			const clamped_pxS_visible = Math.max(0, pxS_visible);
			const clamped_pxE_visible = Math.min(visibleCanvasWidth, pxE_visible);
			const pxW_visible_clamped = Math.max(0, clamped_pxE_visible - clamped_pxS_visible);

			if (pxW_visible_clamped > 0) {
				ctx.fillStyle = highlightColor;
				ctx.fillRect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight);
				if ((buf || peaks) && totalLogicalWidth > 0) {
					ctx.save();
					ctx.beginPath();
					ctx.rect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight);
					ctx.clip();
					if (buf && totalLogicalWidth > 0) {
						// RMS drawing for highlighted part
						drawHorizontalRmsWaveform(
							ctx,
							buf,
							totalLogicalWidth,
							visibleCanvasWidth,
							scrollOffsetPx,
							waveformCanvasHeight,
							waveColor
						);
					} else if (peaks && totalLogicalWidth > 0) {
						// Fallback for peaks
						drawVisibleWaveform(
							ctx,
							null,
							peaks,
							totalLogicalWidth,
							visibleCanvasWidth,
							scrollOffsetPx,
							waveformCanvasHeight,
							waveColor
						);
					}
					ctx.restore();
				}
			}
		}

		const pxCur_logical = timeToLogicalPx(cur, dur, totalLogicalWidth);
		const pxCur_visible = pxCur_logical - scrollOffsetPx;

		if (pxCur_visible >= 0 && pxCur_visible <= visibleCanvasWidth && waveformCanvasHeight > 0) {
			ctx.fillStyle = '#FF0000'; // Red color for seek bar
			ctx.fillRect(pxCur_visible - 0.75, 0, 1.5, waveformCanvasHeight); // Draw a 1.5px wide red bar
		}

		ctx.restore();
		lastDrawnTime = cur;
		lastDrawnScrollOffset = scrollOffsetPx;
		lastDrawnZoomLevel = zoomLevel;
		lastDrawnSegmentIndex = currentActiveIndex;
		lastDrawnBuffer = buf;
		lastDrawnActualDuration = dur;
		lastDrawnIsEditing = isEditingSegment;
		lastDrawnEditStart = editSegmentStartTime;
		lastDrawnEditEnd = editSegmentEndTime;
	}

	let forceNextRedraw = false; function requestRedraw(force = false) { if (force) forceNextRedraw = true; if (isMounted) { drawTimescale(); drawSegmentWaveformUI(); } }

	function animationLoop() {
        if (!isMounted) return;
        const cur = currentPlayTime || 0;
        const dur = actualMediaDuration;
        const buf = currentAudioBuffer;
        const peaks = currentAudioPeaks;
        const currentActiveIdx = activeSegmentIndexForDisplay ?? -1;

        let needsDraw = forceNextRedraw ||
            (buf !== lastDrawnBuffer) ||
            (currentActiveIdx !== lastDrawnSegmentIndex) ||
            (Math.abs(cur - lastDrawnTime) > redrawTimeThreshold) ||
            (Math.abs(scrollOffsetPx - lastDrawnScrollOffset) > 0.5) ||
            (Math.abs(zoomLevel - lastDrawnZoomLevel) > 0.001) ||
            (dur !== lastDrawnActualDuration) ||
            (isEditingSegment !== lastDrawnIsEditing) ||
            (isEditingSegment && (editSegmentStartTime !== lastDrawnEditStart || editSegmentEndTime !== lastDrawnEditEnd));

        forceNextRedraw = false;

        if (needsDraw && visibleCanvasWidth > 0 && (buf || peaks) && dur > 0 && totalLogicalWidth > 0 ) {
            drawTimescale();
            drawSegmentWaveformUI();
        } else if (needsDraw && ((!buf && !peaks) || visibleCanvasWidth <= 0 || dur <= 0 || totalLogicalWidth <= 0)) {
            clearWaveformCanvases();
            lastDrawnTime = cur;
            lastDrawnScrollOffset = scrollOffsetPx;
            lastDrawnZoomLevel = zoomLevel;
            lastDrawnSegmentIndex = currentActiveIdx;
            lastDrawnBuffer = buf;
            lastDrawnActualDuration = dur;
            lastDrawnIsEditing = isEditingSegment;
            lastDrawnEditStart = editSegmentStartTime;
            lastDrawnEditEnd = editSegmentEndTime;
        }

        if (autoScrollEnabled && currentIsPlaying && dur > 0 && totalLogicalWidth > visibleCanvasWidth && !isTrimming && !isEditingSegment) {
            const pxCur_visible = timeToVisiblePx(cur, dur, totalLogicalWidth, scrollOffsetPx);
            const scrollMarginLeft = visibleCanvasWidth * 0.25;
            const scrollMarginRight = visibleCanvasWidth * 0.75;
            let targetScrollOffset = scrollOffsetPx;
            let needsScrollUpdate = false;

            if (pxCur_visible < scrollMarginLeft) {
                targetScrollOffset = timeToLogicalPx(cur, dur, totalLogicalWidth) - scrollMarginLeft;
                needsScrollUpdate = true;
            } else if (pxCur_visible > scrollMarginRight) {
                targetScrollOffset = timeToLogicalPx(cur, dur, totalLogicalWidth) - scrollMarginRight;
                needsScrollUpdate = true;
            }

            if (needsScrollUpdate) {
                targetScrollOffset = Math.max(0, Math.min(targetScrollOffset, maxScrollPx));
                const diff = targetScrollOffset - scrollOffsetPx;
                const moveAmount = diff * 0.1;
                let newScrollOffset = scrollOffsetPx + moveAmount;
                if (Math.abs(diff) < 1) newScrollOffset = targetScrollOffset;
                newScrollOffset = Math.round(newScrollOffset);

                if (Math.abs(newScrollOffset - scrollOffsetPx) > 0) {
                    scrollOffsetPx = newScrollOffset;
                }
            }
        }
        animationFrameId = requestAnimationFrame(animationLoop);
    }

	function resetZoomAndScrollState(clearBuffer = true) {
		zoomLevel = 1; scrollOffsetPx = 0; autoScrollEnabled = true;
		clearTimeout(autoScrollEnableTimer); autoScrollEnableTimer = null;
		lastDrawnTime = -1; lastDrawnScrollOffset = -1; lastDrawnZoomLevel = -1;
		lastDrawnSegmentIndex = -1;
        if (clearBuffer) {
            lastDrawnBuffer = null;
        }
		lastDrawnActualDuration = -1; lastDrawnIsEditing = false;
		lastDrawnEditStart = -1; lastDrawnEditEnd = -1;
		clearWaveformCanvases(); requestRedraw(true);
	}

	onMount(() => {
		isMounted = true;
		webAudioApiSupported = typeof window.AudioContext !== 'undefined' || typeof window.webkitAudioContext !== 'undefined';
		unsubscribeAudioBuffer = transcriptStore.subscribe(ts => {});
		unsubscribePlayer = transcriptStore.subscribe(ts => { });
		unsubscribeSegments = transcriptStore.subscribe(ts => { });

		prevExternalAudioBufferForDuration = externalAudioBuffer;
		if (externalAudioBuffer && externalAudioBuffer.duration > 0) {
			actualMediaDuration = externalAudioBuffer.duration;
		} else if ($transcriptStore.audioBuffer && $transcriptStore.audioBuffer.duration > 0 && !externalAudioBuffer) {
            actualMediaDuration = $transcriptStore.audioBuffer.duration;
        }

		tick().then(() => {
			if (isMounted) {
				if (waveformScrollContainerRef) {
					visibleCanvasWidth = waveformScrollContainerRef.clientWidth || 0;
					if (fixedHeightPx > 0) {
						waveformCanvasHeight = fixedHeightPx - TIMESCALE_HEIGHT;
					} else {
						waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT;
					}
				}
				setupResizeObserver();
				animationFrameId = requestAnimationFrame(animationLoop);
				requestRedraw(true);
			}
		});
	});
	onDestroy(() => {
		isMounted = false;
		unsubscribeAudioBuffer && unsubscribeAudioBuffer();
		unsubscribeSegments && unsubscribeSegments();
		unsubscribePlayer && unsubscribePlayer();
		if (resizeObserverInstance) { resizeObserverInstance.disconnect(); resizeObserverInstance = null; isObserverSetup = false; }
		if (animationFrameId) cancelAnimationFrame(animationFrameId);
		if (debounceScrollTimer) clearTimeout(debounceScrollTimer);
		if (autoScrollEnableTimer) clearTimeout(autoScrollEnableTimer);
		animationFrameId = null;
		window.removeEventListener('mousemove', handleTrimMouseMove);
		window.removeEventListener('mouseup', handleTrimMouseUp);
		window.removeEventListener('mousemove', handleEditMouseMove);
		window.removeEventListener('mouseup', handleEditMouseUp);
		lastDrawnTime = -1; lastDrawnScrollOffset = -1; lastDrawnZoomLevel = -1; lastDrawnSegmentIndex = -1;
		lastDrawnBuffer = null; lastDrawnActualDuration = -1; lastDrawnIsEditing = false;
		lastDrawnEditStart = -1; lastDrawnEditEnd = -1;
	});

		function setupResizeObserver() {
		        if (waveformScrollContainerRef && !isObserverSetup && isMounted && typeof window !== 'undefined' && window.ResizeObserver) {
            isObserverSetup = true;
            // Initial height setting within observer setup
            if (compactMode) {
                waveformCanvasHeight = 32;
            } else {
                waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT;
            }
            resizeObserverInstance = new ResizeObserver((entries) => {
                let needsRedraw = false;
                let needsScrollUpdate = false;
                let newScrollOffset = scrollOffsetPx;

                for (const entry of entries) {
                    if (entry.target === waveformScrollContainerRef) {
                        const newWidth = entry.contentRect.width;
                        const newContainerHeight = entry.contentRect.height;

                        let newWaveformHeight;
                        if (fixedHeightPx > 0) {
                            newWaveformHeight = fixedHeightPx - TIMESCALE_HEIGHT;
                        } else if (compactMode) { // Use compactMode here
                            newWaveformHeight = 32;
                        } else {
                            newWaveformHeight = (newContainerHeight || 80) - TIMESCALE_HEIGHT;
                        }

                        if (newWaveformHeight > 0 && newWaveformHeight !== waveformCanvasHeight) {
                            waveformCanvasHeight = newWaveformHeight;
                            needsRedraw = true;
                        }

                        if (newWidth > 0 && newWidth !== visibleCanvasWidth) {
                            const oldVisibleWidth = visibleCanvasWidth;
                            const oldTotalLogicalWidth = totalLogicalWidth;
                            visibleCanvasWidth = newWidth;
                            const currentMaxScroll = Math.max(0, (visibleCanvasWidth * zoomLevel) - visibleCanvasWidth);

                            if (oldVisibleWidth > 0 && oldTotalLogicalWidth > 0 && oldTotalLogicalWidth > oldVisibleWidth) {
                                const scrollCenterLogicalPx = scrollOffsetPx + oldVisibleWidth / 2;
                                const centerProportion = oldTotalLogicalWidth > 0 ? scrollCenterLogicalPx / oldTotalLogicalWidth : 0;
                                const newTotalLogicalWidthAfterUpdate = visibleCanvasWidth * zoomLevel;
                                newScrollOffset = (centerProportion * newTotalLogicalWidthAfterUpdate) - (visibleCanvasWidth / 2);
                                newScrollOffset = Math.max(0, Math.min(newScrollOffset, Math.max(0, newTotalLogicalWidthAfterUpdate - visibleCanvasWidth)));
                            } else {
                                newScrollOffset = Math.max(0, Math.min(scrollOffsetPx, currentMaxScroll));
                            }

                            if (Math.abs(newScrollOffset - scrollOffsetPx) > 0.5) {
                                scrollOffsetPx = Math.round(newScrollOffset);
                                needsScrollUpdate = true;
                            }
                            needsRedraw = true;
                        } else if (newWidth <= 0 && visibleCanvasWidth !== 0) {
                            visibleCanvasWidth = 0;
                            scrollOffsetPx = 0;
                            needsScrollUpdate = true;
                            clearWaveformCanvases();
                            needsRedraw = false;
                        }
                    }
                }

                if (needsScrollUpdate ) {
                    const wasAutoScrollEnabled = autoScrollEnabled;
                    autoScrollEnabled = false;
                    autoScrollEnableTimer = setTimeout(() => {
                        if (!isTrimming && !isEditingSegment) autoScrollEnabled = wasAutoScrollEnabled;
                        autoScrollEnableTimer = null;
                    }, 100);
                }
                if (needsRedraw) requestRedraw();
            });
            resizeObserverInstance.observe(waveformScrollContainerRef);
            if (waveformScrollContainerRef) { // Ensure this runs after observer is set up
                visibleCanvasWidth = waveformScrollContainerRef.clientWidth;
                if (compactMode) {
                    waveformCanvasHeight = 32;
                } else {
                    waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT;
                }
            }
            requestRedraw(true);
        }
	}

	function handleScroll(event) {

	}

	function handleCanvasClick(e) { const dur = actualMediaDuration; if (isTrimming || isEditingSegment || !segmentWaveformCanvas || !currentAudioBuffer || dur <= 0 || !waveformScrollContainerRef || visibleCanvasWidth <= 0 || totalLogicalWidth <= 0) return; const rect = waveformScrollContainerRef.getBoundingClientRect(); const clickX = e.clientX - rect.left; const time = pxToTime(clickX, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx); if (!autoScrollEnabled) { autoScrollEnabled = true; clearTimeout(autoScrollEnableTimer); autoScrollEnableTimer = null; } dispatch('navigate', { time: time }); }

	function handleZoom(direction) {
		if (!visibleCanvasWidth || visibleCanvasWidth <= 0 || !currentAudioBuffer || !actualMediaDuration) {
			return;
		}
		const oldZoomLevel = zoomLevel;
		let newZoomLevel = direction === 'in' ? oldZoomLevel * zoomStep : oldZoomLevel / zoomStep;
		newZoomLevel = Math.max(minZoomLevel, Math.min(maxZoomLevel, newZoomLevel));

		if (Math.abs(newZoomLevel - oldZoomLevel) < 0.001) { return; }

		const viewCenterPx = scrollOffsetPx + visibleCanvasWidth / 2;
		const timeAtCenter = pxToTime(visibleCanvasWidth / 2, actualMediaDuration, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);

		zoomLevel = newZoomLevel;

		tick().then(() => {
			const newTotalLogicalWidthAfterZoom = totalLogicalWidth;
			let newScrollOffset = timeToLogicalPx(timeAtCenter, actualMediaDuration, newTotalLogicalWidthAfterZoom) - visibleCanvasWidth / 2;

			const newMaxScroll = Math.max(0, newTotalLogicalWidthAfterZoom - visibleCanvasWidth);
			newScrollOffset = Math.max(0, Math.min(newScrollOffset, newMaxScroll));

			scrollOffsetPx = Math.round(newScrollOffset);

			const wasAutoScrollEnabled = autoScrollEnabled; autoScrollEnabled = false; clearTimeout(autoScrollEnableTimer);
			requestRedraw(true);
			autoScrollEnableTimer = setTimeout(() => {
				if (isMounted && !isTrimming && !isEditingSegment) { autoScrollEnabled = wasAutoScrollEnabled; }
				autoScrollEnableTimer = null;
			}, 100);
		});
	}
	function zoomIn() { handleZoom('in'); }
	function zoomOut() { handleZoom('out'); }

	function handlePanStart(event) {
		if (event.button !== 0 || isTrimming || isEditingSegment || draggingHandle) {
			return;
		}
		isPanning = true;
		panStartX = event.clientX;
		panInitialScrollOffsetPx = scrollOffsetPx;

		if (segmentWaveformCanvas) {
			segmentWaveformCanvas.style.cursor = 'grabbing';
		}

		window.addEventListener('mousemove', handlePanMove);
		window.addEventListener('mouseup', handlePanEnd);
		window.addEventListener('mouseleave', handlePanEnd);

		if (autoScrollEnabled) {
			autoScrollEnabled = false;
			clearTimeout(autoScrollEnableTimer);
			autoScrollEnableTimer = null;
		}
	}

	function handlePanMove(event) {
		if (!isPanning) {
			return;
		}
		event.preventDefault();
		const deltaX = event.clientX - panStartX;
		let newScrollOffset = panInitialScrollOffsetPx - deltaX;

		newScrollOffset = Math.max(0, Math.min(newScrollOffset, maxScrollPx));

		if (Math.abs(newScrollOffset - scrollOffsetPx) > 0.5) {
			scrollOffsetPx = Math.round(newScrollOffset);
			requestRedraw(true);
		}
	}

	function handlePanEnd(event) {
		if (!isPanning) {
			return;
		}
		isPanning = false;

		if (segmentWaveformCanvas) {
			segmentWaveformCanvas.style.cursor = (currentAudioBuffer && !isTrimming && !isEditingSegment) ? 'pointer' : 'default';
		}

		window.removeEventListener('mousemove', handlePanMove);
		window.removeEventListener('mouseup', handlePanEnd);
		window.removeEventListener('mouseleave', handlePanEnd);

		if (autoScrollEnableTimer === null && !isTrimming && !isEditingSegment) {
			 autoScrollEnableTimer = setTimeout(() => {
				autoScrollEnabled = true;
				autoScrollEnableTimer = null;
				requestRedraw(true);
			}, 1500);
		}
	}

	function startTrimDrag(handle, event) { if (!isTrimming || !actualMediaDuration || !segmentWaveformCanvas || isEditingSegment) return; event.preventDefault(); draggingHandle = handle; window.addEventListener('mousemove', handleTrimMouseMove); window.addEventListener('mouseup', handleTrimMouseUp, { once: true }); }
	function handleTrimMouseMove(event) {
		if (draggingHandle !== 'trim-left' && draggingHandle !== 'trim-right') return;
		if (!isTrimming || !segmentWaveformCanvas || visibleCanvasWidth <= 0 || !actualMediaDuration) return;
		event.preventDefault();
		const rect = segmentWaveformCanvas.getBoundingClientRect();
		const clickX = Math.max(0, Math.min(visibleCanvasWidth, event.clientX - rect.left));
		let newTime = pxToTime(clickX, actualMediaDuration, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const minDuration = 0.1; let newStartTime = trimStartTime; let newEndTime = trimEndTime;

		const containerRect = waveformScrollContainerRef.getBoundingClientRect();
		const edgeZone = 50; // 50px from either edge
		const mouseClientX = event.clientX;

		if (mouseClientX < containerRect.left + edgeZone) {
			autoScrollDirection = 'left';
			if (!autoScrollRafId) startHandleAutoScroll();
		} else if (mouseClientX > containerRect.right - edgeZone) {
			autoScrollDirection = 'right';
			if (!autoScrollRafId) startHandleAutoScroll();
		} else {
			stopHandleAutoScroll();
		}

		// Calculate newTime based on mouse position relative to the visible part of the waveform area
		const clickXInVisibleArea = Math.max(0, Math.min(visibleCanvasWidth, event.clientX - containerRect.left));
		newTime = pxToTime(clickXInVisibleArea, actualMediaDuration, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);

		if (draggingHandle === 'trim-left') {
			newStartTime = Math.max(0, Math.min(newTime, trimEndTime - minDuration));
		} else { // trim-right
			newEndTime = Math.min(actualMediaDuration, Math.max(newTime, trimStartTime + minDuration));
		}
		if (newStartTime !== trimStartTime || newEndTime !== trimEndTime) { dispatch('trimupdate', { startTime: newStartTime, endTime: newEndTime }); requestRedraw(true); }
	}

	function handleTrimMouseUp() {
		if (draggingHandle === 'trim-left' || draggingHandle === 'trim-right') {
			draggingHandle = null;
			window.removeEventListener('mousemove', handleTrimMouseMove);
			stopHandleAutoScroll();
		}
	}

	function startHandleAutoScroll() {
		if (autoScrollRafId) return; // Already scrolling

		function scrollLoop() {
			if (!draggingHandle || autoScrollDirection === '') {
				stopHandleAutoScroll();
				return;
			}

			const scrollStep = 5; // pixels per frame
			let scrolled = false;

			if (autoScrollDirection === 'left' && scrollOffsetPx > 0) {
				scrollOffsetPx = Math.max(0, scrollOffsetPx - scrollStep);
				scrolled = true;
			} else if (autoScrollDirection === 'right' && scrollOffsetPx < maxScrollPx) {
				scrollOffsetPx = Math.min(maxScrollPx, scrollOffsetPx + scrollStep);
				scrolled = true;
			}

			if (scrolled) {
				// We need to re-evaluate the handle's position as if the mouse is still at the edge,
				// but the content has scrolled. The `handleTrimMouseMove` logic for calculating `newTime`
				// based on a fixed clientX (edge of screen) will effectively do this.
				// So, we can call a simplified version or just rely on the next mousemove event if the user
				// keeps the mouse at the edge.
				// For immediate feedback during auto-scroll without new mouse events:
				let edgeClientX;
				const containerRect = waveformScrollContainerRef.getBoundingClientRect();
				if (autoScrollDirection === 'left') {
					edgeClientX = containerRect.left + edgeZone / 2; // Simulate mouse at mid-edge zone
				} else {
					edgeClientX = containerRect.right - edgeZone / 2;
				}
				const clickXInVisibleArea = Math.max(0, Math.min(visibleCanvasWidth, edgeClientX - containerRect.left));
				let newTime = pxToTime(clickXInVisibleArea, actualMediaDuration, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
				const minDuration = 0.1;

				if (draggingHandle === 'trim-left') {
					const newStartTime = Math.max(0, Math.min(newTime, trimEndTime - minDuration));
					if (newStartTime !== trimStartTime) dispatch('trimupdate', { startTime: newStartTime, endTime: trimEndTime });
				} else { // trim-right
					const newEndTime = Math.min(actualMediaDuration, Math.max(newTime, trimStartTime + minDuration));
					if (newEndTime !== trimEndTime) dispatch('trimupdate', { startTime: trimStartTime, endTime: newEndTime });
				}
				requestRedraw(true);
			}
			autoScrollRafId = requestAnimationFrame(scrollLoop);
		}
		autoScrollRafId = requestAnimationFrame(scrollLoop);
	}

	function stopHandleAutoScroll() {
		if (autoScrollRafId) {
			cancelAnimationFrame(autoScrollRafId);
			autoScrollRafId = null;
		}
		autoScrollDirection = '';
	}

	function startEditDrag(handle, event) {
        if (!isEditingSegment || !actualMediaDuration || !segmentWaveformCanvas || isTrimming) { return; }
		event.preventDefault(); draggingHandle = handle;
		window.addEventListener('mousemove', handleEditMouseMove);
		window.addEventListener('mouseup', handleEditMouseUp, { once: true });
	}
	function handleEditMouseMove(event) {
		if (draggingHandle !== 'edit-left' && draggingHandle !== 'edit-right') return;
		if (!isEditingSegment || !segmentWaveformCanvas || visibleCanvasWidth <= 0 || !actualMediaDuration) return;
		event.preventDefault(); const rect = segmentWaveformCanvas.getBoundingClientRect();
		const clickX = Math.max(0, Math.min(visibleCanvasWidth, event.clientX - rect.left));
		let newTime = pxToTime(clickX, actualMediaDuration, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const minDuration = 0.05; let newStartTime = editSegmentStartTime; let newEndTime = editSegmentEndTime;
		const lowerBound = 0; const upperBound = actualMediaDuration;
		if (draggingHandle === 'edit-left') {
			newStartTime = Math.max(lowerBound, Math.min(newTime, editSegmentEndTime - minDuration));
		} else { newEndTime = Math.min(upperBound, Math.max(newTime, editSegmentStartTime + minDuration)); }
		if (newStartTime !== editSegmentStartTime || newEndTime !== editSegmentEndTime) {
			dispatch('segmentupdate', { startTime: newStartTime, endTime: newEndTime });
		}
	}
	 function handleEditMouseUp() { if (draggingHandle === 'edit-left' || draggingHandle === 'edit-right') { draggingHandle = null; window.removeEventListener('mousemove', handleEditMouseMove); } }
    $: if (isMounted) { if (!isEditingSegment && (draggingHandle === 'edit-left' || draggingHandle === 'edit-right')) { handleEditMouseUp(); } }

    $: if (isMounted && externalAudioBuffer && externalAudioBuffer !== prevExternalAudioBufferForDuration) {
    } else if (isMounted && !externalAudioBuffer && prevExternalAudioBufferForDuration && $transcriptStore.audioBuffer !== prevExternalAudioBufferForDuration) {
    }

	function canDrawWaveform() {
		return visibleCanvasWidth > 0 && waveformCanvasHeight > 0 && actualMediaDuration > 0 && (currentAudioBuffer || currentAudioPeaks);
	}

	$: if (fixedHeightPx > 0 && waveformCanvasHeight !== (fixedHeightPx - TIMESCALE_HEIGHT)) {
		waveformCanvasHeight = fixedHeightPx - TIMESCALE_HEIGHT;
		requestRedraw(true);
	}

	// More aggressive redraw trigger for seek bar movement
	$: if (isMounted && currentPlayTime !== undefined && visibleCanvasWidth > 0 && actualMediaDuration > 0) {
		// Using a threshold slightly larger than typical frame time for smoothness, but small enough for responsiveness.
		// 0.010 seconds = 10ms. A typical frame is ~16.7ms at 60fps.
		if (Math.abs(currentPlayTime - lastDrawnTime) > 0.010) {
			requestRedraw(true);
		} else if (currentPlayTime === 0 && lastDrawnTime !== 0) { // Ensure redraw if time is reset to 0
			requestRedraw(true);
		}
	}
</script>

<div bind:this={componentRootRef} class="interactive-waveform-panel flex flex-row w-full h-full bg-white dark:bg-d-gray-800 border-x border-b border-gray-200 dark:border-border rounded overflow-hidden">
	<div
		bind:this={waveformScrollContainerRef}
		class="waveform-scroll-container flex-grow bg-white dark:bg-d-gray-700 relative overflow-x-auto overflow-y-hidden h-full"
		role="region" aria-label="Interactive Waveform Timeline"
		on:scroll={handleScroll}
	>
		<canvas bind:this={timescaleCanvas} class="timescale-canvas" style="height: {TIMESCALE_HEIGHT}px;" aria-hidden="true" />
		<canvas
			bind:this={segmentWaveformCanvas}
			class="waveform-canvas {(currentAudioBuffer && !isTrimming && !isEditingSegment && !isPanning && showTrimUI) ? 'cursor-pointer' : (isPanning ? 'cursor-grabbing' : 'cursor-default')}"
			style="height: {waveformCanvasHeight}px; top: {TIMESCALE_HEIGHT}px;"
			aria-label="Waveform visualization. Click to seek audio."
			on:click|self={handleCanvasClick}
			on:mousedown|self={handlePanStart}
			on:wheel|preventDefault={(e) => {
				if (e.ctrlKey || e.metaKey) {
					handleZoom(e.deltaY < 0 ? 'in' : 'out');
				} else {
					let scrollAmount = e.deltaX !== 0 ? e.deltaX : e.deltaY;
					// Adjust sensitivity: smaller factor = slower/more controlled scroll per wheel tick
					// Larger factor = faster scroll. Let's try a base multiplier.
					const scrollFactor = 1; // Adjust this factor as needed
					scrollOffsetPx = Math.max(0, Math.min(maxScrollPx, scrollOffsetPx + scrollAmount * scrollFactor));
					requestRedraw(true);
				}
			}} />
		{#if !webAudioApiSupported && isMounted} <div class="overlay-message"><p>Web Audio API not supported.</p></div> {:else if (!currentAudioBuffer && !currentAudioPeaks) && isMounted} <div class="overlay-message"><p>Load audio/video media to view waveform.</p></div> {/if}

		{#if showTrimUI && isTrimming && visibleCanvasWidth > 0 && actualMediaDuration > 0}
			{@const trimStartPx = timeToVisiblePx(trimStartTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			{@const trimEndPx = timeToVisiblePx(trimEndTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			<div class="absolute top-0 bottom-0 left-0 bg-black/30 dark:bg-black/50 pointer-events-none z-[8]" style:width="{Math.max(0, trimStartPx)}px"></div>
			<div class="absolute top-0 bottom-0 right-0 bg-black/30 dark:bg-black/50 pointer-events-none z-[8]" style:left="{Math.min(visibleCanvasWidth, trimEndPx)}px"></div>
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-10" style:left="{trimStartPx}px" on:mousedown|preventDefault={(e) => startTrimDrag('trim-left', e)} role="slider" aria-label="Trim start time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={trimStartTime}> <div class="w-1 h-full bg-red-600 rounded-sm group-hover:ring-2 group-hover:ring-red-400 transition-all"></div> <div class="absolute top-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-red-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(trimStartTime)} </div> </div>
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-10" style:left="{trimEndPx}px" on:mousedown|preventDefault={(e) => startTrimDrag('trim-right', e)} role="slider" aria-label="Trim end time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={trimEndTime}> <div class="w-1 h-full bg-red-600 rounded-sm group-hover:ring-2 group-hover:ring-red-400 transition-all"></div> <div class="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-red-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(trimEndTime)} </div> </div>
		{/if}

		{#if showTrimUI && isEditingSegment && visibleCanvasWidth > 0 && actualMediaDuration > 0}
			{@const editStartPx = timeToVisiblePx(editSegmentStartTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			{@const editEndPx = timeToVisiblePx(editSegmentEndTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-30" style:left="{editStartPx}px" on:mousedown|preventDefault={(e) => startEditDrag('edit-left', e)} role="slider" aria-label="Segment start time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={editSegmentStartTime}>
                <div class="w-1 h-full bg-blue-600 rounded-sm group-hover:ring-2 group-hover:ring-blue-400 transition-all"></div>
                <div class="absolute top-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-blue-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(editSegmentStartTime)} </div>
            </div>
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-30" style:left="{editEndPx}px" on:mousedown|preventDefault={(e) => startEditDrag('edit-right', e)} role="slider" aria-label="Segment end time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={editSegmentEndTime}>
                <div class="w-1 h-full bg-blue-600 rounded-sm group-hover:ring-2 group-hover:ring-blue-400 transition-all"></div>
                <div class="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-blue-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(editSegmentEndTime)} </div>
            </div>
		{/if}
	</div>
	<div class="flex-shrink-0 flex flex-col items-center justify-center space-y-1 px-2 py-1 border-l border-gray-200 dark:border-border bg-white dark:bg-d-gray-800">
		<button class="ui-button-icon-panelheader" title="Zoom In Waveform (Ctrl+Scroll)" aria-label="Zoom In Waveform" on:click="{zoomIn}" disabled="{!canZoomIn || !currentAudioBuffer || visibleCanvasWidth <= 0}">
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM10.5 7.5v6m3-3h-6" />
			</svg>
		</button>
		<button class="ui-button-icon-panelheader" title="Zoom Out Waveform (Ctrl+Scroll)" aria-label="Zoom Out Waveform" on:click="{zoomOut}" disabled="{!canZoomOut || !currentAudioBuffer || visibleCanvasWidth <= 0}">
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM13.5 10.5h-6" />
			</svg>
		</button>
	</div>
</div>

<style lang="postcss">
	.interactive-waveform-panel {
	}
	.waveform-scroll-container {
		scrollbar-width: thin;
		scrollbar-color: transparent transparent; /* Default: transparent */
	}
	.waveform-scroll-container:hover {
		scrollbar-color: #a0aec0 #e2e8f0; /* Light mode: gray-400 thumb, gray-200 track */
	}
	.dark .waveform-scroll-container:hover {
		scrollbar-color: theme('colors.d-gray.500') theme('colors.d-gray.800'); /* Dark mode: d-gray-500 thumb, d-gray-800 track */
	}
	.waveform-scroll-container::-webkit-scrollbar {
		height: 8px;
		width: 8px;
	}
	.waveform-scroll-container::-webkit-scrollbar-track {
		background: transparent; /* Default: transparent */
	}
	.waveform-scroll-container:hover::-webkit-scrollbar-track {
		background: #e2e8f0; /* Light mode: gray-200 */
	}
	.dark .waveform-scroll-container:hover::-webkit-scrollbar-track {
		background: theme('colors.d-gray.800'); /* d-gray-800 */
	}
	.waveform-scroll-container::-webkit-scrollbar-thumb {
		background-color: transparent; /* Default: transparent */
		border-radius: 4px;
	}
	.waveform-scroll-container:hover::-webkit-scrollbar-thumb {
		background-color: #a0aec0; /* Light mode: gray-400 */
	}
	.dark .waveform-scroll-container:hover::-webkit-scrollbar-thumb {
		background-color: theme('colors.d-gray.600'); /* d-gray-600 */
	}

	.timescale-canvas { position: absolute; top: 0; left: 0; width: 100%; display: block; pointer-events: none; z-index: 5; }
	.waveform-canvas { position: absolute; left: 0; width: 100%; display: block; }
	.cursor-grabbing { cursor: grabbing; }
	.overlay-message {
		@apply absolute top-0 left-0 w-full h-full flex items-center justify-center text-xs p-1 bg-white bg-opacity-80 dark:bg-d-gray-900 dark:bg-opacity-80 text-gray-600 dark:text-d-gray-300 pointer-events-none z-30;
		text-align: center;
	}
	.ui-button-icon-panelheader {
		@apply p-1 rounded text-gray-600 dark:text-d-gray-400 bg-gray-100 dark:bg-d-gray-700 hover:bg-gray-200 dark:hover:bg-d-gray-600 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-d-gray-800 focus:bg-gray-200 dark:focus:bg-d-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-gray-100 dark:disabled:hover:bg-d-gray-700;
	}
	.w-5 { width: 1.25rem; }
	.h-5 { height: 1.25rem; }

</style>
