<!-- src/lib/components/projectview/modals/DocumentExportModal.svelte -->
<script>
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { documentDir } from '@tauri-apps/api/path';
  import { invoke } from '@tauri-apps/api/core';
  import { Modal, Input, Label, Select, Button, Helper } from 'flowbite-svelte';
  import { Share, FolderOpen, X } from '@lucide/svelte';

  export let showModal = false;
  export let documentPath = '';

  const dispatch = createEventDispatcher();

  let exportFileName = '';
  let exportFormat = 'docx';
  let exportDirectory = '';
  let modalTitle = 'Export Document';
  let isExporting = false;

  const exportFormats = [
    { value: 'docx', name: 'DOCX (.docx)', disabled: false },
    { value: 'md', name: 'Markdown (.md)', disabled: false },
    { value: 'txt', name: 'Plain Text (.txt)', disabled: false }
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
  }
  $: if (showModal) {
    initializeModalState();
  }

  async function selectExportDirectory() {
    try {
      const selectedPath = await open({
        directory: true,
        defaultPath: exportDirectory || undefined,
        title: 'Select Export Directory'
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
      const dir = exportDirectory.endsWith(PATH_SEPARATOR)
        ? exportDirectory.slice(0, -1)
        : exportDirectory;
      fullExportPath = dir + PATH_SEPARATOR + `${exportFileName}.${exportFormat}`;
    } catch (e) {
      console.error('[DocumentExportModal] Failed to construct path:', e);
      isExporting = false;
      return;
    }

    try {
      if (exportFormat === 'docx') {
        await invoke('export_document_to_docx', {
          documentPathStr: documentPath,
          outputPathStr: fullExportPath
        });
      } else if (exportFormat === 'md') {
        await invoke('export_document_to_markdown', {
          documentPathStr: documentPath,
          outputPathStr: fullExportPath
        });
      } else if (exportFormat === 'txt') {
        await invoke('export_document_to_txt', {
          documentPathStr: documentPath,
          outputPathStr: fullExportPath
        });
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
    <h3
      class="text-lg font-semibold text-gray-900 dark:text-white truncate max-w-[250px]"
      title={modalTitle}
    >
      Export Document
    </h3>
  </div>

  <div class="space-y-5">
    <div class="space-y-2">
      <Label for="doc-export-filename">Filename</Label>
      <Input
        id="doc-export-filename"
        type="text"
        bind:value={exportFileName}
        placeholder="e.g., MyDocument"
        autocomplete="off"
        autocorrect="off"
      />
    </div>

    <div class="space-y-2">
      <Label for="doc-export-format">Export Format</Label>
      <Select id="doc-export-format" items={exportFormats} bind:value={exportFormat} />
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

  <svelte:fragment slot="footer">
    <Button color="alternative" on:click={closeModal} disabled={isExporting} title="Cancel">
      Cancel
    </Button>
    <Button
      color="blue"
      on:click={handleConfirm}
      title="Export to {exportFormat.toUpperCase()}"
      disabled={!exportFileName ||
        exportFileName.trim() === '' ||
        !exportDirectory ||
        exportDirectory.trim() === '' ||
        isExporting}
    >
      {#if isExporting}
        Exporting...
      {:else}
        Export {exportFormat.toUpperCase()}
      {/if}
    </Button>
  </svelte:fragment>
</Modal>
