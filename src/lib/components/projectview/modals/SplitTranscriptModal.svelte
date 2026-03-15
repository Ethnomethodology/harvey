<!-- src/lib/components/projectview/modals/SplitTranscriptModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project, setImportedTranscriptSplit } from '$lib/stores/projectStore.js';
    import { 
        Button, 
        Label, 
        Select, 
        Helper,
        Badge,
        Alert
    } from 'flowbite-svelte';
    import { basename } from '@tauri-apps/api/path';
    import { normalizePath } from '$lib/services/projectService.js';
    import { invoke } from '@tauri-apps/api/core';
    import { AlertTriangle, Split, X, FileText, SquareSplitHorizontal, SquareSplitVertical } from 'lucide-svelte';

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
                        name: f.name
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
                                name: label
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
            return table?.children?.length || 0;
        } catch (e) { return 0; }
    }

    function countRowsInRaw(content) {
        if (!content) return 0;
        try {
            const parsed = JSON.parse(content);
            if (parsed.root) {
                const table = parsed.root.children.find(c => c.type === 'table');
                return table?.children?.length || 0;
            } else if (Array.isArray(parsed)) {
                return parsed.length + 1;
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
        class="fixed inset-0 z-[130] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
        role="dialog"
        aria-modal="true"
        aria-labelledby="split-transcript-modal-title"
        on:click={handleClose}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
            on:click|stopPropagation
        >
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        {#if $project.pendingSplitOrientation === 'vertical'}
                            <SquareSplitVertical size={20} class="text-blue-600 dark:text-blue-400" />
                        {:else}
                            <SquareSplitHorizontal size={20} class="text-blue-600 dark:text-blue-400" />
                        {/if}
                    </div>
                    <h3 id="split-transcript-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        Split Transcript View
                    </h3>
                </div>
                <button on:click={handleClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

            <!-- Body -->
            <div class="p-6 space-y-6">
                <div class="space-y-2">
                    <Label class="text-gray-500 dark:text-gray-400">Current Transcript</Label>
                    <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700 rounded-lg">
                        <div class="flex items-center gap-2 overflow-hidden">
                            <FileText size={16} class="text-gray-400 shrink-0" />
                            <span class="text-sm font-semibold truncate text-gray-700 dark:text-gray-200" title={currentFileName}>
                                {currentFileName}
                            </span>
                        </div>
                        {#if currentTranscriptRowCount > 0}
                            <Badge color="dark" rounded class="px-2 py-0.5 text-[10px] shrink-0 whitespace-nowrap">
                                {currentTranscriptRowCount} rows
                            </Badge>
                        {/if}
                    </div>
                </div>

                <div class="space-y-2">
                    <Label for="partnerSelect">Select Partner Transcript</Label>
                    <Select
                        id="partnerSelect"
                        items={transcriptOptions}
                        bind:value={selectedPartnerPath}
                        disabled={transcriptOptions.length === 0}
                        placeholder="Choose a transcript to compare..."
                    />
                    <div class="flex justify-between items-center mt-1">
                        {#if transcriptOptions.length === 0}
                            <Helper color="orange" class="italic">
                                No other transcripts available to split with.
                            </Helper>
                        {:else}
                            <Helper>Choose the second transcript for side-by-side view.</Helper>
                        {/if}
                        
                        {#if partnerTranscriptRowCount > 0}
                            <Badge color="dark" rounded class="px-2 py-0.5 text-[10px]">
                                {partnerTranscriptRowCount} rows
                            </Badge>
                        {/if}
                    </div>
                </div>

                {#if !isLoadingCounts && selectedPartnerPath && currentTranscriptRowCount > 0 && partnerTranscriptRowCount > 0 && currentTranscriptRowCount !== partnerTranscriptRowCount}
                    <Alert color="yellow" class="items-start">
                        <AlertTriangle slot="icon" class="w-5 h-5 shrink-0" />
                        <div class="flex flex-col gap-1 ml-2">
                            <span class="text-xs font-bold uppercase tracking-wider">Row Count Mismatch</span>
                            <p class="text-[11px] leading-relaxed">
                                These transcripts have different row counts ({currentTranscriptRowCount} vs {partnerTranscriptRowCount}). 
                                Scroll synchronization may not align perfectly.
                            </p>
                        </div>
                    </Alert>
                {/if}
            </div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
                <Button color="alternative" on:click={handleClose} title="Cancel and close">
                    Cancel
                </Button>
                <Button 
                    color="blue" 
                    on:click={handleConfirm} 
                    disabled={!selectedPartnerPath || isLoadingCounts}
                    title={!selectedPartnerPath ? "Please select a partner transcript" : "Open split view"}
                >
                    {#if isLoadingCounts}
                        Checking...
                    {:else}
                        <Split size={18} class="mr-2" />
                        Split View
                    {/if}
                </Button>
            </div>
        </div>
    </div>
{/if}

<style lang="postcss">
</style>
