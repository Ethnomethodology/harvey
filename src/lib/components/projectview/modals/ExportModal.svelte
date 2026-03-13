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
        Input, 
        Label, 
        Select, 
        Button, 
        Helper,
        Modal
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
	let modalElement; // Ref to the modal container
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

	const DEFAULT_EXPORT_FOLDER_NAME = 'exports'; // Name for the default subfolder
	const PATH_SEPARATOR = '/'; // Use forward slash for JS path construction

    // --- NEW: Simple JS dirname equivalent ---
    function simpleDirname(path) {
        if (!path || typeof path !== 'string') return '';
        // Replace backslashes for consistency
        const normalizedPath = path.replace(/\\/g, PATH_SEPARATOR);
        const lastSeparatorIndex = normalizedPath.lastIndexOf(PATH_SEPARATOR);
        if (lastSeparatorIndex === -1) {
            // No separator found, maybe just a filename? Return '.' for current dir? Or empty? Let's return empty for simplicity here.
            return '';
        }
        if (lastSeparatorIndex === 0) {
             // Path is like "/file", dirname is "/"
             return PATH_SEPARATOR;
        }
        // Return the part before the last separator
        return normalizedPath.substring(0, lastSeparatorIndex);
    }
    // --- END NEW ---

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

		// Simplified default directory logic
		// We do NOT reset exportDirectory here to preserve the last used location during the session.
        // If it's the first time (empty), we default to the user's Documents folder.

		if (!exportDirectory) { 
            try {
                const docDir = await documentDir();
                exportDirectory = docDir;
                console.log('[ExportModal] Default export directory set to User Documents:', exportDirectory);
            } catch (err) {
			    // exportDirectory = ''; // It's already empty/falsy
			    console.warn('[ExportModal] Could not determine User Documents directory.', err);
            }
		}
		// END Simplified default directory logic

		// Reset format to default
		exportFormat = 'csv';
		excludeSpeakerNames = false;
		selectedDocxLayout = get(activeLayout) || (DOCX_LAYOUT_OPTIONS.length > 0 ? DOCX_LAYOUT_OPTIONS[0].rustLayoutKey : 'Layout1');

		console.log('[ExportModal] Modal state initialized.', { exportFileName, exportDirectory, modalTitle });
	}


	// Initialize state when modal becomes visible
	$: if (showModal) {
		initializeModalState(); // This sets exportFormat to 'csv' and resets other fields
		// If the initial active layout should be reflected immediately even if the format
		// was already docx/md (which initializeModalState prevents by setting to csv),
		// this would be the place. But since exportFormat becomes 'csv',
		// the following $: block handles the change *to* docx/md correctly.
	}

	// --- Actions ---
	async function selectExportDirectory() {
		console.log('[ExportModal] Opening directory save dialog...');
		try {
			const selectedPath = await open({
				directory: true,
				defaultPath: exportDirectory || undefined, // Pass string path
				title: 'Select Export Directory',
			});

			if (selectedPath && typeof selectedPath === 'string') {
				exportDirectory = selectedPath;
				 console.log('[ExportModal] Export directory selected:', exportDirectory);
			} else {
				 console.log('[ExportModal] Directory selection cancelled or invalid path received.');
				 if (selectedPath) console.warn("[ExportModal] Received non-string path from dialog:", selectedPath)
			}
		} catch (error) {
			console.error('[ExportModal] Error selecting export directory:', error);
			alert(`Failed to select directory: ${error?.message || error}`);
		}
	}

	function handleConfirm() {
		console.log('[ExportModal] Confirming export...');
		// Basic validation
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

        // --- Use manual string concatenation ---
        let fullExportPath = '';
         try {
            const dir = exportDirectory.endsWith(PATH_SEPARATOR) ? exportDirectory.slice(0, -1) : exportDirectory;
            fullExportPath = dir + PATH_SEPARATOR + `${exportFileName}.${exportFormat}`;
            console.log("[ExportModal] Constructed full export path:", fullExportPath);
         } catch (e) {
             console.error("[ExportModal] Failed to construct full export path:", e);
             alert(`Error constructing export path: ${e?.message || e}`);
             return;
         }
         // --- END ---

		dispatch('confirm', {
			filePath: fullExportPath, // Pass the string path
			format: exportFormat,
			layoutChoice: (exportFormat === 'docx' || exportFormat === 'md') ? selectedDocxLayout : undefined,
			excludeSpeakerNames: (exportFormat === 'srt' || exportFormat === 'vtt' || exportFormat === 'ass') ? excludeSpeakerNames : false,
		});
		closeModal();
	}

	function closeModal() {
		showModal = false; // Update bound prop
		dispatch('close'); // Dispatch event
	}

	// --- Keyboard Handling ---
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			closeModal();
		}
		if (showModal && event.key === 'Enter') {
			event.preventDefault();
			 handleConfirm();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if showModal}
	<div
		bind:this={modalElement}
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="export-modal-title"
		tabindex="-1"
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
                        <Share size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="export-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
                        Export Transcript
                    </h3>
                </div>
                <button on:click={closeModal} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

			<div class="p-6 space-y-5">
				<!-- Filename Input -->
				<div class="space-y-2">
					<Label for="export-filename">Filename</Label>
					<Input
						id="export-filename"
						type="text"
						bind:value={exportFileName}
						placeholder="e.g., MyMeetingTranscript"
						autocomplete="off"
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
					<div class="flex items-center space-x-2 pt-1">
						<input
							id="exclude-speakers"
							type="checkbox"
							bind:checked={excludeSpeakerNames}
							class="w-4 h-4 text-blue-600 bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-blue-500 dark:focus:ring-blue-600 focus:ring-2"
						/>
						<Label for="exclude-speakers" class="text-sm font-medium">
							Exclude speaker names from subtitles
						</Label>
					</div>
				{/if}

				<!-- Layout Options (Conditional for DOCX and MD) -->
				{#if exportFormat === 'docx' || exportFormat === 'md'}
				<div class="pt-2 space-y-2">
						<Label id="layout-label">{exportFormat.toUpperCase()} Layout:</Label>
						<div class="grid grid-cols-1 sm:grid-cols-2 gap-2" role="group" aria-labelledby="layout-label">
							{#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
								<button
									type="button"
									class="text-left p-2 border rounded-md transition-colors text-xs focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
									class:bg-blue-500={selectedDocxLayout === layout.rustLayoutKey}
									class:text-white={selectedDocxLayout === layout.rustLayoutKey}
									class:hover:bg-gray-100={selectedDocxLayout !== layout.rustLayoutKey}
									class:dark:hover:bg-gray-700={selectedDocxLayout !== layout.rustLayoutKey}
									class:border-blue-500={selectedDocxLayout === layout.rustLayoutKey}
									class:dark:border-blue-400={selectedDocxLayout === layout.rustLayoutKey}
									class:border-gray-300={selectedDocxLayout !== layout.rustLayoutKey}
									class:dark:border-gray-600={selectedDocxLayout !== layout.rustLayoutKey}
									on:click={() => selectedDocxLayout = layout.rustLayoutKey}
									title={layout.name}
								>
									<div class="font-medium mb-1">{layout.name}</div>
									<div class="{layout.previewClasses} min-h-[20px]">
										{#each layout.columnStyles as style}
											<div class="{style.class}">{style.content}</div>
										{/each}
									</div>
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

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
				<Button color="alternative" on:click={closeModal} title="Cancel">
					Cancel
				</Button>
				<Button
					color="blue"
					on:click={handleConfirm}
                    title="Export to {exportFormat.toUpperCase()}"
					disabled={
						!exportFileName || exportFileName.trim() === '' ||
						!exportDirectory || exportDirectory.trim() === '' ||
						(
							exportFormat !== 'csv' &&
							exportFormat !== 'docx' &&
							exportFormat !== 'srt' &&
							exportFormat !== 'vtt' &&
							exportFormat !== 'md' &&
							exportFormat !== 'ass'
						)
					}
				>
					Export {exportFormat.toUpperCase()}
				</Button>
			</div>
		</div>
	</div>
{/if}