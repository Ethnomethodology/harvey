<script>
    import { onMount, createEventDispatcher } from 'svelte';
    import { project, clearImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import TranscriptEditorPanel from './TranscriptEditorPanel.svelte';
    import MediaPlayer from '../../shared/MediaPlayer.svelte';

    export let itemPath = null; // Receives the full path from DataView

    const dispatch = createEventDispatcher();

    let primaryPanel;
    let secondaryPanel;
    let cleanupSync = () => {};

    let isScrollSyncEnabled = true;

    // Media Player State
    let mediaPath = null;
    let attachments = [];
    let mediaPlayerRef;
    let isVideoHidden = false;
    let currentTime = 0;
    let isPlaying = false;

    // Row counts for comparison
    let primaryRowCount = 0;
    let secondaryRowCount = 0;

    function handleSyncManager(path, enabled) {
        if (path && enabled) {
            attemptSetupSync();
        } else {
            cleanupSync();
        }
    }

    $: handleSyncManager(splitPartnerPath, isScrollSyncEnabled);

    function toggleScrollSync() {
        isScrollSyncEnabled = !isScrollSyncEnabled;
    }

    function attemptSetupSync() {
        console.log('[ImportedTranscriptView] attemptSetupSync called. splitPartnerPath:', splitPartnerPath);
        cleanupSync();
        let attempts = 0;
        const interval = setInterval(() => {
            attempts++;
            if (primaryPanel && secondaryPanel) {
                const el1 = primaryPanel.getScrollElement();
                const el2 = secondaryPanel.getScrollElement();
                if (el1 && el2) {
                    console.log('[ImportedTranscriptView] Both scroll elements found. Starting sync.');
                    clearInterval(interval);
                    startSync(el1, el2);
                }
            }
            if (attempts > 20) {
                console.warn('[ImportedTranscriptView] Sync setup timed out after 20 attempts.');
                clearInterval(interval);
            }
        }, 100);
    }

    function startSync(el1, el2) {
        let isSyncing = false;
        const onScroll1 = () => {
            if (!isSyncing) {
                isSyncing = true;
                el2.scrollTop = el1.scrollTop;
                // el2.scrollLeft = el1.scrollLeft;
                requestAnimationFrame(() => isSyncing = false);
            }
        };
        const onScroll2 = () => {
            if (!isSyncing) {
                isSyncing = true;
                el1.scrollTop = el2.scrollTop;
                // el1.scrollLeft = el2.scrollLeft;
                requestAnimationFrame(() => isSyncing = false);
            }
        };

        el1.addEventListener('scroll', onScroll1);
        el2.addEventListener('scroll', onScroll2);

        cleanupSync = () => {
            console.log('[ImportedTranscriptView] Cleaning up sync listeners.');
            el1.removeEventListener('scroll', onScroll1);
            el2.removeEventListener('scroll', onScroll2);
            cleanupSync = () => {};
        };
    }

    $: splitInfo = $project.importedTranscriptSplits[itemPath];
    $: splitPartnerPath = splitInfo?.partner;
    $: orientation = splitInfo?.orientation || 'horizontal';

    function forwardEvent(event) {
        console.log(`[ImportedTranscriptView] Forwarding event: ${event.type}`);
		dispatch(event.type, event.detail);
	}

    export function playMedia(path) {
        if (path) {
            mediaPath = path;
        }
    }

    async function loadAttachments(path) {
        const projectStoreState = get(project);
        if (!projectStoreState.id || !path) {
            attachments = [];
            mediaPath = null;
            return;
        }

        // Derived relative path
        let assetRelativePath = path.startsWith(projectStoreState.baseDirectory) 
            ? path.substring(projectStoreState.baseDirectory.length) 
            : path;
        assetRelativePath = assetRelativePath.replace(/\\/g, '/').replace(/^\//, '');

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
                    console.log("[ImportedTranscriptView] Loaded attachments:", attachments);
                    // Do not auto-load mediaPath. Wait for user request.
                    // if (attachments.length > 0) {
                    //    mediaPath = convertFileSrc(attachments[0]);
                    // } else {
                    //    mediaPath = null;
                    // }
                    // Reset mediaPath if it was previously set but now invalid? 
                    // Or just leave it? Let's reset if path changed.
                    // Actually, if we switch transcripts, we want to reset.
                    // The reactive statement `$: if (itemPath)` calls this. 
                    // We should probably reset mediaPath at start of loadAttachments or when itemPath changes.
                } else {
                    attachments = [];
                    // mediaPath = null;
                }
            } else {
                attachments = [];
                // mediaPath = null;
            }
        } catch (error) {
            console.error(`[ImportedTranscriptView] Error loading attachments:`, error);
            attachments = [];
            mediaPath = null;
        }
    }

    function handlePlaySegment(event) {
        if (mediaPlayerRef) {
            mediaPlayerRef.playSegment(event.detail.startTime, event.detail.endTime);
        }
    }

    function handlePrimaryRowCount(event) {
        primaryRowCount = event.detail.rowCount;
    }

    function handleSecondaryRowCount(event) {
        secondaryRowCount = event.detail.rowCount;
    }

    // Reset mediaPath and counts when itemPath changes
    $: if (itemPath) {
        mediaPath = null;
        primaryRowCount = 0;
        secondaryRowCount = 0;
        loadAttachments(itemPath);
    }

    onMount(() => {
		console.log('[ImportedTranscriptView] Component container mounted. Transcript path:', itemPath);
        if (itemPath) loadAttachments(itemPath);
	});

</script>

<!-- Main container for the Imported Transcript View -->
<div class="h-full flex flex-col w-full bg-white dark:bg-surface-2 overflow-hidden imported-transcript-view">
    {#if mediaPath}
        <div class="border-b border-gray-200 dark:border-border flex flex-col {!isVideoHidden ? 'h-1/2' : 'h-auto flex-shrink-0'}">
            <MediaPlayer
                bind:this={mediaPlayerRef}
                bind:isVideoMinimized={isVideoHidden}
                bind:localCurrentTime={currentTime}
                bind:localIsPlaying={isPlaying}
                explicitMediaPath={mediaPath}
                autoPlay={true}
                projectId={$project.id}
                showLoopPauseButton={false}
                showDataTranscribeButton={false}
                showDataTrimButton={false} 
                class="{!isVideoHidden ? 'flex-grow min-h-0' : ''}"
            />
        </div>
    {/if}

    {#if splitPartnerPath && primaryRowCount > 0 && secondaryRowCount > 0 && primaryRowCount !== secondaryRowCount}
        <div class="bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800/50 px-4 py-2 flex items-center gap-2 text-amber-800 dark:text-amber-200 text-xs shrink-0">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                <path fill-rule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625l6.28-10.875zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd" />
            </svg>
            <span><strong>Row Count Mismatch:</strong> The primary transcript has {primaryRowCount} rows, while the partner transcript has {secondaryRowCount} rows. Scroll sync may be inaccurate.</span>
        </div>
    {/if}

    <div class="flex-grow min-h-0 overflow-hidden {mediaPath && !isVideoHidden ? 'h-1/2' : 'h-full'}">
        {#if splitPartnerPath}
            <div class="flex h-full w-full divide-gray-300 dark:divide-gray-600 {orientation === 'horizontal' ? 'flex-row divide-x' : 'flex-col divide-y'}">
                <div class="{orientation === 'horizontal' ? 'w-1/2 h-full' : 'h-1/2 w-full'} overflow-hidden flex flex-col">
                    <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex items-center h-8">
                        <span class="truncate">{itemPath.split(/[\\/]/).pop()}</span>
                    </div>
                    <div class="flex-grow overflow-hidden">
                        {#key itemPath}
                            <TranscriptEditorPanel 
                                bind:this={primaryPanel} 
                                itemPath={itemPath} 
                                isPrimary={true} 
                                enableSegmentPlayback={!!mediaPath}
                                on:playsegment={handlePlaySegment}
                                on:rowcountupdated={handlePrimaryRowCount}
                            />
                        {/key}
                    </div>
                </div>
                <div class="{orientation === 'horizontal' ? 'w-1/2 h-full' : 'h-1/2 w-full'} overflow-hidden flex flex-col">
                    <div class="bg-gray-100 dark:bg-surface-3 px-2 py-1 text-xs font-semibold text-gray-600 dark:text-gray-400 border-b border-gray-300 dark:border-gray-600 flex justify-between items-center h-8">
                        <div class="flex items-center min-w-0 flex-grow">
                            <span class="truncate">{splitPartnerPath.split(/[\\/]/).pop()}</span>
                        </div>
                        <button 
                            class="ml-2 flex-shrink-0" 
                            class:text-black={!isScrollSyncEnabled} 
                            class:text-blue-500={isScrollSyncEnabled}
                            class:dark:text-gray-400={!isScrollSyncEnabled} 
                            title={isScrollSyncEnabled ? "Disable Scroll Sync" : "Enable Scroll Sync"}
                            on:click={toggleScrollSync}
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-down-up" viewBox="0 0 16 16">
                                <path fill-rule="evenodd" d="M11.5 15a.5.5 0 0 0 .5-.5V2.707l3.146 3.147a.5.5 0 0 0 .708-.708l-4-4a.5.5 0 0 0-.708 0l-4 4a.5.5 0 1 0 .708.708L11 2.707V14.5a.5.5 0 0 0 .5.5m-7-14a.5.5 0 0 1 .5.5v11.793l3.146-3.147a.5.5 0 0 1 .708.708l-4 4a.5.5 0 0 1-.708 0l-4-4a.5.5 0 0 1 .708-.708L4 13.293V1.5a.5.5 0 0 1 .5-.5"/>
                            </svg>
                        </button>
                        <button 
                            class="hover:text-red-500 ml-2 flex-shrink-0" 
                            title="Close Split"
                            on:click={() => clearImportedTranscriptSplit(itemPath)}
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-lg" viewBox="0 0 16 16">
                                <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z"/>
                            </svg>
                        </button>
                    </div>
                    <div class="flex-grow overflow-hidden">
                        {#key splitPartnerPath}
                            <TranscriptEditorPanel 
                                bind:this={secondaryPanel} 
                                itemPath={splitPartnerPath} 
                                isPrimary={false} 
                                enableSegmentPlayback={!!mediaPath}
                                on:playsegment={handlePlaySegment}
                                on:rowcountupdated={handleSecondaryRowCount}
                            />
                        {/key}
                    </div>
                </div>
            </div>
        {:else}
            {#key itemPath}
                {#if itemPath}
                    <div class="h-full flex flex-col">
                        <div class="flex-grow overflow-hidden">
                            <TranscriptEditorPanel 
                                bind:this={primaryPanel} 
                                itemPath={itemPath} 
                                isPrimary={true} 
                                enableSegmentPlayback={!!mediaPath}
                                on:playsegment={handlePlaySegment}
                                on:rowcountupdated={handlePrimaryRowCount}
                            />
                        </div>
                    </div>
                {:else}
                    <div class="h-full bg-gray-200 dark:bg-d-gray-700 flex items-center justify-center text-gray-500">
                        <span>No transcript path provided to ImportedTranscriptView.</span>
                    </div>
                {/if}
            {/key}
        {/if}
    </div>
</div>

<style>
	.min-h-0 { min-height: 0; }
    /* Removed specific width classes as this component now fills the space given by DataView */
</style>