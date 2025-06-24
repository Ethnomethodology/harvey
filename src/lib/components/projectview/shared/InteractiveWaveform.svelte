<!-- src/lib/components/projectview/transcriptions/InteractiveWaveform.svelte -->
<script>
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js';
	import { transcriptStore, updatePlayerTime } from '$lib/stores/transcriptStore.js';
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';

	/* -------------------------------------------------- */
	/* Props                                              */
	/* -------------------------------------------------- */
	export let isTrimming = false;
	export let trimStartTime = 0;
	export let trimEndTime = 0;
	export let isEditingSegment = false;
	export let editSegmentStartTime = 0;
	export let editSegmentEndTime = 0;

	export let externalAudioBuffer = null;
	export let externalCurrentTime = null;
	export let externalDuration = null;
	export let externalIsPlaying = null;
	export let externalSegments = null;
	export let externalCurrentSegmentIndex = null;

	/* -------------------------------------------------- */
	/* Constants                                          */
	/* -------------------------------------------------- */
	const TIMESCALE_HEIGHT = 20;

	/* -------------------------------------------------- */
	/* Local state                                       */
	/* -------------------------------------------------- */
	let actualMediaDuration = 0;
	let prevExternalAudioBufferForDuration = null;

	/* -------------------------------------------------- */
	/* Refs                                               */
	/* -------------------------------------------------- */
	let timescaleCanvas; // Visible timescale canvas
	let segmentWaveformCanvas; // Visible waveform canvas
	let waveformScrollContainerRef;

	// Offscreen canvases
	let offscreenTimescaleCanvas = null;
	let offscreenWaveformCanvas = null;
	let offscreenWaveformCtx = null;
	let offscreenTimescaleCtx = null;

	/* -------------------------------------------------- */
	/* Waveform state                                     */
	/* -------------------------------------------------- */
	let visibleCanvasWidth = 0;
	let waveformCanvasHeight = 64 - TIMESCALE_HEIGHT; // Default, updated on resize
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
	let isScrolling = false; // For manual scroll detection
	let debounceScrollTimer = null;
	let autoScrollEnabled = true;
	let autoScrollEnableTimer = null;

	// State for forcing redraw of offscreen canvases
	let needsOffscreenWaveformRedraw = true;
	let needsOffscreenTimescaleRedraw = true;

	// Last drawn state for playhead and dynamic elements to optimize main canvas redraw
	let lastDrawnPlayheadTime = -1;
	const playheadRedrawThreshold = 1 / 60; // approx 60fps

	// Keep track of last drawn parameters for offscreen canvases to decide if they need redraw
	let lastOffscreenDrawnScrollOffset = -1;
	let lastOffscreenDrawnZoomLevel = -1;
	let lastOffscreenDrawnSegmentIndex = -1;
	let lastOffscreenDrawnBuffer = null;
	let lastOffscreenDrawnActualDuration = -1;
	let lastOffscreenDrawnIsEditing = false;
	let lastOffscreenDrawnEditStart = -1;
	let lastOffscreenDrawnEditEnd = -1;
	let lastOffscreenVisibleWidth = -1;
	let lastOffscreenWaveformHeight = -1;


	let isPanning = false;
	let panStartX = 0;
	let panInitialScrollOffsetPx = 0;

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

	function drawWaveformBase(targetCtx, buffer, logicalWidthToDraw, visibleWidthToDraw, scrollOffsetToUse, heightToDraw, color) {
		if (!buffer || !targetCtx || logicalWidthToDraw <= 0 || visibleWidthToDraw <= 0 || heightToDraw <= 0) return;
		const data = buffer.getChannelData(0);
		const totalSamples = data.length;
		const mid = heightToDraw / 2;
		targetCtx.strokeStyle = color;
		targetCtx.lineWidth = 1;

		const visibleStartTime = pxToTime(0, actualMediaDuration, logicalWidthToDraw, visibleWidthToDraw, scrollOffsetToUse);
		const visibleEndTime = pxToTime(visibleWidthToDraw, actualMediaDuration, logicalWidthToDraw, visibleWidthToDraw, scrollOffsetToUse);
		const startSampleIndex = Math.max(0, Math.floor(visibleStartTime * buffer.sampleRate));
		const endSampleIndex = Math.min(totalSamples, Math.ceil(visibleEndTime * buffer.sampleRate));

		if (startSampleIndex >= endSampleIndex) return;

		targetCtx.beginPath();
		for (let x = 0; x < visibleWidthToDraw; x++) {
			const logicalX = x + scrollOffsetToUse;
			const sampleStartIndexForPixel = Math.max(startSampleIndex, Math.floor(logicalX * (totalSamples / logicalWidthToDraw)));
			const sampleEndIndexForPixel = Math.min(endSampleIndex, Math.ceil((logicalX + 1) * (totalSamples / logicalWidthToDraw)));
			if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue;
			let minVal = 0, maxVal = 0; // Renamed to avoid conflict with Math.min/max
			for (let i = sampleStartIndexForPixel; i < sampleEndIndexForPixel; i++) {
				const v = data[i];
				if (v > maxVal) maxVal = v;
				if (v < minVal) minVal = v;
			}
			const yTop = mid + maxVal * mid;
			if (x === 0) targetCtx.moveTo(x + 0.5, yTop);
			else targetCtx.lineTo(x + 0.5, yTop);
		}
		targetCtx.stroke();

		targetCtx.beginPath();
		for (let x = visibleWidthToDraw - 1; x >= 0; x--) {
			const logicalX = x + scrollOffsetToUse;
			const sampleStartIndexForPixel = Math.max(startSampleIndex, Math.floor(logicalX * (totalSamples / logicalWidthToDraw)));
			const sampleEndIndexForPixel = Math.min(endSampleIndex, Math.ceil((logicalX + 1) * (totalSamples / logicalWidthToDraw)));
			if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue;
			let minVal = 0; // Renamed
			for (let i = sampleStartIndexForPixel; i < sampleEndIndexForPixel; i++) {
				const v = data[i];
				if (v < minVal) minVal = v;
			}
			const yBottom = mid + minVal * mid;
			if (x === visibleWidthToDraw - 1) targetCtx.moveTo(x + 0.5, yBottom);
			else targetCtx.lineTo(x + 0.5, yBottom);
		}
		targetCtx.stroke();
	}

	function drawWaveformToOffscreen() {
		const buf = currentAudioBuffer;
		const dur = actualMediaDuration;
		const segments = currentSegmentsToDisplay || [];
		const currentActiveIndex = activeSegmentIndexForDisplay ?? -1;
		const seg = segments[currentActiveIndex];

		if (!offscreenWaveformCtx || !buf || dur <= 0 || visibleCanvasWidth <= 0 || waveformCanvasHeight <= 0 || totalLogicalWidth <= 0) {
			if (offscreenWaveformCanvas && offscreenWaveformCtx) {
				offscreenWaveformCtx.clearRect(0, 0, offscreenWaveformCanvas.width, offscreenWaveformCanvas.height);
			}
			return;
		}

		offscreenWaveformCtx.clearRect(0, 0, offscreenWaveformCanvas.width, offscreenWaveformCanvas.height);

		if (buf.length > 0 && totalLogicalWidth > 0) {
			drawWaveformBase(offscreenWaveformCtx, buf, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx, waveformCanvasHeight, '#9ca3af');
		}

		let highlightStartTime = -1;
		let highlightEndTime = -1;
		let highlightColor = 'rgba(59, 130, 246, 0.15)';
		let waveColor = '#3b82f6';

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
				offscreenWaveformCtx.fillStyle = highlightColor;
				offscreenWaveformCtx.fillRect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight);
				if (buf.length > 0 && totalLogicalWidth > 0) {
					offscreenWaveformCtx.save();
					offscreenWaveformCtx.beginPath();
					offscreenWaveformCtx.rect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight);
					offscreenWaveformCtx.clip();
					drawWaveformBase(offscreenWaveformCtx, buf, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx, waveformCanvasHeight, waveColor);
					offscreenWaveformCtx.restore();
				}
			}
		}
		needsOffscreenWaveformRedraw = false;
		lastOffscreenDrawnScrollOffset = scrollOffsetPx;
		lastOffscreenDrawnZoomLevel = zoomLevel;
		lastOffscreenDrawnSegmentIndex = currentActiveIndex;
		lastOffscreenDrawnBuffer = buf;
		lastOffscreenDrawnActualDuration = dur;
		lastOffscreenDrawnIsEditing = isEditingSegment;
		lastOffscreenDrawnEditStart = editSegmentStartTime;
		lastOffscreenDrawnEditEnd = editSegmentEndTime;
		lastOffscreenVisibleWidth = visibleCanvasWidth;
		lastOffscreenWaveformHeight = waveformCanvasHeight;
	}

	function drawTimescaleToOffscreen() {
		const dur = actualMediaDuration;
		const buf = currentAudioBuffer;
		if (!offscreenTimescaleCtx || !buf || dur <= 0 || visibleCanvasWidth <= 0 || TIMESCALE_HEIGHT <= 0 || totalLogicalWidth <= 0) {
			if (offscreenTimescaleCanvas && offscreenTimescaleCtx) offscreenTimescaleCtx.clearRect(0, 0, offscreenTimescaleCanvas.width, offscreenTimescaleCanvas.height);
			return;
		}

		offscreenTimescaleCtx.clearRect(0, 0, offscreenTimescaleCanvas.width, offscreenTimescaleCanvas.height);
		const isDark = document.documentElement.classList.contains('dark');
		offscreenTimescaleCtx.strokeStyle = '#d1d5db';
		offscreenTimescaleCtx.fillStyle = isDark ? '#ffffff' : '#6b7280';
		offscreenTimescaleCtx.font = '10px sans-serif';
		offscreenTimescaleCtx.textBaseline = 'top';

		const minPixelSpacingForLabel = 60;
		const minPixelSpacingForMinorTick = 10;
		const intervals = [0.1, 0.5, 1, 5, 10, 30, 60, 300, 600, 1800, 3600];
		let interval = intervals[0];
		for (let i = 0; i < intervals.length; i++) {
			const currentIntervalPx = timeToLogicalPx(intervals[i], dur, totalLogicalWidth);
			if (currentIntervalPx >= minPixelSpacingForLabel) { interval = intervals[i]; break; }
			if (i === intervals.length - 1) interval = intervals[i];
		}
		let minorInterval = interval / 5;
		let minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth);
		while (minorIntervalPx < minPixelSpacingForMinorTick && minorInterval < interval) {
			minorInterval *= 2;
			minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth);
		}
		if (minorInterval >= interval) minorInterval = 0;

		const visibleStartTime = pxToTime(0, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const visibleEndTime = pxToTime(visibleCanvasWidth, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx);
		const firstMajorTickTime = Math.floor(visibleStartTime / interval) * interval;
		const firstMinorTickTime = minorInterval > 0 ? Math.floor(visibleStartTime / minorInterval) * minorInterval : 0;

		if (minorInterval > 0) {
			for (let time = firstMinorTickTime; time <= visibleEndTime + minorInterval; time += minorInterval) {
				if (Math.abs(time % interval) < 0.0001 && time > 0) continue;
				if (time < 0) continue;
				const px = timeToVisiblePx(time, dur, totalLogicalWidth, scrollOffsetPx);
				if (px >= 0 && px <= visibleCanvasWidth) {
					offscreenTimescaleCtx.beginPath();
					offscreenTimescaleCtx.moveTo(px + 0.5, TIMESCALE_HEIGHT - 5);
					offscreenTimescaleCtx.lineTo(px + 0.5, TIMESCALE_HEIGHT);
					offscreenTimescaleCtx.stroke();
				}
			}
		}
		offscreenTimescaleCtx.textAlign = 'left';
		for (let time = firstMajorTickTime; time <= visibleEndTime + interval; time += interval) {
			if (time < 0) continue;
			const px = timeToVisiblePx(time, dur, totalLogicalWidth, scrollOffsetPx);
			if (px >= -1 && px <= visibleCanvasWidth + 1) {
				const tickHeight = (Math.abs(time % (interval * 5)) < 0.0001 && interval >= 1) ? 10 : 7;
				offscreenTimescaleCtx.beginPath();
				offscreenTimescaleCtx.moveTo(px + 0.5, TIMESCALE_HEIGHT - tickHeight);
				offscreenTimescaleCtx.lineTo(px + 0.5, TIMESCALE_HEIGHT);
				offscreenTimescaleCtx.stroke();
				let labelStr;
				if (time === 0 && interval < 1.0) { if (interval < 0.01) labelStr = "0.000"; else if (interval < 0.1) labelStr = "0.00"; else labelStr = "0.0"; }
				else if (interval < 0.01) { labelStr = time.toFixed(3); }
				else if (interval < 0.1) { labelStr = time.toFixed(2); }
				else if (interval < 1.0) { const mPart = Math.floor(time / 60); const sPart = time % 60; labelStr = mPart > 0 ? `${mPart}:${sPart.toFixed(1)}` : sPart.toFixed(1); }
				else { const h = Math.floor(time / 3600); const m = Math.floor((time % 3600) / 60); const s = Math.floor(time % 60); labelStr = h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : (m > 0 ? `${m}:${String(s).padStart(2, '0')}` : `${s}`); }
				const textWidth = offscreenTimescaleCtx.measureText(labelStr).width;
				if (px + 3 >= 0 && px + 3 + textWidth <= visibleCanvasWidth + 5) {
					offscreenTimescaleCtx.fillText(labelStr, px + 3, 2);
				}
			}
		}
		offscreenTimescaleCtx.beginPath();
		offscreenTimescaleCtx.moveTo(0, TIMESCALE_HEIGHT - 0.5);
		offscreenTimescaleCtx.lineTo(visibleCanvasWidth, TIMESCALE_HEIGHT - 0.5);
		offscreenTimescaleCtx.strokeStyle = '#d1d5db';
		offscreenTimescaleCtx.lineWidth = 1;
		offscreenTimescaleCtx.stroke();
		needsOffscreenTimescaleRedraw = false;
	}

	function drawVisibleCanvases() {
		const cur = currentPlayTime || 0;
		const dur = actualMediaDuration;
		const dpr = window.devicePixelRatio || 1;

		if (segmentWaveformCanvas && offscreenWaveformCanvas) {
			const mainCtx = segmentWaveformCanvas.getContext('2d');
			if (mainCtx) {
				mainCtx.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height);
				if (offscreenWaveformCanvas.width > 0 && offscreenWaveformCanvas.height > 0) {
					mainCtx.drawImage(offscreenWaveformCanvas, 0, 0, offscreenWaveformCanvas.width / dpr, offscreenWaveformCanvas.height / dpr);
				}
				if (dur > 0 && totalLogicalWidth > 0) {
					const pxCur_visible = timeToVisiblePx(cur, dur, totalLogicalWidth, scrollOffsetPx);
					if (pxCur_visible >= -1 && pxCur_visible <= visibleCanvasWidth + 1) {
						mainCtx.save();
						mainCtx.scale(dpr,dpr); // Scale after clear and drawImage, before drawing playhead
						mainCtx.strokeStyle = '#ef4444';
						mainCtx.lineWidth = 1.5;
						mainCtx.beginPath();
						mainCtx.moveTo(pxCur_visible + 0.5, 0);
						mainCtx.lineTo(pxCur_visible + 0.5, waveformCanvasHeight);
						mainCtx.stroke();
						mainCtx.restore();
					}
				}
			}
		}
		if (timescaleCanvas && offscreenTimescaleCanvas) {
			const mainTimescaleCtx = timescaleCanvas.getContext('2d');
			if (mainTimescaleCtx) {
				mainTimescaleCtx.clearRect(0, 0, timescaleCanvas.width, timescaleCanvas.height);
				if (offscreenTimescaleCanvas.width > 0 && offscreenTimescaleCanvas.height > 0) {
					mainTimescaleCtx.drawImage(offscreenTimescaleCanvas, 0, 0, offscreenTimescaleCanvas.width / dpr, offscreenTimescaleCanvas.height / dpr);
				}
			}
		}
		lastDrawnPlayheadTime = cur;
	}

	function clearAllCanvases() {
		const dpr = window.devicePixelRatio || 1;
		if (segmentWaveformCanvas) {
			const ctx = segmentWaveformCanvas.getContext('2d');
			if (ctx) {
				ctx.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height);
				if (visibleCanvasWidth > 0 && waveformCanvasHeight > 0) {
					ctx.save(); ctx.scale(dpr,dpr); ctx.fillStyle = '#6b7280'; ctx.font = `10px sans-serif`; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
					let message = 'Waveform';
					if (!webAudioApiSupported) message = 'Web Audio API not supported.';
					else if (!currentAudioBuffer) message = 'Load media to see waveform.';
					ctx.fillText(message, visibleCanvasWidth / 2, waveformCanvasHeight / 2 );
					ctx.restore();
				}
			}
		}
		if (timescaleCanvas) {
			const ctx = timescaleCanvas.getContext('2d');
			if (ctx) ctx.clearRect(0, 0, timescaleCanvas.width, timescaleCanvas.height);
		}
		if (offscreenWaveformCtx) offscreenWaveformCtx.clearRect(0, 0, offscreenWaveformCanvas.width, offscreenWaveformCanvas.height);
		if (offscreenTimescaleCtx) offscreenTimescaleCtx.clearRect(0, 0, offscreenTimescaleCanvas.width, offscreenTimescaleCanvas.height);
	}


	let forceNextRedraw = false;
	function requestRedraw(force = false) {
		if (force) {
			needsOffscreenWaveformRedraw = true;
			needsOffscreenTimescaleRedraw = true;
		}
	}

	function animationLoop() {
		if (!isMounted) return;

		const curTime = currentPlayTime || 0;
		const audioBuf = currentAudioBuffer;
		const currentDur = actualMediaDuration;
		const activeIdx = activeSegmentIndexForDisplay ?? -1;
		const visWidth = visibleCanvasWidth;
		const waveHeight = waveformCanvasHeight;
		const currentScroll = scrollOffsetPx;
		const currentZoom = zoomLevel;
		const currentIsEdit = isEditingSegment;
		const currentEditStart = editSegmentStartTime;
		const currentEditEnd = editSegmentEndTime;

		if (forceNextRedraw) {
			needsOffscreenWaveformRedraw = true;
			needsOffscreenTimescaleRedraw = true;
			forceNextRedraw = false;
		} else {
			if (audioBuf !== lastOffscreenDrawnBuffer || activeIdx !== lastOffscreenDrawnSegmentIndex ||
				currentScroll !== lastOffscreenDrawnScrollOffset || currentZoom !== lastOffscreenDrawnZoomLevel ||
				currentDur !== lastOffscreenDrawnActualDuration || currentIsEdit !== lastOffscreenDrawnIsEditing ||
				(currentIsEdit && (currentEditStart !== lastOffscreenDrawnEditStart || currentEditEnd !== lastOffscreenDrawnEditEnd)) ||
				visWidth !== lastOffscreenVisibleWidth || waveHeight !== lastOffscreenWaveformHeight
			) {
				needsOffscreenWaveformRedraw = true;
			}
			if (currentScroll !== lastOffscreenDrawnScrollOffset || currentZoom !== lastOffscreenDrawnZoomLevel ||
				currentDur !== lastOffscreenDrawnActualDuration || visWidth !== lastOffscreenVisibleWidth
			) {
				needsOffscreenTimescaleRedraw = true;
			}
		}

		let didOffscreenRedraw = false;
		if (visWidth <= 0 || !audioBuf || currentDur <= 0 || totalLogicalWidth <= 0) {
			clearAllCanvases();
			lastDrawnPlayheadTime = curTime;
		} else {
			if (needsOffscreenWaveformRedraw) {
				drawWaveformToOffscreen();
				didOffscreenRedraw = true;
			}
			if (needsOffscreenTimescaleRedraw) {
				drawTimescaleToOffscreen();
				didOffscreenRedraw = true;
			}

			if (didOffscreenRedraw || Math.abs(curTime - lastDrawnPlayheadTime) > playheadRedrawThreshold) {
				drawVisibleCanvases();
			}
		}


		if (autoScrollEnabled && currentDur > 0 && totalLogicalWidth > visWidth && !isTrimming && !isEditingSegment) {
			const pxCur_visible = timeToVisiblePx(curTime, currentDur, totalLogicalWidth, currentScroll);
			const scrollMarginLeft = visWidth * 0.25;
			const scrollMarginRight = visWidth * 0.75;
			let targetScrollOffset = currentScroll;
			let needsScrollUpdate = false;

			if (pxCur_visible < scrollMarginLeft) {
				targetScrollOffset = timeToLogicalPx(curTime, currentDur, totalLogicalWidth) - scrollMarginLeft;
				needsScrollUpdate = true;
			} else if (pxCur_visible > scrollMarginRight) {
				targetScrollOffset = timeToLogicalPx(curTime, currentDur, totalLogicalWidth) - scrollMarginRight;
				needsScrollUpdate = true;
			}

			if (needsScrollUpdate) {
				targetScrollOffset = Math.max(0, Math.min(targetScrollOffset, maxScrollPx));
				const diff = targetScrollOffset - currentScroll;
				const moveAmount = diff * 0.1;
				let newScrollOffset = currentScroll + moveAmount;
				if (Math.abs(diff) < 1) newScrollOffset = targetScrollOffset;
				newScrollOffset = Math.round(newScrollOffset);
				if (Math.abs(newScrollOffset - scrollOffsetPx) > 0) {
					scrollOffsetPx = newScrollOffset;
					if (waveformScrollContainerRef) waveformScrollContainerRef.scrollLeft = scrollOffsetPx;

				}
			}
		}
		animationFrameId = requestAnimationFrame(animationLoop);
	}

	function resetZoomAndScrollState(clearBuffer = true) {
		zoomLevel = 1;
		scrollOffsetPx = 0;
		autoScrollEnabled = true;
		clearTimeout(autoScrollEnableTimer);
		autoScrollEnableTimer = null;

		lastDrawnPlayheadTime = -1;
		lastOffscreenDrawnScrollOffset = -1;
		lastOffscreenDrawnZoomLevel = -1;
		lastOffscreenDrawnSegmentIndex = -1;
		if (clearBuffer) lastOffscreenDrawnBuffer = null;
		lastOffscreenDrawnActualDuration = -1;
		lastOffscreenDrawnIsEditing = false;
		lastOffscreenDrawnEditStart = -1;
		lastOffscreenDrawnEditEnd = -1;
		lastOffscreenVisibleWidth = -1;
		lastOffscreenWaveformHeight = -1;

		needsOffscreenWaveformRedraw = true;
		needsOffscreenTimescaleRedraw = true;
		clearAllCanvases();
		requestRedraw(true);
	}

	onMount(() => {
		isMounted = true;
		webAudioApiSupported = typeof window.AudioContext !== 'undefined' || typeof window.webkitAudioContext !== 'undefined';

		if (typeof document !== 'undefined') {
			offscreenWaveformCanvas = document.createElement('canvas');
			offscreenWaveformCtx = offscreenWaveformCanvas.getContext('2d');
			offscreenTimescaleCanvas = document.createElement('canvas');
			offscreenTimescaleCtx = offscreenTimescaleCanvas.getContext('2d');
		}

		unsubscribeAudioBuffer = transcriptStore.subscribe(ts => {
			if (externalAudioBuffer === null) {
				const newAudioBuffer = ts.audioBuffer;
				if (newAudioBuffer && newAudioBuffer !== currentAudioBuffer) {
					needsOffscreenWaveformRedraw = true; needsOffscreenTimescaleRedraw = true;
				} else if (!newAudioBuffer && currentAudioBuffer && !externalAudioBuffer) {
                    actualMediaDuration = 0;
					needsOffscreenWaveformRedraw = true; needsOffscreenTimescaleRedraw = true;
                }
			}
		});
		unsubscribePlayer = transcriptStore.subscribe(ts => { /* Player state changes picked up by animationLoop */ });
		unsubscribeSegments = transcriptStore.subscribe(ts => { needsOffscreenWaveformRedraw = true; /* Segments changed */ });

		prevExternalAudioBufferForDuration = externalAudioBuffer;
		if (externalAudioBuffer && externalAudioBuffer.duration > 0) {
			actualMediaDuration = externalAudioBuffer.duration;
		} else if ($transcriptStore.audioBuffer && $transcriptStore.audioBuffer.duration > 0 && !externalAudioBuffer) {
            actualMediaDuration = $transcriptStore.audioBuffer.duration;
        }

		tick().then(() => {
			if (isMounted) {
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

		offscreenWaveformCanvas = null; offscreenWaveformCtx = null;
		offscreenTimescaleCanvas = null; offscreenTimescaleCtx = null;
	});

	function setupResizeObserver() {
		if (waveformScrollContainerRef && !isObserverSetup && isMounted && typeof window !== 'undefined' && window.ResizeObserver) {
			isObserverSetup = true;

			const updateDimensions = () => {
				if (!waveformScrollContainerRef) return false;
				const newWidth = waveformScrollContainerRef.clientWidth;
				const newContainerHeight = waveformScrollContainerRef.offsetHeight;
				const newWaveformHeight = (newContainerHeight || 80) - TIMESCALE_HEIGHT;
				let changed = false;

				if (newWaveformHeight > 0 && newWaveformHeight !== waveformCanvasHeight) {
					waveformCanvasHeight = newWaveformHeight;
					changed = true;
				}
				if (newWidth > 0 && newWidth !== visibleCanvasWidth) {
					visibleCanvasWidth = newWidth;
					changed = true;
				} else if (newWidth <= 0 && visibleCanvasWidth !== 0) {
					visibleCanvasWidth = 0;
					scrollOffsetPx = 0;
					changed = true;
					clearAllCanvases();
				}
				return changed;
			};

			updateDimensions();

			resizeObserverInstance = new ResizeObserver((entries) => {
				let dimensionsChanged = false;
				for (const entry of entries) {
					if (entry.target === waveformScrollContainerRef) {
						if (updateDimensions()) {
							dimensionsChanged = true;
						}
					}
				}
				if (dimensionsChanged) {
					const dpr = window.devicePixelRatio || 1;
					if (visibleCanvasWidth > 0 && TIMESCALE_HEIGHT > 0 && offscreenTimescaleCanvas && offscreenTimescaleCtx) {
						offscreenTimescaleCanvas.width = Math.round(visibleCanvasWidth * dpr);
						offscreenTimescaleCanvas.height = Math.round(TIMESCALE_HEIGHT * dpr);
						offscreenTimescaleCtx.scale(dpr,dpr);
						needsOffscreenTimescaleRedraw = true;
					}
					if (visibleCanvasWidth > 0 && waveformCanvasHeight > 0 && offscreenWaveformCanvas && offscreenWaveformCtx) {
						offscreenWaveformCanvas.width = Math.round(visibleCanvasWidth * dpr);
						offscreenWaveformCanvas.height = Math.round(waveformCanvasHeight * dpr);
						offscreenWaveformCtx.scale(dpr,dpr);
						needsOffscreenWaveformRedraw = true;
					}

					if (segmentWaveformCanvas) {
						segmentWaveformCanvas.width = Math.round(visibleCanvasWidth * dpr);
						segmentWaveformCanvas.height = Math.round(waveformCanvasHeight * dpr);
					}
					if (timescaleCanvas) {
						timescaleCanvas.width = Math.round(visibleCanvasWidth * dpr);
						timescaleCanvas.height = Math.round(TIMESCALE_HEIGHT * dpr);
					}
					requestRedraw(true);
				}
			});
			resizeObserverInstance.observe(waveformScrollContainerRef);

			const dpr = window.devicePixelRatio || 1;
			if (visibleCanvasWidth > 0 && TIMESCALE_HEIGHT > 0 && offscreenTimescaleCanvas && offscreenTimescaleCtx) {
				offscreenTimescaleCanvas.width = Math.round(visibleCanvasWidth * dpr);
				offscreenTimescaleCanvas.height = Math.round(TIMESCALE_HEIGHT * dpr);
				offscreenTimescaleCtx.scale(dpr,dpr);
			}
			if (visibleCanvasWidth > 0 && waveformCanvasHeight > 0 && offscreenWaveformCanvas && offscreenWaveformCtx) {
				offscreenWaveformCanvas.width = Math.round(visibleCanvasWidth * dpr);
				offscreenWaveformCanvas.height = Math.round(waveformCanvasHeight * dpr);
				offscreenWaveformCtx.scale(dpr,dpr);
			}
			if (segmentWaveformCanvas) {
				segmentWaveformCanvas.width = Math.round(visibleCanvasWidth * dpr);
				segmentWaveformCanvas.height = Math.round(waveformCanvasHeight * dpr);
			}
			if (timescaleCanvas) {
				timescaleCanvas.width = Math.round(visibleCanvasWidth * dpr);
				timescaleCanvas.height = Math.round(TIMESCALE_HEIGHT * dpr);
			}
			requestRedraw(true);
		}
	}
	$: if (waveformScrollContainerRef && !isObserverSetup && isMounted) { setupResizeObserver(); }

	function handleScroll(event) {
		const newScrollOffset = Math.round(event.target.scrollLeft);
		if (Math.abs(newScrollOffset - scrollOffsetPx) > 0) {
		    const wasManualScroll = isScrolling;
		    if (autoScrollEnabled && !wasManualScroll && !isTrimming && !isEditingSegment) {
		        autoScrollEnabled = false;
		        clearTimeout(autoScrollEnableTimer);
		        autoScrollEnableTimer = null;
		    }
		    scrollOffsetPx = newScrollOffset;
			needsOffscreenWaveformRedraw = true;
			needsOffscreenTimescaleRedraw = true;
		    isScrolling = true;
		    clearTimeout(debounceScrollTimer);
		    debounceScrollTimer = setTimeout(() => {
		        isScrolling = false;
		        if (!autoScrollEnabled && !autoScrollEnableTimer && !isTrimming && !isEditingSegment) {
		            autoScrollEnableTimer = setTimeout(() => {
		                autoScrollEnabled = true;
		                autoScrollEnableTimer = null;
		                requestRedraw(true);
		            }, 1500);
		        }
		        requestRedraw(true);
		    }, 150);
		    requestRedraw();
		}
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
		zoomLevel = newZoomLevel;

		tick().then(() => {
			const newTotalLogicalWidthAfterZoom = totalLogicalWidth;
			let newScrollOffset = newTotalLogicalWidthAfterZoom - visibleCanvasWidth;
			const newMaxScroll = Math.max(0, newTotalLogicalWidthAfterZoom - visibleCanvasWidth);
			newScrollOffset = Math.max(0, Math.min(newScrollOffset, newMaxScroll));
			scrollOffsetPx = Math.round(newScrollOffset);

			const wasAutoScrollEnabled = autoScrollEnabled; autoScrollEnabled = false; clearTimeout(autoScrollEnableTimer);
			needsOffscreenWaveformRedraw = true; needsOffscreenTimescaleRedraw = true;
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
		if (draggingHandle === 'trim-left') {
			newStartTime = Math.max(0, Math.min(newTime, trimEndTime - minDuration));
		} else { // trim-right
			newEndTime = Math.min(actualMediaDuration, Math.max(newTime, trimStartTime + minDuration));
			// console.log(`[Waveform.trim-right] clientX: ${event.clientX.toFixed(2)}, rect.left: ${rect.left.toFixed(2)}, clickX: ${clickX.toFixed(2)}`);
			// console.log(`[Waveform.trim-right] actualMediaDuration: ${actualMediaDuration.toFixed(3)}, totalWidth: ${totalLogicalWidth.toFixed(2)}, visWidth: ${visibleCanvasWidth.toFixed(2)}, scroll: ${scrollOffsetPx.toFixed(2)}`);
			// console.log(`[Waveform.trim-right] newTime: ${newTime.toFixed(3)}, trimStart: ${trimStartTime.toFixed(3)}, newEnd (pre-dispatch): ${newEndTime.toFixed(3)}`);
		}
		if (newStartTime !== trimStartTime || newEndTime !== trimEndTime) { dispatch('trimupdate', { startTime: newStartTime, endTime: newEndTime }); }
	}
	function handleTrimMouseUp() { if (draggingHandle === 'trim-left' || draggingHandle === 'trim-right') { draggingHandle = null; window.removeEventListener('mousemove', handleTrimMouseMove); } }

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
        // console.log('[Waveform] externalAudioBuffer prop changed, triggering actualMediaDuration update logic.');
    } else if (isMounted && !externalAudioBuffer && prevExternalAudioBufferForDuration && $transcriptStore.audioBuffer !== prevExternalAudioBufferForDuration) {
        // console.log('[Waveform] externalAudioBuffer prop removed, currentAudioBuffer will now rely on store.');
    }
</script>

<div class="waveform-container relative flex items-center space-x-1 bg-gray-100 dark:bg-app-bg-dark h-full">
	<div
		bind:this={waveformScrollContainerRef}
		class="waveform-scroll-container flex-grow bg-white dark:bg-[#3c3c3c] rounded border border-gray-300 dark:border-gray-600 relative overflow-x-scroll overflow-y-hidden h-full"
		role="region" aria-label="Interactive Waveform Timeline"
		on:scroll={handleScroll}
	>
		<canvas bind:this={timescaleCanvas} class="timescale-canvas" style="height: {TIMESCALE_HEIGHT}px;" aria-hidden="true" />
		<canvas
			bind:this={segmentWaveformCanvas}
			class="waveform-canvas {(currentAudioBuffer && !isTrimming && !isEditingSegment && !isPanning) ? 'cursor-pointer' : (isPanning ? 'cursor-grabbing' : 'cursor-default')}"
			style="height: {waveformCanvasHeight}px; top: {TIMESCALE_HEIGHT}px;"
			aria-label="Waveform visualization. Click to seek audio."
			on:click|self={handleCanvasClick}
			on:mousedown|self={handlePanStart}
			on:wheel|preventDefault={(e) => { if (e.ctrlKey || e.metaKey) { handleZoom(e.deltaY < 0 ? 'in' : 'out'); } }} />
		{#if !webAudioApiSupported} <div class="waveform-overlay"><p class="waveform-overlay-text text-red-600">Web Audio API not supported.</p></div> {:else if !currentAudioBuffer} <div class="waveform-overlay"><p class="waveform-overlay-text">Load audio/video media to view waveform.</p></div> {/if}

		{#if isTrimming && visibleCanvasWidth > 0 && actualMediaDuration > 0}
			{@const trimStartPx = timeToVisiblePx(trimStartTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			{@const trimEndPx = timeToVisiblePx(trimEndTime, actualMediaDuration, totalLogicalWidth, scrollOffsetPx)}
			<div class="absolute top-0 bottom-0 left-0 bg-black/30 dark:bg-black/50 pointer-events-none z-[8]" style:width="{Math.max(0, trimStartPx)}px"></div>
			<div class="absolute top-0 bottom-0 right-0 bg-black/30 dark:bg-black/50 pointer-events-none z-[8]" style:left="{Math.min(visibleCanvasWidth, trimEndPx)}px"></div>
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-10" style:left="{trimStartPx}px" on:mousedown|preventDefault={(e) => startTrimDrag('trim-left', e)} role="slider" aria-label="Trim start time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={trimStartTime}> <div class="w-1 h-full bg-red-600 rounded-sm group-hover:ring-2 group-hover:ring-red-400 transition-all"></div> <div class="absolute top-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-red-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(trimStartTime)} </div> </div>
			<div class="absolute top-0 bottom-0 -translate-x-1/2 w-2.5 flex items-center justify-center cursor-ew-resize group z-10" style:left="{trimEndPx}px" on:mousedown|preventDefault={(e) => startTrimDrag('trim-right', e)} role="slider" aria-label="Trim end time" aria-valuemin="0" aria-valuemax={actualMediaDuration} aria-valuenow={trimEndTime}> <div class="w-1 h-full bg-red-600 rounded-sm group-hover:ring-2 group-hover:ring-red-400 transition-all"></div> <div class="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 px-1.5 py-0.5 bg-red-600 text-white text-[10px] font-mono rounded shadow whitespace-nowrap pointer-events-none"> {formatTimestamp(trimEndTime)} </div> </div>
		{/if}

		{#if isEditingSegment && visibleCanvasWidth > 0 && actualMediaDuration > 0}
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

	<div class="flex flex-col space-y-1 flex-shrink-0">
		<button class="ui-button-icon" title="Zoom In Waveform (Ctrl+Scroll)" aria-label="Zoom In Waveform" on:click="{zoomIn}" disabled="{!canZoomIn || !currentAudioBuffer || visibleCanvasWidth <= 0}" > <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM10.5 7.5v6m3-3h-6" /> </svg> </button>
		<button class="ui-button-icon" title="Zoom Out Waveform (Ctrl+Scroll)" aria-label="Zoom Out Waveform" on:click="{zoomOut}" disabled="{!canZoomOut || !currentAudioBuffer || visibleCanvasWidth <= 0}" > <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6"> <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM13.5 10.5h-6" /> </svg> </button>
   </div>
</div>

<style lang="postcss">
	.waveform-container { }
	.waveform-scroll-container { scrollbar-width: thin; scrollbar-color: #a0aec0 #e2e8f0; scrollbar-gutter: stable; }
	.dark .waveform-scroll-container { scrollbar-color: #6b7280 #3c3c3c; }
	.waveform-scroll-container::-webkit-scrollbar { height: 8px; width: 8px; }
	.waveform-scroll-container::-webkit-scrollbar-track { background: #e2e8f0; border-radius: 4px; }
	.dark .waveform-scroll-container::-webkit-scrollbar-track { background: #3c3c3c; }
	.waveform-scroll-container::-webkit-scrollbar-thumb { background-color: #a0aec0; border-radius: 4px; border: 2px solid #e2e8f0; }
	.dark .waveform-scroll-container::-webkit-scrollbar-thumb { background-color: #6b7280; border-color: #3c3c3c;}
	.waveform-scroll-container::-webkit-scrollbar-thumb:hover { background-color: #718096; }
	.dark .waveform-scroll-container::-webkit-scrollbar-thumb:hover { background-color: #4a5568; }
	.timescale-canvas { position: absolute; top: 0; left: 0; width: 100%; display: block; pointer-events: none; z-index: 5; }
	.waveform-canvas { position: absolute; left: 0; width: 100%; display: block; }
	.cursor-grabbing { cursor: grabbing; }
	.waveform-overlay { @apply absolute inset-0 flex items-center justify-center bg-gray-100/70 dark:bg-gray-700/70 pointer-events-none rounded z-[5]; }
	.waveform-overlay-text { @apply text-xs p-1 bg-white/80 dark:bg-gray-900/80 rounded shadow-sm font-sans text-center text-gray-600 dark:text-gray-300; }
	.ui-button-icon { @apply p-1 rounded text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-gray-100 dark:disabled:hover:bg-gray-700; }
	.size-6 { @apply w-6 h-6; }
</style>