<!-- src/lib/components/projectview/data/shared_panels/RightBar.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import { FileCode, Bookmark, Paperclip } from 'lucide-svelte';

    export let itemType = null;

    const dispatch = createEventDispatcher();

    function handleTabClick(tabName) {
        const store = get(panelStateStore);
        if (store.activeInfoPanelTab === tabName) {
            panelStateStore.toggleInfoPanel();
        } else {
            panelStateStore.setActiveInfoPanelTab(tabName);
            if (store.infoPanelCollapsed) {
                panelStateStore.toggleInfoPanel();
            }
        }
        dispatch('tabchange', { tabName });
    }

</script>

<div class="flex flex-col items-center w-8 h-full bg-white dark:bg-gray-950 py-2 space-y-2 border-l border-gray-300 dark:border-gray-700">
    <button
        on:click={() => handleTabClick('metadata')}
        class="p-1 focus:outline-none transition-colors flex items-center justify-center"
        class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed}
        class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed}
        class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:hover:bg-gray-800={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        title="Metadata"
    >
        <FileCode class="w-5 h-5" />
    </button>

    <button
        on:click={() => handleTabClick('highlights')}
        class="p-1 focus:outline-none transition-colors flex items-center justify-center"
        class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed}
        class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed}
        class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:hover:bg-gray-800={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        title="Highlights"
    >
        <Bookmark class="w-5 h-5" />
    </button>

    {#if itemType === 'doc' || itemType === 'imported_transcript' || itemType === 'table'}
        <button
            on:click={() => handleTabClick('attachments')}
            class="p-1 focus:outline-none transition-colors flex items-center justify-center"
            class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed}
            class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed}
            class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:dark:hover:bg-gray-800={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            title="Attachments"
        >
            <Paperclip class="w-5 h-5" />
        </button>
    {/if}
</div>
