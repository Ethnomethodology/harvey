<!-- src/lib/components/projectview/modals/SplitTranscriptModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project, setImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import { basename } from '@tauri-apps/api/path';
    import { normalizePath } from '$lib/services/projectService.js';
    import { invoke } from '@tauri-apps/api/core';
    import { AlertTriangle } from 'lucide-svelte';

    const dispatch = createEventDispatcher();

    let selectedPartnerPath = '';
    let currentFileName = '';
    let transcriptOptions = [];
    
    let currentTranscriptRowCount = 0;
    let partnerTranscriptRowCount = 0;
    let isLoadingCounts = false;

    $: {
        const p = $project;
        const currentPath = p.currentImportedTranscriptPath || p.activeTranscriptPathInDataTab;
        if (currentPath) {
            basename(currentPath).then(name => currentFileName = name);
            
            if (p.currentImportedTranscriptPath) {
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
            } else if (p.activeTranscriptPathInDataTab) {
                // Find associated media file to get its transcripts
                function findMediaFileInTree(nodes, transcriptPath) {
                    if (!Array.isArray(nodes)) return null;
                    for (const node of nodes) {
                        if (node.file_type === 'media' && node.associated_transcripts) {
                            if (node.associated_transcripts.some(t => t.path === transcriptPath)) {
                                return node;
                            }
                        }
                        if (node.children) {
                            const found = findMediaFileInTree(node.children, transcriptPath);
                            if (found) return found;
                        }
                    }
                    return null;
                }

                const mediaFile = findMediaFileInTree(p.files, p.activeTranscriptPathInDataTab);
                if (mediaFile && mediaFile.associated_transcripts) {
                    transcriptOptions = mediaFile.associated_transcripts
                        .filter(t => t.path !== currentPath)
                        .map(t => {
                            let label = t.language_code || 'Original';
                            if (t.name) label += ` (${t.name})`;
                            return {
                                value: t.path,
                                label: label
                            };
                        });
                }
            }
            
            if (transcriptOptions.length > 0 && !selectedPartnerPath) {
                // Pre-select the first option if none selected
                if (!transcriptOptions.some(opt => opt.value === selectedPartnerPath)) {
                    selectedPartnerPath = transcriptOptions[0].value;
                }
            }
        }
    }

    // Reactive row count calculation
    $: if ($project.showSplitTranscriptModal && ($project.currentImportedTranscriptPath || $project.activeTranscriptPathInDataTab) && selectedPartnerPath) {
        calculateRowCounts($project.currentImportedTranscriptPath || $project.activeTranscriptPathInDataTab, selectedPartnerPath);
    }

    async function calculateRowCounts(currentPath, partnerPath) {
        isLoadingCounts = true;
        try {
            // Count current transcript rows (from store if available, else file)
            if ($project.currentImportedTranscriptLexicalJson && $project.currentImportedTranscriptPath === currentPath) {
                currentTranscriptRowCount = countRowsInJson($project.currentImportedTranscriptLexicalJson);
            } else if ($project.currentMediaNoteTranscriptJson && $project.activeTranscriptPathInDataTab === currentPath) {
                currentTranscriptRowCount = countRowsInJson($project.currentMediaNoteTranscriptJson);
            } else {
                const content = await invoke('read_file_content', { path: currentPath });
                currentTranscriptRowCount = countRowsInRaw(content);
            }

            // Count partner transcript rows (always file)
            const partnerContent = await invoke('read_file_content', { path: partnerPath });
            partnerTranscriptRowCount = countRowsInRaw(partnerContent);
        } catch (e) {
            console.error('[SplitTranscriptModal] Error counting rows:', e);
        } finally {
            isLoadingCounts = false;
        }
    }

    function countRowsInJson(jsonString) {
        try {
            const parsed = JSON.parse(jsonString);
            const table = parsed.root.children.find(c => c.type === 'table');
            // Subtract 1 for header row if needed? Standard split transcripts have headers.
            // Let's just compare total children (rows) for structural similarity.
            return table?.children?.length || 0;
        } catch (e) { return 0; }
    }

    function countRowsInRaw(content) {
        if (!content) return 0;
        try {
            const parsed = JSON.parse(content);
            // Handle both Lexical format and raw segment array
            if (parsed.root) {
                const table = parsed.root.children.find(c => c.type === 'table');
                return table?.children?.length || 0;
            } else if (Array.isArray(parsed)) {
                return parsed.length + 1; // +1 for the header we generate on import
            }
            return 0;
        } catch (e) { return 0; }
    }

    function handleConfirm() {
        if (selectedPartnerPath) {
            setImportedTranscriptSplit(
                $project.currentImportedTranscriptPath || $project.activeTranscriptPathInDataTab, 
                selectedPartnerPath, 
                $project.pendingSplitOrientation
            );
            handleClose();
        }
    }

    function handleClose() {
        project.update(p => ({ ...p, showSplitTranscriptModal: false }));
        selectedPartnerPath = '';
        currentTranscriptRowCount = 0;
        partnerTranscriptRowCount = 0;
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
            class="bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            on:click|stopPropagation
        >
            <h2 class="text-lg font-semibold mb-4 text-center">Split Transcript View</h2>

            <div class="space-y-4 mb-6">
                <div class="flex flex-col space-y-1">
                    <span class="text-sm font-medium text-gray-500 dark:text-gray-400">Current Transcript</span>
                    <div class="flex justify-between items-center">
                        <span class="text-md font-semibold truncate flex-grow" title={currentFileName}>{currentFileName}</span>
                        {#if currentTranscriptRowCount > 0}
                            <span class="text-[10px] bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-gray-500">{currentTranscriptRowCount} rows</span>
                        {/if}
                    </div>
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
                    <div class="flex justify-end mt-1">
                        {#if partnerTranscriptRowCount > 0}
                            <span class="text-[10px] bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-gray-500">{partnerTranscriptRowCount} rows</span>
                        {/if}
                    </div>
                    
                    {#if transcriptOptions.length === 0}
                        <p class="text-xs text-orange-500 mt-1">No other imported transcripts available to split with.</p>
                    {/if}
                </div>

                {#if !isLoadingCounts && selectedPartnerPath && currentTranscriptRowCount > 0 && partnerTranscriptRowCount > 0 && currentTranscriptRowCount !== partnerTranscriptRowCount}
                    <div class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800/50 rounded-md flex gap-3 text-amber-800 dark:text-amber-200 transition-all">
                        <AlertTriangle class="w-5 h-5 shrink-0 mt-0.5" />
                        <div class="flex flex-col gap-1">
                            <span class="text-xs font-bold">Row Count Mismatch</span>
                            <p class="text-[11px] leading-relaxed">
                                These transcripts have different row counts ({currentTranscriptRowCount} vs {partnerTranscriptRowCount}). 
                                Scroll synchronization may not align perfectly.
                            </p>
                        </div>
                    </div>
                {/if}
            </div>

            <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                <button class="btn-secondary" on:click={handleClose}>Cancel</button>
                <button 
                    class="btn-primary" 
                    on:click={handleConfirm} 
                    disabled={!selectedPartnerPath || isLoadingCounts}
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
