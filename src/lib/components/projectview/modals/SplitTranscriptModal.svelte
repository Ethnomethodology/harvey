<!-- src/lib/components/projectview/modals/SplitTranscriptModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project, setImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import { basename } from '@tauri-apps/api/path';
    import { normalizePath } from '$lib/services/projectService.js';

    const dispatch = createEventDispatcher();

    let selectedPartnerPath = '';
    let currentFileName = '';
    let transcriptOptions = [];

    $: {
        const p = $project;
        const currentPath = p.currentImportedTranscriptPath;
        if (currentPath) {
            basename(currentPath).then(name => currentFileName = name);
            
            transcriptOptions = p.importedTranscriptFiles
                .map(f => {
                    const fullPath = normalizePath(`${p.baseDirectory}/${f.relativePath || f.relative_path}`);
                    return {
                        path: fullPath,
                        name: f.name || f.relativePath || f.relative_path
                    };
                })
                .filter(f => f.path !== currentPath)
                .map(f => ({
                    value: f.path,
                    label: f.name
                }));
            
            if (transcriptOptions.length > 0 && !selectedPartnerPath) {
                // Pre-select the first option if none selected
                if (!transcriptOptions.some(opt => opt.value === selectedPartnerPath)) {
                    selectedPartnerPath = transcriptOptions[0].value;
                }
            }
        }
    }

    function handleConfirm() {
        if (selectedPartnerPath) {
            setImportedTranscriptSplit($project.currentImportedTranscriptPath, selectedPartnerPath);
            handleClose();
        }
    }

    function handleClose() {
        project.update(p => ({ ...p, showSplitTranscriptModal: false }));
    }

    function handleKeydown(event) {
        if (event.key === 'Escape') {
            handleClose();
        }
    }
</script>

{#if $project.showSplitTranscriptModal}
    <div
        class="fixed inset-0 z-[130] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        on:click={handleClose}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-surface-2 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            on:click|stopPropagation
        >
            <h2 class="text-lg font-semibold mb-4 text-center">Split Transcript View</h2>

            <div class="space-y-4 mb-6">
                <div class="flex flex-col space-y-1">
                    <span class="text-sm font-medium text-gray-500 dark:text-gray-400">Current Transcript</span>
                    <span class="text-md font-semibold truncate" title={currentFileName}>{currentFileName}</span>
                </div>

                <div class="flex flex-col space-y-1">
                    <label for="partnerSelect" class="text-sm font-medium text-gray-500 dark:text-gray-400">Select Partner Transcript</label>
                    <Dropdown
                        containerClasses="w-full"
                        options={transcriptOptions}
                        bind:value={selectedPartnerPath}
                        placeholder="Select a Transcript"
                        disabled={transcriptOptions.length === 0}
                    />
                    {#if transcriptOptions.length === 0}
                        <p class="text-xs text-orange-500 mt-1">No other imported transcripts available to split with.</p>
                    {/if}
                </div>
            </div>

            <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                <button class="btn-secondary" on:click={handleClose}>Cancel</button>
                <button 
                    class="btn-primary" 
                    on:click={handleConfirm} 
                    disabled={!selectedPartnerPath}
                >
                    Split
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
