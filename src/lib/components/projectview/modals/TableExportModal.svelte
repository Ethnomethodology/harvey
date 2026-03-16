<!-- src/lib/components/projectview/modals/TableExportModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
    import { documentDir } from '@tauri-apps/api/path';
	import { invoke } from '@tauri-apps/api/core';
    import { 
		Modal,
        Input, 
        Label, 
        Select, 
        Button, 
        Helper,
        Badge
    } from 'flowbite-svelte';
    import { Share, FolderOpen, X } from 'lucide-svelte';

	export let showModal = false;
	export let tablePath = '';
    export let getExportData = null; // Optional function to get formatted data from the UI

	const dispatch = createEventDispatcher();

	let exportFileName = '';
	let exportFormat = 'xlsx';
	let exportDirectory = '';
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
			Export Table
		</h3>
	</div>

	<div class="space-y-5">
		<div class="space-y-2">
			<Label for="table-export-filename">Filename</Label>
			<Input
				id="table-export-filename"
				type="text"
				bind:value={exportFileName}
				placeholder="e.g., MyTable"
				autocomplete="off"
				autocorrect="off"
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

	<svelte:fragment slot="footer">
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
	</svelte:fragment>
</Modal>