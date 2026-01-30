<!-- harvey-1.0/src/lib/components/projectview/MediaPlayer.svelte -->

<script>
	import { project } from '$lib/stores/projectStore.js';
	import {
		transcriptStore,
		updatePlayerTime,
		setPlayerDuration,
		togglePlayerPlaying,
		setAudioBuffer, // This will be used to set both buffer and peaks
		
	} from '$lib/stores/transcriptStore.js';
	import { get } from 'svelte/store'; // Ensure get is imported
	import { readFile } from '@tauri-apps/plugin-fs';
	import { open } from '@tauri-apps/plugin-dialog';
	import { invoke, convertFileSrc } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event'; // Restored listener
	import { onMount, onDestroy, tick, createEventDispatcher } from 'svelte';
	import { handleTrimMediaConfirm, refreshProjectFiles, getAssetMetadata } from '$lib/services/projectService.js';
	let waveformWorker = new Worker(new URL('$lib/workers/waveformWorker.js', import.meta.url), { type: 'module' });
	let currentWaveformLoadId = 0;
	let waveformLoadData = new Map();

	function initializeWaveformWorker() {
		waveformWorker.onmessage = async (event) => {
			const { type, payload, id } = event.data;
			if (id !== currentWaveformLoadId) {
				console.log(`[MediaPlayer] Discarding old waveform data (ID: ${id}, current: ${currentWaveformLoadId})`);
                waveformLoadData.delete(id);
				return; // Ignore old responses
			}

			const audioBuffer = waveformLoadData.get(id);
			waveformLoadData.delete(id);

			if (!audioBuffer) {
				console.error(`[MediaPlayer] Could not find audioBuffer for loadId ${id}`);
				return;
			}

			if (type === 'DECODE_AUDIO_COMPLETE') {
				const { peaks } = payload;
				localAudioBuffer = audioBuffer; // Set local buffer for this component instance
				if (!explicitMediaPath) {
					// For the main player, we proceed to handle global state and caching.
					const currentProject = get(project);
					const projectId = currentProject.id;
					const assetRelativePath = $transcriptStore.selectedMediaFile?.relative_path;

					// Step 2: Check for cached waveform data.
					if (projectId && assetRelativePath) {
						try {
							const metadata = await getAssetMetadata(assetRelativePath);
							if (metadata && metadata.waveform_data && metadata.waveform_data.length > 0) {
								const cachedPeaks = new Float32Array(new Uint8Array(metadata.waveform_data).buffer);
								setAudioBuffer(audioBuffer, cachedPeaks); // Set both buffer and cached peaks
								console.log(`[MediaPlayer] Waveform loaded from cache for ${assetRelativePath}.`);
								return; // Successfully loaded from cache
							}
						} catch (e) {
							console.warn(`[MediaPlayer] Error fetching metadata for waveform, will generate new one. Error:`, e);
						}
					}

					// Step 3: If no cached data, use generated peaks, set in store, and save to DB.
					console.log(`[MediaPlayer] No cached waveform data found for ${assetRelativePath}. Using newly generated peaks.`);
					setAudioBuffer(audioBuffer, peaks ? new Float32Array(peaks) : null); // Set buffer and newly generated peaks

					if (projectId && assetRelativePath && xmlPath && peaks) {
						try {
							const u8_peaks = new Uint8Array(new Float32Array(peaks).buffer);
							const s = $transcriptStore.selectedMediaFile;
							const metadataPayload = {
								...s,
								file_name: s.name,
								file_path: s.path,
								last_modified: new Date().toISOString(),
								waveform_data: Array.from(u8_peaks)
							};

							await invoke('update_asset_metadata_command', {
								projectXmlPathStr: xmlPath,
								assetRelativePath: assetRelativePath,
								metadataPayload: metadataPayload,
								customFieldsPayload: null,
								assetType: 'media'
							});
						} catch (error) {
							console.error(`[MediaPlayer] Failed to save generated waveform to DB:`, error);
						}
					}
				}
			} else if (type === 'DECODE_AUDIO_ERROR') {
				console.error(`[MediaPlayer] Error from waveform worker (ID: ${id}):`, payload.error);
				// Handle error, e.g., clear waveform, show message
				localAudioBuffer = null;
				if (!explicitMediaPath) setAudioBuffer(null, null);
			}
		};
		waveformWorker.onerror = (error) => {
			console.error('[MediaPlayer] Waveform worker error:', error);
			// Handle worker errors
		};
	}
	
	// import { register, unregisterAll } from '@tauri-apps/plugin-global-shortcut'; // Removed JS API

	const dispatch = createEventDispatcher();

	// --- Component Props ---
	export let videoElement = null;
	export let isTrimming = false; // For main transcriptions player's trim mode
	export let trimStartTime = 0;
	export let trimEndTime = 0;
	export let isEditingSegment = false; // For main transcriptions player's segment editing loop
	export let editSegmentStartTime = 0;
	export let editSegmentEndTime = 0;
    export let projectId = null; // Added for explicit project ID passing
    export let xmlPath = null;

	export let explicitMediaPath = null; // New prop to directly set the media source for this instance
    export let autoPlay = false;

	// Props for inline trim looping
	export let loopStartTime = 0;
	export let loopEndTime = 0;
	export let enableLooping = false;

	// Conditional UI for buttons
	export let showLoopPauseButton = true; // Default to true for main player
	export let showDataTranscribeButton = false; // Default to false
	export let showDataTrimButton = false; // Default to false
	export let showMainTrimButton = true; // Default to true

	$: console.log('[MediaPlayer] projectId prop updated:', projectId);

	// --- Internal State ---
	let localMediaUrl = ''; // URL for the <video> src
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

	// --- Playback Speed Custom Dropdown State ---
	let showPlaybackSpeedMenu = false;
	let playbackSpeedButtonElement = null;
	let playbackSpeedMenuPosition = { x: 0, y: 0 };
	let playbackSpeedMenuRef = null;
	function changePlaybackRate(event) {
		selectedPlaybackRate = parseFloat(event.target.value);
		if (videoElement) {
			videoElement.playbackRate = selectedPlaybackRate;
		}
	}

	function togglePlaybackSpeedMenu() {
		if (showPlaybackSpeedMenu) {
			showPlaybackSpeedMenu = false;
		} else {
			if (playbackSpeedButtonElement) {
				const rect = playbackSpeedButtonElement.getBoundingClientRect();
				playbackSpeedMenuPosition = {
					x: rect.left + window.scrollX,
					y: rect.bottom + window.scrollY + 2
				};
			}
			showPlaybackSpeedMenu = true;
		}
	}

	function selectPlaybackRate(rate) {
		selectedPlaybackRate = rate;
		if (videoElement) videoElement.playbackRate = rate;
		showPlaybackSpeedMenu = false;
	}
	// --- Volume Control State ---
	let currentVolume = 1;
	let isMuted = false;
	let previousVolume = 1;
	const ICON_VOLUME_UP = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-volume-up-fill" viewBox="0 0 16 16"><path d="M11.536 14.01A8.47 8.47 0 0 0 14.026 8a8.47 8.47 0 0 0-2.49-6.01l-.708.707A7.48 7.48 0 0 1 13.025 8c0 2.071-.84 3.946-2.197 5.303z"/><path d="M10.121 12.596A6.48 6.48 0 0 0 12.025 8a6.48 6.48 0 0 0-1.904-4.596l-.707.707A5.48 5.48 0 0 1 11.025 8a5.48 5.48 0 0 1-1.61 3.89z"/><path d="M8.707 11.182A4.5 4.5 0 0 0 10.025 8a4.5 4.5 0 0 0-1.318-3.182L8 5.525A3.5 3.5 0 0 1 9.025 8 3.5 3.5 0 0 1 8 10.475zM6.717 3.55A.5.5 0 0 1 7 4v8a.5.5 0 0 1-.812.39L3.825 10.5H1.5A.5.5 0 0 1 1 10V6a.5.5 0 0 1 .5-.5h2.325l2.363-1.89a.5.5 0 0 1 .529-.06"/></svg>`;
	const ICON_VOLUME_DOWN = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-volume-down-fill" viewBox="0 0 16 16"><path d="M9 4a.5.5 0 0 0-.812-.39L5.825 5.5H3.5A.5.5 0 0 0 3 6v4a.5.5 0 0 0 .5.5h2.325l2.363 1.89A.5.5 0 0 0 9 12zm3.025 4a4.5 4.5 0 0 1-1.318 3.182L10 10.475A3.5 3.5 0 0 0 11.025 8 3.5 3.5 0 0 0 10 5.525l.707-.707A4.5 4.5 0 0 1 12.025 8"/></svg>`;
	const ICON_VOLUME_MUTE = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-volume-mute-fill" viewBox="0 0 16 16"><path d="M6.717 3.55A.5.5 0 0 1 7 4v8a.5.5 0 0 1-.812.39L3.825 10.5H1.5A.5.5 0 0 1 1 10V6a.5.5 0 0 1 .5-.5h2.325l2.363-1.89a.5.5 0 0 1 .529-.06m7.137 2.096a.5.5 0 0 1 0 .708L12.207 8l1.647 1.646a.5.5 0 0 1-.708.708L11.5 8.707l-1.646 1.647a.5.5 0 0 1-.708-.708L10.793 8 9.146 6.354a.5.5 0 1 1 .708-.708L11.5 7.293l1.646-1.647a.5.5 0 0 1 .708 0"/></svg>`;

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

	// --- Video Minimize State & Icons ---
	export let isVideoMinimized = false;
	const ICON_MINIMIZE_VIDEO = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrows-collapse" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M1 8a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 0 1h-13A.5.5 0 0 1 1 8m7-8a.5.5 0 0 1 .5.5v3.793l1.146-1.147a.5.5 0 0 1 .708.708l-2 2a.5.5 0 0 1-.708 0l-2-2a.5.5 0 1 1 .708-.708L7.5 4.293V.5A.5.5 0 0 1 8 0m-.5 11.707-1.146 1.147a.5.5 0 0 1-.708-.708l2-2a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1-.708.708L8.5 11.707V15.5a.5.5 0 0 1-1 0z"/></svg>`;
	const ICON_MAXIMIZE_VIDEO = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrows-expand" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M1 8a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 0 1h-13A.5.5 0 0 1 1 8M7.646.146a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1-.708.708L8.5 1.707V5.5a.5.5 0 0 1-1 0V1.707L6.354 2.854a.5.5 0 1 1-.708-.708zM8 10a.5.5 0 0 1 .5.5v3.793l1.146-1.147a.5.5 0 0 1 .708.708l-2 2a.5.5 0 0 1-.708 0l-2-2a.5.5 0 0 1 .708-.708L7.5 14.293V10.5A.5.5 0 0 1 8 10"/></svg>`;

	function toggleMinimizeVideo() {
		isVideoMinimized = !isVideoMinimized;
	}

	// --- Progress Bar Tooltip State ---
	let progressTooltipElement;
	let progressBarElement; // bind:this to the progress bar input
	let showProgressTooltip = false;
	let progressTooltipText = '00:00:00';
	let progressTooltipLeft = '0px';
    let progressTooltipTop = '0px';

	// --- Overlay Icon State & Icons ---
	let isHoveringVideo = false;
	const ICON_PLAY_OVERLAY = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" fill="currentColor" class="bi bi-play-circle-fill" viewBox="0 0 16 16" style="filter: drop-shadow(0 0 5px rgba(0,0,0,0.7));"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM6.79 5.093A.5.5 0 0 0 6 5.5v5a.5.5 0 0 0 .79.407l3.5-2.5a.5.5 0 0 0 0-.814l-3.5-2.5z"/></svg>`;
	const ICON_PAUSE_OVERLAY = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" fill="currentColor" class="bi bi-pause-circle-fill" viewBox="0 0 16 16" style="filter: drop-shadow(0 0 5px rgba(0,0,0,0.7));"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM6.25 5C5.56 5 5 5.56 5 6.25v3.5a1.25 1.25 0 1 0 2.5 0v-3.5C7.5 5.56 6.94 5 6.25 5zm3.5 0c-.69 0-1.25.56-1.25 1.25v3.5a1.25 1.25 0 1 0 2.5 0v-3.5C11 5.56 10.44 5 9.75 5z"/></svg>`;

	// --- Rewind/Forward Icons & Functions ---
	const ICON_REWIND = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/></svg>`;
	const ICON_FORWARD = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/></svg>`;
	const ICON_CAMERA = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-camera" viewBox="0 0 16 16"><path d="M15 12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h1.172a3 3 0 0 0 2.12-.879l.83-.828A1 1 0 0 1 6.827 3h2.344a1 1 0 0 1 .707.293l.828.828A3 3 0 0 0 12.828 5H14a1 1 0 0 1 1 1zM2 4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-1.172a2 2 0 0 1-1.414-.586l-.828-.828A2 2 0 0 0 9.172 2H6.828a2 2 0 0 0-1.414.586l-.828-.828A2 2 0 0 1 3.172 4z"/><path d="M8 11a2.5 2.5 0 1 1 0-5 2.5 2.5 0 0 1 0 5m0 1a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7M3 6.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0"/></svg>`;
	const ICON_CC = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-badge-cc" viewBox="0 0 16 16"><path d="M3.708 7.755c0-1.111.488-1.753 1.319-1.753.681 0 1.138.47 1.186 1.107H7.36V7c-.052-1.186-1.024-2-2.342-2C3.414 5 2.5 6.05 2.5 7.751v.747c0 1.7.905 2.73 2.518 2.73 1.314 0 2.285-.792 2.342-1.939v-.114H6.213c-.048.615-.496 1.05-1.186 1.05-.84 0-1.319-.62-1.319-1.727zm6.14 0c0-1.111.488-1.753 1.318-1.753.682 0 1.139.47 1.187 1.107H13.5V7c-.053-1.186-1.024-2-2.342-2C9.554 5 8.64 6.05 8.64 7.751v.747c0 1.7.905 2.73 2.518 2.73 1.314 0 2.285-.792 2.342-1.939v-.114h-1.147c-.048.615-.497 1.05-1.187 1.05-.839 0-1.318-.62-1.318-1.727z"/><path d="M14 3a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM2 2a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z"/></svg>`;

	// --- Subtitle State ---
    // availableSubtitles and menu state removed
	let ccButtonElement = null;
	let activeSubtitleTrackPath = null;
	let activeSubtitleUrl = null;
	let activeSubtitleLang = 'en';
	let activeSubtitleLabel = 'Subtitles';

	function extractLangFromFilename(filename) {
		if (!filename || typeof filename !== 'string') return null;
		const namePart = filename.substring(0, filename.lastIndexOf('.')).toLowerCase();
		if (namePart === 'en' || namePart.includes('english')) return 'en';
		if (namePart === 'es' || namePart.includes('spanish')) return 'es';
		if (namePart === 'fr' || namePart.includes('french')) return 'fr';
		if (namePart === 'de' || namePart.includes('german')) return 'de';
		if (namePart.length === 2) return namePart;
		return null;
	}

	async function handleSelectSubtitles() {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Subtitles',
                    extensions: ['srt', 'vtt', 'ass']
                }]
            });

            if (selected && typeof selected === 'string') {
                const name = selected.split(/[\\/]/).pop();
                await selectSubtitleTrack({ path: selected, name: name });
            }
        } catch (error) {
            console.error('[MediaPlayer] Error opening subtitle dialog:', error);
        }
	}

    function handleSubtitleContextMenu(event) {
        event.preventDefault();
        if (activeSubtitleUrl) {
             selectSubtitleTrack(null); // Turn off
             console.log('[MediaPlayer] Subtitles disabled via context menu.');
        }
    }

	async function selectSubtitleTrack(subtitleEntry) {
		console.log('[MediaPlayer] Attempting to select subtitle track:', subtitleEntry);
		// Revoke previous object URL if it exists, to prevent memory leaks
		if (activeSubtitleUrl && activeSubtitleUrl.startsWith('blob:')) {
			URL.revokeObjectURL(activeSubtitleUrl);
			console.log('[MediaPlayer] Revoked previous subtitle object URL:', activeSubtitleUrl);
		}
        
        activeSubtitleUrl = null;
        await tick();

		if (subtitleEntry && subtitleEntry.path) {
			activeSubtitleTrackPath = subtitleEntry.path;
			activeSubtitleLang = extractLangFromFilename(subtitleEntry.name) || 'en';
			activeSubtitleLabel = subtitleEntry.name.substring(0, subtitleEntry.name.lastIndexOf('.')) || 'Subtitles';

			try {
				let subtitleDataUrl;
				if (subtitleEntry.name.toLowerCase().endsWith('.srt')) {
					console.log('[MediaPlayer] SRT file selected, invoking conversion:', subtitleEntry.path);
					const vttContent = await invoke('convert_srt_to_vtt_command', { srtPathStr: subtitleEntry.path });
					if (typeof vttContent === 'string') {
						const blob = new Blob([vttContent], { type: 'text/vtt' });
						subtitleDataUrl = URL.createObjectURL(blob);
						console.log('[MediaPlayer] SRT converted to VTT blob URL:', subtitleDataUrl);
					} else {
						throw new Error('SRT to VTT conversion did not return a string.');
					}
				} else { // Assume .vtt or other directly supported format
                    console.log('[MediaPlayer] Reading subtitle file directly:', subtitleEntry.path);
                    const fileData = await readFile(subtitleEntry.path);
                    
                    // Convert to string to check for WEBVTT header
                    const decoder = new TextDecoder('utf-8');
                    let content = decoder.decode(fileData);
                    
                    if (!content.trim().startsWith('WEBVTT')) {
                        console.log('[MediaPlayer] Missing WEBVTT header, prepending...');
                        content = 'WEBVTT\n\n' + content;
                    }

                    const blob = new Blob([content], { type: 'text/vtt' });
                    subtitleDataUrl = URL.createObjectURL(blob);
                    console.log('[MediaPlayer] Created Blob URL for subtitle:', subtitleDataUrl);
				}
				activeSubtitleUrl = subtitleDataUrl;
				console.log(`[MediaPlayer] Set active subtitle: URL=${activeSubtitleUrl}, Lang=${activeSubtitleLang}, Label=${activeSubtitleLabel}`);

                await tick();
                if (videoElement && videoElement.textTracks && videoElement.textTracks.length > 0) {
                    // Enable the last added track (which should correspond to our new activeSubtitleUrl)
                    // Or just enable all for now, or find the one matching.
                    // Usually there's only one if we replace activeSubtitleUrl.
                    for (let i = 0; i < videoElement.textTracks.length; i++) {
                        videoElement.textTracks[i].mode = 'showing';
                    }
                }

			} catch (e) {
				console.error('[MediaPlayer] Error processing subtitle file:', e);
				project.update(p => ({ ...p, statusMessage: 'Error loading subtitle track.', error: String(e) }));
				activeSubtitleTrackPath = null;
				activeSubtitleUrl = null;
			}
		} else {
			console.log('[MediaPlayer] Disabling subtitles.');
			activeSubtitleTrackPath = null;
			activeSubtitleUrl = null; // This will trigger the #key block to remove the <track>
			activeSubtitleLang = 'en';
			activeSubtitleLabel = 'Subtitles';
		}
	}

	function handleClickOutsidePlaybackSpeedMenu(event) {
		if (showPlaybackSpeedMenu && playbackSpeedMenuRef && !playbackSpeedMenuRef.contains(event.target) && playbackSpeedButtonElement && !playbackSpeedButtonElement.contains(event.target)) {
			showPlaybackSpeedMenu = false;
		}
	}


	async function handleScreenshot() {
		
		const currentProjectXmlPath = get(project)?.xmlPath;

		if (!currentProjectXmlPath) {
			project.update(p => ({ ...p, statusMessage: 'Project XML path not found.', error: 'Screenshot failed.', isLoading: false }));
			console.error('Project XML path not found for screenshot.');
			return;
		}

		if (!videoElement || !localMediaUrl || videoElement.videoWidth === 0 || videoElement.videoHeight === 0) {
			project.update(p => ({ ...p, statusMessage: 'Media not loaded or video dimensions unavailable.', error: 'Screenshot failed.' }));
			console.error('Screenshot attempt failed: No videoElement, localMediaUrl, or video dimensions are zero.');
			return;
		}

		// User feedback was "iff being played". For now, let's interpret this as "if media is active and has a frame to show".
		// If strict "must be actively playing" is needed, add: if (videoElement.paused) { ... }
		// For now, allowing screenshot from a paused frame.

		
		project.update(p => ({ ...p, statusMessage: 'Capturing screenshot...', isLoading: true, error: null }));

		try {
			const canvas = document.createElement('canvas');
			canvas.width = videoElement.videoWidth;
			canvas.height = videoElement.videoHeight;
			const ctx = canvas.getContext('2d');

			if (!ctx) {
				project.update(p => ({ ...p, statusMessage: 'Failed to get canvas context.', error: 'Screenshot failed.', isLoading: false }));
				console.error('Failed to get canvas 2D context.');
				return;
			}

			ctx.drawImage(videoElement, 0, 0, canvas.width, canvas.height);

			const dataUrl = canvas.toDataURL('image/png');
			// console.log('Screenshot data URL:', dataUrl.substring(0, 100) + '...'); // Log a snippet

			// For now, just log the data URL. Next step will involve sending this to Tauri.
			// To prepare for Tauri, which will take base64 data *without* the prefix:
			const base64ImageData = dataUrl.replace(/^data:image\/png;base64,/, '');

			// --- Begin Tauri Invocation ---
			console.log('Base64 image data ready. Invoking Tauri command...');

			// const currentProjectId = get(project)?.id; // Removed
			if (!projectId) { // Changed to use prop
				project.update(p => ({ ...p, statusMessage: 'Project ID (UUID) not found.', error: 'Screenshot failed.', isLoading: false })); // Clarified error
				console.error('Project ID (UUID) not found for screenshot. This is needed by backend.');
				return;
			}

			let mediaFileName = "unknown_media";
			const currentSelectedMedia = get(transcriptStore)?.selectedMediaFile;
			if (explicitMediaPath) {
					const pathParts = explicitMediaPath.split(/[\/]/);
					mediaFileName = pathParts.pop() || mediaFileName;
			} else if (currentSelectedMedia && currentSelectedMedia.path) {
					const pathParts = currentSelectedMedia.path.split(/[\/]/);
					mediaFileName = pathParts.pop() || mediaFileName;
			} else if (loadedPathFromProp) {
					const pathParts = loadedPathFromProp.split(/[\/]/);
					mediaFileName = pathParts.pop() || mediaFileName;
			}

			// *** Actual Tauri invoke call ***
			await invoke('save_screenshot', {
				projectXmlPathStr: currentProjectXmlPath, // New parameter
				projectId: projectId, // Existing prop
				mediaFileName: mediaFileName,
				timestamp: localCurrentTime,
				imageDataBase64: base64ImageData
			});

			project.update(p => ({ ...p, statusMessage: `Screenshot saved from ${mediaFileName}!`, isLoading: false, error: null }));
			console.log('Screenshot successfully processed by Tauri.');
			await refreshProjectFiles();
			console.log('[MediaPlayer] Project files refreshed after screenshot.');
			// --- End Tauri Invocation ---

		} catch (err) {
			const errorMessage = typeof err === 'string' ? err : (err.message || 'Unknown error');
			project.update(p => ({ ...p, statusMessage: 'Error saving screenshot.', error: `Save failed: ${errorMessage}`, isLoading: false }));
			console.error('Error during screenshot saving via Tauri:', err);
		}
	}

	function rewind10s() {
		if (!videoElement || isLoadingMedia) return;
		const newTime = Math.max(0, videoElement.currentTime - 10);
		seekTo(newTime);
	}
	function forward10s() {
		if (!videoElement || isLoadingMedia || !localDuration) return;
		const newTime = Math.min(localDuration, videoElement.currentTime + 10);
		seekTo(newTime);
	}

	

	let unlistenShortcutFn; // Renamed from unlistenShortcut

	onMount(async () => {
		initializeWaveformWorker();

		document.addEventListener('click', handleClickOutsidePlaybackSpeedMenu, true);

		const setupShortcutListener = async () => {
			try {
				console.log('[MediaPlayer] Setting up Tauri event listener for "shortcut-event"...');
				unlistenShortcutFn = await listen('shortcut-event', (event) => {
					// console.log('[MediaPlayer] Tauri "shortcut-event" received:', event); // Removed this line
					if (event.payload === 'rewind') {
						if (typeof rewind10s === 'function') rewind10s();
						else console.error('[MediaPlayer] rewind10s function not found!');
					} else if (event.payload === 'play-pause') {
						if (typeof handleTogglePlay === 'function') handleTogglePlay();
						else console.error('[MediaPlayer] handleTogglePlay function not found!');
					} else if (event.payload === 'forward') {
						if (typeof forward10s === 'function') forward10s();
						else console.error('[MediaPlayer] forward10s function not found!');
					}
				});
				console.log('[MediaPlayer] Tauri event listener for "shortcut-event" set up.');
			} catch (err) {
				console.error('[MediaPlayer] Error setting up Tauri "shortcut-event" listener:', err);
			}
		};
		setupShortcutListener();

        return () => {
			if (waveformWorker) {
				waveformWorker.terminate();
				waveformWorker = null;
			}
			document.removeEventListener('click', handleClickOutsidePlaybackSpeedMenu, true);
			if (unlistenShortcutFn) { // Use renamed variable
				console.log('[MediaPlayer] Cleaning up "shortcut-event" listener in onMount return.');
				unlistenShortcutFn();
			}
        };
    });

	onDestroy(() => {
		if (waveformWorker) {
			waveformWorker.terminate();
			waveformWorker = null;
		}
		if (activeSubtitleUrl && activeSubtitleUrl.startsWith('blob:')) {
			URL.revokeObjectURL(activeSubtitleUrl);
			console.log('[MediaPlayer] Revoked active subtitle object URL on destroy:', activeSubtitleUrl);
		}
		document.removeEventListener('click', handleClickOutsidePlaybackSpeedMenu, true);
		if (unlistenShortcutFn) { // Use renamed variable
			console.log('[MediaPlayer] Cleaning up "shortcut-event" listener in onDestroy.');
			unlistenShortcutFn();
		}
		// Removed try...catch for unregisterAll
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

    let isAudio = false;
    $: isAudio = loadedPathFromProp ? getMimeType(loadedPathFromProp).startsWith('audio/') : false;
    $: if (isAudio) isVideoMinimized = true;

	// Reactive block to load media when explicitMediaPath changes or (if not explicit) when global selectedMediaFile changes
	$: {
        const mediaPathToLoad = explicitMediaPath || $transcriptStore.selectedMediaFile?.path;

        const loadMedia = async (path) => {
            if (isLoadingMedia || path === loadedPathFromProp) {
                return;
            }

            // Aggressively terminate any ongoing worker process
            if (waveformWorker) {
                waveformWorker.terminate();
                // Re-initialize the worker for the next job
                waveformWorker = new Worker(new URL('$lib/workers/waveformWorker.js', import.meta.url), { type: 'module' });
                initializeWaveformWorker(); // Re-attach listeners
            }
            currentWaveformLoadId++; // Invalidate any pending messages from the old worker

            isLoadingMedia = true;
            if (isTrimming && !explicitMediaPath) cancelTrimMode();

            // Reset local state
            localMediaUrl = '';
            localAudioBuffer = null;
            localDuration = 0;
            localCurrentTime = 0;
            localIsPlaying = false;
            isMediaReadyForProcessing = false;

            try {
                const assetUrl = await convertFileSrc(path);
                loadedPathFromProp = path;
                localMediaUrl = assetUrl;

                if (!explicitMediaPath) {
                    setAudioBuffer(null, null);
                }

                await tick();
                if (videoElement) {
                    videoElement.load();
                }

            } catch (error) {
                console.error(`[MediaPlayer] Error getting asset URL for ${path}:`, error);
                if (!explicitMediaPath) { // Only update global store error if this is the main player
                    project.update((p) => ({
                        ...p,
                        error: `Failed to load media: ${error?.message || error}`,
                        statusMessage: 'Error loading media.'
                    }));
                    setAudioBuffer(null, null);
                    setPlayerDuration(0);
                    updatePlayerTime(0);
                    togglePlayerPlaying(false);
                } else {
                    dispatch('mediaLoadError', { path: path, error: error?.message || error });
                }
                localMediaUrl = '';
                loadedPathFromProp = null;
            } finally {
                isLoadingMedia = false;
            }
        };

        const unloadMedia = () => {
            if (isTrimming && !explicitMediaPath) cancelTrimMode();
            loadedPathFromProp = null;
            if (localMediaUrl !== '') {
                localMediaUrl = '';
                localDuration = 0;
                localCurrentTime = 0;
                localIsPlaying = false;
            }
            if (localAudioBuffer) {
                localAudioBuffer = null;
            }
            if (!explicitMediaPath) {
                if ($transcriptStore.audioBuffer) setAudioBuffer(null, null);
                if ($transcriptStore.player.duration > 0) setPlayerDuration(0);
                if ($transcriptStore.player.currentTime > 0) updatePlayerTime(0);
                if ($transcriptStore.player.isPlaying) togglePlayerPlaying(false);
            }
            isLoadingMedia = false;
            isMediaReadyForProcessing = false;
        };

        if (mediaPathToLoad) {
            if (mediaPathToLoad !== loadedPathFromProp) {
                 loadMedia(mediaPathToLoad);
            }
        } else {
            if (loadedPathFromProp) {
                unloadMedia();
            }
        }
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
            } else if (enableLooping && loopEndTime > loopStartTime && explicitMediaPath) { // Added for inline trim looping
				if (currentTime < loopStartTime || currentTime >= loopEndTime) {
					video.currentTime = loopStartTime;
					currentTime = loopStartTime;
					if (video.paused && video.currentTime === loopStartTime) video.play().catch(console.error);
				}
			}
            localCurrentTime = currentTime;
			if (!explicitMediaPath) updatePlayerTime(currentTime); // Update global for main player
		}
	}
	async function onLoadedMetadata(event) {
        if (event.target && typeof event.target.duration === 'number' && !isNaN(event.target.duration)) {
            const duration = event.target.duration;
            isMediaReadyForProcessing = true;
            localDuration = duration;
            localCurrentTime = 0;
            if (videoElement) videoElement.currentTime = 0;
            if (progressBarElement) progressBarElement.value = '0'; // Ensure progress bar visually resets

            if (!explicitMediaPath) {
                setPlayerDuration(duration);
                updatePlayerTime(0);
            }
            // Asynchronously decode audio for waveform
            decodeAudioForWaveform();
        } else {
            localDuration = 0;
            localCurrentTime = 0;
            if (videoElement) videoElement.currentTime = 0;
            if (progressBarElement) progressBarElement.value = '0'; // Ensure progress bar visually resets

            if (!explicitMediaPath) {
                setPlayerDuration(0);
                updatePlayerTime(0);
            }
        }
        if (videoElement) {
            if (autoPlay) {
                videoElement.play().catch(e => console.warn("[MediaPlayer] Auto-play failed:", e));
            }
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

    async function decodeAudioForWaveform() {
        if (!loadedPathFromProp) {
            return;
        }

        const loadId = currentWaveformLoadId;
        const currentProject = get(project);
        const projectId = currentProject.id;
        const assetRelativePath = $transcriptStore.selectedMediaFile?.relative_path;

        // 1. Check for cached waveform data first
        if (projectId && assetRelativePath) {
            try {
                console.log('[MediaPlayer] Checking for cached waveform data...');
                const metadata = await getAssetMetadata(assetRelativePath);
                if (metadata && metadata.waveform_data && metadata.waveform_data.length > 0) {
                    console.log('[MediaPlayer] Cached waveform data found.');
                    const cachedPeaks = new Float32Array(new Uint8Array(metadata.waveform_data).buffer);
                    // Set the peaks, but not the audio buffer. The buffer will be loaded on demand.
                    setAudioBuffer(null, cachedPeaks);
                    if (metadata.duration_seconds) {
                        setPlayerDuration(metadata.duration_seconds);
                    }
                    console.log(`[MediaPlayer] Waveform loaded from cache for ${assetRelativePath}.`);
                    return; // Exit early
                } else {
                    console.log('[MediaPlayer] No cached waveform data found.');
                }
            } catch (e) {
                console.log('[MediaPlayer] Error fetching metadata for waveform:', e);
                console.warn(`[MediaPlayer] Error fetching metadata for waveform, will generate new one. Error:`, e);
            }
        }

        // 2. If no cached data, proceed with decoding
        try {
            const fileData = await readFile(loadedPathFromProp);
            const arrayBuffer = fileData.buffer; // Get the underlying ArrayBuffer

            // Always decode audio on the main thread to get the AudioBuffer for playback
            const audioContext = new (window.AudioContext || window.webkitAudioContext)();
            const decodedAudioBuffer = await audioContext.decodeAudioData(arrayBuffer);

            // If no cached data, proceed with generating peaks using the worker
            console.log(`[MediaPlayer] No cached waveform data found for ${assetRelativePath}. Sending to worker for peak generation.`);
            const channelData = decodedAudioBuffer.getChannelData(0); // Assuming mono or taking first channel
            const transferableChannelData = new Float32Array(channelData); // Create a new Float32Array to transfer
            waveformWorker.postMessage({
                type: 'GENERATE_PEAKS',
                payload: {
                    channelData: transferableChannelData,
                    sampleRate: decodedAudioBuffer.sampleRate,
                    filePath: loadedPathFromProp
                },
                id: loadId
            }, [transferableChannelData.buffer]); // Transfer the buffer of the new Float32Array

            // Store decodedAudioBuffer locally for use in onmessage handler when worker responds
            waveformLoadData.set(loadId, decodedAudioBuffer);

        } catch (error) {
            waveformLoadData.delete(loadId);
            console.error(`[MediaPlayer] Error reading or decoding audio file for waveform:`, error);
            if (!explicitMediaPath) {
                setAudioBuffer(null, null);
            }
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
            setAudioBuffer(null, null);
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

	function portal(node) {
		document.body.appendChild(node);
		return {
			destroy() {
				if (node.parentNode) {
					node.parentNode.removeChild(node);
				}
			}
		};
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

		progressTooltipLeft = `${progressBarRect.left + clampedTooltipCenter}px`;
        progressTooltipTop = `${progressBarRect.top - 10}px`;
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

		// cancelAnimationFrame(seekRafId); // No longer using rAF for seekTo
		// seekRafId = requestAnimationFrame(() => { // No longer using rAF for seekTo
		videoElement.currentTime = clamped;
		localCurrentTime = clamped;
		if (!explicitMediaPath) updatePlayerTime(clamped);
		// }); // No longer using rAF for seekTo
	}
	// let seekRafId = null; // No longer using rAF for seekTo

    // Button handlers for Data context
    function handleDataTranscribeClick() {
        dispatch('requestDataTranscribe', { mediaPath: explicitMediaPath });
    }
    async function handleDataTrimClick() {
        let buffer = localAudioBuffer;
        let ready = isMediaReadyForProcessing;

        if (!buffer) {
            
            if (!loadedPathFromProp) {
                dispatch('mediaLoadError', { path: explicitMediaPath, error: 'Cannot process audio for trimming.' });
                return;
            }
            try {
                isLoadingMedia = true;
                // Post message to worker to decode audio for trimming
                currentWaveformLoadId++;
                const loadId = currentWaveformLoadId;
                const fileData = await readFile(loadedPathFromProp);
                const arrayBuffer = fileData.buffer;

                const audioContext = new (window.AudioContext || window.webkitAudioContext)();
                const decodedAudioBuffer = await audioContext.decodeAudioData(arrayBuffer);
                const channelData = decodedAudioBuffer.getChannelData(0); // Assuming mono or taking first channel
                const transferableChannelData = new Float32Array(channelData); // Create a new Float32Array to transfer

                waveformWorker.postMessage({
                    type: 'GENERATE_PEAKS',
                    payload: {
                        channelData: transferableChannelData,
                        sampleRate: decodedAudioBuffer.sampleRate,
                        filePath: loadedPathFromProp
                    },
                    id: loadId
                }, [transferableChannelData.buffer]);

                const workerResponse = await new Promise((resolve, reject) => {
                    const handleWorkerMessage = (event) => {
                        if (event.data.id === loadId) {
                            waveformWorker.removeEventListener('message', handleWorkerMessage);
                            if (event.data.type === 'DECODE_AUDIO_COMPLETE') {
                                // The worker now sends back null for audioBuffer, as it doesn't have the full AudioBuffer
                                // We need to use the decodedAudioBuffer from the main thread.
                                resolve(decodedAudioBuffer);
                            } else {
                                reject(new Error(event.data.payload.error || 'Worker decoding failed'));
                            }
                        }
                    };
                    waveformWorker.addEventListener('message', handleWorkerMessage);
                });

                localAudioBuffer = workerResponse;
                buffer = workerResponse;
                ready = true;
                isMediaReadyForProcessing = true;
                
            } catch (error) {
                console.error(`[MediaPlayer] Lazy decode for trim failed:`, error);
                dispatch('mediaLoadError', { path: explicitMediaPath, error: 'Failed to decode audio for trimming.' });
                ready = false;
            } finally {
                isLoadingMedia = false;
            }
        }

        dispatch('requestDataTrim', {
            mediaPath: explicitMediaPath,
            duration: localDuration,
            audioBuffer: buffer,
            isReady: ready
        });
    }

    // Determine which player state to display
    $: displayTime = explicitMediaPath ? localCurrentTime : ($transcriptStore.player.currentTime || 0);
    $: displayDuration = explicitMediaPath ? localDuration : ($transcriptStore.player.duration || 0);
    $: displayIsPlaying = explicitMediaPath ? localIsPlaying : $transcriptStore.player.isPlaying;

    

</script>

<div class="p-1 flex flex-col bg-gray-50 dark:bg-surface-2 h-full">
	<div
		class="w-full flex-grow min-h-0 bg-black relative cursor-pointer"
		class:hidden={isVideoMinimized}
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
					crossorigin="anonymous"
				>
					{#if activeSubtitleUrl}
						<track kind="subtitles" src={activeSubtitleUrl} srclang={activeSubtitleLang} label={activeSubtitleLabel} default />
					{/if}
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
		<div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-d-gray-400">
				<span>No media selected or media failed to load</span>
			</div>
		{/if}
	</div>

	<!-- Custom Controls Bar -->
	<div
		class="flex flex-col items-center justify-between flex-shrink-0 w-full space-y-1 px-2 pb-1 bg-gray-100 dark:bg-surface-3 rounded-b-md border border-gray-300 dark:border-border shadow-md"
		style="position: relative; z-index: 100;"
	>
		<!-- Timeline with Tooltip -->
		<div class="relative w-full" style="z-index: 20;"> <!-- Stacking for timeline within control bar -->
			<input
				type="range"
				bind:this={progressBarElement}
				class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-text-secondary video-progress"
				min="0"
				max={displayDuration > 0 ? displayDuration : 0}
				bind:value={displayTime}
				on:input={(e) => seekTo(parseFloat(e.target.value))}
				on:mousemove={handleMouseMoveOnProgressBar}
				on:mouseleave={handleMouseLeaveProgressBar}
				disabled={!localMediaUrl || isLoadingMedia || displayDuration <= 0}
				aria-label="Video progress bar"
				style="--progress: {displayDuration > 0 ? displayTime / displayDuration : 0};"
				autocomplete="off"
				autocorrect="off"
			/>
			<span
				use:portal
				bind:this={progressTooltipElement}
				class="fixed bg-black text-white text-xs p-1 rounded pointer-events-none whitespace-nowrap z-[9999]"
				style="top: {progressTooltipTop}; left: {progressTooltipLeft}; transform: translate(-50%, -100%); display: {showProgressTooltip ? 'block' : 'none'};"
			>
				{progressTooltipText}
			</span>
		</div>
		<!-- Single row for all controls, managing space with gap -->
		<div class="flex items-center w-full gap-x-2 flex-wrap">
			<!-- Rewind Button -->
			<button
				on:click={rewind10s}
				class="ui-button-icon"
				title="Rewind 10s"
				aria-label="Rewind 10 seconds"
				disabled={!localMediaUrl || isLoadingMedia}
			>
				{@html ICON_REWIND}
			</button>

			<!-- Play/Pause Button -->
			<button
				on:click={handleTogglePlay}
				class="ui-button-icon"
				disabled={!localMediaUrl || isLoadingMedia}
				aria-label={displayIsPlaying ? 'Pause' : 'Play'}
			>
				{#if displayIsPlaying}
					<!-- New Pause Icon -->
					<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-pause-fill" viewBox="0 0 16 16">
					  <path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5m5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5"/>
					</svg>
				{:else}
					<!-- New Play Icon -->
					<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-play-fill" viewBox="0 0 16 16">
					  <path d="m11.596 8.697-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393"/>
					</svg>
				{/if}
			</button>

			<!-- Forward Button -->
			<button
				on:click={forward10s}
				class="ui-button-icon"
				title="Forward 10s"
				aria-label="Forward 10 seconds"
				disabled={!localMediaUrl || isLoadingMedia || !localDuration}
			>
				{@html ICON_FORWARD}
			</button>

			<!-- Time Display -->
			<span class="text-xs font-mono text-gray-600 dark:text-d-gray-400 tabular-nums whitespace-nowrap">
				{formatTime(displayTime)} / {formatTime(displayDuration)}
			</span>

			<!-- Conditional Data Transcribe Button -->
			{#if showDataTranscribeButton}
			<button
				on:click={handleDataTranscribeClick}
				class="btn-action text-xs"
				title="Transcribe this media in main Transcriptions tab"
				disabled={!localMediaUrl || isLoadingMedia}
			>
				Transcribe
			</button>
			{/if}

			<!-- Spacer 1: Pushes the middle group -->
			<div class="flex-grow"></div>

			<!-- Centered Group: Playback Speed, Screenshot, Trim -->
			<!-- Playback Speed Selector -->
			<button
				bind:this={playbackSpeedButtonElement}
				on:click={togglePlaybackSpeedMenu}
				class="ui-button-icon text-xs min-w-[48px]"
				title="Playback Speed"
				aria-label="Select playback speed"
				aria-haspopup="true"
				aria-expanded={showPlaybackSpeedMenu}
				disabled={!localMediaUrl || isLoadingMedia}
			>
				{selectedPlaybackRate}x
			</button>

			<!-- Screenshot Button -->
			<button
				on:click={handleScreenshot}
				class="ui-button-icon"
				title="Take screenshot"
				aria-label="Take screenshot of current video frame"
				disabled={!localMediaUrl || isLoadingMedia || !projectId || isAudio}
			>
				{@html ICON_CAMERA}
			</button>

			<!-- Conditional Trim Buttons -->
			{#if showDataTrimButton}
				<button
					on:click={handleDataTrimClick}
					class="ui-button-icon"
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
					<button on:click={enterTrimMode} class="ui-button-icon" title="Trim Media" disabled={isTrimDisabled}>
						<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-5">
							<path stroke-linecap="round" stroke-linejoin="round" d="m7.848 8.25 1.536.887M7.848 8.25a3 3 0 1 1-5.196-3 3 3 0 0 1 5.196 3Zm1.536.887a2.165 2.165 0 0 1 1.083 1.839c.005.351.054.695.14 1.024M9.384 9.137l2.077 1.199M7.848 15.75l1.536-.887m-1.536.887a3 3 0 1 1-5.196 3 3 3 0 0 1 5.196-3Zm1.536-.887a2.165 2.165 0 0 0 1.083-1.838c.005-.352.054-.695.14-1.025m-1.223 2.863 2.077-1.199m0-3.328a4.323 4.323 0 0 1 2.068-1.379l5.325-1.628a4.5 4.5 0 0 1 2.48-.044l.803.215-7.794 4.5m-2.882-1.664A4.33 4.33 0 0 0 10.607 12m3.736 0 7.794 4.5-.802.215a4.5 4.5 0 0 1-2.48-.043l-5.326-1.629a4.324 4.324 0 0 1-2.068-1.379M14.343 12l-2.882 1.664" />
						</svg>
						<span class="sr-only">Trim</span>
					</button>
				{/if}
			{/if}

			<!-- Spacer 2: Pushes the right group -->
			<div class="flex-grow"></div>

			<!-- CC/Subtitle Button (MOVED HERE) -->
			<button
				bind:this={ccButtonElement}
				on:click={handleSelectSubtitles}
				on:contextmenu={handleSubtitleContextMenu}
				class="ui-button-icon"
				title="Select Subtitles (Right-click to disable)"
				aria-label="Select Subtitles"
				disabled={!localMediaUrl || isLoadingMedia || isAudio}
			>
				{@html ICON_CC}
			</button>


			<!-- Mute Button -->
			<button
				on:click={toggleMute}
				class="ui-button-icon"
				disabled={!localMediaUrl || isLoadingMedia}
				aria-label={isMuted ? 'Unmute' : 'Mute'}
			>
				{@html isMuted ? ICON_VOLUME_MUTE : ICON_VOLUME_UP}
			</button>

			<!-- Volume Slider -->
			<input
				type="range"
				class="w-16 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-text-secondary volume-slider"
				min="0"
				max="1"
				step="0.05"
				bind:value={currentVolume}
				on:input={handleVolumeChange}
				disabled={!localMediaUrl || isLoadingMedia || !videoElement}
				aria-label="Volume control"
				style="--progress: {currentVolume};"
				autocomplete="off"
				autocorrect="off"
			/>

			<!-- Minimize/Maximize Video Button -->
			<button
				on:click={toggleMinimizeVideo}
				class="ui-button-icon"
				title={isVideoMinimized ? 'Show Media' : 'Hide Media'}
				aria-label={isVideoMinimized ? 'Show Media' : 'Hide Media'}
				disabled={!localMediaUrl || isLoadingMedia || isAudio}
			>
				{#if isVideoMinimized}
					{@html ICON_MAXIMIZE_VIDEO}
				{:else}
					{@html ICON_MINIMIZE_VIDEO}
				{/if}
			</button>
		</div>
	</div>
</div>

<!-- Subtitle menu removed -->

{#if showPlaybackSpeedMenu}
	<div
		bind:this={playbackSpeedMenuRef}
		class="fixed z-50 bg-white dark:bg-d-gray-700 border border-gray-300 dark:border-border rounded-md shadow-lg py-1 text-xs min-w-[80px]"
		style="left: {playbackSpeedMenuPosition.x}px; top: {playbackSpeedMenuPosition.y}px;"
		role="menu"
	>
		{#each playbackRates as rate (rate)}
			<button
				on:click={() => selectPlaybackRate(rate)}
				class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-d-gray-600 text-gray-800 dark:text-d-gray-200"
				class:bg-blue-100={selectedPlaybackRate === rate}
				class:dark:bg-blue-800={selectedPlaybackRate === rate}
				role="menuitemradio"
				aria-checked={selectedPlaybackRate === rate}
			>{rate}x</button>
		{/each}
	</div>
{/if}
<style>
	/*
	REMOVED: #video-container-wrapper:fullscreen and #video-container-wrapper:fullscreen video styles
	as fullscreen functionality is removed.
	*/

	

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
		background: linear-gradient(to right, var(--color-text-primary) calc(var(--progress, 0) * 100%), var(--color-text-secondary) calc(var(--progress, 0) * 100%));
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
		background: var(--color-accent-primary);
		border-color: transparent;
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
		background: var(--color-accent-primary);
		border-color: transparent;
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
		background: linear-gradient(to right, var(--color-text-primary) calc(var(--progress, 0) * 100%), var(--color-text-secondary) calc(var(--progress, 0) * 100%));
	}
	.volume-slider:hover {
		opacity: 1;
	}
	.volume-slider::-webkit-slider-thumb {
		width: 0.875rem; /* 14px */
		height: 0.875rem; /* 14px */
	}
	.dark .volume-slider::-webkit-slider-thumb {
		background: var(--color-accent-primary);
		border-color: transparent;
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
		background: var(--color-accent-primary);
		border-color: transparent;
	}
</style>