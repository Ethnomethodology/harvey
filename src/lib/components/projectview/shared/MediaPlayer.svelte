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
	export let isMediaReadyForProcessing = false; // Default to false

	// --- Playback Speed State ---
	const playbackRates = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];
	let selectedPlaybackRate = 1;

	function changePlaybackRate(event) {
		selectedPlaybackRate = parseFloat(event.target.value);
		if (videoElement) {
			videoElement.playbackRate = selectedPlaybackRate;
		}
	}

	// --- Volume Control State ---
	let currentVolume = 1;
	let isMuted = false;
	let previousVolume = 1;
	const ICON_VOLUME_UP = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M11.536 14.01A8.47 8.47 0 0 0 14.026 8a8.47 8.47 0 0 0-2.49-6.01l-.708.707A7.476 7.476 0 0 1 13.025 8c0 2.071-.84 3.946-2.197 5.303zM10.121 12.596A6.48 6.48 0 0 0 12.025 8a6.48 6.48 0 0 0-1.904-4.596l-.707.707A5.482 5.482 0 0 1 11.025 8a5.482 5.482 0 0 1-1.61 3.89zm-2.07-2.07A4.486 4.486 0 0 0 9.025 8a4.486 4.486 0 0 0-1.004-2.524l-.707.707A3.488 3.488 0 0 1 8.025 8c0 .966-.39 1.841-1.031 2.476l.707.707M6.717 4.04A.5.5 0 0 1 7 4.5v7a.5.5 0 0 1-.812.39L3.825 9.5H1.5A.5.5 0 0 1 1 9V7a.5.5 0 0 1 .5-.5h2.325l2.363-2.39a.5.5 0 0 1 .529-.07z"/></svg>`;
	const ICON_VOLUME_MUTE = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M6.717 4.04A.5.5 0 0 1 7 4.5v7a.5.5 0 0 1-.812.39L3.825 9.5H1.5A.5.5 0 0 1 1 9V7a.5.5 0 0 1 .5-.5h2.325l2.363-2.39a.5.5 0 0 1 .529-.07zM11.031 8.031c0 .966-.39 1.841-1.031 2.476l-.707-.707A3.488 3.488 0 0 0 9.025 8c0-.966-.39-1.841-1.031-2.476l.707-.707A3.488 3.488 0 0 0 11.025 8M12.5 8c0-1.306-.474-2.475-1.232-3.369l-.707.707A4.486 4.486 0 0 1 11.525 8a4.486 4.486 0 0 1-1.004 2.524l.707.707A5.482 5.482 0 0 0 12.5 8m2.071-3.992L13.864 3.3A8.47 8.47 0 0 0 11.536.99l-.707.707A7.476 7.476 0 0 1 13.125 3C13.125 4.38 12.78 5.625 12.15 6.68l.708.707c.722-1.196 1.143-2.567 1.143-4.018M10.5 8a.5.5 0 0 0-.5.5v1.5H8.5a.5.5 0 0 0 0 1H10v1.5a.5.5 0 0 0 1 0V11h1.5a.5.5 0 0 0 0-1H11V8.5a.5.5 0 0 0-.5-.5"/></svg>`;

	function handleVolumeChange(event) {
		currentVolume = parseFloat(event.target.value);
		if (videoElement) {
			videoElement.volume = currentVolume;
			videoElement.muted = currentVolume === 0;
		}
		isMuted = currentVolume === 0;
	}

	function toggleMute() {
		if (!videoElement) return;
		isMuted = !isMuted;
		if (isMuted) {
			previousVolume = videoElement.volume;
			videoElement.volume = 0;
			currentVolume = 0;
		} else {
			videoElement.volume = previousVolume > 0 ? previousVolume : 0.1;
			currentVolume = videoElement.volume;
		}
		videoElement.muted = isMuted;
	}

	// --- Fullscreen State ---
	let isFullscreen = false;
	let playerContainerElement = null; // For requesting fullscreen on the container
	const ICON_FULLSCREEN_ENTER = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M1.5 1a.5.5 0 0 0-.5.5v4a.5.5 0 0 1-1 0v-4A1.5 1.5 0 0 1 1.5 0h4a.5.5 0 0 1 0 1zM10 .5a.5.5 0 0 1 .5-.5h4A1.5 1.5 0 0 1 16 1.5v4a.5.5 0 0 1-1 0v-4a.5.5 0 0 0-.5-.5h-4a.5.5 0 0 1-.5-.5M.5 10a.5.5 0 0 1 .5.5v4a.5.5 0 0 0 .5.5h4a.5.5 0 0 1 0 1h-4A1.5 1.5 0 0 1 0 14.5v-4a.5.5 0 0 1 .5-.5m15 0a.5.5 0 0 1 .5.5v4a1.5 1.5 0 0 1-1.5 1.5h-4a.5.5 0 0 1 0-1h4a.5.5 0 0 0 .5-.5v-4a.5.5 0 0 1 .5-.5"/></svg>`;
	const ICON_FULLSCREEN_EXIT = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" viewBox="0 0 16 16"><path d="M5.5 0a.5.5 0 0 1 .5.5v4A1.5 1.5 0 0 1 4.5 6h-4a.5.5 0 0 1 0-1h4a.5.5 0 0 0 .5-.5v-4a.5.5 0 0 1 .5-.5m5 0a.5.5 0 0 1 .5.5v4a.5.5 0 0 0 .5.5h4a.5.5 0 0 1 0 1h-4A1.5 1.5 0 0 1 10 4.5v-4a.5.5 0 0 1 .5-.5M0 10.5a.5.5 0 0 1 .5-.5h4A1.5 1.5 0 0 1 6 11.5v4a.5.5 0 0 1-1 0v-4a.5.5 0 0 0-.5-.5h-4a.5.5 0 0 1-.5-.5m10 0a.5.5 0 0 1 .5-.5h4a.5.5 0 0 0 .5.5v4a.5.5 0 0 1-1 0v-4a.5.5 0 0 0-.5-.5h-4a.5.5 0 0 1-.5-.5"/></svg>`;

	// --- Progress Bar Tooltip State ---
	let progressTooltipElement;
	let progressBarElement; // bind:this to the progress bar input
	let showProgressTooltip = false;
	let progressTooltipText = '00:00:00';
	let progressTooltipLeft = '0px';

	// --- Overlay Icon State & Icons ---
	let isHoveringVideo = false;
	const ICON_PLAY_OVERLAY = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" fill="currentColor" class="bi bi-play-circle-fill" viewBox="0 0 16 16" style="filter: drop-shadow(0 0 5px rgba(0,0,0,0.7));"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM6.79 5.093A.5.5 0 0 0 6 5.5v5a.5.5 0 0 0 .79.407l3.5-2.5a.5.5 0 0 0 0-.814l-3.5-2.5z"/></svg>`;
	const ICON_PAUSE_OVERLAY = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" fill="currentColor" class="bi bi-pause-circle-fill" viewBox="0 0 16 16" style="filter: drop-shadow(0 0 5px rgba(0,0,0,0.7));"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM6.25 5C5.56 5 5 5.56 5 6.25v3.5a1.25 1.25 0 1 0 2.5 0v-3.5C7.5 5.56 6.94 5 6.25 5zm3.5 0c-.69 0-1.25.56-1.25 1.25v3.5a1.25 1.25 0 1 0 2.5 0v-3.5C11 5.56 10.44 5 9.75 5z"/></svg>`;

	// --- Rewind/Forward Icons & Functions ---
	const ICON_REWIND = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2v1z"/><path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.16c-.12.1-.12.284 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466z"/></svg>`;
	const ICON_FORWARD = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2v1z"/><path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36-1.966A.25.25 0 0 1 8 4.466z"/></svg>`;

	function rewind30s() {
		if (!videoElement || isLoadingMedia) return;
		const newTime = Math.max(0, videoElement.currentTime - 30);
		seekTo(newTime);
	}
	function forward30s() {
		if (!videoElement || isLoadingMedia || !localDuration) return;
		const newTime = Math.min(localDuration, videoElement.currentTime + 30);
		seekTo(newTime);
	}

	async function toggleFullscreen() {
		if (!document.fullscreenEnabled || !playerContainerElement) return; // Use playerContainerElement
		try {
			if (!document.fullscreenElement) {
				await playerContainerElement.requestFullscreen(); // Request fullscreen on container
			} else {
				await document.exitFullscreen();
			}
		} catch (err) {
			console.error("Fullscreen toggle error:", err);
		}
		// isFullscreen will be updated by the event listener
	}

	function handleFullscreenChange() {
		isFullscreen = !!document.fullscreenElement;
	}

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
		document.addEventListener('fullscreenchange', handleFullscreenChange);
        return () => {
            if (audioContext && audioContext.state !== 'closed') {
                audioContext.close().catch(console.error);
            }
			document.removeEventListener('fullscreenchange', handleFullscreenChange);
        };
    });
	onDestroy(() => {
        if (audioContext && audioContext.state !== 'closed') {
            audioContext.close().catch(console.error);
        }
        if (currentBlobUrl) {
            URL.revokeObjectURL(currentBlobUrl);
        }
		document.removeEventListener('fullscreenchange', handleFullscreenChange);
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
                             console.log(`[MediaPlayer] DECODE_SUCCESS: Audio decoded for ${mediaPathToLoad}. localAudioBuffer is now ${localAudioBuffer ? 'set (AudioBuffer object)' : 'null'}. Duration of buffer: ${localAudioBuffer?.duration}s`);
                             if (!explicitMediaPath) setAudioBuffer(decodedBuffer); // Update global for main player
                         } catch (decodeError) {
                             console.error(`[MediaPlayer] DECODE_FAILED: Critical error decoding audio for ${mediaPathToLoad}. Error:`, decodeError);
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
                    // isMediaReadyForProcessing is set in the finally block
                } finally {
                    isLoadingMedia = false;
                    // Fallback for localDuration if video metadata didn't provide it but AudioBuffer did
                    if (localAudioBuffer && localAudioBuffer.duration > 0 && (localDuration === 0 || localDuration === undefined || isNaN(localDuration))) {
                        console.log(`[MediaPlayer] Updating localDuration (was: ${localDuration}) with localAudioBuffer.duration (${localAudioBuffer.duration}).`);
                        localDuration = localAudioBuffer.duration;
                        if (!explicitMediaPath) {
                            setPlayerDuration(localDuration);
                        }
                    }
                    // Update isMediaReadyForProcessing based on the final state of buffer and duration
                    console.log(`[MediaPlayer] CHECK_READY_STATE: For ${mediaPathToLoad || loadedPathFromProp || 'unknown media'} - localAudioBuffer is ${localAudioBuffer ? 'PRESENT' : 'NULL'}, localDuration is ${localDuration}.`);
                    if (localAudioBuffer && localDuration > 0) {
                        isMediaReadyForProcessing = true;
                        console.log(`[MediaPlayer] SET_READY_STATE: isMediaReadyForProcessing set to TRUE for ${mediaPathToLoad || loadedPathFromProp}`);
                    } else {
                        isMediaReadyForProcessing = false;
                        console.log(`[MediaPlayer] SET_READY_STATE: isMediaReadyForProcessing set to FALSE for ${mediaPathToLoad || loadedPathFromProp}. Reason: localAudioBuffer is ${localAudioBuffer ? 'PRESENT' : 'NULL'}, localDuration is ${localDuration}`);
                    }
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
                isMediaReadyForProcessing = false; // Explicitly false when no media path
                console.log(`[MediaPlayer] MEDIA_UNLOADED: Resetting state for ${loadedPathFromProp || 'previous media'}. isMediaReadyForProcessing is now ${isMediaReadyForProcessing}.`);
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
            if (videoElement) videoElement.currentTime = 0; // Explicitly set video element's time
            if (!explicitMediaPath) {
                setPlayerDuration(duration);
                updatePlayerTime(0);
            }
        } else {
            localDuration = 0;
            localCurrentTime = 0;
            if (videoElement) videoElement.currentTime = 0; // Explicitly set video element's time
            if (!explicitMediaPath) {
                setPlayerDuration(0);
                updatePlayerTime(0);
            }
        }
        if (videoElement) {
            localIsPlaying = !videoElement.paused;
            if (!explicitMediaPath) togglePlayerPlaying(!videoElement.paused);
            videoElement.playbackRate = selectedPlaybackRate;
            videoElement.volume = currentVolume; // Initialize volume
			videoElement.muted = isMuted;
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
        isMediaReadyForProcessing = false; // Ensure it's false on error too
        console.log(`[MediaPlayer] MEDIA_ERROR_STATE: Error during playback for ${explicitMediaPath || 'unknown media'}. isMediaReadyForProcessing is ${isMediaReadyForProcessing}. Error: ${errorMsg}`);
    }

	// --- Utility Functions ---
	function formatTime(totalSeconds) {
        if (isNaN(totalSeconds) || totalSeconds < 0) return '00:00';
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = Math.floor(totalSeconds % 60);
        return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }

	function formatTimeWithHours(totalSeconds) {
		if (isNaN(totalSeconds) || totalSeconds < 0) return '00:00:00';
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = Math.floor(totalSeconds % 60);
		return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
	}

	// --- Progress Bar Tooltip Handlers ---
	function handleMouseMoveOnProgressBar(event) {
		if (!localDuration || !progressBarElement || !progressTooltipElement) return;

		const progressBarRect = progressBarElement.getBoundingClientRect();
		const mouseX_relative = event.clientX - progressBarRect.left; // Cursor's X relative to progress bar's start

		// Calculate hover time based on the true mouse position
		const percent = Math.max(0, Math.min(1, mouseX_relative / progressBarRect.width));
		const hoverTime = percent * localDuration;
		progressTooltipText = formatTimeWithHours(hoverTime);

		// Calculate the ideal center position for the tooltip (directly under mouse)
		let idealTooltipCenter = mouseX_relative;

		// Adjust idealTooltipCenter to prevent tooltip edges from going outside progressBarElement
		const tooltipWidth = progressTooltipElement.offsetWidth;
		const minAllowedCenter = tooltipWidth / 2;
		const maxAllowedCenter = progressBarRect.width - (tooltipWidth / 2);

		let clampedTooltipCenter;
		if (progressBarRect.width < tooltipWidth) { // Tooltip wider than bar
			clampedTooltipCenter = progressBarRect.width / 2; // Center tooltip on the bar
		} else {
			clampedTooltipCenter = Math.max(minAllowedCenter, Math.min(idealTooltipCenter, maxAllowedCenter));
		}

		progressTooltipLeft = `${clampedTooltipCenter}px`;
		showProgressTooltip = true;
	}

	function handleMouseLeaveProgressBar() {
		showProgressTooltip = false;
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
            audioBuffer: localAudioBuffer,
            isReady: isMediaReadyForProcessing // Add this line
        });
    }

    // Determine which player state to display
    $: displayTime = explicitMediaPath ? localCurrentTime : ($transcriptStore.player.currentTime || 0);
    $: displayDuration = explicitMediaPath ? localDuration : ($transcriptStore.player.duration || 0);
    $: displayIsPlaying = explicitMediaPath ? localIsPlaying : $transcriptStore.player.isPlaying;

</script>

<div class="p-1 flex flex-col bg-gray-50 dark:bg-gray-800" bind:this={playerContainerElement}>
	<div
		class="w-full max-w-[36rem] aspect-video bg-black relative mx-auto cursor-pointer"
		id="video-container-wrapper"
		on:click={handleTogglePlay}
		role="button"
		aria-label="Play or pause video"
		on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleTogglePlay(); }}
		tabindex="0"
		on:mouseenter={() => { isHoveringVideo = true; }}
	on:mouseleave={() => { isHoveringVideo = false; }}
	>
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
					controlslist="nodownload noremoteplayback"
					tabindex="-1"
					><!-- tabindex -1 to keep it out of tab order as we have custom controls -->
				</video>
			{/key}
			<!-- Overlay Icon Div -->
			<div
				class="absolute inset-0 flex items-center justify-center pointer-events-none"
				style="color: white; opacity: { (isHoveringVideo || (!displayIsPlaying && !isLoadingMedia && localMediaUrl)) ? 0.85 : 0 }; transition: opacity 0.2s ease-in-out;"
			>
				{#if displayIsPlaying}
					{@html ICON_PAUSE_OVERLAY}
				{:else}
					{@html ICON_PLAY_OVERLAY}
				{/if}
			</div>
		{:else}
			<div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400">
				<span>No media selected or media failed to load</span>
			</div>
		{/if}
	</div>

	<!-- Custom Controls Bar -->
	<div class="flex flex-col items-center justify-between flex-shrink-0 max-w-[36rem] mx-auto w-full mt-1 space-y-1">
		<!-- Timeline with Tooltip -->
		<div class="relative w-full">
			<input
				type="range"
				bind:this={progressBarElement}
				class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-gray-700 video-progress"
				min="0"
				max={displayDuration > 0 ? displayDuration : 0}
				bind:value={displayTime}
				on:input={(e) => seekTo(parseFloat(e.target.value))}
				on:mousemove={handleMouseMoveOnProgressBar}
				on:mouseleave={handleMouseLeaveProgressBar}
				disabled={!localMediaUrl || isLoadingMedia || displayDuration <= 0}
				aria-label="Video progress bar"
			/>
			<span
				bind:this={progressTooltipElement}
				class="absolute bg-black text-white text-xs p-1 rounded pointer-events-none whitespace-nowrap"
				style="bottom: 16px; transform: translateX(-50%); display: {showProgressTooltip ? 'block' : 'none'}; left: {progressTooltipLeft};"
			>
				{progressTooltipText}
			</span>
		</div>
		<!-- Single row for all controls, managing space with gap -->
		<div class="flex items-center w-full gap-x-2 flex-wrap">
			<!-- Rewind Button -->
			<button
				on:click={rewind30s}
				class="btn-control"
				title="Rewind 30s"
				aria-label="Rewind 30 seconds"
				disabled={!localMediaUrl || isLoadingMedia}
			>
				{@html ICON_REWIND}
			</button>

			<!-- Play/Pause Button -->
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

			<!-- Forward Button -->
			<button
				on:click={forward30s}
				class="btn-control"
				title="Forward 30s"
				aria-label="Forward 30 seconds"
				disabled={!localMediaUrl || isLoadingMedia || !localDuration}
			>
				{@html ICON_FORWARD}
			</button>

			<!-- Time Display -->
			<span class="text-xs font-mono text-gray-600 dark:text-gray-400 tabular-nums whitespace-nowrap">
				{formatTime(displayTime)} / {formatTime(displayDuration)}
			</span>

			<!-- Playback Speed Selector -->
			<select
				id="playbackSpeedSelect"
				class="btn-control text-xs"
				on:change={changePlaybackRate}
				bind:value={selectedPlaybackRate}
				title="Playback Speed"
				disabled={!localMediaUrl || isLoadingMedia}
			>
				{#each playbackRates as rate}
					<option value={rate}>{rate}x</option>
				{/each}
			</select>

			<!-- Loop Button (if showLoopPauseButton is true) -->
			{#if showLoopPauseButton}
			<button
				class="btn-control inline-flex items-center space-x-1 text-sm"
				on:click={toggleLoop}
				title={isLooping ? 'Loop while editing' : 'Pause while editing'}
				aria-label={isLooping ? 'Loop while editing' : 'Pause while editing'}
			>
				{@html isLooping ? LOOP_ICON : PAUSE_ICON}
				<span class="ml-1 text-xs hidden sm:inline">
					{isLooping ? 'Loop' : 'Pause'}
				</span>
			</button>
			{/if}

			<!-- Conditional Trim Buttons -->
			{#if showNotesTrimButton}
				<button
					on:click={handleNotesTrimClick}
					class="btn-control"
					title="Trim this media"
					disabled={isLoadingMedia || !isMediaReadyForProcessing}
				>
					<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-5">
						<path stroke-linecap="round" stroke-linejoin="round" d="m7.848 8.25 1.536.887M7.848 8.25a3 3 0 1 1-5.196-3 3 3 0 0 1 5.196 3Zm1.536.887a2.165 2.165 0 0 1 1.083 1.839c.005.351.054.695.14 1.024M9.384 9.137l2.077 1.199M7.848 15.75l1.536-.887m-1.536.887a3 3 0 1 1-5.196 3 3 3 0 0 1 5.196-3Zm1.536-.887a2.165 2.165 0 0 0 1.083-1.838c.005-.352.054-.695.14-1.025m-1.223 2.863 2.077-1.199m0-3.328a4.323 4.323 0 0 1 2.068-1.379l5.325-1.628a4.5 4.5 0 0 1 2.48-.044l.803.215-7.794 4.5m-2.882-1.664A4.33 4.33 0 0 0 10.607 12m3.736 0 7.794 4.5-.802.215a4.5 4.5 0 0 1-2.48-.043l-5.326-1.629a4.324 4.324 0 0 1-2.068-1.379M14.343 12l-2.882 1.664" />
					</svg>
					<span class="sr-only">Trim</span>
				</button>
			{:else if showMainTrimButton && !explicitMediaPath}
				{#if isTrimming}
					<button on:click={confirmTrim} class="btn-action-trim text-xs" title="Confirm Trim">Trim</button>
					<button on:click={cancelTrimMode} class="btn-action-cancel text-xs" title="Cancel Trim">Cancel</button>
				{:else}
					<button
						on:click={enterTrimMode}
						class="btn-control"
						title="Trim Media"
						disabled={isTrimDisabled}
					>
						<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-5">
							<path stroke-linecap="round" stroke-linejoin="round" d="m7.848 8.25 1.536.887M7.848 8.25a3 3 0 1 1-5.196-3 3 3 0 0 1 5.196 3Zm1.536.887a2.165 2.165 0 0 1 1.083 1.839c.005.351.054.695.14 1.024M9.384 9.137l2.077 1.199M7.848 15.75l1.536-.887m-1.536.887a3 3 0 1 1-5.196 3 3 3 0 0 1 5.196-3Zm1.536-.887a2.165 2.165 0 0 0 1.083-1.838c.005-.352.054-.695.14-1.025m-1.223 2.863 2.077-1.199m0-3.328a4.323 4.323 0 0 1 2.068-1.379l5.325-1.628a4.5 4.5 0 0 1 2.48-.044l.803.215-7.794 4.5m-2.882-1.664A4.33 4.33 0 0 0 10.607 12m3.736 0 7.794 4.5-.802.215a4.5 4.5 0 0 1-2.48-.043l-5.326-1.629a4.324 4.324 0 0 1-2.068-1.379M14.343 12l-2.882 1.664" />
						</svg>
						<span class="sr-only">Trim</span>
					</button>
				{/if}
			{/if}

			<!-- Conditional Notes Transcribe Button -->
			{#if showNotesTranscribeButton}
			<button
				on:click={handleNotesTranscribeClick}
				class="btn-action text-xs"
				title="Transcribe this media in main Transcriptions tab"
				disabled={!localMediaUrl || isLoadingMedia}
			>
				Transcribe
			</button>
			{/if}

			<!-- Spacer to push fullscreen to the right if needed, or rely on flex-wrap and natural spacing -->
			<div class="flex-grow"></div>

			<!-- Mute Button -->
			<button
				on:click={toggleMute}
				class="btn-control"
				disabled={!localMediaUrl || isLoadingMedia}
				aria-label={isMuted ? 'Unmute' : 'Mute'}
			>
				{@html isMuted ? ICON_VOLUME_MUTE : ICON_VOLUME_UP}
			</button>

			<!-- Volume Slider -->
			<input
				type="range"
				class="w-16 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-gray-700 volume-slider"
				min="0"
				max="1"
				step="0.05"
				bind:value={currentVolume}
				on:input={handleVolumeChange}
				disabled={!localMediaUrl || isLoadingMedia || !videoElement}
				aria-label="Volume control"
			/>

			<!-- Fullscreen Button -->
			<button
				on:click={toggleFullscreen}
				class="btn-control"
				disabled={!localMediaUrl || isLoadingMedia || !playerContainerElement}
				aria-label={isFullscreen ? 'Exit Fullscreen' : 'Enter Fullscreen'}
			>
				{@html isFullscreen ? ICON_FULLSCREEN_EXIT : ICON_FULLSCREEN_ENTER}
			</button>
		</div>
	</div>
</div>

<style>
	/* Ensure this is defined if not already part of your global styles or Tailwind imports */
	#video-container-wrapper:fullscreen { /* Target the wrapper for fullscreen */
		max-width: 100% !important;
		max-height: 100% !important;
		width: 100% !important;
		height: 100% !important;
		display: flex;
	flex-direction: column;
	}
	#video-container-wrapper:fullscreen video {
		object-fit: contain;
		width: 100% !important;
	height: 100% !important;
	}


	.btn-control {
		padding: 0.35rem; /* Slightly smaller padding for denser controls */
		background: #e5e7eb; /* bg-gray-200 */
		color: #1f2937; /* text-gray-800 */
		border: 1px solid #d1d5db; /* border-gray-300 */
		border-radius: 0.25rem; /* rounded-sm for a bit tighter look */
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
	.btn-control svg { /* Default icon size */
		width: 1.15em;
		height: 1.15em;
	}
	.size-5 { /* For specific icons if needed, like trim */
        width: 1.25rem;
        height: 1.25rem;
    }


	.btn-action {
		padding: 0.35rem 0.75rem; /* Slightly smaller */
		background: #3b82f6; /* bg-blue-500 */
		color: white;
		border: none;
		border-radius: 0.25rem; /* rounded-sm */
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
		padding: 0.35rem 0.75rem;
		background: #10b981; /* bg-emerald-500 */
		color: white;
		border: none;
		border-radius: 0.25rem;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background-color 0.15s;
	}
	.btn-action-trim:hover {
		background: #059669; /* hover:bg-emerald-600 */
	}
	.btn-action-cancel {
		padding: 0.35rem 0.75rem;
		background: #ef4444; /* bg-red-500 */
		color: white;
		border: none;
		border-radius: 0.25rem;
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

	/* Custom styling for range inputs */
	.video-progress { /* Keep existing styles if they work, or adjust */
		-webkit-appearance: none;
		appearance: none;
		width: 100%;
		height: 0.5rem; /* 8px */
		border-radius: 0.25rem; /* 4px */
		background: #d1d5db; /* bg-gray-300 */
		outline: none;
		opacity: 0.9;
		transition: opacity .15s ease-in-out;
	}
	.dark .video-progress {
		background: #4b5563; /* dark:bg-gray-600 */
	}
	.video-progress:hover {
		opacity: 1;
	}
	.video-progress::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 1rem; /* 16px */
		height: 1rem; /* 16px */
		border-radius: 50%;
		background: #3b82f6; /* theme color, e.g. blue-500 */
		cursor: pointer;
		border: 2px solid white; /* Optional: add a border to the thumb */
	}
	.dark .video-progress::-webkit-slider-thumb {
		background: #2563eb; /* dark theme color */
		border-color: #374151; /* dark border for thumb */
	}
	.video-progress::-moz-range-thumb {
		width: 0.875rem; /* 14px */
		height: 0.875rem; /* 14px */
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 1px solid white;
	}
	.dark .video-progress::-moz-range-thumb {
		background: #2563eb;
		border-color: #374151;
	}

	.volume-slider {
		-webkit-appearance: none;
		appearance: none;
		/* width: 100%; */ /* Already has w-16 */
		height: 0.5rem; /* 8px */
		border-radius: 0.25rem; /* 4px */
		background: #d1d5db; /* bg-gray-300 */
		outline: none;
		opacity: 0.9;
		transition: opacity .15s ease-in-out;
	}
	.dark .volume-slider {
		background: #4b5563; /* dark:bg-gray-600 */
	}
	.volume-slider:hover {
		opacity: 1;
	}
	.volume-slider::-webkit-slider-thumb {
		width: 0.875rem; /* 14px */
		height: 0.875rem; /* 14px */
	}
	.dark .volume-slider::-webkit-slider-thumb {
		background: #2563eb; /* dark theme color */
		border-color: #374151; /* dark border for thumb */
	}
	.volume-slider::-moz-range-thumb {
		width: 0.75rem; /* 12px */
		height: 0.75rem; /* 12px */
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 1px solid white;
	}
	.dark .volume-slider::-moz-range-thumb {
		background: #2563eb;
		border-color: #374151;
	}

</style>