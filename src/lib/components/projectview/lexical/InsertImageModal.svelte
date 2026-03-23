<!-- src/lib/components/projectview/lexical/InsertImageModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Button, Spinner } from 'flowbite-svelte';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { Image as ImageIcon, FileImage, Paperclip, FolderOpen, HardDrive } from '@lucide/svelte';

    export let showModal = false;
    export let documentPath = '';

    const dispatch = createEventDispatcher();

    let activeTab = 'attachments';
    let isLoadingAttachments = false;
    let localAttachments = [];
    let projectImages = [];

    $: if (showModal && documentPath) {
        activeTab = 'attachments';
        loadAttachments();
        loadProjectImages();
    }

    async function loadAttachments() {
        isLoadingAttachments = true;
        localAttachments = [];
        const projectStoreState = get(project);

        if (!projectStoreState.id || !documentPath) {
            isLoadingAttachments = false;
            return;
        }

        let relPath = documentPath;
        if (documentPath.startsWith(projectStoreState.baseDirectory)) {
            relPath = documentPath.substring(projectStoreState.baseDirectory.length);
            relPath = relPath.replace(/\\/g, '/').replace(/^\//, '');
        }

        try {
            const result = await invoke('get_asset_metadata_command', {
                projectId: projectStoreState.id,
                assetRelativePath: relPath
            });

            if (result && result.custom_fields_json) {
                const customFields = JSON.parse(result.custom_fields_json);
                const attachmentsField = customFields.find(f => f.key === 'attachments');
                if (attachmentsField && attachmentsField.value) {
                    const fileAttachments = JSON.parse(attachmentsField.value);
                    localAttachments = fileAttachments.filter(a => typeof a === 'string' && /\.(png|jpe?g|gif|webp|svg)$/i.test(a));
                }
            }
        } catch (error) {
            console.error('Error loading attachments for modal:', error);
        } finally {
            isLoadingAttachments = false;
        }
    }

    function loadProjectImages() {
        const p = get(project);
        const base = (p.baseDirectory || '').replace(/\/+$/, '');
        projectImages = (p.imageFiles || []).map(img => ({
            name: img.name,
            path: img.relativePath ? `${base}/${img.relativePath}` : ''
        })).filter(img => img.path);
    }

    function handleSelectAttached(attachmentPath) {
        dispatch('insert_attached', { path: attachmentPath });
        showModal = false;
    }

    function handleSelectProjectImage(imgObj) {
        dispatch('insert_external', { path: imgObj.path });
        showModal = false;
    }

    async function handleSelectLocalFile() {
        try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({
                multiple: false,
                filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'] }]
            });
            if (selected) {
                dispatch('insert_external', { path: selected });
                showModal = false;
            }
        } catch (e) {
            console.error(e);
        }
    }

    function getFileName(path) {
        return path ? path.split(/[/\\]/).pop() : '';
    }

    function getThumbnailSrc(path) {
        try { return convertFileSrc(path); } catch { return ''; }
    }
</script>

<Modal bind:open={showModal} size="lg" autoclose={false} outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
>
    <div slot="header" class="flex items-center gap-2">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <ImageIcon class="w-5 h-5 text-blue-600 dark:text-blue-400" />
        </div>
        <div class="flex flex-col">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white leading-tight">Insert Image</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Choose from attachments or upload</p>
        </div>
    </div>

    <div class="flex min-h-[18rem]">
        <!-- Left vertical tab nav -->
        <div class="flex flex-col border-r border-gray-200 dark:border-gray-700 w-36 shrink-0 -ml-2">
            <button
                class="flex items-center gap-2 px-3 py-3 text-sm font-medium text-left transition-colors
                    {activeTab === 'attachments'
                        ? 'bg-blue-50 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 border-l-2 border-blue-500'
                        : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeTab = 'attachments'}
            >
                <Paperclip size={15} /> Attachments
            </button>
            <button
                class="flex items-center gap-2 px-3 py-3 text-sm font-medium text-left transition-colors
                    {activeTab === 'project'
                        ? 'bg-blue-50 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 border-l-2 border-blue-500'
                        : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeTab = 'project'}
            >
                <FileImage size={15} /> Images
            </button>
            <button
                class="flex items-center gap-2 px-3 py-3 text-sm font-medium text-left transition-colors
                    {activeTab === 'local'
                        ? 'bg-blue-50 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 border-l-2 border-blue-500'
                        : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                on:click={() => activeTab = 'local'}
            >
                <HardDrive size={15} /> From Files
            </button>
        </div>

        <!-- Right content panel -->
        <div class="flex-1 pl-4 overflow-hidden">
            {#if activeTab === 'attachments'}
                <div class="h-72 overflow-y-auto">
                    {#if isLoadingAttachments}
                        <div class="flex justify-center items-center h-full"><Spinner /></div>
                    {:else if localAttachments.length === 0}
                        <p class="text-sm text-gray-500 italic flex justify-center items-center h-full">No images attached to this document.</p>
                    {:else}
                        <div class="grid grid-cols-2 gap-2">
                            {#each localAttachments as attachment}
                                <button
                                    class="overflow-hidden border border-gray-200 dark:border-gray-700 hover:border-blue-400 dark:hover:border-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/30 rounded text-left text-sm transition-colors"
                                    on:click={() => handleSelectAttached(attachment)}
                                    title={getFileName(attachment)}
                                >
                                    <div class="w-full h-24 bg-gray-100 dark:bg-gray-800 flex items-center justify-center overflow-hidden">
                                        <img
                                            src={getThumbnailSrc(attachment)}
                                            alt={getFileName(attachment)}
                                            class="w-full h-full object-contain"
                                            on:error={(e) => e.currentTarget.style.display='none'}
                                        />
                                    </div>
                                    <p class="px-2 py-1 truncate text-xs text-gray-700 dark:text-gray-300">{getFileName(attachment)}</p>
                                </button>
                            {/each}
                        </div>
                    {/if}
                </div>
            {:else if activeTab === 'project'}
                <div class="h-72 overflow-y-auto">
                    {#if projectImages.length === 0}
                        <p class="text-sm text-gray-500 italic flex justify-center items-center h-full">No images found in this project.</p>
                    {:else}
                        <div class="grid grid-cols-2 gap-2">
                            {#each projectImages as img}
                                <button
                                    class="overflow-hidden border border-gray-200 dark:border-gray-700 hover:border-blue-400 dark:hover:border-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/30 rounded text-left text-sm transition-colors"
                                    on:click={() => handleSelectProjectImage(img)}
                                    title={img.name}
                                >
                                    <div class="w-full h-24 bg-gray-100 dark:bg-gray-800 flex items-center justify-center overflow-hidden">
                                        <img
                                            src={getThumbnailSrc(img.path)}
                                            alt={img.name}
                                            class="w-full h-full object-contain"
                                            on:error={(e) => e.currentTarget.style.display='none'}
                                        />
                                    </div>
                                    <p class="px-2 py-1 truncate text-xs text-gray-700 dark:text-gray-300">{img.name}</p>
                                </button>
                            {/each}
                        </div>
                    {/if}
                </div>
            {:else if activeTab === 'local'}
                <div class="h-72 flex flex-col justify-center items-center">
                    <HardDrive size={48} class="text-gray-400 mb-4" />
                    <p class="mb-4 text-sm text-gray-600 dark:text-gray-400 text-center px-4">
                        Select an image from your computer. It will be copied into the project's attachments folder.
                    </p>
                    <Button color="blue" on:click={handleSelectLocalFile}>
                        <FolderOpen size={16} class="mr-2" /> Browse Files...
                    </Button>
                </div>
            {/if}
        </div>
    </div>

    <div slot="footer" class="flex justify-end gap-2 w-full">
        <Button color="alternative" on:click={() => showModal = false}>Cancel</Button>
    </div>
</Modal>
