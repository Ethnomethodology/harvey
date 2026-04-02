<script>
  import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
  import { transcriptStore } from '$lib/stores/transcriptStore.js';

  export let audioBuffer = null;
  export let externalPeaks = null;
  export let currentTime = 0;
  export let duration = 0;
  export let isEditingSegment = false;
  export let editSegmentStartTime = 0;
  export let editSegmentEndTime = 0;

  const TIMESCALE_WIDTH = 35;
  const BAR_THICKNESS_PX = 2;
  const BAR_SPACING_PX = 1;
  const BAR_UNIT_HEIGHT_PX = BAR_THICKNESS_PX + BAR_SPACING_PX;
  const RMS_GAIN_FACTOR = 12.0;
  const PEAK_GAIN_FACTOR = 2.5;
  const MIN_BAR_LENGTH_PX = 1;

  // Optimization: Off-screen cache canvases for the waveform
  let baseCacheCanvas = null;
  let highlightCacheCanvas = null;
  let lastCacheParams = {
    buffer: null,
    peaks: null,
    zoom: -1,
    width: -1,
    isDark: false,
    logicalHeight: -1,
    dpr: -1
  };

  let waveformCanvas;
  let timescaleCanvas;
  let componentContainer;
  let waveformAreaContainerRef; // This is now the scroll container

  let visibleCanvasHeight = 0;
  let waveformCanvasWidth = 0;
  let webAudioApiSupported = true;
  let resizeObserverInstance;
  let isMounted = false;
  let animationFrameId = null;

  let scrollOffsetPy = 0;

  const maxZoomLevel = 10;
  const minZoomLevel = 1;
  let zoomLevel = maxZoomLevel;
  const zoomStep = 1.2;

  let lastDrawnTime = -1;
  let lastDrawnBufferOrPeaks = null;
  let lastDrawnActualDuration = -1;
  const redrawTimeThreshold = 1 / 60;

  let seekBarStyle = 'display: none;';
  let segmentHighlightStyle = 'display: none;';

  const dispatch = createEventDispatcher();

  // Determine platform-specific modifier key name
  const isMac =
    typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
  const modKeyName = isMac ? 'Cmd' : 'Ctrl';

  let draggingHandle = null;

  function startEditDrag(handle, event) {
    if (!isEditingSegment || !duration || !waveformCanvas) return;
    event.preventDefault();
    event.stopPropagation();
    draggingHandle = handle;
    window.addEventListener('mousemove', handleEditMouseMove);
    window.addEventListener('mouseup', handleEditMouseUp, { once: true });
  }

  function handleEditMouseMove(event) {
    if (!draggingHandle || !waveformAreaContainerRef) return;
    event.preventDefault();

    const rect = waveformAreaContainerRef.getBoundingClientRect();
    const clickY_in_viewport = event.clientY - rect.top;

    let newTime = pyToTime(clickY_in_viewport, duration, visibleCanvasHeight, scrollOffsetPy);

    const minDuration = 0.05;
    let newStartTime = editSegmentStartTime;
    let newEndTime = editSegmentEndTime;

    if (draggingHandle === 'edit-top') {
      newStartTime = Math.max(0, Math.min(newTime, editSegmentEndTime - minDuration));
    } else {
      newEndTime = Math.min(duration, Math.max(newTime, editSegmentStartTime + minDuration));
    }

    if (newStartTime !== editSegmentStartTime || newEndTime !== editSegmentEndTime) {
      dispatch('segmentupdate', { startTime: newStartTime, endTime: newEndTime });
    }
  }

  function handleEditMouseUp() {
    draggingHandle = null;
    window.removeEventListener('mousemove', handleEditMouseMove);
  }

  let segments = [];
  let currentSegmentIndex = -1;
  let currentSegment = null;
  let lastDrawnCurrentSegment = null;

  // Auto-scroll state
  let autoScrollEnabled = true;
  let isProgrammaticScroll = false;
  let userScrollTimeout = null;

  transcriptStore.subscribe((value) => {
    segments = value.segments || [];
    currentSegmentIndex = value.player?.currentSegmentIndex ?? -1;
    currentSegment =
      currentSegmentIndex >= 0 && currentSegmentIndex < segments.length
        ? segments[currentSegmentIndex]
        : null;
    if (isMounted && lastDrawnCurrentSegment !== currentSegment) {
      requestRedraw(true);
      lastDrawnCurrentSegment = currentSegment;
    }
  });

  function formatTimescaleTimeVertical(sec, totalDuration) {
    if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '0:00';
    const tot = Math.floor(sec);
    const minutes = Math.floor(tot / 60);
    const seconds = tot % 60;
    if (totalDuration >= 3600) {
      const hours = Math.floor(minutes / 60);
      const remainingMinutes = minutes % 60;
      return `${hours}:${String(remainingMinutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }
    return `${String(minutes)}:${String(seconds).padStart(2, '0')}`;
  }

  function timeToLogicalPy(time, mediaDuration, viewHeight) {
    if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
    const contentLogicalHeight = viewHeight * zoomLevel;
    const proportion = Math.max(0, Math.min(1, time / mediaDuration));
    return proportion * contentLogicalHeight;
  }

  function pyToTime(py, mediaDuration, viewHeight, currentScrollOffsetPy = 0) {
    if (!mediaDuration || mediaDuration <= 0 || !viewHeight || viewHeight <= 0) return 0;
    const contentLogicalHeight = viewHeight * zoomLevel;
    const logicalPy = py + currentScrollOffsetPy;
    const proportion = Math.max(0, Math.min(1, logicalPy / contentLogicalHeight));
    return proportion * mediaDuration;
  }

  /**
   * Unified bar-style rendering for the vertical waveform.
   */
  function renderWaveformBars(ctx, buffer, peaks, width, logicalHeight, color) {
    if (!ctx || width <= 0 || logicalHeight <= 0) return;
    if (!buffer && !peaks) return;

    const midX = width / 2;
    ctx.fillStyle = color;

    const barThickness = BAR_THICKNESS_PX;
    const spacing = BAR_SPACING_PX;
    const step = barThickness + spacing;

    const gain = buffer ? RMS_GAIN_FACTOR : PEAK_GAIN_FACTOR;

    if (buffer) {
      const data = buffer.getChannelData(0);
      const totalSamples = data.length;
      const samplesPerPixel = totalSamples / logicalHeight;

      for (let y = 0; y < logicalHeight; y += step) {
        const startSample = Math.floor(y * samplesPerPixel);
        const endSample = Math.ceil((y + barThickness) * samplesPerPixel);
        if (startSample >= totalSamples) break;

        let sumSquares = 0;
        let count = 0;
        for (let i = startSample; i < Math.min(endSample, totalSamples); i++) {
          sumSquares += data[i] * data[i];
          count++;
        }

        const rms = count > 0 ? Math.sqrt(sumSquares / count) : 0;
        const cappedRms = Math.min(1.0, rms * gain);

        const useableWidth = midX * 0.9;
        const barHalfWidth = Math.max(MIN_BAR_LENGTH_PX, cappedRms * useableWidth);

        ctx.fillRect(midX - barHalfWidth, y, barHalfWidth * 2, barThickness);
      }
    } else if (peaks) {
      const numPeakBlocks = peaks.length / 2;
      const peaksPerPixel = numPeakBlocks / logicalHeight;

      for (let y = 0; y < logicalHeight; y += step) {
        const peakIdx = Math.floor(y * peaksPerPixel);
        if (peakIdx * 2 + 1 >= peaks.length) break;

        const min = peaks[peakIdx * 2];
        const max = peaks[peakIdx * 2 + 1];

        const cappedMin = Math.max(-1.0, min * gain);
        const cappedMax = Math.min(1.0, max * gain);

        const useableWidth = midX * 0.9;
        const xLeft = midX + cappedMin * useableWidth;
        const xRight = midX + cappedMax * useableWidth;
        const barWidth = Math.max(MIN_BAR_LENGTH_PX, xRight - xLeft);

        ctx.fillRect(xLeft, y, barWidth, barThickness);
      }
    }
  }

  function updateWaveformCache() {
    const buf = audioBuffer;
    const peaks = externalPeaks || $transcriptStore.audioBufferPeaks;
    const isDark = document.documentElement.classList.contains('dark');
    const dpr = window.devicePixelRatio || 1;

    if (!buf && !peaks) return;

    const logicalHeight = Math.round(visibleCanvasHeight * zoomLevel);
    const width = Math.round(waveformCanvasWidth);

    if (
      lastCacheParams.buffer === buf &&
      lastCacheParams.peaks === peaks &&
      lastCacheParams.zoom === zoomLevel &&
      lastCacheParams.width === width &&
      lastCacheParams.isDark === isDark &&
      lastCacheParams.logicalHeight === logicalHeight &&
      lastCacheParams.dpr === dpr &&
      baseCacheCanvas
    ) {
      return;
    }

    const reqW = Math.round(width * dpr);
    const reqH = Math.round(logicalHeight * dpr);

    if (!baseCacheCanvas) baseCacheCanvas = document.createElement('canvas');
    if (!highlightCacheCanvas) highlightCacheCanvas = document.createElement('canvas');

    baseCacheCanvas.width = reqW;
    baseCacheCanvas.height = reqH;
    highlightCacheCanvas.width = reqW;
    highlightCacheCanvas.height = reqH;

    const baseCtx = baseCacheCanvas.getContext('2d');
    const highlightCtx = highlightCacheCanvas.getContext('2d');

    if (!baseCtx || !highlightCtx) return;

    const waveColor = isDark ? '#737373' : '#9ca3af';
    const activeWaveColor = isDark ? '#60a5fa' : '#2563eb';

    baseCtx.save();
    baseCtx.scale(dpr, dpr);
    renderWaveformBars(baseCtx, buf, peaks, width, logicalHeight, waveColor);
    baseCtx.restore();

    highlightCtx.save();
    highlightCtx.scale(dpr, dpr);
    renderWaveformBars(highlightCtx, buf, peaks, width, logicalHeight, activeWaveColor);
    highlightCtx.restore();

    lastCacheParams = {
      buffer: buf,
      peaks: peaks,
      zoom: zoomLevel,
      width: width,
      isDark: isDark,
      logicalHeight: logicalHeight,
      dpr: dpr
    };
  }

  function clearWaveformCanvases() {
    [timescaleCanvas, waveformCanvas].forEach((canvas) => {
      if (canvas) {
        const ctx = canvas.getContext('2d');
        if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
      }
    });
    if (waveformCanvas && waveformCanvasWidth > 0 && visibleCanvasHeight > 0) {
      const ctx = waveformCanvas.getContext('2d');
      if (ctx) {
        const dpr = window.devicePixelRatio || 1;
        ctx.save();
        ctx.scale(dpr, dpr);
        ctx.fillStyle = document.documentElement.classList.contains('dark') ? '#A3A3A3' : '#6b7280';
        ctx.font = `10px sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        let message = 'Vertical Waveform';
        if (!webAudioApiSupported) message = 'Web Audio API not supported.';
        else if (!audioBuffer && !$transcriptStore.audioBufferPeaks)
          message = 'Load media for waveform.';
        ctx.fillText(message, waveformCanvasWidth / 2, visibleCanvasHeight / 2);
        ctx.restore();
      }
    }
  }

  function drawVerticalTimescale() {
    const mediaDur = duration;
    const bufOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
    const dpr = window.devicePixelRatio || 1;
    const logicalHeight = visibleCanvasHeight * zoomLevel;

    if (
      !timescaleCanvas ||
      !bufOrPeaks ||
      mediaDur <= 0 ||
      logicalHeight <= 0 ||
      TIMESCALE_WIDTH <= 0
    ) {
      if (timescaleCanvas) {
        timescaleCanvas.width = 0;
        timescaleCanvas.height = 0;
      }
      return;
    }
    const ctx = timescaleCanvas.getContext('2d');
    if (!ctx) return;

    const reqW = Math.round(TIMESCALE_WIDTH * dpr);
    const reqH = Math.round(logicalHeight * dpr);
    if (timescaleCanvas.width !== reqW || timescaleCanvas.height !== reqH) {
      timescaleCanvas.width = reqW;
      timescaleCanvas.height = reqH;
    }
    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, TIMESCALE_WIDTH, logicalHeight);

    const isDark = document.documentElement.classList.contains('dark');
    ctx.fillStyle = isDark ? '#171717' : '#F3F4F6'; // gray-900 dark, gray-100 light
    ctx.fillRect(0, 0, TIMESCALE_WIDTH, logicalHeight);

    ctx.strokeStyle = isDark ? '#404040' : '#d1d5db';
    ctx.fillStyle = isDark ? '#E5E5E5' : '#000000';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    const minPixelSpacingForLabel = 30;
    const intervals = [0.1, 0.5, 1, 5, 10, 30, 60, 300, 600, 1800, 3600];
    let interval = intervals[0];
    for (const i of intervals) {
      const intervalPy = timeToLogicalPy(i, mediaDur, visibleCanvasHeight);
      if (intervalPy >= minPixelSpacingForLabel) {
        interval = i;
        break;
      }
      interval = i;
    }

    const textPadding = 5;
    for (let time = 0; time <= mediaDur; time += interval) {
      const logicalY = timeToLogicalPy(time, mediaDur, visibleCanvasHeight);
      const tickWidth = Math.abs(time % (interval * 5)) < 0.0001 && interval >= 1 ? 7 : 5;
      ctx.beginPath();
      ctx.moveTo(TIMESCALE_WIDTH - tickWidth, logicalY + 0.5);
      ctx.lineTo(TIMESCALE_WIDTH, logicalY + 0.5);
      ctx.stroke();
      const labelStr = formatTimescaleTimeVertical(time, mediaDur);
      ctx.fillText(labelStr, TIMESCALE_WIDTH - textPadding - 2, logicalY);
    }
    ctx.beginPath();
    ctx.moveTo(TIMESCALE_WIDTH - 0.5, 0);
    ctx.lineTo(TIMESCALE_WIDTH - 0.5, logicalHeight);
    ctx.strokeStyle = isDark ? '#404040' : '#d1d5db';
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.restore();
  }

  function drawWaveformUI() {
    const mediaDur = duration;
    const dpr = window.devicePixelRatio || 1;
    const logicalHeight = visibleCanvasHeight * zoomLevel;

    if (!waveformCanvas || mediaDur <= 0 || logicalHeight <= 0 || waveformCanvasWidth <= 0) {
      if (waveformCanvas) {
        waveformCanvas.width = 0;
        waveformCanvas.height = 0;
      }
      return;
    }
    const ctx = waveformCanvas.getContext('2d');
    if (!ctx) return;

    const reqW = Math.round(waveformCanvasWidth * dpr);
    const reqH = Math.round(logicalHeight * dpr);
    if (waveformCanvas.width !== reqW || waveformCanvas.height !== reqH) {
      waveformCanvas.width = reqW;
      waveformCanvas.height = reqH;
    }

    updateWaveformCache();

    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, waveformCanvasWidth, logicalHeight);

    if (baseCacheCanvas) {
      ctx.drawImage(baseCacheCanvas, 0, 0, reqW, reqH, 0, 0, waveformCanvasWidth, logicalHeight);
    }

    // Handle segment highlighting
    if (currentSegment && highlightCacheCanvas) {
      const segmentStartTime = Number(currentSegment.start_time);
      const segmentEndTime = Number(currentSegment.end_time);
      if (!isNaN(segmentStartTime) && !isNaN(segmentEndTime) && segmentEndTime > segmentStartTime) {
        const logicalTop = timeToLogicalPy(segmentStartTime, mediaDur, visibleCanvasHeight);
        const logicalBottom = timeToLogicalPy(segmentEndTime, mediaDur, visibleCanvasHeight);
        const h = logicalBottom - logicalTop;

        if (h > 0) {
          ctx.save();
          ctx.beginPath();
          ctx.rect(0, logicalTop, waveformCanvasWidth, h);
          ctx.clip();
          ctx.drawImage(
            highlightCacheCanvas,
            0,
            0,
            reqW,
            reqH,
            0,
            0,
            waveformCanvasWidth,
            logicalHeight
          );
          ctx.restore();
        }
      }
    }

    ctx.restore();

    lastDrawnTime = currentTime;
    lastDrawnBufferOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;
    lastDrawnActualDuration = mediaDur;
  }

  let forceNextRedraw = false;
  function requestRedraw(force = false) {
    if (force) forceNextRedraw = true;
    if (isMounted && animationFrameId === null) {
      animationFrameId = requestAnimationFrame(animationLoop);
    }
  }

  function animationLoop() {
    if (!isMounted) {
      animationFrameId = null;
      return;
    }
    const cur = currentTime;
    const mediaDur = duration;
    const currentBufOrPeaks = audioBuffer || $transcriptStore.audioBufferPeaks;

    let needsDraw =
      forceNextRedraw ||
      currentBufOrPeaks !== lastDrawnBufferOrPeaks ||
      mediaDur !== lastDrawnActualDuration;

    forceNextRedraw = false;

    if (needsDraw && visibleCanvasHeight > 0 && currentBufOrPeaks && mediaDur > 0) {
      drawVerticalTimescale();
      drawWaveformUI();
    } else if (needsDraw) {
      clearWaveformCanvases();
      lastDrawnTime = cur;
      lastDrawnBufferOrPeaks = currentBufOrPeaks;
      lastDrawnActualDuration = mediaDur;
    }
    animationFrameId = requestAnimationFrame(animationLoop);
  }

  onMount(() => {
    isMounted = true;
    webAudioApiSupported =
      typeof window.AudioContext !== 'undefined' ||
      typeof window.webkitAudioContext !== 'undefined';
    tick().then(() => {
      if (isMounted && waveformAreaContainerRef) {
        setupResizeObserver();
      }
    });
  });

  onDestroy(() => {
    isMounted = false;
    if (resizeObserverInstance) resizeObserverInstance.disconnect();
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (userScrollTimeout) clearTimeout(userScrollTimeout);

    baseCacheCanvas = null;
    highlightCacheCanvas = null;
  });

  function setupResizeObserver() {
    if (
      waveformAreaContainerRef &&
      !resizeObserverInstance &&
      isMounted &&
      typeof window !== 'undefined' &&
      window.ResizeObserver
    ) {
      resizeObserverInstance = new ResizeObserver((entries) => {
        for (const entry of entries) {
          if (entry.target === waveformAreaContainerRef) {
            const newHeight = Math.max(0, entry.contentRect.height);
            const newWidth = Math.max(0, entry.contentRect.width);
            let changed = false;
            if (newHeight !== visibleCanvasHeight) {
              visibleCanvasHeight = newHeight;
              changed = true;
            }
            const newWaveformCanvasWidth = Math.max(0, newWidth - TIMESCALE_WIDTH);
            if (newWaveformCanvasWidth !== waveformCanvasWidth) {
              waveformCanvasWidth = newWaveformCanvasWidth;
              changed = true;
            }
            if (changed) requestRedraw(true);
          }
        }
      });
      resizeObserverInstance.observe(waveformAreaContainerRef);
      tick().then(() => {
        if (waveformAreaContainerRef) {
          visibleCanvasHeight = Math.max(0, waveformAreaContainerRef.clientHeight);
          waveformCanvasWidth = Math.max(0, waveformAreaContainerRef.clientWidth - TIMESCALE_WIDTH);
          requestRedraw(true);
        }
      });
    }
  }

  function handleContainerClick(event) {
    const mediaDur = duration;
    if (
      !waveformAreaContainerRef ||
      !$transcriptStore.audioBufferPeaks ||
      mediaDur <= 0 ||
      visibleCanvasHeight <= 0
    )
      return;

    if (event.target === timescaleCanvas) return;

    if (userScrollTimeout) clearTimeout(userScrollTimeout);
    autoScrollEnabled = true;

    const rect = waveformAreaContainerRef.getBoundingClientRect();
    const clickY_in_viewport = event.clientY - rect.top;
    const time = pyToTime(clickY_in_viewport, mediaDur, visibleCanvasHeight, scrollOffsetPy);

    // Calculate relative click position (ratio) for centering
    const ratio = clickY_in_viewport / visibleCanvasHeight;

    dispatch('navigate', { time, ratio });
  }

  async function handleZoom(direction) {
    if (!$transcriptStore.audioBufferPeaks || !waveformAreaContainerRef || !visibleCanvasHeight)
      return;

    const oldZoomLevel = zoomLevel;
    let newZoomLevel = direction === 'in' ? oldZoomLevel * zoomStep : oldZoomLevel / zoomStep;
    newZoomLevel = Math.max(minZoomLevel, Math.min(newZoomLevel, maxZoomLevel));
    if (Math.abs(newZoomLevel - oldZoomLevel) < 0.001) return;

    const timeAtCenter = pyToTime(
      visibleCanvasHeight / 2,
      duration,
      visibleCanvasHeight,
      scrollOffsetPy
    );
    zoomLevel = newZoomLevel;
    await tick();

    const newLogicalYForTimeAtCenter = timeToLogicalPy(timeAtCenter, duration, visibleCanvasHeight);
    let newScrollOffsetPy = newLogicalYForTimeAtCenter - visibleCanvasHeight / 2;
    const contentLogicalHeight = visibleCanvasHeight * zoomLevel;
    const maxScroll = Math.max(0, contentLogicalHeight - visibleCanvasHeight);
    newScrollOffsetPy = Math.max(0, Math.min(newScrollOffsetPy, maxScroll));

    waveformAreaContainerRef.scrollTop = newScrollOffsetPy;
    scrollOffsetPy = Math.round(newScrollOffsetPy);
    requestRedraw(true);
  }

  export function scrollToTime(time, centerRatio = 0.5) {
    if (!isMounted || !waveformAreaContainerRef || duration <= 0) return;

    const logicalY = timeToLogicalPy(time, duration, visibleCanvasHeight);
    // Use the provided centerRatio (defaults to 0.5 for middle)
    let newScrollTop = logicalY - visibleCanvasHeight * centerRatio;

    const contentLogicalHeight = visibleCanvasHeight * zoomLevel;
    const maxScroll = Math.max(0, contentLogicalHeight - visibleCanvasHeight);
    newScrollTop = Math.max(0, Math.min(newScrollTop, maxScroll));

    if (Math.abs(newScrollTop - scrollOffsetPy) > 1) {
      isProgrammaticScroll = true;
      waveformAreaContainerRef.scrollTop = newScrollTop;
      tick().then(() => {
        isProgrammaticScroll = false;
      });
    }
  }

  function zoomIn() {
    handleZoom('in');
  }
  function zoomOut() {
    handleZoom('out');
  }

  function handleWaveformScroll(event) {
    if (event.target) {
      if (!isProgrammaticScroll) {
        autoScrollEnabled = false;
        if (userScrollTimeout) clearTimeout(userScrollTimeout);
        userScrollTimeout = setTimeout(() => {
          autoScrollEnabled = true;
        }, 3000); // Re-enable after 3s
      }
      scrollOffsetPy = Math.round(event.target.scrollTop);
    }
  }

  function resetScrollAndZoom(resetZoomToo = true) {
    if (resetZoomToo) zoomLevel = maxZoomLevel;
    scrollOffsetPy = 0;
    if (waveformAreaContainerRef) waveformAreaContainerRef.scrollTop = 0;
    requestRedraw(true);
  }

  let prevAudioBuffer = audioBuffer;
  let prevDuration = duration;
  let prevStorePeaks = $transcriptStore.audioBufferPeaks;

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
      resetScrollAndZoom(true);
    } else {
      requestRedraw();
    }
  }

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
      resetScrollAndZoom(true);
    } else {
      requestRedraw();
    }
  }

  // Reactive update for seek bar position. No redraw needed, just style update.
  $: if (isMounted && Math.abs(currentTime - lastDrawnTime) > redrawTimeThreshold / 2) {
    lastDrawnTime = currentTime;
  }

  // Auto-scroll logic
  $: if (
    isMounted &&
    autoScrollEnabled &&
    (audioBuffer || $transcriptStore.audioBufferPeaks) &&
    duration > 0 &&
    waveformAreaContainerRef
  ) {
    const logicalY = timeToLogicalPy(currentTime, duration, visibleCanvasHeight);

    const buffer = visibleCanvasHeight * 0.2; // 20% buffer from top/bottom
    const viewTop = scrollOffsetPy;
    const viewBottom = scrollOffsetPy + visibleCanvasHeight;

    if (logicalY < viewTop + buffer || logicalY > viewBottom - buffer) {
      let newScrollTop = logicalY - visibleCanvasHeight / 2; // Center it

      const contentLogicalHeight = visibleCanvasHeight * zoomLevel;
      const maxScroll = Math.max(0, contentLogicalHeight - visibleCanvasHeight);
      newScrollTop = Math.max(0, Math.min(newScrollTop, maxScroll));

      if (Math.abs(newScrollTop - scrollOffsetPy) > 1) {
        isProgrammaticScroll = true;
        waveformAreaContainerRef.scrollTop = newScrollTop;
        tick().then(() => {
          isProgrammaticScroll = false;
        });
      }
    }
  }

  // Reactive style for HTML seek bar
  $: {
    if (
      isMounted &&
      (audioBuffer || $transcriptStore.audioBufferPeaks) &&
      duration > 0 &&
      visibleCanvasHeight > 0
    ) {
      const logicalY = timeToLogicalPy(currentTime, duration, visibleCanvasHeight);
      const screenY = logicalY - scrollOffsetPy; // For visibility check

      if (!isNaN(logicalY) && isFinite(logicalY)) {
        seekBarStyle = `top: ${logicalY}px; visibility: ${screenY >= -1.5 && screenY <= visibleCanvasHeight + 1.5 ? 'visible' : 'hidden'};`;
      } else {
        seekBarStyle = 'display: none;';
      }
    } else {
      seekBarStyle = 'display: none;';
    }
  }

  // Reactive style for HTML segment highlight
  $: {
    if (isMounted && currentSegment && duration > 0 && visibleCanvasHeight > 0) {
      const segmentStartTime = Number(currentSegment.start_time);
      const segmentEndTime = Number(currentSegment.end_time);

      if (!isNaN(segmentStartTime) && !isNaN(segmentEndTime) && segmentEndTime > segmentStartTime) {
        const logicalTop = timeToLogicalPy(segmentStartTime, duration, visibleCanvasHeight);
        const logicalBottom = timeToLogicalPy(segmentEndTime, duration, visibleCanvasHeight);
        const logicalHeight = Math.max(0, logicalBottom - logicalTop);

        // For visibility check
        const screenTop = logicalTop - scrollOffsetPy;
        const screenBottom = logicalBottom - scrollOffsetPy;

        if (logicalHeight > 0 && screenTop < visibleCanvasHeight && screenBottom > 0) {
          segmentHighlightStyle = `top: ${logicalTop}px; height: ${logicalHeight}px; display: block;`;
        } else {
          segmentHighlightStyle = 'display: none;';
        }
      } else {
        segmentHighlightStyle = 'display: none;';
      }
    } else {
      segmentHighlightStyle = 'display: none;';
    }
  }

  $: canZoomIn =
    isMounted && zoomLevel < maxZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);
  $: canZoomOut =
    isMounted && zoomLevel > minZoomLevel && (audioBuffer || $transcriptStore.audioBufferPeaks);
</script>

<div
  bind:this={componentContainer}
  class="vertical-waveform-panel flex flex-col w-full h-full bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded overflow-hidden"
>
  <div
    class="flex-shrink-0 p-0.5 flex items-center justify-end space-x-0.5 border-b border-gray-300 dark:border-gray-700 w-full"
  >
    <button
      class="ui-button-icon-panelheader"
      title="Zoom In Waveform ({modKeyName}+Scroll)"
      aria-label="Zoom In Waveform"
      on:click={zoomIn}
      disabled={!canZoomIn}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="1.5"
        stroke="currentColor"
        class="w-5 h-5"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM10.5 7.5v6m3-3h-6"
        />
      </svg>
    </button>
    <button
      class="ui-button-icon-panelheader"
      title="Zoom Out Waveform ({modKeyName}+Scroll)"
      aria-label="Zoom Out Waveform"
      on:click={zoomOut}
      disabled={!canZoomOut}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="1.5"
        stroke="currentColor"
        class="w-5 h-5"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607ZM13.5 10.5h-6"
        />
      </svg>
    </button>
  </div>

  <div
    bind:this={waveformAreaContainerRef}
    class="waveform-scroll-container flex flex-grow min-h-0 relative overflow-y-auto"
    on:scroll={handleWaveformScroll}
    on:click={handleContainerClick}
  >
    <div class="flex relative" style="height: {visibleCanvasHeight * zoomLevel}px; width: 100%;">
      <canvas
        bind:this={timescaleCanvas}
        class="timescale-canvas-vertical shrink-0"
        style="width: {TIMESCALE_WIDTH}px; height: 100%;"
        aria-hidden="true"
      ></canvas>
      <div class="relative flex-grow h-full">
        <canvas
          bind:this={waveformCanvas}
          class="waveform-canvas-vertical absolute top-0 left-0 w-full h-full cursor-pointer"
          aria-label="Vertical waveform visualization. Click to seek audio."
        ></canvas>
        {#if (audioBuffer || $transcriptStore.audioBufferPeaks) && duration > 0}
          <div class="vertical-seek-bar" style={seekBarStyle}></div>
          <div class="segment-highlight-window" style={segmentHighlightStyle}></div>

          {#if isEditingSegment}
            <!-- Handles -->
            {@const editStartPy = timeToLogicalPy(
              editSegmentStartTime,
              duration,
              visibleCanvasHeight
            )}
            {@const editEndPy = timeToLogicalPy(editSegmentEndTime, duration, visibleCanvasHeight)}

            <!-- Top Handle (Start) -->
            <div
              class="absolute left-0 w-full h-2.5 -translate-y-1/2 flex flex-col items-center justify-center cursor-row-resize group z-30"
              style="top: {editStartPy}px;"
              on:mousedown|preventDefault={(e) => startEditDrag('edit-top', e)}
            >
              <div class="w-full h-0.5 bg-blue-600 group-hover:h-1 transition-all"></div>
            </div>

            <!-- Bottom Handle (End) -->
            <div
              class="absolute left-0 w-full h-2.5 -translate-y-1/2 flex flex-col items-center justify-center cursor-row-resize group z-30"
              style="top: {editEndPy}px;"
              on:mousedown|preventDefault={(e) => startEditDrag('edit-bottom', e)}
            >
              <div class="w-full h-0.5 bg-blue-600 group-hover:h-1 transition-all"></div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
    {#if !webAudioApiSupported && isMounted}
      <div class="overlay-message"><p>Web Audio API not supported.</p></div>
    {:else if !audioBuffer && !$transcriptStore.audioBufferPeaks && isMounted}
      <div class="overlay-message"><p>Load audio/video media for waveform.</p></div>
    {/if}
  </div>
</div>

<style lang="postcss">
  .vertical-waveform-panel {
    /* Basic styling for the panel itself */
  }
  .timescale-canvas-vertical {
    display: block;
    position: sticky;
    left: 0;
    z-index: 20;
    background-color: #fff; /* Match panel bg */
  }
  .dark .timescale-canvas-vertical {
    background-color: theme('colors.gray.900'); /* Match panel bg dark */
  }
  .waveform-canvas-vertical {
    display: block;
  }
  .overlay-message {
    @apply absolute top-0 left-0 w-full h-full flex items-center justify-center text-xs p-1 bg-white bg-opacity-80 dark:bg-gray-950 dark:bg-opacity-80 text-gray-600 dark:text-gray-400 pointer-events-none z-30;
    text-align: center;
  }
  .ui-button-icon-panelheader {
    /* Standardized button style for panel headers */
    @apply p-1 rounded text-gray-600 dark:text-gray-600 bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-400 dark:focus:ring-blue-500 dark:ring-offset-gray-900 focus:bg-gray-200 dark:focus:bg-gray-700 transition duration-150 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-gray-100 dark:disabled:hover:bg-gray-800;
  }

  .waveform-scroll-container {
    scrollbar-width: thin;
    scrollbar-color: transparent transparent;
  }
  .waveform-scroll-container:hover {
    scrollbar-color: #a0aec0 #e2e8f0;
  }
  .dark .waveform-scroll-container:hover {
    scrollbar-color: theme('colors.gray.700') theme('colors.gray.900');
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
    background-color: #a0aec0;
  }
  .dark .waveform-scroll-container:hover::-webkit-scrollbar-thumb {
    background-color: theme('colors.gray.700');
  }
  .waveform-scroll-container:hover::-webkit-scrollbar-track {
    background: #e2e8f0;
  }
  .dark .waveform-scroll-container:hover::-webkit-scrollbar-track {
    background: theme('colors.gray.900');
  }
  .vertical-seek-bar {
    position: absolute;
    left: 0;
    width: 100%;
    height: 1.5px;
    background-color: #ef4444;
    pointer-events: none;
    z-index: 10;
  }
  .segment-highlight-window {
    position: absolute;
    left: 0;
    width: 100%;
    background-color: rgba(147, 197, 253, 0.4);
    pointer-events: none;
    z-index: 5;
  }
</style>
