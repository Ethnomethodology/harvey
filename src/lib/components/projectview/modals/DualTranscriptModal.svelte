<!-- src/lib/components/projectview/modals/DualTranscriptModal.svelte -->
<script>
    import { onMount } from 'svelte';
    import { transcriptStore, activateDualMode, setDualTranscriptModal } from '$lib/stores/transcriptStore.js';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';

    let primaryPath = '';
    let secondaryPath = '';
    
    let primarySegmentCount = 0;
    let secondarySegmentCount = 0;
    let isLoadingCounts = false;

    $: transcriptOptions = ($transcriptStore.selectedMediaFile?.associated_transcripts || []).map(t => {
        let label = t.language_code || 'Original';
        if (t.name) label += ` (${t.name})`;
        return {
            value: t.path,
            label: label
        };
    });

    // Initialize paths from store if possible
    onMount(() => {
        const store = $transcriptStore;
        primaryPath = store.currentTranscriptPath || '';
        secondaryPath = store.secondaryTranscriptPath || '';
        
        if (!primaryPath && transcriptOptions.length > 0) {
            primaryPath = transcriptOptions[0].value;
        }
        if (!secondaryPath && transcriptOptions.length > 1) {
            secondaryPath = transcriptOptions[1].value;
        }
    });

    $: if (primaryPath || secondaryPath) {
        updateSegmentCounts(primaryPath, secondaryPath);
    }

    async function getSegmentCount(path) {
        if (!path) return 0;
        try {
            const jsonString = await invoke('load_transcript_json', { transcriptPath: path });
            const data = JSON.parse(jsonString);
            // Lexical table format: root -> children -> table -> children (rows)
            const table = data.root.children.find(c => c.type === 'table');
            return table?.children?.length || 0;
        } catch (e) {
            console.error(`[DualTranscriptModal] Error counting segments for ${path}:`, e);
            return 0;
        }
    }

    async function updateSegmentCounts(pPath, sPath) {
        isLoadingCounts = true;
        const [pCount, sCount] = await Promise.all([
            getSegmentCount(pPath),
            getSegmentCount(sPath)
        ]);
        primarySegmentCount = pCount;
        secondarySegmentCount = sCount;
        isLoadingCounts = false;
    }

    async function handleView() {
        if (primaryPath === secondaryPath) {
            message('Please select two different transcripts.', { title: 'Invalid Selection', type: 'warning' });
            return;
        }
        if (primarySegmentCount !== secondarySegmentCount) {
            return; // Should be disabled anyway
        }

        await activateDualMode(primaryPath, secondaryPath);
    }

    function handleClose() {
        setDualTranscriptModal(false);
    }

    function handleKeydown(event) {
        if (event.key === 'Escape') {
            handleClose();
        }
    }

    $: hasMismatch = !isLoadingCounts && primaryPath && secondaryPath && primaryPath !== secondaryPath && primarySegmentCount !== secondarySegmentCount;
    $: isSame = primaryPath && secondaryPath && primaryPath === secondaryPath;
    $: canView = !isLoadingCounts && primaryPath && secondaryPath && !isSame && !hasMismatch;
</script>

{#if $transcriptStore.showDualTranscriptModal}
    <div
        class="fixed inset-0 z-[130] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        on:click={handleClose}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            on:click|stopPropagation
        >
            <h2 class="text-lg font-semibold mb-4 text-center">Dual Transcript Mode</h2>

            <div class="space-y-4 mb-6">
                <div class="flex flex-col space-y-1">
                    <label for="primarySelect" class="text-sm font-medium text-gray-500 dark:text-gray-400">Primary Transcript</label>
                    <Dropdown
                        containerClasses="w-full"
                        options={transcriptOptions}
                        bind:value={primaryPath}
                        placeholder="Select Primary Transcript"
                    />
                    {#if primarySegmentCount > 0}
                        <div class="flex justify-end mt-1">
                            <span class="text-[10px] bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-gray-500">{primarySegmentCount} segments</span>
                        </div>
                    {/if}
                </div>

                <div class="flex flex-col space-y-1">
                    <label for="secondarySelect" class="text-sm font-medium text-gray-500 dark:text-gray-400">Secondary Transcript</label>
                    <Dropdown
                        containerClasses="w-full"
                        options={transcriptOptions}
                        bind:value={secondaryPath}
                        placeholder="Select Secondary Transcript"
                    />
                    {#if secondarySegmentCount > 0}
                        <div class="flex justify-end mt-1">
                            <span class="text-[10px] bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-gray-500">{secondarySegmentCount} segments</span>
                        </div>
                    {/if}
                </div>

                {#if hasMismatch}
                    <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/50 rounded-md flex gap-3 text-red-800 dark:text-red-200 transition-all">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 shrink-0 mt-0.5">
                            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z" clip-rule="evenodd" />
                        </svg>
                        <div class="flex flex-col gap-1">
                            <span class="text-xs font-bold">Segment Mismatch</span>
                            <p class="text-[11px] leading-relaxed">
                                The number of segments must match between the two transcripts ({primarySegmentCount} vs {secondarySegmentCount}).
                            </p>
                        </div>
                    </div>
                {:else if isSame && primaryPath}
                    <p class="text-xs text-orange-500 text-center">Please select two different transcripts.</p>
                {/if}
            </div>

            <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                <button class="btn-secondary" on:click={handleClose}>Cancel</button>
                <button 
                    class="btn-primary" 
                    on:click={handleView} 
                    disabled={!canView}
                >
                    View
                </button>
            </div>
        </div>
    </div>
{/if}

<style lang="postcss">
    .btn-primary {
        @apply px-4 py-2 rounded-md font-medium transition-colors bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed;
    }
    .btn-secondary {
        @apply px-4 py-2 rounded-md font-medium transition-colors bg-gray-200 text-gray-800 dark:bg-gray-700 dark:text-gray-200 hover:bg-gray-300 dark:hover:bg-gray-600;
    }
</style>
