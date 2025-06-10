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
	let timescaleCanvas;
	let segmentWaveformCanvas;
	let waveformScrollContainerRef;

	/* -------------------------------------------------- */
	/* Waveform state                                     */
	/* -------------------------------------------------- */
	let visibleCanvasWidth = 0;
	let waveformCanvasHeight = 64 - TIMESCALE_HEIGHT;
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

	$: currentAudioBuffer = externalAudioBuffer ?? $transcriptStore.audioBuffer;
	$: currentPlayTime = externalCurrentTime ?? $transcriptStore.player.currentTime;
	$: currentIsPlaying = externalIsPlaying ?? $transcriptStore.player.isPlaying;
	$: currentSegmentsToDisplay = externalSegments ?? $transcriptStore.segments;
	$: activeSegmentIndexForDisplay = externalCurrentSegmentIndex ?? $transcriptStore.player.currentSegmentIndex;

	$: {
		if (currentAudioBuffer && currentAudioBuffer !== prevExternalAudioBufferForDuration) {
			if (currentAudioBuffer.duration > 0) {
				actualMediaDuration = currentAudioBuffer.duration;
				// console.log(`[Waveform] actualMediaDuration synced from currentAudioBuffer: ${actualMediaDuration}`);
				zoomLevel = 1;
				scrollOffsetPx = 0;
				if (waveformScrollContainerRef) waveformScrollContainerRef.scrollLeft = 0;
				requestRedraw(true);
			} else {
				actualMediaDuration = 0;
				// console.warn(`[Waveform] currentAudioBuffer is present but has duration 0 or less. actualMediaDuration set to 0.`);
			}
			prevExternalAudioBufferForDuration = currentAudioBuffer;
		} else if (!currentAudioBuffer && prevExternalAudioBufferForDuration) {
			actualMediaDuration = 0;
			prevExternalAudioBufferForDuration = null;
			// console.log(`[Waveform] currentAudioBuffer became null. actualMediaDuration reset to 0.`);
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

	function drawVisibleWaveform(ctx, buffer, logicalWidth, visibleWidth, scrollOffset, height, color) { if (!buffer || !ctx || logicalWidth <= 0 || visibleWidth <= 0 || height <= 0) return; const data = buffer.getChannelData(0); const totalSamples = data.length; const mid = height / 2; ctx.strokeStyle = color; ctx.lineWidth = 1; const visibleStartTime = pxToTime(0, actualMediaDuration, logicalWidth, visibleWidth, scrollOffset); const visibleEndTime = pxToTime(visibleWidth, actualMediaDuration, logicalWidth, visibleWidth, scrollOffset); const startSampleIndex = Math.max(0, Math.floor(visibleStartTime * buffer.sampleRate)); const endSampleIndex = Math.min(totalSamples, Math.ceil(visibleEndTime * buffer.sampleRate)); if (startSampleIndex >= endSampleIndex) return; ctx.beginPath(); for (let x = 0; x < visibleWidth; x++) { const logicalX = x + scrollOffset; const sampleStartIndexForPixel = Math.max(startSampleIndex, Math.floor(logicalX * (totalSamples / logicalWidth))); const sampleEndIndexForPixel = Math.min(endSampleIndex, Math.ceil((logicalX + 1) * (totalSamples / logicalWidth))); if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue; let min = 0, max = 0; for (let i = sampleStartIndexForPixel; i < sampleEndIndexForPixel; i++) { const v = data[i]; if (v > max) max = v; if (v < min) min = v; } const yTop = mid + max * mid; if (x === 0) ctx.moveTo(x + 0.5, yTop); else ctx.lineTo(x + 0.5, yTop); } ctx.stroke(); ctx.beginPath(); for (let x = visibleWidth - 1; x >= 0; x--) { const logicalX = x + scrollOffset; const sampleStartIndexForPixel = Math.max(startSampleIndex, Math.floor(logicalX * (totalSamples / logicalWidth))); const sampleEndIndexForPixel = Math.min(endSampleIndex, Math.ceil((logicalX + 1) * (totalSamples / logicalWidth))); if (sampleStartIndexForPixel >= sampleEndIndexForPixel) continue; let min = 0; for (let i = sampleStartIndexForPixel; i < sampleEndIndexForPixel; i++) { const v = data[i]; if (v < min) min = v; } const yBottom = mid + min * mid; if (x === visibleWidth - 1) ctx.moveTo(x + 0.5, yBottom); else ctx.lineTo(x + 0.5, yBottom); } ctx.stroke(); }
	function clearWaveformCanvases() { if (segmentWaveformCanvas) { const c = segmentWaveformCanvas.getContext('2d'); if (c) c.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height); } if (timescaleCanvas) { const c = timescaleCanvas.getContext('2d'); if (c) c.clearRect(0, 0, timescaleCanvas.width, timescaleCanvas.height); } if (segmentWaveformCanvas && visibleCanvasWidth > 0 && waveformCanvasHeight > 0) { const c = segmentWaveformCanvas.getContext('2d'); if (c) { const dpr = window.devicePixelRatio || 1; c.save(); c.scale(dpr, dpr); c.fillStyle = '#6b7280'; c.font = `10px sans-serif`; c.textAlign = 'center'; c.textBaseline = 'middle'; let message = 'Waveform'; if (!webAudioApiSupported) message = 'Web Audio API not supported.'; else if (!currentAudioBuffer) message = 'Load media to see waveform.'; c.fillText(message, visibleCanvasWidth / 2, waveformCanvasHeight / 2 ); c.restore(); } } }
	function drawTimescale() { const dur = actualMediaDuration; const buf = currentAudioBuffer; const dpr = window.devicePixelRatio || 1; if (!timescaleCanvas || !buf || dur <= 0 || visibleCanvasWidth <= 0 || TIMESCALE_HEIGHT <= 0 || totalLogicalWidth <= 0) { if (timescaleCanvas) { timescaleCanvas.width = 0; timescaleCanvas.height = 0; } return; } const ctx = timescaleCanvas.getContext('2d'); if (!ctx) return; const reqW = Math.round(visibleCanvasWidth * dpr); const reqH = Math.round(TIMESCALE_HEIGHT * dpr); if (timescaleCanvas.width !== reqW || timescaleCanvas.height !== reqH) { timescaleCanvas.width = reqW; timescaleCanvas.height = reqH; } ctx.save(); ctx.scale(dpr, dpr); ctx.clearRect(0, 0, visibleCanvasWidth, TIMESCALE_HEIGHT); const isDark = document.documentElement.classList.contains('dark'); ctx.strokeStyle = '#d1d5db'; ctx.fillStyle = isDark ? '#ffffff' : '#6b7280'; ctx.font = '10px sans-serif'; ctx.textBaseline = 'top'; const minPixelSpacingForLabel = 60; const minPixelSpacingForMinorTick = 10; const intervals = [0.1, 0.5, 1, 5, 10, 30, 60, 300, 600, 1800, 3600]; let interval = intervals[0]; let intervalPx = timeToLogicalPx(interval, dur, totalLogicalWidth); for (let i = 0; i < intervals.length; i++) { const currentIntervalPx = timeToLogicalPx(intervals[i], dur, totalLogicalWidth); if (currentIntervalPx >= minPixelSpacingForLabel) { interval = intervals[i]; intervalPx = currentIntervalPx; break; } if (i === intervals.length - 1) { interval = intervals[i]; intervalPx = currentIntervalPx; } } let minorInterval = interval / 5; let minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth); while (minorIntervalPx < minPixelSpacingForMinorTick && minorInterval < interval) { minorInterval *= 2; minorIntervalPx = timeToLogicalPx(minorInterval, dur, totalLogicalWidth); } if (minorInterval >= interval) minorInterval = 0; const visibleStartTime = pxToTime(0, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx); const visibleEndTime = pxToTime(visibleCanvasWidth, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx); const firstMajorTickTime = Math.floor(visibleStartTime / interval) * interval; const firstMinorTickTime = minorInterval > 0 ? Math.floor(visibleStartTime / minorInterval) * minorInterval : 0; if (minorInterval > 0) { for (let time = firstMinorTickTime; time <= visibleEndTime + minorInterval; time += minorInterval) { if (Math.abs(time % interval) < 0.0001 && time > 0) continue; if (time < 0) continue; const px = timeToVisiblePx(time, dur, totalLogicalWidth, scrollOffsetPx); if (px >= 0 && px <= visibleCanvasWidth) { ctx.beginPath(); ctx.moveTo(px + 0.5, TIMESCALE_HEIGHT - 5); ctx.lineTo(px + 0.5, TIMESCALE_HEIGHT); ctx.stroke(); } } } ctx.textAlign = 'left'; for (let time = firstMajorTickTime; time <= visibleEndTime + interval; time += interval) { if (time < 0) continue; const px = timeToVisiblePx(time, dur, totalLogicalWidth, scrollOffsetPx); if (px >= -1 && px <= visibleCanvasWidth + 1) { const tickHeight = (Math.abs(time % (interval * 5)) < 0.0001 && interval >= 1) ? 10 : 7; ctx.beginPath(); ctx.moveTo(px + 0.5, TIMESCALE_HEIGHT - tickHeight); ctx.lineTo(px + 0.5, TIMESCALE_HEIGHT); ctx.stroke(); const label = formatTimescaleTime(time, dur); const textWidth = ctx.measureText(label).width; const textPadding = 3; if (px + textPadding >= 0 && px + textPadding + textWidth <= visibleCanvasWidth + 5) { ctx.fillText(label, px + textPadding, 2); } } } ctx.beginPath(); ctx.moveTo(0, TIMESCALE_HEIGHT - 0.5); ctx.lineTo(visibleCanvasWidth, TIMESCALE_HEIGHT - 0.5); ctx.strokeStyle = '#d1d5db'; ctx.lineWidth = 1; ctx.stroke(); ctx.restore(); }
	function drawSegmentWaveformUI() { const buf = currentAudioBuffer; const cur = currentPlayTime || 0; const dur = actualMediaDuration; const segments = currentSegmentsToDisplay || []; const currentActiveIndex = activeSegmentIndexForDisplay ?? -1; const seg = segments[currentActiveIndex]; const dpr = window.devicePixelRatio || 1; if (!segmentWaveformCanvas || !buf || dur <= 0 || visibleCanvasWidth <= 0 || waveformCanvasHeight <= 0 || totalLogicalWidth <= 0) { if(segmentWaveformCanvas) { const c = segmentWaveformCanvas.getContext('2d'); if(c) c.clearRect(0, 0, segmentWaveformCanvas.width, segmentWaveformCanvas.height); segmentWaveformCanvas.width = 0; segmentWaveformCanvas.height = 0; } return; } const ctx = segmentWaveformCanvas.getContext('2d'); if (!ctx) return; const reqW = Math.round(visibleCanvasWidth * dpr); const reqH = Math.round(waveformCanvasHeight * dpr); if (segmentWaveformCanvas.width !== reqW || segmentWaveformCanvas.height !== reqH) { segmentWaveformCanvas.width = reqW; segmentWaveformCanvas.height = reqH; } ctx.save(); ctx.scale(dpr, dpr); ctx.clearRect(0, 0, visibleCanvasWidth, waveformCanvasHeight); if (buf.length > 0 && totalLogicalWidth > 0) { drawVisibleWaveform(ctx, buf, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx, waveformCanvasHeight, '#9ca3af'); } let highlightStartTime = -1; let highlightEndTime = -1; let highlightColor = 'rgba(59, 130, 246, 0.15)'; let waveColor = '#3b82f6'; if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) { highlightStartTime = editSegmentStartTime; highlightEndTime = editSegmentEndTime; } else if (currentActiveIndex >= 0 && currentActiveIndex < segments.length && seg) { const segStartTime = Number(seg.start_time); const segEndTime = Number(seg.end_time); if (!isNaN(segStartTime) && !isNaN(segEndTime) && segEndTime >= segStartTime) { highlightStartTime = segStartTime; highlightEndTime = segEndTime; } else { console.warn(`[Waveform Draw] Invalid time data for ACTIVE segment ${currentActiveIndex}.`); } } if (highlightStartTime >= 0 && highlightEndTime >= highlightStartTime) { const pxS_logical = timeToLogicalPx(highlightStartTime, dur, totalLogicalWidth); const pxE_logical = timeToLogicalPx(highlightEndTime, dur, totalLogicalWidth); const pxS_visible = pxS_logical - scrollOffsetPx; const pxE_visible = pxE_logical - scrollOffsetPx; const clamped_pxS_visible = Math.max(0, pxS_visible); const clamped_pxE_visible = Math.min(visibleCanvasWidth, pxE_visible); const pxW_visible_clamped = Math.max(0, clamped_pxE_visible - clamped_pxS_visible); if (pxW_visible_clamped >= 0) { ctx.fillStyle = highlightColor; ctx.fillRect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight); if (pxW_visible_clamped > 0 && buf.length > 0 && totalLogicalWidth > 0) { ctx.save(); ctx.beginPath(); ctx.rect(clamped_pxS_visible, 0, pxW_visible_clamped, waveformCanvasHeight); ctx.clip(); drawVisibleWaveform(ctx, buf, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx, waveformCanvasHeight, waveColor); ctx.restore(); } } else { console.warn(`[Waveform Draw] Highlight width negative ${currentActiveIndex}.`); } } const pxCur_logical = timeToLogicalPx(cur, dur, totalLogicalWidth); const pxCur_visible = pxCur_logical - scrollOffsetPx; if (pxCur_visible >= -1 && pxCur_visible <= visibleCanvasWidth + 1) { ctx.strokeStyle = '#ef4444'; ctx.lineWidth = 1.5; ctx.beginPath(); ctx.moveTo(pxCur_visible + 0.5, 0); ctx.lineTo(pxCur_visible + 0.5, waveformCanvasHeight); ctx.stroke(); } ctx.restore(); lastDrawnTime = cur; lastDrawnScrollOffset = scrollOffsetPx; lastDrawnZoomLevel = zoomLevel; lastDrawnSegmentIndex = currentActiveIndex; lastDrawnBuffer = buf; lastDrawnActualDuration = dur; lastDrawnIsEditing = isEditingSegment; lastDrawnEditStart = editSegmentStartTime; lastDrawnEditEnd = editSegmentEndTime; }

	let forceNextRedraw = false; function requestRedraw(force = false) { if (force) forceNextRedraw = true; if (isMounted) { drawTimescale(); drawSegmentWaveformUI(); } }

	function animationLoop() { if (!isMounted) return; const cur = currentPlayTime || 0; const dur = actualMediaDuration; const buf = currentAudioBuffer; const currentActiveIdx = activeSegmentIndexForDisplay ?? -1; let needsDraw = forceNextRedraw || (buf && buf !== lastDrawnBuffer) || (currentActiveIdx !== lastDrawnSegmentIndex) || (Math.abs(cur - lastDrawnTime) > redrawTimeThreshold) || (Math.abs(scrollOffsetPx - lastDrawnScrollOffset) > 0.5) || (Math.abs(zoomLevel - lastDrawnZoomLevel) > 0.001) || (dur !== lastDrawnActualDuration) || (isEditingSegment !== lastDrawnIsEditing) || (isEditingSegment && (editSegmentStartTime !== lastDrawnEditStart || editSegmentEndTime !== lastDrawnEditEnd)); forceNextRedraw = false; if (needsDraw && visibleCanvasWidth > 0 && buf && dur > 0 && totalLogicalWidth > 0 ) { drawTimescale(); drawSegmentWaveformUI(); } else if (needsDraw && (!buf || visibleCanvasWidth <= 0 || dur <= 0 || totalLogicalWidth <= 0)) { clearWaveformCanvases(); lastDrawnTime = cur; lastDrawnScrollOffset = scrollOffsetPx; lastDrawnZoomLevel = zoomLevel; lastDrawnSegmentIndex = currentActiveIdx; lastDrawnBuffer = buf; lastDrawnActualDuration = dur; lastDrawnIsEditing = isEditingSegment; lastDrawnEditStart = editSegmentStartTime; lastDrawnEditEnd = editSegmentEndTime; } if (autoScrollEnabled && dur > 0 && totalLogicalWidth > visibleCanvasWidth && !isTrimming && !isEditingSegment) { const pxCur_visible = timeToVisiblePx(cur, dur, totalLogicalWidth, scrollOffsetPx); const scrollMarginLeft = visibleCanvasWidth * 0.25; const scrollMarginRight = visibleCanvasWidth * 0.75; let targetScrollOffset = scrollOffsetPx; let needsScrollUpdate = false; if (pxCur_visible < scrollMarginLeft) { targetScrollOffset = timeToLogicalPx(cur, dur, totalLogicalWidth) - scrollMarginLeft; needsScrollUpdate = true; } else if (pxCur_visible > scrollMarginRight) { targetScrollOffset = timeToLogicalPx(cur, dur, totalLogicalWidth) - scrollMarginRight; needsScrollUpdate = true; } if (needsScrollUpdate) { targetScrollOffset = Math.max(0, Math.min(targetScrollOffset, maxScrollPx)); const diff = targetScrollOffset - scrollOffsetPx; const moveAmount = diff * 0.1; let newScrollOffset = scrollOffsetPx + moveAmount; if (Math.abs(diff) < 1) newScrollOffset = targetScrollOffset; newScrollOffset = Math.round(newScrollOffset); if (Math.abs(newScrollOffset - scrollOffsetPx) > 0) { scrollOffsetPx = newScrollOffset; if (waveformScrollContainerRef) waveformScrollContainerRef.scrollLeft = scrollOffsetPx; } } } animationFrameId = requestAnimationFrame(animationLoop); }

	function resetZoomAndScrollState(clearBuffer = true) {
		zoomLevel = 1; scrollOffsetPx = 0; autoScrollEnabled = true;
		clearTimeout(autoScrollEnableTimer); autoScrollEnableTimer = null;
		if (waveformScrollContainerRef) waveformScrollContainerRef.scrollLeft = 0;
		lastDrawnTime = -1; lastDrawnScrollOffset = -1; lastDrawnZoomLevel = -1;
		lastDrawnSegmentIndex = -1; if (clearBuffer) lastDrawnBuffer = null;
		lastDrawnActualDuration = -1; lastDrawnIsEditing = false;
		lastDrawnEditStart = -1; lastDrawnEditEnd = -1;
		clearWaveformCanvases(); requestRedraw(true);
	}

	onMount(() => {
		isMounted = true;
		webAudioApiSupported = typeof window.AudioContext !== 'undefined' || typeof window.webkitAudioContext !== 'undefined';
		unsubscribeAudioBuffer = transcriptStore.subscribe(ts => {
			if (externalAudioBuffer === null) {
				const newAudioBuffer = ts.audioBuffer;
				if (newAudioBuffer && newAudioBuffer !== currentAudioBuffer) {
				} else if (!newAudioBuffer && currentAudioBuffer && !externalAudioBuffer) {
                    actualMediaDuration = 0;
                }
			}
		});
		unsubscribePlayer = transcriptStore.subscribe(ts => { /* Player state changes picked up by animationLoop */ });
		unsubscribeSegments = transcriptStore.subscribe(ts => { /* Segments changes picked up by animationLoop */ });

		prevExternalAudioBufferForDuration = externalAudioBuffer;
		if (externalAudioBuffer && externalAudioBuffer.duration > 0) {
			actualMediaDuration = externalAudioBuffer.duration;
			// console.log(`[Waveform] actualMediaDuration initialized from prop onMount: ${actualMediaDuration}`);
		} else if ($transcriptStore.audioBuffer && $transcriptStore.audioBuffer.duration > 0 && !externalAudioBuffer) {
            actualMediaDuration = $transcriptStore.audioBuffer.duration;
            // console.log(`[Waveform] actualMediaDuration initialized from store onMount: ${actualMediaDuration}`);
        }

		tick().then(() => {
			if (isMounted) {
				if (waveformScrollContainerRef) {
					visibleCanvasWidth = waveformScrollContainerRef.clientWidth || 0;
					waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT;
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

	function setupResizeObserver() { if (waveformScrollContainerRef && !isObserverSetup && isMounted && typeof window !== 'undefined' && window.ResizeObserver) { isObserverSetup = true; waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT; resizeObserverInstance = new ResizeObserver((entries) => { let needsRedraw = false; let needsScrollUpdate = false; let newScrollOffset = scrollOffsetPx; for (const entry of entries) { if (entry.target === waveformScrollContainerRef) { const newWidth = entry.contentRect.width; const newContainerHeight = entry.contentRect.height; const newWaveformHeight = (newContainerHeight || 80) - TIMESCALE_HEIGHT; if (newWaveformHeight > 0 && newWaveformHeight !== waveformCanvasHeight) { waveformCanvasHeight = newWaveformHeight; needsRedraw = true; } if (newWidth > 0 && newWidth !== visibleCanvasWidth) { const oldVisibleWidth = visibleCanvasWidth; const oldTotalLogicalWidth = totalLogicalWidth; visibleCanvasWidth = newWidth; const currentMaxScroll = Math.max(0, (visibleCanvasWidth * zoomLevel) - visibleCanvasWidth); if (oldVisibleWidth > 0 && oldTotalLogicalWidth > 0 && oldTotalLogicalWidth > oldVisibleWidth) { const scrollCenterLogicalPx = scrollOffsetPx + oldVisibleWidth / 2; const centerProportion = oldTotalLogicalWidth > 0 ? scrollCenterLogicalPx / oldTotalLogicalWidth : 0; const newTotalLogicalWidthAfterUpdate = visibleCanvasWidth * zoomLevel; newScrollOffset = (centerProportion * newTotalLogicalWidthAfterUpdate) - (visibleCanvasWidth / 2); newScrollOffset = Math.max(0, Math.min(newScrollOffset, Math.max(0, newTotalLogicalWidthAfterUpdate - visibleCanvasWidth))); } else { newScrollOffset = Math.max(0, Math.min(scrollOffsetPx, currentMaxScroll)); } if (Math.abs(newScrollOffset - scrollOffsetPx) > 0.5) { scrollOffsetPx = Math.round(newScrollOffset); needsScrollUpdate = true; } needsRedraw = true; } else if (newWidth <= 0 && visibleCanvasWidth !== 0) { visibleCanvasWidth = 0; scrollOffsetPx = 0; needsScrollUpdate = true; clearWaveformCanvases(); needsRedraw = false; } } } if (needsScrollUpdate && waveformScrollContainerRef) { const wasAutoScrollEnabled = autoScrollEnabled; autoScrollEnabled = false; waveformScrollContainerRef.scrollLeft = scrollOffsetPx; autoScrollEnableTimer = setTimeout(() => { if (!isTrimming && !isEditingSegment) autoScrollEnabled = wasAutoScrollEnabled; autoScrollEnableTimer = null; }, 100); } if (needsRedraw) requestRedraw(); }); resizeObserverInstance.observe(waveformScrollContainerRef); if (waveformScrollContainerRef) { visibleCanvasWidth = waveformScrollContainerRef.clientWidth; waveformCanvasHeight = (waveformScrollContainerRef.offsetHeight || 80) - TIMESCALE_HEIGHT; } requestRedraw(true); } }
	$: if (waveformScrollContainerRef && !isObserverSetup && isMounted) { setupResizeObserver(); }

	function handleScroll(event) { const newScrollOffset = Math.round(event.target.scrollLeft); if (Math.abs(newScrollOffset - scrollOffsetPx) > 0) { const wasManualScroll = isScrolling; if (autoScrollEnabled && !wasManualScroll && !isTrimming && !isEditingSegment) { autoScrollEnabled = false; clearTimeout(autoScrollEnableTimer); autoScrollEnableTimer = null; } scrollOffsetPx = newScrollOffset; isScrolling = true; clearTimeout(debounceScrollTimer); debounceScrollTimer = setTimeout(() => { isScrolling = false; if (!autoScrollEnabled && !autoScrollEnableTimer && !isTrimming && !isEditingSegment) { autoScrollEnableTimer = setTimeout(() => { autoScrollEnabled = true; autoScrollEnableTimer = null; requestRedraw(true); }, 1500); } requestRedraw(true); }, 150); requestRedraw(); } }

	function handleCanvasClick(e) { const dur = actualMediaDuration; if (isTrimming || isEditingSegment || !segmentWaveformCanvas || !currentAudioBuffer || dur <= 0 || !waveformScrollContainerRef || visibleCanvasWidth <= 0 || totalLogicalWidth <= 0) return; const rect = waveformScrollContainerRef.getBoundingClientRect(); const clickX = e.clientX - rect.left; const time = pxToTime(clickX, dur, totalLogicalWidth, visibleCanvasWidth, scrollOffsetPx); if (!autoScrollEnabled) { autoScrollEnabled = true; clearTimeout(autoScrollEnableTimer); autoScrollEnableTimer = null; } dispatch('navigate', { time: time }); }

	function handleZoom(direction) {
		// if (waveformScrollContainerRef) { // scrollOffsetPx is captured if needed, but new logic recalculates.
		// 	scrollOffsetPx = waveformScrollContainerRef.scrollLeft;
		// }
		if (!visibleCanvasWidth || visibleCanvasWidth <= 0 || !currentAudioBuffer || !actualMediaDuration) {
			return;
		}
		const oldZoomLevel = zoomLevel;
		// const oldTotalLogicalWidth = totalLogicalWidth; // Not strictly needed for new logic
		// const initialScrollOffsetPx = scrollOffsetPx; // Not strictly needed for new logic

		let newZoomLevel = direction === 'in' ? oldZoomLevel * zoomStep : oldZoomLevel / zoomStep;
		newZoomLevel = Math.max(minZoomLevel, Math.min(maxZoomLevel, newZoomLevel));

		if (Math.abs(newZoomLevel - oldZoomLevel) < 0.001) { return; }

		// const viewCenterTimeBeforeZoom = pxToTime(visibleCanvasWidth / 2, actualMediaDuration, oldTotalLogicalWidth, visibleCanvasWidth, initialScrollOffsetPx); // Not needed for new logic

		zoomLevel = newZoomLevel;

		tick().then(() => {
			const newTotalLogicalWidthAfterZoom = totalLogicalWidth; // This correctly uses the updated zoomLevel
			const newMaxScroll = Math.max(0, newTotalLogicalWidthAfterZoom - visibleCanvasWidth);

			// *** This is the core change ***
			let newScrollOffset = newTotalLogicalWidthAfterZoom - visibleCanvasWidth;

			newScrollOffset = Math.max(0, Math.min(newScrollOffset, newMaxScroll));
			scrollOffsetPx = Math.round(newScrollOffset);

			const wasAutoScrollEnabled = autoScrollEnabled; autoScrollEnabled = false; clearTimeout(autoScrollEnableTimer);
			if (waveformScrollContainerRef) {
				waveformScrollContainerRef.scrollLeft = scrollOffsetPx;
				// console.log(`[Waveform.handleZoom tick] waveformScrollContainerRef.scrollLeft set to: ${scrollOffsetPx}`);
			}
			requestRedraw(true);
			autoScrollEnableTimer = setTimeout(() => {
				if (isMounted && !isTrimming && !isEditingSegment) { autoScrollEnabled = wasAutoScrollEnabled; }
				autoScrollEnableTimer = null;
			}, 100);
		});
	}
	function zoomIn() { handleZoom('in'); }
	function zoomOut() { handleZoom('out'); }

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
		<canvas bind:this={segmentWaveformCanvas} class="waveform-canvas {(currentAudioBuffer && !isTrimming && !isEditingSegment) ? 'cursor-pointer' : 'cursor-default'}" style="height: {waveformCanvasHeight}px; top: {TIMESCALE_HEIGHT}px;" aria-label="Waveform visualization. Click to seek audio." on:click|self={handleCanvasClick} on:wheel|preventDefault={(e) => { if (e.ctrlKey || e.metaKey) { handleZoom(e.deltaY < 0 ? 'in' : 'out'); } }} />
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
	.waveform-overlay { @apply absolute inset-0 flex items-center justify-center bg-gray-100/70 dark:bg-gray-700/70 pointer-events-none rounded z-[5]; }
	.waveform-overlay-text { @apply text-xs p-1 bg-white/80 dark:bg-gray-900/80 rounded shadow-sm font-sans text-center text-gray-600 dark:text-gray-300; }
	.ui-button-icon { @apply p-1 rounded text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-800 focus:bg-gray-200 dark:focus:bg-gray-600 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-gray-100 dark:disabled:hover:bg-gray-700; }
	.size-6 { @apply w-6 h-6; }
</style>