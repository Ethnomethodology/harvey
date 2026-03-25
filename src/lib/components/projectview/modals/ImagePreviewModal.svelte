<script>
    import { Modal, Button } from 'flowbite-svelte';
    import { createEventDispatcher } from 'svelte';
    import { ImageIcon, Trash2, Plus, X } from '@lucide/svelte';
    import { convertFileSrc } from '@tauri-apps/api/core';

    export let showModal = false;
    export let imagePath = '';

    const dispatch = createEventDispatcher();

    function getFileName(path) {
        return path ? path.split(/[/\\]/).pop() : '';
    }

    function getThumbnailSrc(path) {
        try {
            return convertFileSrc(path);
        } catch {
            return '';
        }
    }

    function handleInsert() {
        dispatch('insert', { path: imagePath });
        showModal = false;
    }

    function handleDelete() {
        dispatch('delete', { path: imagePath });
        showModal = false;
    }

    function handleCancel() {
        showModal = false;
        dispatch('cancel');
    }
</script>

<Modal
    bind:open={showModal}
    size="lg"
    autoclose={false}
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-0 bg-white dark:bg-gray-900"
    headerClass="px-4 py-3 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50 dark:bg-gray-800"
    footerClass="px-4 py-3 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50 dark:bg-gray-800"
>
    <div slot="header" class="flex items-center gap-2">
        <div class="p-1.5 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <ImageIcon class="w-4 h-4 text-blue-600 dark:text-blue-400" />
        </div>
        <h3 class="text-base font-semibold text-gray-900 dark:text-white truncate" title={getFileName(imagePath)}>
            {getFileName(imagePath)}
        </h3>
    </div>

    <!-- Image Preview Area -->
    <div class="w-full h-[400px] flex items-center justify-center bg-gray-100 dark:bg-gray-950 overflow-hidden relative">
        {#if imagePath}
            <img
                src={getThumbnailSrc(imagePath)}
                alt="Attachment Preview"
                class="max-w-full max-h-full object-contain"
            />
        {:else}
            <p class="text-sm text-gray-500 italic">No image to preview.</p>
        {/if}
    </div>

    <!-- Footer Actions -->
    <div slot="footer" class="flex justify-between w-full">
        <Button color="red" on:click={handleDelete} class="gap-2">
            <Trash2 class="w-4 h-4" /> Delete
        </Button>
        <div class="flex gap-2">
            <Button color="alternative" on:click={handleCancel}>
                <X class="w-4 h-4 mr-2" /> Cancel
            </Button>
            <Button color="blue" on:click={handleInsert}>
                <Plus class="w-4 h-4 mr-2" /> Insert
            </Button>
        </div>
    </div>
</Modal>
