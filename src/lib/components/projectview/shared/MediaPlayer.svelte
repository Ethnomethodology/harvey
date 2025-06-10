<!-- harvey-1.0/src/lib/components/projectview/MediaPlayer.svelte -->

<script>
	import { project } from '$lib/stores/projectStore.js';
	import {
		transcriptStore,
		updatePlayerTime,
		setPlayerDuration,
		togglePlayerPlaying,
		setAudioBuffer
	} from '$lib/stores/transcriptStore.js';
	import { get } from 'svelte/store';
	import { readFile } from '@tauri-apps/plugin-fs';
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
	import { handleTrimMediaConfirm } from '$lib/services/projectService.js'; // Keep for trim confirm logic

	const dispatch = createEventDispatcher();

	// Loop/Pause Toggle State & Icons (only if showLoopPauseButton is true)
	let isLooping = false;
	const LOOP_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-repeat" viewBox="0 0 16 16"><path d="M11 5.466V4H5a4 4 0 0 0-3.584 5.777.5.5 0 1 1-.896.446A5 5 0 0 1 5 3h6V1.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36 1.966a.25.25 0 0 1-.41-.192m3.81.086a.5.5 0 0 1 .67.225A5 5 0 0 1 11 13H5v1.466a.25.25 0 0 1-.41.192l-2.36-1.966a.25.25 0 0 1 0-.384l2.36-1.966a.25.25 0 0 1 .41.192V12h6a4 4 0 0 0 3.585-5.777.5.5 0 0 1 .225-.67Z"/></svg>`;
	const PAUSE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pause" viewBox="0 0 16 16"><path d="M6 3.5a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-1 0V4a.5.5 0 0 1 .5-.5m4 0a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-1 0V4a.5.5 0 0 1 .5-.5"/></svg>`;
	function toggleLoop() {
	  isLooping = !isLooping;
	}

	// --- Component Props ---
	export let videoElement = null;
	export let isTrimming = false; // For main transcriptions player's trim mode
	export let trimStartTime = 0;
	export let trimEndTime = 0;
	export let isEditingSegment = false; // For main transcriptions player's segment editing loop
	export let editSegmentStartTime = 0;
	export let editSegmentEndTime = 0;

	export let explicitMediaPath = null; // New prop to directly set the media source for this instance

	// Conditional UI for buttons
	export let showLoopPauseButton = true; // Default to true for main player
	export let showNotesTranscribeButton = false; // Default to false
	export let showNotesTrimButton = false; // Default to false
	export let showMainTrimButton = true; // Default to true

	// --- Internal State ---
	let localMediaUrl = ''; // URL for the <video> src
	let currentBlobUrl = null; // To manage Object URL lifecycle
	let isLoadingMedia = false;
	let loadedPathFromProp = null; // Keep track of the loaded explicit path

	// Local player state (independent of global store's player state unless this is the main player)
	// Exported to allow parent components to read these values via a ref (bind:this)
	export let localCurrentTime = 0;
	export let localDuration = 0;
	export let localIsPlaying = false;
	export let localAudioBuffer = null;
	export let isMediaReadyForProcessing = false;

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
            case 'm4a': return 'audio/mp4';
            case 'aac': return 'audio/aac';
            case 'flac': return 'audio/flac';
            case 'mp4': return 'video/mp4';
            case 'mov': return 'video/quicktime';
            case 'webm': return 'video/webm';
            case 'avi': return 'video/x-msvideo';
            case 'mkv': return 'video/x-matroska';
            default: return '';
        }
    }

	// Reactive block to load media when explicitMediaPath changes or (if not explicit) when global selectedMediaFile changes
	$: {
        const mediaPathToLoad = explicitMediaPath || $transcriptStore.selectedMediaFile?.path;

        (async () => {
            if (mediaPathToLoad) {
                if (mediaPathToLoad === loadedPathFromProp && currentBlobUrl) return; // Already loaded this explicit path
                if (isLoadingMedia) return;

                isLoadingMedia = true;
                if (isTrimming && !explicitMediaPath) cancelTrimMode(); // Cancel trim if global media changes (main player context)

                if (currentBlobUrl) {
                    URL.revokeObjectURL(currentBlobUrl);
                    currentBlobUrl = null;
                }
                localMediaUrl = '';
                localAudioBuffer = null;
                localDuration = 0;
                localCurrentTime = 0;
                localIsPlaying = false;
                loadedPathFromProp = null;

                // If this is the main player (no explicitMediaPath), update global store too
                if (!explicitMediaPath) {
                    setAudioBuffer(null);
                    setPlayerDuration(0);
                    updatePlayerTime(0);
                    togglePlayerPlaying(false);
                }


                try {
                    const fileData = await readFile(mediaPathToLoad);
                    const mimeType = getMimeType(mediaPathToLoad);
                    const blob = new Blob([fileData], { type: mimeType });
                    const newUrl = URL.createObjectURL(blob);
                    currentBlobUrl = newUrl;
                    loadedPathFromProp = mediaPathToLoad;
                    localMediaUrl = newUrl;

                    let decodedBuffer = null;
                    if (webAudioApiSupported && audioContext && audioContext.state !== 'closed') {
                         const arrayBuffer = fileData.buffer.slice(fileData.byteOffset, fileData.byteOffset + fileData.byteLength);
                         if (audioContext.state === 'suspended') await audioContext.resume();
                         try {
                             decodedBuffer = await audioContext.decodeAudioData(arrayBuffer);
                             localAudioBuffer = decodedBuffer;
                             if (!explicitMediaPath) setAudioBuffer(decodedBuffer); // Update global for main player
                         } catch (decodeError) {
                             console.error('[MediaPlayer] Error decoding audio:', decodeError);
                             localAudioBuffer = null;
                             if (!explicitMediaPath) setAudioBuffer(null);
                         }
                    } else {
                         console.warn('[MediaPlayer] AudioContext unavailable, skipping waveform.');
                         localAudioBuffer = null;
                         if (!explicitMediaPath) setAudioBuffer(null);
                    }
                    await tick();
                    videoElement?.load();

                } catch (error) {
                    console.error(`[MediaPlayer] Error processing file ${mediaPathToLoad}:`, error);
                    if (!explicitMediaPath) { // Only update global store error if this is the main player
                        project.update((p) => ({
                            ...p,
                            error: `Failed to load media: ${error?.message || error}`,
                            statusMessage: 'Error loading media.'
                        }));
                        setAudioBuffer(null);
                        setPlayerDuration(0);
                        updatePlayerTime(0);
                        togglePlayerPlaying(false);
                    } else {
                        // For explicit path, maybe dispatch an error event or set local error state
                        dispatch('mediaLoadError', { path: mediaPathToLoad, error: error?.message || error });
                    }
                    localMediaUrl = '';
                    currentBlobUrl = null;
                    loadedPathFromProp = null;
                    localAudioBuffer = null;
                    localDuration = 0;
                    localCurrentTime = 0;
                    localIsPlaying = false;
                } finally {
                    isLoadingMedia = false;
                    isMediaReadyForProcessing = (localAudioBuffer && localDuration > 0);
                }
            } else { // No mediaPathToLoad
                if (isTrimming && !explicitMediaPath) cancelTrimMode();
                loadedPathFromProp = null;
                if (currentBlobUrl) {
                    URL.revokeObjectURL(currentBlobUrl);
                    currentBlobUrl = null;
                }
                if (localMediaUrl !== '') {
                    localMediaUrl = '';
                    localDuration = 0;
                    localCurrentTime = 0;
                    localIsPlaying = false;
                }
                if (localAudioBuffer) {
                    localAudioBuffer = null;
                }
                // If this is the main player, update global store too
                if (!explicitMediaPath) {
                    if ($transcriptStore.audioBuffer) setAudioBuffer(null); // from transcriptStore
                    if ($transcriptStore.player.duration > 0) setPlayerDuration(0); // from transcriptStore
                    if ($transcriptStore.player.currentTime > 0) updatePlayerTime(0); // from transcriptStore
                    if ($transcriptStore.player.isPlaying) togglePlayerPlaying(false); // from transcriptStore
                }
                isLoadingMedia = false;
                isMediaReadyForProcessing = false;
            }
        })();
    }


	// --- Player Controls ---
	export function handleTogglePlay() {
        if (!videoElement || !localMediaUrl || isLoadingMedia) return;
        if (videoElement.paused || videoElement.ended) {
            videoElement.play().catch(console.error);
        } else {
            videoElement.pause();
        }
    }

	// --- Video Element Event Handlers ---
	function onPlay()  {
        localIsPlaying = true;
        if (!explicitMediaPath) togglePlayerPlaying(true);
    }
	function onPause() {
        localIsPlaying = false;
        if (!explicitMediaPath) togglePlayerPlaying(false);
    }
	function onTimeUpdate(event) {
		if (isLoadingMedia || !localDuration) return;
		const video = event.target;
		let currentTime = video.currentTime;
		const duration = video.duration;

		if (typeof currentTime === 'number' && !isNaN(currentTime) && duration > 0) {
			// Loop logic specific to main transcriptions view's trim/edit modes
            if (!explicitMediaPath) {
                if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
                    if (currentTime < editSegmentStartTime || currentTime >= editSegmentEndTime) {
                        video.currentTime = editSegmentStartTime;
                        currentTime = editSegmentStartTime;
                        if(video.paused && video.currentTime === editSegmentStartTime) video.play().catch(console.error);
                    }
                } else if (isTrimming && trimEndTime > trimStartTime) {
                    if (currentTime < trimStartTime || currentTime >= trimEndTime) {
                        video.currentTime = trimStartTime;
                        currentTime = trimStartTime;
                        if(video.paused && video.currentTime === trimStartTime) video.play().catch(console.error);
                    }
                }
            }
            localCurrentTime = currentTime;
			if (!explicitMediaPath) updatePlayerTime(currentTime); // Update global for main player
		}
	}
	function onLoadedMetadata(event) {
        if (event.target && typeof event.target.duration === 'number' && !isNaN(event.target.duration)) {
            const duration = event.target.duration;
            localDuration = duration;
            localCurrentTime = 0;
            if (!explicitMediaPath) {
                setPlayerDuration(duration);
                updatePlayerTime(0);
            }
        } else {
            localDuration = 0;
            localCurrentTime = 0;
            if (!explicitMediaPath) {
                setPlayerDuration(0);
                updatePlayerTime(0);
            }
        }
        if (videoElement) {
            localIsPlaying = !videoElement.paused;
            if (!explicitMediaPath) togglePlayerPlaying(!videoElement.paused);
        }
    }
	function onSeeked() {
        if (videoElement && !isLoadingMedia) {
            localCurrentTime = videoElement.currentTime;
            if (!explicitMediaPath) updatePlayerTime(localCurrentTime);
        }
    }
	function onEnded() {
		localIsPlaying = false;
		if (!explicitMediaPath) togglePlayerPlaying(false);

        const duration = localDuration;
		if (!explicitMediaPath && isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
			localCurrentTime = editSegmentStartTime;
            if (videoElement) videoElement.currentTime = editSegmentStartTime;
			updatePlayerTime(editSegmentStartTime);
		} else if (!explicitMediaPath && isTrimming && trimEndTime > trimStartTime) {
			localCurrentTime = trimStartTime;
            if (videoElement) videoElement.currentTime = trimStartTime;
			updatePlayerTime(trimStartTime);
		} else {
            localCurrentTime = duration || 0;
			if (!explicitMediaPath) updatePlayerTime(duration || 0);
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
        if (!explicitMediaPath) { // Global player error
            project.update((p) => ({ ...p, error: `Media Error: ${errorMsg}`, statusMessage: 'Error playing media.' }));
            togglePlayerPlaying(false);
            setPlayerDuration(0);
            updatePlayerTime(0);
            setAudioBuffer(null);
        } else { // Local player error
            dispatch('mediaPlayError', { path: explicitMediaPath, error: errorMsg });
        }
        localIsPlaying = false;
        localDuration = 0;
        localCurrentTime = 0;
        localAudioBuffer = null;
        isMediaReadyForProcessing = false;
    }

	// --- Utility Functions ---
	function formatTime(totalSeconds) {
        if (isNaN(totalSeconds) || totalSeconds < 0) return '00:00';
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = Math.floor(totalSeconds % 60);
        return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }

	// --- Trim Mode Functions (mostly for main player, can be called via ref) ---
	export function enterTrimMode() {
		if (isEditingSegment && !explicitMediaPath) { // Only relevant for main player
			alert("Cannot enter trim mode while editing a segment.");
			return;
		}
		const currentProj = get(project); // Still needed for other project properties if any
		const currentTs = get(transcriptStore);
		const currentTimeToUse = explicitMediaPath ? localCurrentTime : currentTs.player.currentTime;
		const segmentsToUse = explicitMediaPath ? [] : currentTs.segments;
		const durationToUse = explicitMediaPath ? localDuration : currentTs.player.duration;
        const audioBufferToUse = explicitMediaPath ? localAudioBuffer : currentTs.audioBuffer;


		if (!durationToUse || isLoadingMedia || !audioBufferToUse || isTrimming) return;

		let segmentStartTime = 0;
		let segmentEndTime = durationToUse;
        if (!explicitMediaPath) { // Segment-based trim init only for main player
            const currentSegment = segmentsToUse.find(s => currentTimeToUse >= s.start_time && currentTimeToUse < s.end_time);
            if (currentSegment) {
                segmentStartTime = currentSegment.start_time;
                segmentEndTime = currentSegment.end_time;
            }
        }

		isTrimming = true; // This prop is passed in, so setting it here makes it an output effectively
		trimStartTime = segmentStartTime; // This prop is passed in
		trimEndTime = segmentEndTime;   // This prop is passed in

        // Notify parent that trim mode has been entered with these times
        dispatch('trimModeEntered', { startTime: trimStartTime, endTime: trimEndTime });


		if ((explicitMediaPath ? localIsPlaying : $transcriptStore.player.isPlaying) && videoElement) {
			 if (videoElement.currentTime < trimStartTime || videoElement.currentTime >= trimEndTime) {
				 videoElement.currentTime = trimStartTime;
                 localCurrentTime = trimStartTime;
				 if (!explicitMediaPath) updatePlayerTime(trimStartTime);
			 }
		}
	}

	export function cancelTrimMode() { // Can be called via ref
        isTrimming = false;
        trimStartTime = 0;
        trimEndTime = 0;
        dispatch('trimModeCancelled');
    }

	async function confirmTrim() { // For main player context usually
		if (!isTrimming || !(loadedPathFromProp || get(transcriptStore).selectedMediaFile?.path)) return;
        const pathToTrim = loadedPathFromProp || get(transcriptStore).selectedMediaFile?.path;
		console.log(`[MediaPlayer] Confirming trim for ${pathToTrim} from ${trimStartTime.toFixed(3)}s to ${trimEndTime.toFixed(3)}s.`);

		try {
			project.update(p => ({ ...p, isLoading: true, statusMessage: 'Trimming media...' }));
			await handleTrimMediaConfirm(pathToTrim, trimStartTime, trimEndTime);
			project.update(p => ({ ...p, isLoading: false, statusMessage: 'Trim complete.' }));
			alert('Media trimmed successfully!');
		} catch (error) {
			console.error('[MediaPlayer] Trim failed:', error);
			project.update(p => ({ ...p, isLoading: false, error: `Trim failed: ${error.message || error}`, statusMessage: 'Trim failed.' }));
			alert(`Failed to trim media: ${error.message || error}`);
		} finally {
			cancelTrimMode();
		}
	}

	export function updateTrimTimes(newStartTime, newEndTime) { // Can be called via ref
        if (isTrimming && typeof newStartTime === 'number' && typeof newEndTime === 'number') {
            trimStartTime = newStartTime;
            trimEndTime = newEndTime;
        }
    }

	$: isTrimDisabled = isTrimming || !localMediaUrl || isLoadingMedia || !localAudioBuffer || (isEditingSegment && !explicitMediaPath);


	export function seekTo(seconds) {
		if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return;
		if (!videoElement) return;
		const duration = videoElement.duration || 0;
		let clamped = Math.max(0, Math.min(seconds, duration));

        if (!explicitMediaPath) { // Only apply trim/edit clamping for main player
            if (isEditingSegment && editSegmentEndTime > editSegmentStartTime) {
                 clamped = Math.max(editSegmentStartTime, Math.min(clamped, editSegmentEndTime - 0.001));
            } else if (isTrimming && trimEndTime > trimStartTime) {
                 clamped = Math.max(trimStartTime, Math.min(clamped, trimEndTime - 0.001));
            }
        }

		cancelAnimationFrame(seekRafId);
		seekRafId = requestAnimationFrame(() => {
			videoElement.currentTime = clamped;
            localCurrentTime = clamped;
			if (!explicitMediaPath) updatePlayerTime(clamped);
		});
	}
	let seekRafId = null;

    // Button handlers for Notes context
    function handleNotesTranscribeClick() {
        dispatch('requestNotesTranscribe', { mediaPath: explicitMediaPath });
    }
    function handleNotesTrimClick() {
        dispatch('requestNotesTrim', {
            mediaPath: explicitMediaPath,
            duration: localDuration,
            audioBuffer: localAudioBuffer
        });
    }

    // Determine which player state to display
    $: displayTime = explicitMediaPath ? localCurrentTime : $transcriptStore.player.currentTime;
    $: displayDuration = explicitMediaPath ? localDuration : $transcriptStore.player.duration;
    $: displayIsPlaying = explicitMediaPath ? localIsPlaying : $transcriptStore.player.isPlaying;

</script>

<div class="p-1 flex flex-col bg-gray-50 dark:bg-gray-800">
	<div class="w-full max-w-[36rem] aspect-video bg-black relative mx-auto mb-1">
		{#if isLoadingMedia}
			<div class="absolute inset-0 flex items-center justify-center text-gray-400 animate-pulse"><span>Loading media...</span></div>
		{:else if localMediaUrl}
			{#key localMediaUrl}
				<video
					class="absolute inset-0 w-full h-full object-contain"
					src={localMediaUrl}
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
			<div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400"><span>No media selected or media failed to load</span></div>
		{/if}
	</div>
	<div class="flex items-center justify-between flex-shrink-0 max-w-[36rem] mx-auto w-full">
		<div class="flex items-center space-x-3">
			<button
				on:click={handleTogglePlay}
				class="btn-control"
				disabled={!localMediaUrl || isLoadingMedia}
				aria-label={displayIsPlaying ? 'Pause' : 'Play'}
			>
				{#if displayIsPlaying}
					 <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5zm5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5z" /></svg>
				{:else}
					 <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M11.596 8.697l-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393z" /></svg>
				{/if}
			</button>
			<span class="text-xs font-mono text-gray-600 dark:text-gray-400 tabular-nums whitespace-nowrap">
				{formatTime(displayTime)} / {formatTime(displayDuration)}
			</span>
			{#if showLoopPauseButton}
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
            {/if}
		</div>
		<div class="flex items-center space-x-2">
            {#if showNotesTranscribeButton}
                <button
                    on:click={handleNotesTranscribeClick}
                    class="btn-action"
                    title="Transcribe this media in main Transcriptions tab"
                    disabled={!localMediaUrl || isLoadingMedia}
                >
                    Transcribe
                </button>
            {/if}

            {#if showNotesTrimButton}
                 <button
                    on:click={handleNotesTrimClick}
                    class="btn-control"
                    title="Trim this media"
                    disabled={isLoadingMedia || !isMediaReadyForProcessing}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                        <path stroke-linecap="round" stroke-linejoin="round" d="m7.848 8.25 1.536.887M7.848 8.25a3 3 0 1 1-5.196-3 3 3 0 0 1 5.196 3Zm1.536.887a2.165 2.165 0 0 1 1.083 1.839c.005.351.054.695.14 1.024M9.384 9.137l2.077 1.199M7.848 15.75l1.536-.887m-1.536.887a3 3 0 1 1-5.196 3 3 3 0 0 1 5.196-3Zm1.536-.887a2.165 2.165 0 0 0 1.083-1.838c.005-.352.054-.695.14-1.025m-1.223 2.863 2.077-1.199m0-3.328a4.323 4.323 0 0 1 2.068-1.379l5.325-1.628a4.5 4.5 0 0 1 2.48-.044l.803.215-7.794 4.5m-2.882-1.664A4.33 4.33 0 0 0 10.607 12m3.736 0 7.794 4.5-.802.215a4.5 4.5 0 0 1-2.48-.043l-5.326-1.629a4.324 4.324 0 0 1-2.068-1.379M14.343 12l-2.882 1.664" />
                    </svg>
                    <span class="sr-only">Trim</span>
                </button>
            {:else if showMainTrimButton && !explicitMediaPath}
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
            {/if}
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
    .size-6 {
        width: 1.5rem;
        height: 1.5rem;
    }
</style>