<!-- src/lib/components/projectview/modals/ImageExportModal.svelte -->
<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
    import { 
		Modal,
        Input, 
        Label, 
        Select, 
        Button, 
        Helper,
        Checkbox
    } from 'flowbite-svelte';
    import { Share, FolderOpen, X, Image } from '@lucide/svelte';

    export let showModal = false;
    export let defaultFileName = 'export.png';
    export let exportTypeLabel = 'Image';
    export let showAnnotations = true;

    const dispatch = createEventDispatcher();

    // Local state
    let exportFileName = '';
    let exportFormat = 'png';
    let exportDirectory = '';
    let includeAnnotations = false;
    let modalTitle = `Export ${exportTypeLabel}`;

    const exportFormats = [
        { value: 'png', name: 'PNG (.png)', disabled: false },
        { value: 'jpg', name: 'JPEG (.jpg)', disabled: false }
    ];

    const PATH_SEPARATOR = '/';

    async function initializeModalState() {
        if (defaultFileName) {
            // Normalize path separators to forward slash without regex to avoid parser issues
            const normalized = defaultFileName.split('\\').join('/');
            const filenameWithExt = normalized.split('/').pop();
            const parts = filenameWithExt.split('.');
            
            if (parts.length > 1) {
                exportFormat = parts.pop().toLowerCase();
                if (exportFormat === 'jpeg') exportFormat = 'jpg';
                exportFileName = parts.join('.');
            } else {
                exportFileName = parts[0] || 'export';
                exportFormat = 'png';
            }
        } else {
            exportFileName = 'export';
            exportFormat = 'png';
        }

        if (!exportDirectory) {
            try {
                exportDirectory = await documentDir();
            } catch (e) {
                console.warn('[ImageExportModal] Failed to get document directory:', e);
            }
        }

        modalTitle = `Export ${exportTypeLabel}: ${exportFileName}.${exportFormat}`;
    }

    $: if (showModal) {
        initializeModalState();
    }

    async function selectExportDirectory() {
        try {
            const selectedPath = await open({
                directory: true,
                defaultPath: exportDirectory || undefined,
                title: 'Select Export Directory',
            });

            if (selectedPath && typeof selectedPath === 'string') {
                exportDirectory = selectedPath;
            }
        } catch (error) {
            console.error('[ImageExportModal] Error selecting export directory:', error);
        }
    }

    function handleConfirm() {
        if (!exportFileName || exportFileName.trim() === '') {
            alert('Please enter a filename.');
            return;
        }
        if (!exportDirectory || exportDirectory.trim() === '') {
            alert('Please select an export directory.');
            return;
        }

        // Robust path normalization using split/join instead of regex
        const normalizedDir = exportDirectory.split('\\').join(PATH_SEPARATOR);
        
        const dir = normalizedDir.endsWith(PATH_SEPARATOR) 
            ? normalizedDir.slice(0, -1) 
            : normalizedDir;
        
        const fullExportPath = dir + PATH_SEPARATOR + exportFileName + '.' + exportFormat;

        dispatch('export', {
            filePath: fullExportPath,
            includeAnnotations
        });
        closeModal();
    }

    function closeModal() {
        showModal = false;
        dispatch('close');
    }
</script>

<Modal
	bind:open={showModal}
	size="sm"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={closeModal}
>
	<div slot="header" class="flex items-center gap-2">
		<Share class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
			{`Export ${exportTypeLabel}`}
		</h3>
	</div>

	<div class="space-y-5">
		<!-- Filename Input -->
		<div class="space-y-2">
			<Label for="export-filename">Filename</Label>
			<Input
				id="export-filename"
				type="text"
				bind:value={exportFileName}
				placeholder="e.g., MyExportedImage"
				autocomplete="off"
				autocorrect="off"
			/>
		</div>

		<!-- Format Dropdown -->
		<div class="space-y-2">
			<Label for="export-format">Export Format</Label>
			<Select
				id="export-format"
				items={exportFormats}
				bind:value={exportFormat}
			/>
		</div>

		<!-- Directory Selection -->
		<div class="space-y-2">
			<Label for="export-directory">Destination Directory</Label>
			<div class="flex gap-2">
				<Input
					id="export-directory"
					type="text"
					bind:value={exportDirectory}
					readonly
					class="flex-grow cursor-not-allowed bg-gray-50 dark:bg-gray-800"
				/>
				<Button color="alternative" on:click={selectExportDirectory} class="px-3" title="Browse">
					<FolderOpen size={18} />
				</Button>
			</div>
		</div>

		<!-- Include Annotations Checkbox -->
		{#if showAnnotations}
		<div class="pt-2">
			<Checkbox bind:checked={includeAnnotations}>
				Include Annotations
			</Checkbox>
		</div>
		{/if}
	</div>

	<svelte:fragment slot="footer">
		<Button color="alternative" on:click={closeModal} title="Cancel">
			Cancel
		</Button>
		<Button
			color="blue"
			on:click={handleConfirm}
			title="Export to {exportFormat.toUpperCase()}"
			disabled={!exportFileName || exportFileName.trim() === '' || !exportDirectory || exportDirectory.trim() === ''}
		>
			Export {exportFormat.toUpperCase()}
		</Button>
	</svelte:fragment>
</Modal>