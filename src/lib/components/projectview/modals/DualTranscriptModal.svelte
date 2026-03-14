<!-- src/lib/components/projectview/modals/DualTranscriptModal.svelte -->
<script>
    import { onMount } from 'svelte';
    import { transcriptStore, activateDualMode, setDualTranscriptModal } from '$lib/stores/transcriptStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';
    import { 
        Button, 
        Label, 
        Select, 
        Badge,
        Alert
    } from 'flowbite-svelte';
    import { AlertTriangle, Rows2, X, Info } from 'lucide-svelte';

    let primaryPath = '';
    let secondaryPath = '';
    
    let primarySegmentCount = 0;
    let secondarySegmentCount = 0;
    let isLoadingCounts = false;

    $: transcriptOptions = ($transcriptStore.selectedMediaFile?.associated_transcripts || []).map(t => {
        let name = t.language_code || 'Original';
        if (t.name) name += ` (${t.name})`;
        // IMPORTANT: Use absolute path (t.path) for loading, but keep relativePath for matching
        return {
            value: t.path,
            name: name,
            relPath: t.relativePath
        };
    });

    function getBaseName(path) {
        if (!path) return '';
        return path.split(/[\\/]/).pop();
    }

    // Initialize paths from store if possible
    function initializeSelections() {
        const store = $transcriptStore;
        
        // currentTranscriptPath in store is usually relative
        const currentPath = store.activeTranscript?.path || store.currentTranscriptPath;
        console.log('[DualTranscriptModal] Initializing selections. currentPath from store:', currentPath);
        
        if (currentPath && transcriptOptions.length > 0) {
            // 1. Try matching against relPath (since store path is usually relative)
            let match = transcriptOptions.find(o => o.relPath === currentPath);
            
            // 2. Try exact match against value (absolute path)
            if (!match) {
                match = transcriptOptions.find(o => o.value === currentPath);
            }

            // 3. Try matching by filename (base name) as final fallback
            if (!match) {
                const currentBase = getBaseName(currentPath);
                match = transcriptOptions.find(o => getBaseName(o.value) === currentBase);
                if (match) console.log('[DualTranscriptModal] Matched by base name:', currentBase);
            }

            if (match) {
                primaryPath = match.value;
                console.log('[DualTranscriptModal] Set primaryPath to:', primaryPath);
            } else {
                console.log('[DualTranscriptModal] No match found for currentPath. Defaulting to first option.');
                primaryPath = transcriptOptions[0].value;
            }
        } else if (transcriptOptions.length > 0) {
            primaryPath = transcriptOptions[0].value;
        }

        // Auto-select secondary if not set or if it matches primary
        if (!secondaryPath || secondaryPath === primaryPath) {
            // Match secondaryTranscriptPath (might be absolute or relative)
            const sPath = store.secondaryTranscriptPath;
            const sMatch = sPath ? transcriptOptions.find(o => o.value === sPath || o.relPath === sPath) : null;

            if (sMatch && sMatch.value !== primaryPath) {
                secondaryPath = sMatch.value;
            } else if (transcriptOptions.length > 1) {
                const other = transcriptOptions.find(o => o.value !== primaryPath);
                if (other) {
                    secondaryPath = other.value;
                }
            }
        }
        console.log('[DualTranscriptModal] Final selections -> Primary:', primaryPath, 'Secondary:', secondaryPath);
    }

    onMount(() => {
        if ($transcriptStore.showDualTranscriptModal) {
            initializeSelections();
        }
    });

    // Reactive selection if modal is opened/re-opened
    let lastModalState = false;
    $: if ($transcriptStore.showDualTranscriptModal && !lastModalState) {
        initializeSelections();
        lastModalState = true;
    } else if (!$transcriptStore.showDualTranscriptModal) {
        lastModalState = false;
    }

    $: if (primaryPath || secondaryPath) {
        updateSegmentCounts(primaryPath, secondaryPath);
    }

    async function getSegmentCount(path) {
        if (!path) return 0;
        try {
            const jsonString = await invoke('load_transcript_json', { transcriptPath: path });
            const data = JSON.parse(jsonString);
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
            return;
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
        class="fixed inset-0 z-[130] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dual-transcript-modal-title"
        on:click={handleClose}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
            on:click|stopPropagation
            role="document"
        >
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        <Rows2 size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="dual-transcript-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        Dual Transcript Mode
                    </h3>
                </div>
                <button on:click={handleClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

            <div class="p-6 space-y-5">
                <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800/50 p-3 rounded-lg flex gap-3">
                    <Info size={18} class="text-blue-600 dark:text-blue-400 shrink-0 mt-0.5" />
                    <p class="text-xs text-blue-800 dark:text-blue-300 leading-relaxed">
                        Compare two transcripts side-by-side. Transcripts must have an identical number of segments to be compatible.
                    </p>
                </div>

                <div class="space-y-4">
                    <div class="space-y-2">
                        <div class="flex justify-between items-center">
                            <Label for="primarySelect">Primary Transcript</Label>
                            {#if primarySegmentCount > 0}
                                <Badge color="blue" size="xs" class="font-mono">{primarySegmentCount} segments</Badge>
                            {/if}
                        </div>
                        <Select
                            id="primarySelect"
                            items={transcriptOptions}
                            bind:value={primaryPath}
                            placeholder="Select Primary Transcript"
                        />
                    </div>

                    <div class="space-y-2">
                        <div class="flex justify-between items-center">
                            <Label for="secondarySelect">Secondary Transcript</Label>
                            {#if secondarySegmentCount > 0}
                                <Badge color="indigo" size="xs" class="font-mono">{secondarySegmentCount} segments</Badge>
                            {/if}
                        </div>
                        <Select
                            id="secondarySelect"
                            items={transcriptOptions}
                            bind:value={secondaryPath}
                            placeholder="Select Secondary Transcript"
                        />
                    </div>
                </div>

                {#if hasMismatch}
                    <Alert color="red" class="mt-4">
                        <div class="flex items-start gap-3">
                            <AlertTriangle size={18} class="shrink-0 mt-0.5" />
                            <div class="space-y-1">
                                <span class="text-xs font-bold">Segment Mismatch</span>
                                <p class="text-[11px] leading-relaxed">
                                    Transcripts must have identical segment counts ({primarySegmentCount} vs {secondarySegmentCount}).
                                </p>
                            </div>
                        </div>
                    </Alert>
                {:else if isSame && primaryPath}
                    <div class="text-center p-2">
                        <p class="text-xs text-orange-500 font-medium italic">Please select two different transcripts.</p>
                    </div>
                {/if}
            </div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
                <Button color="alternative" on:click={handleClose} title="Cancel">
                    Cancel
                </Button>
                <Button 
                    color="blue" 
                    on:click={handleView} 
                    disabled={!canView}
                    title={!canView ? 'Transcripts are not compatible' : 'Activate dual view'}
                >
                    Enable Dual View
                </Button>
            </div>
        </div>
    </div>
{/if}

<style lang="postcss">
</style>