<!-- src/lib/components/projectview/notes/shared_panels/RightBar.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    const dispatch = createEventDispatcher();

    // New icon for Metadata tab
    const METADATA_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-file-earmark-code w-5 h-5" viewBox="0 0 16 16"> <path d="M14 4.5V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h5.5zm-3 0A1.5 1.5 0 0 1 9.5 3V1H4a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4.5z"/> <path d="M8.646 6.646a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1 0 .708l-2 2a.5.5 0 0 1-.708-.708L10.293 9 8.646 7.354a.5.5 0 0 1 0-.708m-1.292 0a.5.5 0 0 0-.708 0l-2 2a.5.5 0 0 0 0 .708l2 2a.5.5 0 0 0 .708-.708L5.707 9l1.647-1.646a.5.5 0 0 0 0-.708"/> </svg>`;

    let currentActiveTab = 'metadata'; // Default active tab

    // Subscribe to store changes if active tab is managed globally
    panelStateStore.subscribe(store => {
        if (store.activeInfoPanelTab) {
            currentActiveTab = store.activeInfoPanelTab;
        }
    });

    function setActiveTab(tabName) {
        panelStateStore.setActiveInfoPanelTab(tabName);
        dispatch('tabchange', { tabName });
    }

</script>

<div class="flex flex-col items-center w-8 h-full bg-white dark:bg-gray-700 py-2 space-y-2 shadow-md">
    <button
        on:click={() => setActiveTab('metadata')}
        class="p-2 rounded-md focus:outline-none transition-colors"
        class:text-blue-600={currentActiveTab === 'metadata'}
        class:dark:text-blue-400={currentActiveTab === 'metadata'}
        class:hover:bg-gray-300={currentActiveTab !== 'metadata'}
        class:dark:hover:bg-gray-600={currentActiveTab !== 'metadata'}
        class:text-gray-700={currentActiveTab !== 'metadata'}
        class:dark:text-gray-300={currentActiveTab !== 'metadata'}
        title="Metadata"
    >
        {@html METADATA_ICON_SVG}
    </button>
    <!-- Add other icon buttons for future tabs here -->
</div>

<style>
    /* Ensure the width is strictly controlled */
    .w-12 {
        width: 3rem; /* 48px */
    }
</style>
