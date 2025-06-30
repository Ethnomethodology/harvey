<script>
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js'; // For types if needed, or direct access

	export let audioBuffer = null;
	export let currentTime = 0;
	export let duration = 0;

	const TIMESCALE_WIDTH = 35; // For vertical timescale, increased for padding
	const BAR_THICKNESS_PX = 2;
	const BAR_SPACING_PX = 1;
	const BAR_UNIT_HEIGHT_PX = BAR_THICKNESS_PX + BAR_SPACING_PX;
	const RMS_GAIN_FACTOR = 1.5; // To amplify RMS effect on bar length
	const MIN_BAR_HALF_LENGTH_PX = 0.5; // Minimum length of one side of the bar from center

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

	let seekBarStyle = 'display: none;'; // For HTML seek bar
	let segmentHighlightStyle = 'display: none;'; // For HTML segment highlight

	const dispatch = createEventDispatcher();

	let debugLastClickY = null;
	let debugScrollOffsetAtClick = null;
	let debugTimeAtClick = null;
	let debugCurrentTimeForSeekbar = null;
	let debugScrollOffsetForSeekbar = null;
	let debugCalculatedScreenYForSeekbar = null;

	let segments = [];
	let currentSegmentIndex = -1;
	let currentSegment = null;

	transcriptStore.subscribe(value => {
		segments = value.segments || [];
		currentSegmentIndex = value.player?.currentSegmentIndex ?? -1;
		if (currentSegmentIndex >= 0 && currentSegmentIndex < segments.length) {
			currentSegment = segments[currentSegmentIndex];
		} else {
			currentSegment = null;
		}
		// Request redraw if segment changes and component is mounted
		if (isMounted && lastDrawnCurrentSegment !== currentSegment) {
			requestRedraw(true);
			lastDrawnCurrentSegment = currentSegment; // Update last drawn segment
		}
	});
	let lastDrawnCurrentSegment = null; // To track changes in currentSegment for redraw

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

	function drawVerticalWaveform(ctx, buffer, peaksData, canvasClientHeight, canvasWidth, color, overrideScrollOffsetY = null) {
		// canvasClientHeight is visibleCanvasHeight
		if (!ctx || canvasClientHeight <= 0 || canvasWidth <= 0) return;
		if (!buffer && (!peaksData || peaksData.length === 0)) return;

		const midX = canvasWidth / 2;
		// const isDark = document.documentElement.classList.contains('dark'); // Removed, color is passed as param
		ctx.fillStyle = color; // Use the passed 'color' parameter for filling bars

		const currentScrollToUse = overrideScrollOffsetY !== null ? overrideScrollOffsetY : scrollOffsetPy;
		const contentLogicalHeight = canvasClientHeight * zoomLevel;

		if (!buffer || contentLogicalHeight <= 0) { // Bar rendering relies on raw buffer
			// Optionally draw flat lines or nothing if no buffer
			for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen += BAR_UNIT_HEIGHT_PX) {
				ctx.fillRect(midX - 1, yPx_screen, 2, BAR_THICKNESS_PX); // Draw a minimal center line
			}
			return;
		}

		const data = buffer.getChannelData(0);
		const totalSamples = data.length;
		if (totalSamples === 0) return;

		const samplesPerLogicalPixel = totalSamples / contentLogicalHeight;

		for (let yPx_screen = 0; yPx_screen < canvasClientHeight; yPx_screen += BAR_UNIT_HEIGHT_PX) {
			const logicalY_bar_top = yPx_screen + currentScrollToUse;
			const logicalY_bar_bottom = (yPx_screen + BAR_THICKNESS_PX) + currentScrollToUse;

			let startSample = Math.max(0, Math.floor(logicalY_bar_top * samplesPerLogicalPixel));
			let endSample = Math.max(0, Math.ceil(logicalY_bar_bottom * samplesPerLogicalPixel));

			// Ensure endSample is at least startSample + 1 if startSample is a valid index, to get at least one sample.
			if (startSample < totalSamples) {
				endSample = Math.max(endSample, startSample + 1);
			}
			endSample = Math.min(totalSamples, endSample); // Re-cap by totalSamples

			if (startSample >= endSample) {
				// This case means no valid sample range for the bar (e.g., at the very end of audio or beyond).
				// MIN_BAR_HALF_LENGTH_PX in the drawing logic will handle visual representation.
				// For data consistency, treat as RMS 0.
				const displayHalfLength = MIN_BAR_HALF_LENGTH_PX;
                ctx.fillRect(midX - displayHalfLength, yPx_screen, displayHalfLength * 2, BAR_THICKNESS_PX);
				continue;
			}

			let sumOfSquares = 0;
			for (let i = startSample; i < endSample; i++) {
				const sample = data[i];
				sumOfSquares += sample * sample;
			}

			const numberOfSamplesInBar = endSample - startSample;
			const rms = numberOfSamplesInBar > 0 ? Math.sqrt(sumOfSquares / numberOfSamplesInBar) : 0;

			const scaledRms = rms * RMS_GAIN_FACTOR;
			// Cap scaledRms at 1.0 to ensure barHalfLengthFromRms doesn't exceed midX
			// (assuming RMS is normalized 0-1, gain might push it over 1)
			const cappedRms = Math.min(1.0, scaledRms);

			const barHalfLengthFromRms = cappedRms * midX;
			const displayHalfLength = Math.max(MIN_BAR_HALF_LENGTH_PX, barHalfLengthFromRms);

			// Draw bar symmetrically from center
			ctx.fillRect(
				midX - displayHalfLength,
				yPx_screen,
				displayHalfLength * 2,
				BAR_THICKNESS_PX
			);
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
		const textPadding = 5; // For label positioning

		for (let time = firstMajorTickTime; time <= mediaDur + interval; time += interval) { // Iterate slightly beyond mediaDur to catch last labels
			if (time < 0) continue;

			const logicalY = timeToLogicalPy(time, mediaDur, visibleCanvasHeight);
			const screenY = logicalY - scrollOffsetPy; // Convert logical Y to screen Y

			// Check if the tick or label is reasonably within the visible canvas height
			// Allowing labels to be drawn if their center is slightly outside, but ticks only if line is inside.
			const labelHeightApproximation = 10; // Approximate height of the label text
			if (screenY >= -labelHeightApproximation && screenY <= visibleCanvasHeight + labelHeightApproximation) {

				// Draw tick mark only if its line is within the canvas bounds
				if (screenY >= 0 && screenY <= visibleCanvasHeight) {
					ctx.beginPath();
					const tickWidth = (Math.abs(time % (interval * 5)) < 0.0001 && interval >= 1) ? 7 : 5;
					ctx.moveTo(TIMESCALE_WIDTH - tickWidth, screenY + 0.5);
					ctx.lineTo(TIMESCALE_WIDTH, screenY + 0.5);
					ctx.stroke();
				}

				// Draw label text
				const labelStr = formatTimescaleTimeVertical(time, mediaDur);
				// Check if the label text itself will be mostly visible
				// Using a simpler check for brevity, can be refined with measureText if needed for perfect centering
				if (screenY - (labelHeightApproximation / 2) >= 0 && screenY + (labelHeightApproximation / 2) <= visibleCanvasHeight) {
					ctx.fillText(labelStr, TIMESCALE_WIDTH - textPadding - 2, screenY);
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

		// Highlight current segment (drawn in the same dpr-scaled context as main waveform)
		if (currentSegment && mediaDur > 0) {
			const segmentStartTime = Number(currentSegment.start_time);
			const segmentEndTime = Number(currentSegment.end_time);

			if (!isNaN(segmentStartTime) && !isNaN(segmentEndTime) && segmentEndTime > segmentStartTime) {
				const segmentStartY_logical = timeToLogicalPy(segmentStartTime, mediaDur, visibleCanvasHeight);
				const segmentEndY_logical = timeToLogicalPy(segmentEndTime, mediaDur, visibleCanvasHeight);

				// Removed erroneous redeclaration block that included segmentStartY_onScreen
				// and duplicate declarations of segmentStartY_logical, segmentEndY_logical.

				// Calculate screen coordinates for canvas clipping, matching HTML element logic (rounded)
				const canvasClipY_unbounded = Math.round(segmentStartY_logical - scrollOffsetPy);
				const canvasClipBottom_unbounded = Math.round(segmentEndY_logical - scrollOffsetPy);

				// Determine the visible portion on the canvas for clipping
				const finalCanvasClipY = Math.max(0, canvasClipY_unbounded);
				const finalCanvasClipBottom = Math.min(visibleCanvasHeight, canvasClipBottom_unbounded);
				const canvasClipHeight = Math.max(0, finalCanvasClipBottom - finalCanvasClipY);

				if (canvasClipHeight > 0) {
					// Background highlight is now an HTML element.
					// Still draw the waveform within the segment with a different color, using aligned clipping.
					if ((buf || peaks) && visibleCanvasHeight > 0) {
						ctx.save();
						ctx.beginPath();
						ctx.rect(0, finalCanvasClipY, waveformCanvasWidth, canvasClipHeight);
						ctx.clip();

						// Calculate the effective scroll offset for drawing the segment's content accurately
						// so that segmentStartTime's data aligns with the top of the clip region (finalCanvasClipY)
						const dataOffsetForSegmentDraw = segmentStartY_logical - finalCanvasClipY;

						// Pass the specific color and the calculated data offset
						drawVerticalWaveform(ctx, buf, peaks, visibleCanvasHeight, waveformCanvasWidth, '#2563eb', dataOffsetForSegmentDraw);
						ctx.restore();
					}
				}
			}
		}
		// Ensure lastDrawnCurrentSegment is updated after attempting to draw,
		// so redraw is triggered if currentSegment changes.
		// This is already handled by the subscription, but also good to note here for logic flow.

		// Red seek bar is now an HTML element, removed from canvas drawing.
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
		if (!waveformScrollDiv || (!audioBuffer && !$transcriptStore.audioBufferPeaks) || mediaDur <= 0 || visibleCanvasHeight <= 0) return;

		const rect = event.currentTarget.getBoundingClientRect(); // event.currentTarget is waveformScrollDiv
		const clickY_in_viewport = event.clientY - rect.top;

		// Use pyToTime for accurate time calculation considering zoom and scroll
		// Use component state scrollOffsetPy for consistency with seek bar rendering
		const time = pyToTime(clickY_in_viewport, mediaDur, visibleCanvasHeight, scrollOffsetPy);

		debugLastClickY = clickY_in_viewport;
		debugScrollOffsetAtClick = scrollOffsetPy;
		debugTimeAtClick = time;

		console.log("WaveformClick:", { clickY: debugLastClickY, scrollAtClick: debugScrollOffsetAtClick, timeAtClick: debugTimeAtClick });
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
				requestRedraw(); // Redraws canvas elements, reactive styles update due to scrollOffsetPy change
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

	// Reactive style for HTML seek bar
	$: {
		if (isMounted && (audioBuffer || $transcriptStore.audioBufferPeaks) && duration > 0 && visibleCanvasHeight > 0) {
			const logicalY = timeToLogicalPy(currentTime, duration, visibleCanvasHeight);
			const screenY = logicalY - scrollOffsetPy; // screenY is a float

			debugCurrentTimeForSeekbar = currentTime;
			debugScrollOffsetForSeekbar = scrollOffsetPy;
			debugCalculatedScreenYForSeekbar = screenY;

			console.log("WaveformSeek:", { seekTime: debugCurrentTimeForSeekbar, scrollAtSeek: debugScrollOffsetForSeekbar, calcScreenY: debugCalculatedScreenYForSeekbar });

			if (!isNaN(screenY) && isFinite(screenY)) {
				// Use float for top. Height is fixed, visibility check uses float.
				seekBarStyle = `top: ${screenY}px; visibility: ${screenY >= -1.5 && screenY <= visibleCanvasHeight + 1.5 ? 'visible' : 'hidden'};`;
			} else {
				seekBarStyle = 'display: none;'; // Hide if position is invalid
				debugCalculatedScreenYForSeekbar = null; // Reset if invalid
			}
		} else {
			seekBarStyle = 'display: none;';
			console.log("WaveformSeek: Hiding seek bar (no audio/duration/etc.)");
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

				const screenTop_float = logicalTop - scrollOffsetPy;
				const screenBottom_float = logicalBottom - scrollOffsetPy;

				const height_float = Math.max(0, screenBottom_float - screenTop_float);

				if (height_float > 0 && screenTop_float < visibleCanvasHeight && screenBottom_float > 0) {
					segmentHighlightStyle = `top: ${screenTop_float}px; height: ${height_float}px; display: block;`;
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
				style="height: {visibleCanvasHeight * zoomLevel}px;"
			></canvas>
			{#if (audioBuffer || $transcriptStore.audioBufferPeaks) && duration > 0}
				<div class="vertical-seek-bar" style={seekBarStyle}></div>
			{/if}
			{#if currentSegment && duration > 0}
				<div class="segment-highlight-window" style={segmentHighlightStyle}></div>
			{/if}
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
	.vertical-seek-bar {
		position: absolute;
		left: 0;
		width: 100%;
		height: 1.5px; /* Consistent with horizontal waveform's canvas line weight */
		background-color: #ef4444; /* Red color */
		pointer-events: none; /* So it doesn't interfere with clicks on the canvas */
		z-index: 10; /* Ensure it's above the waveform canvas */
	}
	.segment-highlight-window {
		position: absolute;
		left: 0;
		width: 100%;
		background-color: rgba(147, 197, 253, 0.4); /* Consistent blue highlight */
		pointer-events: none;
		z-index: 5; /* Below seek bar, above waveform */
	}
</style>
