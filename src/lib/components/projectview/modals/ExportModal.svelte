<!-- src/lib/components/projectview/modals/ExportModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';
	import { activeLayout } from '$lib/stores/layoutStore.js';
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
    import { Share, FolderOpen, X } from 'lucide-svelte';

	export let showModal = false;
	// Prop to receive the path of the transcript being exported
	export let transcriptPath = '';

	const dispatch = createEventDispatcher();

	// Local state for the modal form
	let exportFileName = '';
	let exportFormat = 'csv'; // Default to CSV
	// Initialize with active layout, fallback to the first option in DOCX_LAYOUT_OPTIONS or 'Layout1' if store is undefined initially
	let selectedDocxLayout;
	let exportDirectory = '';
	let excludeSpeakerNames = false; // New state for subtitles
	let modalTitle = 'Export Transcript'; // Title state

	// Available export formats
	const exportFormats = [
		{ value: 'csv', name: 'CSV (.csv)', disabled: false },
		{ value: 'docx', name: 'DOCX (.docx)', disabled: false },
		{ value: 'md', name: 'Markdown (.md)', disabled: false },
		{ value: 'srt', name: 'SRT (.srt)', disabled: false },
		{ value: 'vtt', name: 'Web VTT (.vtt)', disabled: false },
		{ value: 'ass', name: 'Advanced SubStation Alpha (.ass)', disabled: false }, // Added ASS
	];

	const PATH_SEPARATOR = '/'; // Use forward slash for JS path construction

	// --- Initialization ---
	async function initializeModalState() { // Keep async in case other async ops are added later
		const currentProject = get(project);
		const currentTranscriptPath = transcriptPath; // Use the prop

		// Default filename: Use transcript name if available, otherwise media stem, then generic
		let baseName = 'transcript';
		let currentTranscriptName = '';
		if (currentTranscriptPath) {
			 currentTranscriptName = currentTranscriptPath.split(/[\\/]/).pop() || '';
			 if (currentTranscriptName) {
				 baseName = currentTranscriptName.replace(/\.json$/i, ''); // Remove .json extension
				 modalTitle = `Export Transcript: ${currentTranscriptName}`;
			 } else {
				  modalTitle = 'Export Transcript';
			 }
		} else {
			 const mediaName = currentProject.selectedMediaFile?.name;
			 if (mediaName) {
				 baseName = mediaName.replace(/\.[^/.]+$/, ''); // Media name without extension
			 }
			 modalTitle = 'Export Transcript';
		}
		exportFileName = baseName;

		if (!exportDirectory) { 
            try {
                const docDir = await documentDir();
                exportDirectory = docDir;
            } catch (err) {
			    console.warn('[ExportModal] Could not determine User Documents directory.', err);
            }
		}

		// Reset format to default
		exportFormat = 'csv';
		excludeSpeakerNames = false;
		selectedDocxLayout = get(activeLayout) || (DOCX_LAYOUT_OPTIONS.length > 0 ? DOCX_LAYOUT_OPTIONS[0].rustLayoutKey : 'Layout1');
	}


	// Initialize state when modal becomes visible
	$: if (showModal) {
		initializeModalState();
	}

	// --- Actions ---
	async function selectExportDirectory() {
		try {
			const selectedPath = await open({
				directory: true,
				defaultPath: exportDirectory || undefined, // Pass string path
				title: 'Select Export Directory',
			});

			if (selectedPath && typeof selectedPath === 'string') {
				exportDirectory = selectedPath;
			}
		} catch (error) {
			console.error('[ExportModal] Error selecting export directory:', error);
			alert(`Failed to select directory: ${error?.message || error}`);
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
		 if (!exportFormat) {
			 alert('Please select an export format.');
			 return;
		 }

        let fullExportPath = '';
         try {
            const dir = exportDirectory.endsWith(PATH_SEPARATOR) ? exportDirectory.slice(0, -1) : exportDirectory;
            fullExportPath = dir + PATH_SEPARATOR + `${exportFileName}.${exportFormat}`;
         } catch (e) {
             console.error("[ExportModal] Failed to construct export path:", e);
             return;
         }

		dispatch('confirm', {
			filePath: fullExportPath, // Pass the string path
			format: exportFormat,
			layoutChoice: (exportFormat === 'docx' || exportFormat === 'md') ? selectedDocxLayout : undefined,
			excludeSpeakerNames: (exportFormat === 'srt' || exportFormat === 'vtt' || exportFormat === 'ass') ? excludeSpeakerNames : false,
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
	size="md"
	autoclose={false}
	outsideclose={true}
	class="w-full"
	backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
	dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
	bodyClass="p-0 overflow-hidden bg-white dark:bg-gray-900"
	headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
	footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
	on:close={closeModal}
>
	<div slot="header" class="flex items-center gap-2">
		<Share class="w-5 h-5 text-gray-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
			Export Transcript
		</h3>
	</div>

	<div class="p-6 space-y-5 overflow-y-auto max-h-[70vh] custom-scrollbar">
		<!-- Filename Input -->
		<div class="space-y-2">
			<Label for="export-filename">Filename</Label>
			<Input
				id="export-filename"
				type="text"
				bind:value={exportFileName}
				placeholder="e.g., MyMeetingTranscript"
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
			 {#if exportFormat === 'md'}
				<Helper class="italic">Markdown export supports basic styling (bold, italic). Other rich text formatting will be converted to plain text.</Helper>
			 {:else if exportFormat === 'ass'}
				<Helper class="italic">ASS export provides standard subtitles with styling support (bold, italic, underline, strikethrough, color).</Helper>
			 {:else if exportFormat === 'csv'}
				<Helper class="italic">Comma-separated values file (.csv)</Helper>
			 {:else if exportFormat === 'docx'}
				<Helper class="italic">Microsoft Word document (.docx)</Helper>
			 {:else if exportFormat === 'srt'}
				<Helper class="italic">SubRip Subtitle file (.srt)</Helper>
			 {:else if exportFormat === 'vtt'}
				<Helper class="italic">Web Video Text Tracks file (.vtt)</Helper>
			 {/if}
		</div>

		<!-- Subtitle Options (Conditional for SRT, VTT, ASS) -->
		{#if exportFormat === 'srt' || exportFormat === 'vtt' || exportFormat === 'ass'}
			<div class="pt-1">
				<Checkbox bind:checked={excludeSpeakerNames}>
					Exclude speaker names from subtitles
				</Checkbox>
			</div>
		{/if}

		<!-- Layout Options (Conditional for DOCX and MD) -->
		{#if exportFormat === 'docx' || exportFormat === 'md'}
		<div class="pt-2 space-y-3">
				<Label id="layout-label" class="font-semibold text-sm">{exportFormat.toUpperCase()} Layout</Label>
				<div class="grid grid-cols-1 sm:grid-cols-2 gap-3" role="group" aria-labelledby="layout-label">
					{#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
						<button
							type="button"
							class="text-left p-3 border rounded-xl transition-all relative {selectedDocxLayout === layout.rustLayoutKey ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-400' : 'border-gray-200 dark:border-gray-700 hover:border-blue-300'}"
							on:click={() => selectedDocxLayout = layout.rustLayoutKey}
							title={layout.name}
						>
							<div class="font-bold text-[11px] mb-2 {selectedDocxLayout === layout.rustLayoutKey ? 'text-blue-700 dark:text-blue-300' : ''}">
								{layout.name}
							</div>
							<div class="{layout.previewClasses} min-h-[20px] opacity-80 rounded overflow-hidden border border-gray-100 dark:border-gray-800 bg-white dark:bg-gray-800">
								{#each layout.columnStyles as style}
									<div class="{style.class} !p-1 !text-[9px] leading-tight flex items-center justify-center text-center">{style.content}</div>
								{/each}
							</div>
							{#if selectedDocxLayout === layout.rustLayoutKey}
								<div class="absolute top-2 right-2 w-1.5 h-1.5 bg-blue-500 rounded-full"></div>
							{/if}
						</button>
					{/each}
				</div>
			</div>
		{/if}

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
	</div>

	<svelte:fragment slot="footer">
		<Button color="alternative" on:click={closeModal} title="Cancel">
			Cancel
		</Button>
		<Button
			color="blue"
			on:click={handleConfirm}
			title="Export to {exportFormat.toUpperCase()}"
			disabled={
				!exportFileName || exportFileName.trim() === '' ||
				!exportDirectory || exportDirectory.trim() === ''
			}
		>
			Export {exportFormat.toUpperCase()}
		</Button>
	</svelte:fragment>
</Modal>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-200 dark:bg-gray-700 rounded-full;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-300 dark:bg-gray-600;
    }
</style>