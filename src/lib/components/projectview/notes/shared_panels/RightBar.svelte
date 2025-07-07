<!-- src/lib/components/projectview/notes/shared_panels/RightBar.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    const dispatch = createEventDispatcher();

    // Using a Heroicon "information-circle"
    const METADATA_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6"> <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm8.706-1.442c1.146-.573 2.437.463 2.126 1.706l-.709 2.836.042-.02a.75.75 0 0 1 .67 1.34l-.04.022c-1.147.573-2.438-.463-2.127-1.706l.71-2.836-.042.02a.75.75 0 1 1-.671-1.34l.041-.022ZM12 9a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z" clip-rule="evenodd" /> </svg>`;

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

<div class="flex flex-col items-center w-12 h-full bg-gray-200 dark:bg-gray-700 py-2 space-y-2 shadow-md">
    <button
        on:click={() => setActiveTab('metadata')}
        class="p-2 rounded-md focus:outline-none transition-colors"
        class:bg-blue-500={currentActiveTab === 'metadata'}
        class:text-white={currentActiveTab === 'metadata'}
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
