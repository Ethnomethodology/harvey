<!-- harvey-1.0/src/lib/components/projectview/MediaPlayer.svelte -->

<script>
	import { project }	from '$lib/stores/projectStore.js';
	import {
		transcriptStore,
		updatePlayerTime,
		setPlayerDuration,
		togglePlayerPlaying,
		setAudioBuffer
	} from '$lib/stores/transcriptStore.js';
	// --- Import get ---
	import { get } from 'svelte/store';
	import { readFile } from '@tauri-apps/plugin-fs';
	import { onMount, onDestroy, tick } from 'svelte';
	// --- Import service functions ---
	import { handleTrimMediaConfirm, refreshProjectFiles } from '$lib/services/projectService.js';

	// Loop/Pause Toggle State & Icons
	let isLooping = false;
	const LOOP_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-repeat" viewBox="0 0 16 16"><path d="M11 5.466V4H5a4 4 0 0 0-3.584 5.777.5.5 0 1 1-.896.446A5 5 0 0 1 5 3h6V1.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36 1.966a.25.25 0 0 1-.41-.192m3.81.086a.5.5 0 0 1 .67.225A5 5 0 0 1 11 13H5v1.466a.25.25 0 0 1-.41.192l-2.36-1.966a.25.25 0 0 1 0-.384l2.36-1.966a.25.25 0 0 1 .41.192V12h6a4 4 0 0 0 3.585-5.777.5.5 0 0 1 .225-.67Z"/></svg>`;
	const PAUSE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pause" viewBox="0 0 16 16"><path d="M6 3.5a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-1 0V4a.5.5 0 0 1 .5-.5m4 0a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-1 0V4a.5.5 0 0 1 .5-.5"/></svg>`;
	function toggleLoop() {
	  isLooping = !isLooping;
	}

	// --- Component State ---
	export let videoElement = null;
	// Trim props (bound from parent)
	export let isTrimming = false;
	export let trimStartTime = 0;
	export let trimEndTime = 0;
	// --- NEW: Edit props (passed from parent) ---
	export let isEditingSegment = false;
	export let editSegmentStartTime = 0;
	export let editSegmentEndTime = 0;


	let mediaUrl = '';
	let currentBlobUrl = null;
	let isLoadingMedia = false;
	let loadedPath = null; // Keep track of the loaded original path

	// --- Audio Context State ---
	let audioContext = null;
	let webAudioApiSupported = true;


	onMount(() => {
        try {
            if (!audioContext || audioContext.state === 'closed') {
                audioContext = new (window.AudioContext || window.webkitAudioContext)();
            }
        } catch (e) {
            webAudioApiSupported = false;
        }
        return () => {
            if (audioContext && audioContext.state !== 'closed') {
                audioContext.close().catch(console.error);
            }
        };
    });
	onDestroy(() => {
        if (audioContext && audioContext.state !== 'closed') {
            audioContext.close().catch(console.error);
        }
        if (currentBlobUrl) {
            URL.revokeObjectURL(currentBlobUrl);
        }
    });

	// --- File Handling & Audio Processing ---
	function getMimeType(filePath) {
        const extension = filePath?.split('.').pop()?.toLowerCase();
        switch (extension) {
            case 'wav': return 'audio/wav';
            case 'mp3': return 'audio/mpeg';
            case 'ogg': return 'audio/ogg';
            case 'm4a': return 'audio/mp4'; // Common for AAC
            case 'aac': return 'audio/aac';
            case 'flac': return 'audio/flac';
            case 'mp4': return 'video/mp4';
            case 'mov': return 'video/quicktime';
            case 'webm': return 'video/webm';
            case 'avi': return 'video/x-msvideo';
            case 'mkv': return 'video/x-matroska';
            default: return ''; // Let the browser try to figure it out
        }
    }
	$: {
        const selectedPath = $transcriptStore.selectedMediaFile?.path;
        (async () => {
            if (selectedPath) {
                // If the path hasn't changed and we have a URL, do nothing
                if (selectedPath === loadedPath && currentBlobUrl) return;
                // If we're already loading, wait
                if (isLoadingMedia) return;

                isLoadingMedia = true;
                if (isTrimming) cancelTrimMode(); // Cancel trim if media changes

                // Cleanup previous Blob URL if exists
                if (currentBlobUrl) {
                    URL.revokeObjectURL(currentBlobUrl);
                    currentBlobUrl = null;
                }
                mediaUrl = ''; // Reset media URL immediately
                setAudioBuffer(null); // Clear audio buffer
                setPlayerDuration(0); // Reset duration
                updatePlayerTime(0);  // Reset time
                togglePlayerPlaying(false); // Ensure player state is paused
                loadedPath = null; // Reset loaded path

                try {
                    // Read file as raw bytes
                    const fileData = await readFile(selectedPath);
                    const mimeType = getMimeType(selectedPath);
                    const blob = new Blob([fileData], { type: mimeType });
                    const newUrl = URL.createObjectURL(blob);
                    currentBlobUrl = newUrl; // Store the new Blob URL
                    loadedPath = selectedPath; // Store loaded path

                    mediaUrl = newUrl;

                    let decodedBuffer = null;
                    // Attempt to decode audio for waveform if supported
                    if (webAudioApiSupported && audioContext && audioContext.state !== 'closed') {
                         // Need ArrayBuffer for decodeAudioData
                         const arrayBuffer = fileData.buffer.slice(fileData.byteOffset, fileData.byteOffset + fileData.byteLength);
                         if (audioContext.state === 'suspended') await audioContext.resume();
                         try {
                             decodedBuffer = await audioContext.decodeAudioData(arrayBuffer);
                             setAudioBuffer(decodedBuffer);
                         } catch (decodeError) {
                             console.error('[MediaPlayer] Error decoding audio:', decodeError);
                             setAudioBuffer(null); // Ensure buffer is null on error
                         }
                    } else {
                         console.warn('[MediaPlayer] AudioContext unavailable, skipping waveform.');
                         setAudioBuffer(null); // Ensure buffer is null
                    }

                    await tick(); // Wait for DOM update
                    videoElement?.load(); // Trigger video element load

                } catch (error) {
                    console.error(`[MediaPlayer] Error processing file ${selectedPath}:`, error);
                    project.update((p) => ({
                        ...p,
                        error: `Failed to load media: ${error?.message || error}`,
                        statusMessage: 'Error loading media.'
                    }));
                    mediaUrl = '';
                    currentBlobUrl = null;
                    loadedPath = null;
                    setAudioBuffer(null);
                    setPlayerDuration(0);
                    updatePlayerTime(0);
                    togglePlayerPlaying(false);
                } finally {
                    isLoadingMedia = false;
                }
            } else {
                // Handle case where selectedMediaFile becomes null
                if (isTrimming) cancelTrimMode();
                loadedPath = null;
                if (currentBlobUrl) {
                    URL.revokeObjectURL(currentBlobUrl);
                    currentBlobUrl = null;
                }
                if (mediaUrl !== '') {
                    mediaUrl = '';
                    setPlayerDuration(0);
                    updatePlayerTime(0);
                    togglePlayerPlaying(false);
                }
                if ($transcriptStore.audioBuffer) {
                    setAudioBuffer(null);
                }
                isLoadingMedia = false;
            }
        })();
    }

	// --- Player Controls ---
	export function handleTogglePlay() {
        if (!videoElement || !mediaUrl || isLoadingMedia) return;
        if (videoElement.paused || videoElement.ended) {
            videoElement.play().catch(console.error); // Play returns a promise
        } else {
            videoElement.pause();
        }
    }

	// --- Video Element Event Handlers ---
	function onPlay()  { togglePlayerPlaying(true); }
	function onPause() { togglePlayerPlaying(false); }
	function onTimeUpdate(event) {
		if (isLoadingMedia || !$transcriptStore.player.duration) return;
		const video = event.target;
		let currentTime = video.currentTime;
		const duration = video.duration;

		if (typeof currentTime === 'number' && !isNaN(currentTime) && duration > 0) {
			let looped = false;
			// Check for segment edit looping first
			if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
				if (currentTime < editSegmentStartTime || currentTime >= editSegmentEndTime) {
					// Loop back to the start of the segment being edited
					console.log(`[MediaPlayer] Looping edit segment: ${currentTime.toFixed(3)} -> ${editSegmentStartTime.toFixed(3)}`);
					video.currentTime = editSegmentStartTime;
					currentTime = editSegmentStartTime; // Update local currentTime after setting video element
					looped = true;
					 // If looping caused pause, play again
					if(video.paused && video.currentTime === editSegmentStartTime) {
						video.play().catch(console.error);
					}
				}
			}
			// Check for trim looping if not already looped by edit
			else if (isTrimming && trimEndTime > trimStartTime) {
				if (currentTime < trimStartTime || currentTime >= trimEndTime) {
					 console.log(`[MediaPlayer] Looping trim segment: ${currentTime.toFixed(3)} -> ${trimStartTime.toFixed(3)}`);
					video.currentTime = trimStartTime;
					currentTime = trimStartTime; // Update local currentTime after setting video element
					looped = true;
					 // If looping caused pause, play again
					 if(video.paused && video.currentTime === trimStartTime) {
						video.play().catch(console.error);
					}
				}
			}
			// Always update store time, reflects actual player time after potential loop
			updatePlayerTime(currentTime);
		}
	}
	function onLoadedMetadata(event) {
        if (event.target && typeof event.target.duration === 'number' && !isNaN(event.target.duration)) {
            const duration = event.target.duration;
            setPlayerDuration(duration);
            updatePlayerTime(0); // Reset time on new metadata
        } else {
            // Handle cases like Infinite duration (streams) or invalid metadata
            setPlayerDuration(0);
            updatePlayerTime(0);
        }
        // Sync playing state after metadata loaded (e.g., if autoplay was involved)
        if (videoElement) togglePlayerPlaying(!videoElement.paused);
    }
	function onSeeked() {
        // Update time in store after seeking completes
        if (videoElement && !isLoadingMedia) {
            const currentTime = videoElement.currentTime;
            updatePlayerTime(currentTime);
        }
    }
	function onEnded() {
		console.log('[MediaPlayer] onEnded event');
		togglePlayerPlaying(false);
		const duration = $transcriptStore.player.duration;
		// If ending during edit/trim loop, jump to start instead of end
		if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
			updatePlayerTime(editSegmentStartTime);
			if (videoElement) videoElement.currentTime = editSegmentStartTime;
		} else if (isTrimming && trimEndTime > trimStartTime) {
			updatePlayerTime(trimStartTime);
			if (videoElement) videoElement.currentTime = trimStartTime;
		} else {
            // Go to the very end (or 0 if duration is missing)
			updatePlayerTime(duration || 0);
		}
	 }
	function onError(event) {
        console.error('[MediaPlayer] onError event', event?.target?.error);
        let errorMsg = 'Unknown video error';
        if (event.target?.error) {
            switch (event.target.error.code) {
                case MediaError.MEDIA_ERR_ABORTED: errorMsg = 'Playback aborted by user.'; break;
                case MediaError.MEDIA_ERR_NETWORK: errorMsg = 'Network error caused playback failure.'; break;
                case MediaError.MEDIA_ERR_DECODE: errorMsg = 'Media decoding error.'; break;
                case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED: errorMsg = 'Media format not supported.'; break;
                default: errorMsg = `An unknown error occurred (Code: ${event.target.error.code})`; break;
            }
        }
        project.update((p) => ({
            ...p,
            error: `Media Error: ${errorMsg}`,
            statusMessage: 'Error playing media.'
        }));
        togglePlayerPlaying(false);
        setPlayerDuration(0);
        updatePlayerTime(0);
        setAudioBuffer(null); // Clear audio buffer on error too
    }

	// --- REMOVED: onTranscribe function ---

	// --- Utility Functions ---
	function formatTime(totalSeconds) {
        if (isNaN(totalSeconds) || totalSeconds < 0) return '00:00';
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = Math.floor(totalSeconds % 60);
        return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }

	// --- Trim Mode Functions ---
	function enterTrimMode() {
		// Prevent entering trim mode if already editing segment
		if (isEditingSegment) {
			alert("Cannot enter trim mode while editing a segment.");
			return;
		}
		const currentTranscriptState = get(transcriptStore); // Get current transcript store state
		const currentProjectState = get(project); // Get current project store state (for global things if needed)
		const currentTime = currentTranscriptState.player.currentTime;
		const segments = currentTranscriptState.segments;
		const duration = currentTranscriptState.player.duration;

		if (!duration || isLoadingMedia || !$transcriptStore.audioBuffer || isTrimming) return;

		// Find the segment containing the current playhead time
		let segmentStartTime = 0;
		let segmentEndTime = duration;
		const currentSegment = segments.find(s => currentTime >= s.start_time && currentTime < s.end_time);

		if (currentSegment) {
			segmentStartTime = currentSegment.start_time;
			segmentEndTime = currentSegment.end_time;
			console.log(`[MediaPlayer] Trim init to current segment: ${segmentStartTime.toFixed(3)} - ${segmentEndTime.toFixed(3)}`);
		} else {
			// If no segment at current time, maybe default to full duration or last/next segment?
			// Defaulting to full duration for now.
			console.log(`[MediaPlayer] Trim init: No segment at current time ${currentTime.toFixed(3)}. Defaulting to full duration.`);
			segmentStartTime = 0;
			segmentEndTime = duration;
		}

		isTrimming   = true;
		trimStartTime = segmentStartTime;
		trimEndTime   = segmentEndTime;
		 // Ensure player loops within new bounds if playing
		if ($transcriptStore.player.isPlaying && videoElement) {
			 // If current time is outside the new trim range, jump to the start
			 if (videoElement.currentTime < trimStartTime || videoElement.currentTime >= trimEndTime) {
				 videoElement.currentTime = trimStartTime;
				 updatePlayerTime(trimStartTime); // Update store immediately
			 }
		}
	}

	function cancelTrimMode() {
        isTrimming = false;
        // Optionally reset trim times, or keep them for potential re-entry? Resetting seems safer.
        trimStartTime = 0;
        trimEndTime = 0;
    }

	async function confirmTrim() {
		if (!isTrimming || !loadedPath) return;
		console.log(`[MediaPlayer] Confirming trim for ${loadedPath} from ${trimStartTime.toFixed(3)}s to ${trimEndTime.toFixed(3)}s.`);

		try {
			project.update(p => ({ ...p, isLoading: true, statusMessage: 'Trimming media...' }));
			await handleTrimMediaConfirm(loadedPath, trimStartTime, trimEndTime);
			// Success handling (alert, status update) might be better handled in the service/store
			project.update(p => ({ ...p, isLoading: false, statusMessage: 'Trim complete.' })); // Assuming service updates status
			alert('Media trimmed successfully!'); // Simple feedback for now
            // After successful trim, potentially reload project files or just update UI state?
            // await refreshProjectFiles(); // Maybe call this if backend replaces the file
		} catch (error) {
			console.error('[MediaPlayer] Trim failed:', error);
			project.update(p => ({ ...p, isLoading: false, error: `Trim failed: ${error.message || error}`, statusMessage: 'Trim failed.' }));
			alert(`Failed to trim media: ${error.message || error}`);
		} finally {
			cancelTrimMode(); // Exit trim mode regardless of success/failure
		}
	}

	export function updateTrimTimes(newStartTime, newEndTime) {
        if (isTrimming && typeof newStartTime === 'number' && typeof newEndTime === 'number') {
            trimStartTime = newStartTime;
            trimEndTime = newEndTime;
            // If player is currently outside new bounds due to external update, maybe adjust?
            // Or rely on the onTimeUpdate loop? Relying on loop seems less intrusive.
        }
    }

	// Reactive variables used in template
	// REMOVED isTranscribeDisabled
	// Disable trim if trimming, loading, no audio buffer, OR if editing another segment
	$: isTrimDisabled = isTrimming || !mediaUrl || isLoadingMedia || !$transcriptStore.audioBuffer || isEditingSegment;


	// Exported seekTo method
	export function seekTo(seconds) {
		if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return;
		if (!videoElement) return;
		const duration = videoElement.duration || 0;
		let clamped = Math.max(0, Math.min(seconds, duration));

		// If editing/trimming, clamp seek within the active bounds
        // Subtract a tiny amount from end time to prevent looping immediately on seek
		if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
			 clamped = Math.max(editSegmentStartTime, Math.min(clamped, editSegmentEndTime - 0.001));
		} else if (isTrimming && trimEndTime > trimStartTime) {
			 clamped = Math.max(trimStartTime, Math.min(clamped, trimEndTime - 0.001));
		}

		// Use requestAnimationFrame for smoother seeking state update
		cancelAnimationFrame(seekRafId); // Cancel previous seek request if any
		seekRafId = requestAnimationFrame(() => {
			videoElement.currentTime = clamped;
			// Update store time immediately for responsiveness, onSeeked confirms later
			updatePlayerTime(clamped);
		});
	}
	let seekRafId = null; // Keep track of RAF ID for seeking

</script>

<div class="p-1 flex flex-col bg-gray-50 dark:bg-gray-800">
	<div class="w-full max-w-[36rem] aspect-video bg-black relative mx-auto mb-1">
		{#if isLoadingMedia}
			<div class="absolute inset-0 flex items-center justify-center text-gray-400 animate-pulse"><span>Loading media...</span></div>
		{:else if mediaUrl}
			{#key mediaUrl}
				<video
					class="absolute inset-0 w-full h-full object-contain"
					src={mediaUrl}
					bind:this={videoElement}
					on:play={onPlay}
					on:pause={onPause}
					on:ended={onEnded}
					on:timeupdate={onTimeUpdate}
					on:loadedmetadata={onLoadedMetadata}
					on:seeked={onSeeked}
					on:error={onError}
					preload="metadata"
					controls
					controlslist="nodownload noremoteplayback"
				></video>
			{/key}
		{:else}
			<div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400"><span>No media selected</span></div>
		{/if}
	</div>
	<div class="flex items-center justify-between flex-shrink-0 max-w-[36rem] mx-auto w-full">
		<div class="flex items-center space-x-3">
			<button
				on:click={handleTogglePlay}
				class="btn-control"
				disabled={!mediaUrl || isLoadingMedia}
				aria-label={$transcriptStore.player.isPlaying ? 'Pause' : 'Play'}
			>
				{#if $transcriptStore.player.isPlaying}
					 <!-- Pause Icon -->
					 <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5zm5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5z" /></svg>
				{:else}
					 <!-- Play Icon -->
					 <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M11.596 8.697l-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393z" /></svg>
				{/if}
			</button>
			<span class="text-xs font-mono text-gray-600 dark:text-gray-400 tabular-nums whitespace-nowrap">
				{formatTime($transcriptStore.player.currentTime)} / {formatTime($transcriptStore.player.duration)}
			</span>
			<button
				class="btn-control ml-2 inline-flex items-center space-x-1 text-sm"
				on:click={toggleLoop}
				title={isLooping ? 'Loop while editing' : 'Pause while editing'}
				aria-label={isLooping ? 'Loop while editing' : 'Pause while editing'}
			>
				{@html isLooping ? LOOP_ICON : PAUSE_ICON}
				<span class="ml-1">
					{isLooping ? 'Loop while editing' : 'Pause while editing'}
				</span>
			</button>
		</div>
		<div class="flex items-center space-x-2">
			<button
				on:click={enterTrimMode}
				class="btn-control"
				title="Trim Media"
				disabled={isTrimDisabled}
			>
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                <path stroke-linecap="round" stroke-linejoin="round" d="m7.848 8.25 1.536.887M7.848 8.25a3 3 0 1 1-5.196-3 3 3 0 0 1 5.196 3Zm1.536.887a2.165 2.165 0 0 1 1.083 1.839c.005.351.054.695.14 1.024M9.384 9.137l2.077 1.199M7.848 15.75l1.536-.887m-1.536.887a3 3 0 1 1-5.196 3 3 3 0 0 1 5.196-3Zm1.536-.887a2.165 2.165 0 0 0 1.083-1.838c.005-.352.054-.695.14-1.025m-1.223 2.863 2.077-1.199m0-3.328a4.323 4.323 0 0 1 2.068-1.379l5.325-1.628a4.5 4.5 0 0 1 2.48-.044l.803.215-7.794 4.5m-2.882-1.664A4.33 4.33 0 0 0 10.607 12m3.736 0 7.794 4.5-.802.215a4.5 4.5 0 0 1-2.48-.043l-5.326-1.629a4.324 4.324 0 0 1-2.068-1.379M14.343 12l-2.882 1.664" />
              </svg>              
			<span class="sr-only">Trim</span>
			</button>
			{#if isTrimming}
				<button on:click={confirmTrim} class="btn-action-trim" title="Confirm Trim">Trim</button>
				<button on:click={cancelTrimMode} class="btn-action-cancel" title="Cancel Trim">Cancel</button>
			{/if}
			<!-- REMOVED Transcribe Button -->
		</div>
	</div>
</div>

<style>
	.btn-control {
		padding: 0.5rem;
		background: #e5e7eb; /* bg-gray-200 */
        color: #1f2937; /* text-gray-800 */
		border: 1px solid #d1d5db; /* border-gray-300 */
		border-radius: 0.375rem; /* rounded-md */
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		transition: background-color 0.15s ease-in-out;
	}
    .dark .btn-control {
        background: #4b5563; /* dark:bg-gray-600 */
        border-color: #6b7280; /* dark:border-gray-500 */
        color: #f3f4f6; /* dark:text-gray-100 */
    }
	.btn-control:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn-control:hover:not(:disabled) {
		background: #d1d5db; /* hover:bg-gray-300 */
	}
    .dark .btn-control:hover:not(:disabled) {
        background: #6b7280; /* dark:hover:bg-gray-500 */
    }
	.btn-control svg {
		width: 1em;
		height: 1em;
	}

	.btn-action {
		padding: 0.4rem 1rem;
		background: #3b82f6; /* bg-blue-500 */
		color: white;
		border: none;
		border-radius: 0.375rem; /* rounded-md */
		cursor: pointer;
		font-size: 0.875rem; /* text-sm */
		font-weight: 500; /* font-medium */
		white-space: nowrap;
		display: inline-flex;
		align-items: center;
		gap: 0.25rem; /* space-x-1 equivalent */
		transition: background-color 0.15s;
	}
	.btn-action:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		background: #9ca3af; /* bg-gray-400 */
	}
	.btn-action:hover:not(:disabled) {
		background: #2563eb; /* hover:bg-blue-600 */
	}
    .dark .btn-action:disabled {
        background: #6b7280; /* dark:bg-gray-500 */
        opacity: 0.5;
    }

	.btn-action-trim {
		padding: 0.4rem 1rem;
		background: #10b981; /* bg-emerald-500 */
		color: white;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background-color 0.15s;
	}
	.btn-action-trim:hover {
		background: #059669; /* hover:bg-emerald-600 */
	}
	.btn-action-cancel {
		padding: 0.4rem 1rem;
		background: #ef4444; /* bg-red-500 */
		color: white;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background-color 0.15s;
	}
	.btn-action-cancel:hover {
		background: #dc2626; /* hover:bg-red-600 */
	}

	.animate-pulse {
		animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 0.6;
		}
		50% {
			opacity: 0.3;
		}
	}
	.sr-only { /* Screen reader only */
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}
</style>