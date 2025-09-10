<script>
    import { onMount } from 'svelte';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { basename, extname as getFileExtname, sep as getPathSep, resolve } from '@tauri-apps/api/path';
    import MinimalMediaPlayer from './MinimalMediaPlayer.svelte';

    export let itemPath = null;
    export let itemType = null;

    let attachments = [];
    let isLoading = true;
    let previousProcessedItemPath = null;
    let currentTrackIndex = -1;
    let currentSrc = null;

    const MUSIC_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-file-earmark-music" viewBox="0 0 16 16"><path d="M11 6.64a1 1 0 0 0-1.243-.97l-1 .25A1 1 0 0 0 8 6.89v4.306A2.6 2.6 0 0 0 7 11c-.5 0-.974.134-1.338.377-.36.24-.662.628-.662 1.123s.301.883.662 1.123c.364.243.839.377 1.338.377s.974-.134 1.338-.377c.36-.24.662.628.662-1.123V8.89l2-.5z"/><path d="M14 14V4.5L9.5 0H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2M9.5 3A1.5 1.5 0 0 0 11 4.5h2V14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1h5.5z"/></svg>`;
    const PLAY_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-play-circle-fill" viewBox="0 0 16 16"><path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0M6.79 5.093A.5.5 0 0 0 6 5.5v5a.5.5 0 0 0 .79.407l3.5-2.5a.5.5 0 0 0 0-.814z"/></svg>`;

    function getFileName(path) {
        return path.split(/[\/\\]/).pop() || path;
    }

    function playTrack(index) {
        if (index >= 0 && index < attachments.length) {
            currentTrackIndex = index;
            currentSrc = convertFileSrc(attachments[index]);
        }
    }

    function playNext() {
        const nextIndex = (currentTrackIndex + 1) % attachments.length;
        playTrack(nextIndex);
    }

    function playPrevious() {
        const prevIndex = (currentTrackIndex - 1 + attachments.length) % attachments.length;
        playTrack(prevIndex);
    }

    async function getOriginalAssetDetails(selectedPath, projectStoreState) {
        if (!selectedPath || !projectStoreState || !projectStoreState.baseDirectory) return null;
        let originalRelativePath = selectedPath.startsWith(projectStoreState.baseDirectory) ? selectedPath.substring(projectStoreState.baseDirectory.length) : selectedPath;
        originalRelativePath = originalRelativePath.replace(/\\/g, '/').replace(/^\//, '');
        return { originalRelativePath };
    }

    async function loadAttachments(assetRelativePathToLoad) {
        isLoading = true;
        attachments = [];
        currentSrc = null;
        currentTrackIndex = -1;
        const projectStoreState = get(project);
        if (!projectStoreState.id || !assetRelativePathToLoad) {
            isLoading = false;
            return;
        }

        try {
            const result = await invoke('get_asset_metadata_command', {
                projectId: projectStoreState.id,
                assetRelativePath: assetRelativePathToLoad
            });

            if (result && result.custom_fields_json) {
                const customFields = JSON.parse(result.custom_fields_json);
                const attachmentsField = customFields.find(f => f.key === 'attachments');
                if (attachmentsField && attachmentsField.value) {
                    attachments = JSON.parse(attachmentsField.value);
                }
            }
        } catch (error) {
            console.error(`[AttachmentsPanel] Error loading metadata for ${assetRelativePathToLoad}:`, error);
        } finally {
            isLoading = false;
            previousProcessedItemPath = assetRelativePathToLoad;
        }
    }

    $: {
        (async () => {
            const currentProjectStoreState = get(project);
            if (itemPath && itemType === 'doc' && currentProjectStoreState?.baseDirectory) {
                const newOriginalDetails = await getOriginalAssetDetails(itemPath, currentProjectStoreState);
                const newDerivedRelativePath = newOriginalDetails?.originalRelativePath;

                if (newDerivedRelativePath && newDerivedRelativePath !== previousProcessedItemPath) {
                    await loadAttachments(newDerivedRelativePath);
                } else if (!newDerivedRelativePath) {
                    attachments = [];
                    previousProcessedItemPath = null;
                    currentSrc = null;
                    currentTrackIndex = -1;
                }
            } else {
                attachments = [];
                previousProcessedItemPath = null;
                currentSrc = null;
                currentTrackIndex = -1;
            }
        })();
    }
</script>

<div class="h-full bg-white dark:bg-gray-800 flex flex-col overflow-hidden">
    <div class="p-2">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 border-b border-gray-200 dark:border-gray-600 pb-2 mb-2">
            Attachments
        </h3>
    </div>
    <div class="flex-grow overflow-y-auto min-h-0">
        {#if isLoading}
            <p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-4">Loading...</p>
        {:else if attachments.length > 0}
            <ul class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each attachments as attachment, i (attachment)}
                    <li
                        class="p-2 flex items-center justify-between group cursor-pointer"
                        class:bg-blue-100={currentTrackIndex === i}
                        class:dark:bg-blue-800={currentTrackIndex === i}
                        on:click={() => playTrack(i)}
                    >
                        <div class="flex items-center space-x-3 truncate">
                            <span class="text-gray-400">{@html MUSIC_ICON_SVG}</span>
                            <span class="text-sm text-gray-800 dark:text-gray-200 truncate" title={attachment}>
                                {getFileName(attachment)}
                            </span>
                        </div>
                        <button class="text-gray-500 dark:text-gray-400 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity" title="Play" on:click|stopPropagation={() => playTrack(i)}>
                            {@html PLAY_ICON_SVG}
                        </button>
                    </li>
                {/each}
            </ul>
        {:else}
            <p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-4">
                No attachments found for this document.
            </p>
        {/if}
    </div>
    <div class="flex-shrink-0">
        <MinimalMediaPlayer
            src={currentSrc}
            on:ended={playNext}
            on:next={playNext}
            on:previous={playPrevious}
        />
    </div>
</div>
