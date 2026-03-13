<!-- src/lib/components/projectview/modals/TableExportModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
	import { invoke } from '@tauri-apps/api/core';
    import { 
        Input, 
        Label, 
        Select, 
        Button, 
        Helper,
        Badge, Modal
    } from 'flowbite-svelte';
    import { Share, FolderOpen, X } from 'lucide-svelte';

	export let showModal = false;
	export let tablePath = '';
    export let getExportData = null; // Optional function to get formatted data from the UI

	const dispatch = createEventDispatcher();

	let exportFileName = '';
	let exportFormat = 'xlsx';
	let exportDirectory = '';
	let modalElement;
	let modalTitle = 'Export Table';
    let isExporting = false;

	const exportFormats = [
		{ name: 'Excel (.xlsx)', value: 'xlsx' },
		{ name: 'CSV (.csv)', value: 'csv' },
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
            if (getExportData) {
                const formattedDataObj = await getExportData();
                if (formattedDataObj) {
                    const { data, headers, styles } = formattedDataObj;
                    if (exportFormat === 'xlsx') {
                        await invoke('export_formatted_table_to_xlsx', { data, headers, styles, outputPathStr: fullExportPath });
                    } else if (exportFormat === 'csv') {
                        await invoke('export_formatted_table_to_csv', { data, headers, outputPathStr: fullExportPath });
                    }
                } else {
                    if (exportFormat === 'xlsx') {
                        await invoke('export_table_to_xlsx', { tablePathStr: tablePath, outputPathStr: fullExportPath });
                    } else if (exportFormat === 'csv') {
                        await invoke('export_table_to_csv', { tablePathStr: tablePath, outputPathStr: fullExportPath });
                    }
                }
            } else {
                if (exportFormat === 'xlsx') {
                    await invoke('export_table_to_xlsx', { tablePathStr: tablePath, outputPathStr: fullExportPath });
                } else if (exportFormat === 'csv') {
                    await invoke('export_table_to_csv', { tablePathStr: tablePath, outputPathStr: fullExportPath });
                }
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
		aria-labelledby="table-export-modal-title"
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
                    <h3 id="table-export-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
                        Export Table
                    </h3>
                </div>
                <button on:click={closeModal} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all">
                    <X size={20} />
                </button>
            </div>

			<div class="p-6 space-y-5">
				<div class="space-y-2">
					<Label for="table-export-filename">Filename</Label>
					<Input
						id="table-export-filename"
						type="text"
						bind:value={exportFileName}
						placeholder="e.g., MyTable"
					/>
				</div>

				 <div class="space-y-2">
					<Label for="table-export-format">Export Format</Label>
					<Select
						id="table-export-format"
						items={exportFormats}
						bind:value={exportFormat}
					/>
                    <Helper class="italic">
                        {#if exportFormat === 'xlsx'}
                            Exports as an Excel spreadsheet (.xlsx)
                        {:else if exportFormat === 'csv'}
                            Exports as a comma-separated values file (.csv)
                        {/if}
                    </Helper>
				</div>

				<div class="space-y-2">
					<Label for="table-export-directory">Destination Directory</Label>
					<div class="flex gap-2">
						<Input
							id="table-export-directory"
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
				<Button color="alternative" on:click={closeModal} disabled={isExporting}>
					Cancel
				</Button>
				<Button
					color="blue"
					on:click={handleConfirm}
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
				</Button>
			</div>
		</div>
	</div>
{/if}
