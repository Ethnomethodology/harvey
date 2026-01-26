<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { project } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';

    export let itemPath = null;

    const dispatch = createEventDispatcher();

    const PLAY_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-play-fill" viewBox="0 0 16 16"><path d="m11.596 8.697-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393z"/></svg>`;
    const PAUSE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-pause-fill" viewBox="0 0 16 16"><path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5m5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5"/></svg>`;
    const REWIND_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/></svg>`;
    const FORWARD_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16"><path d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/></svg>`;
    const REPEAT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-repeat" viewBox="0 0 16 16"><path d="M11 5.466V4H5a4 4 0 0 0-3.584 5.777.5.5 0 1 1-.896.446A5 5 0 0 1 5 3h6V1.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36 1.966a.25.25 0 0 1-.41-.192m3.81.086a.5.5 0 0 1 .67.225A5 5 0 0 1 11 13H5v1.466a.25.25 0 0 1-.41.192l-2.36-1.966a.25.25 0 0 1 0-.384l2.36-1.966a.25.25 0 0 1 .41.192V12h6a4 4 0 0 0 3.585-5.777.5.5 0 0 1 .225-.67Z"/></svg>`;
    const VOLUME_HIGH_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-volume-up-fill" viewBox="0 0 16 16"><path d="M11.536 14.01A8.47 8.47 0 0 0 14.026 8a8.47 8.47 0 0 0-2.49-6.01l-.708.707A7.48 7.48 0 0 1 13.025 8c0 2.071-.84 3.946-2.197 5.303z"/><path d="M10.121 12.59A6.48 6.48 0 0 0 12.025 8a6.48 6.48 0 0 0-1.904-4.59l-.707.707A5.48 5.48 0 0 1 11.025 8a5.48 5.48 0 0 1-1.61 3.88z"/><path d="M8.707 11.18A4.5 4.5 0 0 0 10.025 8a4.5 4.5 0 0 0-1.318-3.18l-.707.707A3.5 3.5 0 0 1 9.025 8a3.5 3.5 0 0 1-1.025 2.47zM6.717 3.55A.5.5 0 0 1 7 4v8a.5.5 0 0 1-.812.39L3.825 10.5H1.5A.5.5 0 0 1 1 10V6a.5.5 0 0 1 .5-.5h2.325l2.393-1.85A.5.5 0 0 1 6.717 3.55"/></svg>`;
    const THREE_DOTS_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16"><path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/></svg>`;

    let audio;
    let src = null;
    let paused = true;
    let currentTime = 0;
    let duration = 0;
    let volume = 1;
    let repeat = false;
    let lastLoadedSrc = null;
    let lastActiveTranscriptPath = null;

    let attachments = [];
    let showMenu = false;
    let menuButton;

    async function loadAttachments() {
        const projectStoreState = get(project);
        const activePath = itemPath || 
                           projectStoreState.activeTranscriptPathInDataTab || 
                           projectStoreState.currentImportedTranscriptPath ||
                           (projectStoreState.selectedDocumentPath && projectStoreState.selectedDocumentPath.toLowerCase().endsWith('.json') ? projectStoreState.selectedDocumentPath : null);

        console.log("[ThinMediaPlayer] loadAttachments called for:", activePath);
        
        if (!projectStoreState.id || !activePath) {
            console.log("[ThinMediaPlayer] No project ID or active path, clearing attachments.");
            attachments = [];
            src = null;
            return;
        }

        if (activePath !== lastActiveTranscriptPath) {
            console.log("[ThinMediaPlayer] Transcript path changed, resetting src.");
            lastActiveTranscriptPath = activePath;
            src = null; 
            lastLoadedSrc = null;
        }

        // Derive relative path
        let assetRelativePath = activePath.startsWith(projectStoreState.baseDirectory) 
            ? activePath.substring(projectStoreState.baseDirectory.length) 
            : activePath;
        assetRelativePath = assetRelativePath.replace(/\\/g, '/').replace(/^\//, '');
        console.log("[ThinMediaPlayer] Derived relative path:", assetRelativePath);

        try {
            const result = await invoke('get_asset_metadata_command', {
                projectId: projectStoreState.id,
                assetRelativePath: assetRelativePath
            });

            if (result && result.custom_fields_json) {
                const customFields = JSON.parse(result.custom_fields_json);
                const attachmentsField = customFields.find(f => f.key === 'attachments');
                if (attachmentsField && attachmentsField.value) {
                    attachments = JSON.parse(attachmentsField.value);
                    console.log("[ThinMediaPlayer] Loaded attachments array:", attachments);
                    if (attachments.length > 0 && !src) {
                        console.log("[ThinMediaPlayer] Auto-selecting first track.");
                        selectTrack(attachments[0]);
                    }
                } else {
                    console.log("[ThinMediaPlayer] No attachments field found in metadata.");
                    attachments = [];
                }
            } else {
                console.log("[ThinMediaPlayer] No metadata result for asset.");
                attachments = [];
            }
        } catch (error) {
            console.error(`[ThinMediaPlayer] Error loading attachments:`, error);
            attachments = [];
        }
    }

    function selectTrack(path) {
        console.log("[ThinMediaPlayer] selectTrack called with:", path);
        src = convertFileSrc(path);
        console.log("[ThinMediaPlayer] converted src:", src);
        showMenu = false;
    }

    $: if (audio && src && src !== lastLoadedSrc) {
        console.log("[ThinMediaPlayer] Reactive src update. New src:", src);
        lastLoadedSrc = src;
        audio.src = src;
        audio.load();
        currentTime = 0;
        // Don't auto-play unless user clicks play, 
        // but if they were already playing, keep playing.
        if (!paused) {
            audio.play().catch(e => console.error("[ThinMediaPlayer] play() failed:", e));
        }
    }

    function togglePlay(e) {
        if (e) {
            e.preventDefault();
            e.stopPropagation();
        }
        console.log("[ThinMediaPlayer] togglePlay. Current state - paused:", paused, "src:", src);
        if (!src) {
            console.warn("[ThinMediaPlayer] Cannot play: No src set.");
            return;
        }
        
        if (paused) {
            paused = false;
            // The reactive block above or Svelte's bind:paused will handle the actual audio.play()
            // but for safety in user-initiated actions:
            tick().then(() => {
                if (audio.paused) {
                    audio.play().catch(e => {
                        console.error("[ThinMediaPlayer] Manual play() failed:", e);
                        paused = true;
                    });
                }
            });
        } else {
            paused = true;
        }
    }

    import { tick } from 'svelte';

    function rewind10s(e) {
        if (e) {
            e.preventDefault();
            e.stopPropagation();
        }
        if (!audio) return;
        currentTime = Math.max(0, currentTime - 10);
    }

    function forward10s(e) {
        if (e) {
            e.preventDefault();
            e.stopPropagation();
        }
        if (!audio || !duration) return;
        currentTime = Math.min(duration, currentTime + 10);
    }

    function toggleRepeat(e) {
        if (e) {
            e.preventDefault();
            e.stopPropagation();
        }
        repeat = !repeat;
        console.log("[ThinMediaPlayer] Repeat toggled:", repeat);
    }

    function handleEnded() {
        console.log("[ThinMediaPlayer] Track ended. Repeat:", repeat);
        if (repeat) {
            currentTime = 0;
            audio.play().catch(e => console.error("ThinMediaPlayer repeat failed:", e));
        } else {
            paused = true;
        }
    }

    function formatTime(seconds) {
        if (isNaN(seconds) || !isFinite(seconds)) return "0:00";
        const minutes = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${minutes}:${secs < 10 ? '0' : ''}${secs}`;
    }

    function getFileName(path) {
        return path.split(/[/\\]/).pop() || path;
    }

    function handleClickOutside(event) {
        if (showMenu && menuButton && !menuButton.contains(event.target)) {
            showMenu = false;
        }
    }

    onMount(() => {
        console.log("[ThinMediaPlayer] Mounted.");
        loadAttachments();
        window.addEventListener('click', handleClickOutside);
    });

    onDestroy(() => {
        window.removeEventListener('click', handleClickOutside);
    });

    $: if (itemPath || $project.activeTranscriptPathInDataTab || $project.currentImportedTranscriptPath || ($project.selectedDocumentPath && $project.selectedDocumentPath.toLowerCase().endsWith('.json'))) {
        loadAttachments();
    }
</script>

<div class="flex items-center space-x-2 px-2 flex-grow min-w-0 h-full no-drag pointer-events-auto">
    <audio
        bind:this={audio}
        bind:paused
        bind:currentTime
        bind:duration
        bind:volume
        on:ended={handleEnded}
    ></audio>

    <div class="flex items-center space-x-1 flex-shrink-0">
        <button on:click={rewind10s} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary transition-colors" title="Rewind 10s">
            {@html REWIND_ICON}
        </button>
        <button on:click={togglePlay} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary transition-colors" title={paused ? 'Play' : 'Pause'}>
            {@html !paused ? PAUSE_ICON : PLAY_ICON}
        </button>
        <button on:click={forward10s} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary transition-colors" title="Forward 10s">
            {@html FORWARD_ICON}
        </button>
        <button on:click={toggleRepeat} class="p-1 transition-colors" class:text-blue-500={repeat} class:text-gray-600={!repeat} class:dark:text-text-secondary={!repeat} title="Repeat">
            {@html REPEAT_ICON}
        </button>
    </div>

    <div class="flex items-center space-x-2 flex-grow min-w-0 max-w-[300px]">
        <span class="text-[10px] text-gray-500 font-mono w-8 text-right flex-shrink-0 tabular-nums">{formatTime(currentTime)}</span>
        <input 
            type="range" 
            class="flex-grow h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
            min="0" 
            max={duration || 1} 
            step="0.001"
            bind:value={currentTime}
            disabled={!src}
            on:click|stopPropagation
            on:mousedown|stopPropagation
        >
        <span class="text-[10px] text-gray-500 font-mono w-8 flex-shrink-0 tabular-nums">{formatTime(duration)}</span>
    </div>

    <div class="flex items-center space-x-1 flex-shrink-0">
        <span class="text-gray-600 dark:text-text-secondary">{@html VOLUME_HIGH_ICON}</span>
        <input type="range" class="w-16 h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer" value={volume * 100} on:input={(e) => volume = e.target.value / 100} on:click|stopPropagation on:mousedown|stopPropagation>
    </div>

    <div class="relative flex-shrink-0 h-full flex items-center">
        <button 
            bind:this={menuButton}
            on:click|stopPropagation={() => showMenu = !showMenu}
            class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary transition-colors"
            title="Switch Media"
        >
            {@html THREE_DOTS_ICON}
        </button>

        {#if showMenu && attachments.length > 0}
            <div class="fixed mt-1 w-64 bg-white dark:bg-surface-2 border border-gray-200 dark:border-border shadow-2xl z-[10000] py-1 max-h-60 overflow-y-auto rounded-md" 
                 style="top: 40px; right: 20%;">
                <div class="px-3 py-1 text-[10px] font-bold text-gray-400 uppercase tracking-wider border-b border-gray-100 dark:border-border mb-1">
                    Attached Media
                </div>
                {#each attachments as attachment}
                    <button 
                        on:click|stopPropagation={() => selectTrack(attachment)}
                        class="w-full text-left px-3 py-2 text-xs hover:bg-blue-50 dark:hover:bg-surface-3 transition-colors truncate block"
                        title={attachment}
                    >
                        {getFileName(attachment)}
                    </button>
                {/each}
            </div>
        {/if}
    </div>
</div>

<style>
    .no-drag {
        -webkit-app-region: no-drag;
    }
    input[type='range'] {
        @apply appearance-none bg-transparent;
    }
    input[type='range']::-webkit-slider-runnable-track {
        @apply w-full h-1 bg-gray-300 dark:bg-gray-600 rounded;
    }
    input[type='range']::-webkit-slider-thumb {
        @apply appearance-none h-3 w-3 bg-blue-500 dark:bg-accent-primary rounded-full -mt-1 cursor-pointer shadow-sm;
    }
</style>