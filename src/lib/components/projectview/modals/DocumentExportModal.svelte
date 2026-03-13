<!-- src/lib/components/projectview/modals/DocumentExportModal.svelte -->
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
        Modal
    } from 'flowbite-svelte';
    import { Share, FolderOpen, X } from 'lucide-svelte';

	export let showModal = false;
	export let documentPath = '';

	const dispatch = createEventDispatcher();

	let exportFileName = '';
	let exportFormat = 'docx';
	let exportDirectory = '';
	let modalElement;
	let modalTitle = 'Export Document';
    let isExporting = false;

	const exportFormats = [
		{ value: 'docx', name: 'DOCX (.docx)', disabled: false },
		{ value: 'md', name: 'Markdown (.md)', disabled: false },
		{ value: 'txt', name: 'Plain Text (.txt)', disabled: false },
	];

	const PATH_SEPARATOR = '/'; // Assuming forward slash for consistency in JS path manipulation

    function simpleDirname(path) {
        if (!path || typeof path !== 'string') return '';
        const normalizedPath = path.replace(/\\/g, PATH_SEPARATOR);
        const lastSeparatorIndex = normalizedPath.lastIndexOf(PATH_SEPARATOR);
        if (lastSeparatorIndex === -1) return '';
        if (lastSeparatorIndex === 0) return PATH_SEPARATOR;
        return normalizedPath.substring(0, lastSeparatorIndex);
    }

		async function initializeModalState() {
			if (documentPath) {
				 const fileName = documentPath.split(/[\\/]/).pop() || '';
				 if (fileName) {
					 exportFileName = fileName.replace(/\.json$/i, '');
					 modalTitle = `Export Document: ${fileName}`;
				 } else {
					  modalTitle = 'Export Document';
	                  exportFileName = 'document';
				 }
	             		} else {
	                          modalTitle = 'Export Document';
	             			 exportFileName = 'document';
	             		}
	             
	                     if (!exportDirectory) {
	                         try {
	                             const docDir = await documentDir();
	                             exportDirectory = docDir;
	                         } catch (e) {
	                             console.warn('[DocumentExportModal] Failed to get document directory:', e);
	                         }
	                     }
	             
	             		exportFormat = 'docx';
	                     isExporting = false;
	             	}	$: if (showModal) {
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
			console.error('[DocumentExportModal] Error selecting directory:', error);
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
             console.error("[DocumentExportModal] Failed to construct path:", e);
             isExporting = false;
             return;
         }

         try {
             if (exportFormat === 'docx') {
                 await invoke('export_document_to_docx', { documentPathStr: documentPath, outputPathStr: fullExportPath });
             } else if (exportFormat === 'md') {
                 await invoke('export_document_to_markdown', { documentPathStr: documentPath, outputPathStr: fullExportPath });
             } else if (exportFormat === 'txt') {
                 await invoke('export_document_to_txt', { documentPathStr: documentPath, outputPathStr: fullExportPath });
             }
             
             dispatch('confirm', { filePath: fullExportPath, format: exportFormat });
             closeModal();
         } catch (e) {
             console.error(`[DocumentExportModal] Export failed:`, e);
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
		aria-labelledby="doc-export-modal-title"
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
                    <h3 id="doc-export-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate max-w-[250px]" title="{modalTitle}">
                        Export Document
                    </h3>
                </div>
                <button on:click={closeModal} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

			<div class="p-6 space-y-5">
				<div class="space-y-2">
					<Label for="doc-export-filename">Filename</Label>
					<Input
						id="doc-export-filename"
						type="text"
						bind:value={exportFileName}
						placeholder="e.g., MyDocument"
					/>
				</div>

				 <div class="space-y-2">
					<Label for="doc-export-format">Export Format</Label>
					<Select
						id="doc-export-format"
						items={exportFormats}
						bind:value={exportFormat}
					/>
                    <Helper class="italic">
                        {#if exportFormat === 'docx'}
                            Exports as a formatted Word document (.docx)
                        {:else if exportFormat === 'md'}
                            Exports as Markdown with basic formatting (.md)
                        {:else if exportFormat === 'txt'}
                            Exports as plain text (.txt)
                        {/if}
                    </Helper>
				</div>

				<div class="space-y-2">
					<Label for="doc-export-directory">Destination Directory</Label>
					<div class="flex gap-2">
						<Input
							id="doc-export-directory"
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
				<Button color="alternative" on:click={closeModal} disabled={isExporting} title="Cancel">
					Cancel
				</Button>
				<Button
					color="blue"
					on:click={handleConfirm}
                    title="Export to {exportFormat.toUpperCase()}"
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
