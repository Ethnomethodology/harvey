<!-- src/lib/components/projectview/data/shared_panels/RightBar.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { get } from 'svelte/store';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    export let itemType = null;

    const dispatch = createEventDispatcher();

    const METADATA_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-file-earmark-code w-5 h-5" viewBox="0 0 16 16"> <path d="M14 4.5V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h5.5zm-3 0A1.5 1.5 0 0 1 9.5 3V1H4a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4.5z"/> <path d="M8.646 6.646a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1 0 .708l-2 2a.5.5 0 0 1-.708-.708L10.293 9 8.646 7.354a.5.5 0 0 1 0-.708m-1.292 0a.5.5 0 0 0-.708 0l-2 2a.5.5 0 0 0 0 .708l2 2a.5.5 0 0 0 .708-.708L5.707 9l1.647-1.646a.5.5 0 0 0 0-.708"/> </svg>`;
    const HIGHLIGHTS_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-bookmarks w-5 h-5" viewBox="0 0 16 16"> <path d="M2 4a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v11.5a.5.5 0 0 1-.777.416L7 13.101l-4.223 2.815A.5.5 0 0 1 2 15.5zm2-1a1 1 0 0 0-1 1v10.566l3.723-2.482a.5.5 0 0 1 .554 0L11 14.566V4a1 1 0 0 0-1-1z"/> <path d="M4.268 1H12a1 1 0 0 1 1 1v11.768l.223.148A.5.5 0 0 0 14 13.5V2a2 2 0 0 0-2-2H6a2 2 0 0 0-1.732 1"/> </svg>`;
    const ATTACHMENTS_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-paperclip w-5 h-5" viewBox="0 0 16 16"> <path d="M4.5 3a2.5 2.5 0 0 1 5 0v9a1.5 1.5 0 0 1-3 0V5a.5.5 0 0 1 1 0v7a.5.5 0 0 0 1 0V3a1.5 1.5 0 1 0-3 0v9a2.5 2.5 0 0 0 5 0V5a.5.5 0 0 1 1 0v7a3.5 3.5 0 1 1-7 0z"/> </svg>`;

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

<div class="flex flex-col items-center w-8 h-full bg-white dark:bg-gray-700 py-2 space-y-2">
    <button
        on:click={() => handleTabClick('metadata')}
        class="p-2 focus:outline-none transition-colors"
        class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed}
        class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed}
        class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:hover:bg-gray-600={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'metadata' && !$panelStateStore.infoPanelCollapsed)}
        title="Metadata"
    >
        {@html METADATA_ICON_SVG}
    </button>

    <button
        on:click={() => handleTabClick('highlights')}
        class="p-2 focus:outline-none transition-colors"
        class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed}
        class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed}
        class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:hover:bg-gray-600={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'highlights' && !$panelStateStore.infoPanelCollapsed)}
        title="Highlights"
    >
        {@html HIGHLIGHTS_ICON_SVG}
    </button>

    {#if itemType === 'doc'}
        <button
            on:click={() => handleTabClick('attachments')}
            class="p-2 focus:outline-none transition-colors"
            class:text-blue-600={$panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed}
            class:dark:text-blue-400={$panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed}
            class:hover:bg-gray-300={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:dark:hover:bg-gray-600={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:text-gray-700={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            class:dark:text-gray-300={!($panelStateStore.activeInfoPanelTab === 'attachments' && !$panelStateStore.infoPanelCollapsed)}
            title="Attachments"
        >
            {@html ATTACHMENTS_ICON_SVG}
        </button>
    {/if}
</div>
