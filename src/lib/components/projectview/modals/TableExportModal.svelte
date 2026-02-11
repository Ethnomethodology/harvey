<!-- src/lib/components/projectview/modals/TableExportModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
	import { invoke } from '@tauri-apps/api/core';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';

	export let showModal = false;
	export let tablePath = '';

	const dispatch = createEventDispatcher();

	let exportFileName = '';
	let exportFormat = 'xlsx';
	let exportDirectory = '';
	let modalElement;
	let modalTitle = 'Export Table';
    let isExporting = false;

	const exportFormats = [
		{ value: 'xlsx', label: 'Excel (.xlsx)', disabled: false },
		{ value: 'csv', label: 'CSV (.csv)', disabled: false },
	];

	const PATH_SEPARATOR = '/'; 

    async function initializeModalState() {
        if (tablePath) {
             const fileName = tablePath.split(/[\/]/).pop() || '';
             if (fileName) {
                 exportFileName = fileName.replace(/\.(csv|xlsx)$/i, '');
                 modalTitle = `Export Table: ${fileName}`;
             } else {
                  modalTitle = 'Export Table';
                  exportFileName = 'table';
             }
        } else {
              modalTitle = 'Export Table';
              exportFileName = 'table';
        }

        if (!exportDirectory) {
            try {
                const docDir = await documentDir();
                exportDirectory = docDir;
            } catch (e) {
                console.warn('[TableExportModal] Failed to get document directory:', e);
            }
        }

        exportFormat = 'xlsx';
        isExporting = false;
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
			console.error('[TableExportModal] Error selecting directory:', error);
		}
	}

	async function handleConfirm() {
		if (!exportFileName || exportFileName.trim() === '') {
			alert('Please enter a filename.');
			return;
		}
		 if (!exportDirectory || exportDirectory.trim() === '') {
			alert('Please select an export directory.');
			return;
		}

        isExporting = true;
        
        let fullExportPath = '';
         try {
            const dir = exportDirectory.endsWith(PATH_SEPARATOR) ? exportDirectory.slice(0, -1) : exportDirectory;
            fullExportPath = dir + PATH_SEPARATOR + `${exportFileName}.${exportFormat}`;
         } catch (e) {
             console.error("[TableExportModal] Failed to construct path:", e);
             isExporting = false;
             return;
         }

         try {
             if (exportFormat === 'xlsx') {
                 await invoke('export_table_to_xlsx', { tablePathStr: tablePath, outputPathStr: fullExportPath });
             } else if (exportFormat === 'csv') {
                 await invoke('export_table_to_csv', { tablePathStr: tablePath, outputPathStr: fullExportPath });
             }
             
             dispatch('confirm', { filePath: fullExportPath, format: exportFormat });
             closeModal();
         } catch (e) {
             console.error(`[TableExportModal] Export failed:`, e);
             alert(`Export failed: ${e?.message || e}`);
         } finally {
             isExporting = false;
         }
	}

	function closeModal() {
		showModal = false;
		dispatch('close');
	}

	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			closeModal();
		}
        if (showModal && event.key === 'Enter' && !isExporting) {
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
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
		aria-labelledby="table-export-modal-title"
		tabindex="-1"
	>
		<div
			class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-md m-4 flex flex-col text-gray-800 dark:text-gray-200"
			on:click|stopPropagation
			role="document"
		>
			<h2 id="table-export-modal-title" class="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-5 truncate" title="{modalTitle}">
				{modalTitle}
			</h2>

			<div class="space-y-4 text-sm text-gray-700 dark:text-gray-300">
				<div>
					<label for="table-export-filename" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Filename:</label>
					<input
						id="table-export-filename"
						type="text"
						bind:value={exportFileName}
						class="input-field w-full bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-200 focus:ring-blue-500 focus:border-blue-500"
						placeholder="e.g., MyTable"
					/>
				</div>

				 <div>
					<label for="table-export-format" class="block font-medium text-gray-700 dark:text-gray-300 mb-1">Format:</label>
					<Dropdown
						containerClasses="w-full"
						options={exportFormats}
						bind:value={exportFormat}
						placeholder="Select a Format"
					/>
                    {#if exportFormat === 'xlsx'}
                        <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">Exports as an Excel spreadsheet.</p>
                    {:else if exportFormat === 'csv'}
                        <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">Exports as a comma-separated values file.</p>
                    {/if}
				</div>

				<div>
					<label for="table-export-directory" class="block font-medium text-gray-700 dark:text-gray-300 mb-1 pt-2">Export To:</label>
					<div class="flex space-x-2">
						<input
							id="table-export-directory"
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

			<div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-600 mt-6">
				<button type="button" on:click={closeModal} class="btn-secondary" disabled={isExporting}>
					Cancel
				</button>
				<button
					type="button"
					on:click={handleConfirm}
					class="btn-primary"
					disabled={
						!exportFileName || exportFileName.trim() === '' ||
						!exportDirectory || exportDirectory.trim() === '' ||
                        isExporting
					}
				>
					{#if isExporting}
                        Exporting...
                    {:else}
                        Export {exportFormat.toUpperCase()}
                    {/if}
				</button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
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
	 .dark .input-field {
		 @apply bg-gray-700 border-gray-600 text-gray-200 placeholder-gray-400;
	 }
	 .dark .input-field:read-only {
		 @apply bg-gray-600 border-gray-500 text-gray-300 cursor-not-allowed;
	 }
</style>
