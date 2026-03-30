<!-- src/lib/components/projectview/modals/DualTranscriptModal.svelte -->
<script>
    import { onMount } from 'svelte';
    import { transcriptStore, activateDualMode, setDualTranscriptModal } from '$lib/stores/transcriptStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';
    import { 
		Modal,
        Button, 
        Label, 
        Select, 
        Badge,
        Alert
    } from 'flowbite-svelte';
    import { AlertTriangle, Rows2, Info } from '@lucide/svelte';

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

    $: hasMismatch = !isLoadingCounts && primaryPath && secondaryPath && primaryPath !== secondaryPath && primarySegmentCount !== secondarySegmentCount;
    $: isSame = primaryPath && secondaryPath && primaryPath === secondaryPath;
    $: canView = !isLoadingCounts && primaryPath && secondaryPath && !isSame && !hasMismatch;
</script>

<Modal
	bind:open={$transcriptStore.showDualTranscriptModal}
	size="md"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={handleClose}
>
	<div slot="header" class="flex items-center gap-2">
		<Rows2 class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			Compare Transcripts
		</h3>
	</div>

	<div class="space-y-5">
		<div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800/50 p-3 rounded-lg flex gap-3">
			<Info size={18} class="text-blue-600 dark:text-blue-400 shrink-0 mt-0.5" />
			<p class="text-xs text-blue-800 dark:text-blue-300 leading-relaxed">
				Compare two transcripts in interleaved mode. Transcripts must have an identical number of segments to be compatible.
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

	<svelte:fragment slot="footer">
		<Button color="alternative" on:click={handleClose} title="Cancel">
			Cancel
		</Button>
		<Button 
			color="blue" 
			on:click={handleView} 
			disabled={!canView}
			title={!canView ? 'Transcripts are not compatible' : 'Activate dual view'}
		>
			Compare
		</Button>
	</svelte:fragment>
</Modal>