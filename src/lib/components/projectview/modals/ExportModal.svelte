<!-- src/lib/components/projectview/ExportModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js';
	import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';
	import { activeLayout } from '$lib/stores/layoutStore.js'; // Added
	import { open } from '@tauri-apps/plugin-dialog';
	// --- REMOVED: No fs functions imported for path manipulation ---

	export let showModal = false;
	// Prop to receive the path of the transcript being exported
	export let transcriptPath = '';

	const dispatch = createEventDispatcher();

	// Local state for the modal form
	let exportFileName = '';
	let exportFormat = 'csv'; // Default to CSV
	let selectedDocxLayout = get(activeLayout) || 'Layout1'; // Initialize with active layout or default to Layout1
	let exportDirectory = '';
	let modalElement; // Ref to the modal container
	let modalTitle = 'Export Transcript'; // Title state

	// Available export formats
	const exportFormats = [
		{ value: 'csv', label: 'CSV (.csv)', disabled: false },
		{ value: 'docx', label: 'DOCX (.docx)', disabled: false },
		{ value: 'md', label: 'Markdown (.md)', disabled: false },
		{ value: 'srt', label: 'SRT (.srt)', disabled: false },
		{ value: 'vtt', label: 'Web VTT (.vtt)', disabled: false },
		{ value: 'ass', label: 'Advanced SubStation Alpha (.ass)', disabled: false }, // Added ASS
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
		exportDirectory = ''; // Initialize
		if (currentTranscriptPath) {
			exportDirectory = simpleDirname(currentTranscriptPath);
			if (exportDirectory) {
				console.log('[ExportModal] Default export directory set from transcriptPath:', exportDirectory);
			}
		}

		if (!exportDirectory) { // This will be true if transcriptPath was empty or simpleDirname returned empty
			exportDirectory = ''; // Ensure it's explicitly empty
			console.warn('[ExportModal] Could not determine directory from transcriptPath. Defaulting to empty.');
		}
		// END Simplified default directory logic

		// Reset format to default
		exportFormat = 'csv';

		console.log('[ExportModal] Modal state initialized.', { exportFileName, exportDirectory, modalTitle });
	}


	// Initialize state when modal becomes visible
	$: if (showModal) {
		initializeModalState();
		// When modal becomes visible, if docx/md is selected, sync with activeLayout
		if (exportFormat === 'docx' || exportFormat === 'md') {
			selectedDocxLayout = get(activeLayout) || 'Layout1';
		}
	}

	// When exportFormat changes, update selectedDocxLayout if necessary
	let previousExportFormat = exportFormat;
	$: {
		if (showModal) {
			if ((exportFormat === 'docx' || exportFormat === 'md') &&
			    (previousExportFormat !== 'docx' && previousExportFormat !== 'md')) {
				// Switched TO docx/md
				selectedDocxLayout = get(activeLayout) || 'Layout1';
				console.log(`[ExportModal] Switched to DOCX/MD format, synced selectedDocxLayout to: ${selectedDocxLayout}`);
			}
			previousExportFormat = exportFormat;
		}
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
		if (event.key === 'Enter') {
			event.preventDefault();
			 const confirmButton = modalElement?.querySelector('.btn-primary');
			 if (confirmButton && !confirmButton.disabled) {
				 handleConfirm();
			 }
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
		class="fixed inset-0 z-[120] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="export-modal-title"
	>
		<div
			class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md m-4 flex flex-col text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
		>
			<h2 id="export-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-5 truncate" title="{modalTitle}">
				{modalTitle}
			</h2>

			<div class="space-y-4 text-sm text-gray-700 dark:text-gray-300">
				<!-- Filename Input -->
				<div>
					<label for="export-filename" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Filename:</label>
					<input
						id="export-filename"
						type="text"
						bind:value={exportFileName}
						class="input-field w-full bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-200 focus:ring-blue-500 focus:border-blue-500"
						placeholder="e.g., MyMeetingTranscript"
					/>
				</div>

				<!-- Format Dropdown -->
				 <div>
					<label for="export-format" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Format:</label>
					<select
						id="export-format"
						bind:value={exportFormat}
						class="input-field w-full bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-200 focus:ring-blue-500 focus:border-blue-500"
					>
						{#each exportFormats as formatOption (formatOption.value)}
							<option value="{formatOption.value}" disabled={formatOption.disabled}>
								{formatOption.label}
							</option>
						{/each}
					</select>
					 {#if exportFormat !== 'csv' && exportFormat !== 'docx' && exportFormat !== 'srt' && exportFormat !== 'vtt' && exportFormat !== 'md' && exportFormat !== 'ass'}
						<p class="mt-1 text-xs text-orange-600 dark:text-orange-400">This format is not yet fully implemented. All listed formats are available.</p>
					 {:else if exportFormat === 'md'}
						<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">Markdown export supports basic styling (bold, italic). Other rich text formatting will be converted to plain text.</p>
					 {:else if exportFormat === 'ass'}
						<p class="mt-1 text-xs text-gray-500 dark:text-gray-400">ASS export supports styling (bold, italic, underline, color). Highlights are converted to text color changes. Other styles stripped.</p>
					 {/if}
				</div>

				<!-- Layout Options (Conditional for DOCX and MD) -->
				{#if exportFormat === 'docx' || exportFormat === 'md'}
				<div class="pt-2">
						<label class="block font-medium text-gray-700 dark:text-gray-300 mb-1.5">{exportFormat.toUpperCase()} Layout:</label>
						<div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
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
				<div>
					<label for="export-directory" class="block font-medium text-gray-700 dark:text-gray-300 mb-1 pt-2">Export To:</label>
					<div class="flex space-x-2">
						<input
							id="export-directory"
							type="text"
							bind:value={exportDirectory}
							class="input-field flex-grow bg-gray-100 dark:bg-gray-600 border-gray-300 dark:border-gray-500 text-gray-600 dark:text-gray-300 cursor-not-allowed"
							readonly
							placeholder="Select directory..."
						/>
						<button type="button" on:click={selectExportDirectory} class="btn-secondary flex-shrink-0 text-xs px-3 py-1.5">
							Browse
						</button>
					</div>
				</div>
			</div>

			<!-- Footer Buttons -->
			<div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-600 mt-6">
				<button type="button" on:click={closeModal} class="btn-secondary">
					Cancel
				</button>
				<button
					type="button"
					on:click={handleConfirm}
					class="btn-primary"
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
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
	/* Reuse button/input styles from other components */
	.btn-primary, .btn-secondary {
		@apply px-4 py-2 rounded-md shadow-sm text-sm font-medium transition duration-150 ease-in-out;
	}
	.btn-primary {
		@apply bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-gray-400 dark:disabled:bg-gray-600;
	}
	 .btn-secondary {
		@apply bg-gray-200 text-gray-700 hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500 dark:focus:ring-offset-gray-800;
	}
	.input-field {
		@apply block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 text-sm;
	}
	 /* Add dark mode styling */
	 .dark .input-field {
		 @apply bg-gray-700 border-gray-600 text-gray-200 placeholder-gray-400;
	 }
	 .dark .input-field:read-only {
		 @apply bg-gray-600 border-gray-500 text-gray-300 cursor-not-allowed;
	 }

	 select.input-field {
		 @apply appearance-none pr-8; /* Add padding for dropdown arrow */
		 background-image: url('data:image/svg+xml;charset=US-ASCII,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2024%2024%22%20fill%3D%22%236b7280%22%3E%3Cpath%20d%3D%22M7.293%209.293a1%201%200%20011.414%200L12%2012.586l3.293-3.293a1%201%200%20111.414%201.414l-4%204a1%201%200%2001-1.414%200l-4-4a1%201%200%20010-1.414z%22%20clip-rule%3D%22evenodd%22%20fill-rule%3D%22evenodd%22%3E%3C%2Fpath%3E%3C%2Fsvg%3E');
		 background-repeat: no-repeat;
		 background-size: 1rem 1rem; /* 16px */
		 background-position: right 0.5rem center;
	 }
	 .dark select.input-field {
		 background-image: url('data:image/svg+xml;charset=US-ASCII,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2024%2024%22%20fill%3D%22%23d1d5db%22%3E%3Cpath%20d%3D%22M7.293%209.293a1%201%200%20011.414%200L12%2012.586l3.293-3.293a1%201%200%20111.414%201.414l-4%204a1%201%200%2001-1.414%200l-4-4a1%201%200%20010-1.414z%22%20clip-rule%3D%22evenodd%22%20fill-rule%3D%22evenodd%22%3E%3C%2Fpath%3E%3C%2Fsvg%3E');
	 }

	 select.input-field option[disabled] {
		 color: #9ca3af; /* gray-400 */
	 }
	  .dark select.input-field option[disabled] {
		 color: #6b7280; /* gray-500 */
	 }
</style>