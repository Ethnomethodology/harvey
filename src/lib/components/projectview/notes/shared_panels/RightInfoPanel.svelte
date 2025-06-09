<!-- src/lib/components/projectview/notes/shared_panels/RightInfoPanel.svelte -->
<script>
    import { onMount, onDestroy } from 'svelte';
    import { slide } from 'svelte/transition';
    import { sineInOut } from 'svelte/easing';
    import StickiesIcon from '$lib/components/icons/StickiesIcon.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import CategoryTooltip from '../CategoryTooltip.svelte';

    // This panel is now generic, props might be needed later
    // export let itemPath = null;
    // export let itemType = null; // e.g., 'document', 'table'

    let tooltipVisible = false;
    let tooltipContentName = 'Highlights';
    let tooltipContentFiles = [];
    let tooltipX = 0;
    let tooltipY = 0;

    function showTooltip(event) {
        if (!$panelStateStore.rightCollapsed) {
            return;
        }
        const buttonRect = event.currentTarget.getBoundingClientRect();
        tooltipContentName = 'Highlights Panel';
        tooltipContentFiles = [{ name: 'View and manage item metadata and highlights.' }];
        tooltipX = buttonRect.right + 8;
        tooltipY = buttonRect.top;
        tooltipVisible = true;
    }

    function hideTooltip() {
        tooltipVisible = false;
    }

    onMount(() => {
        console.log('[RightInfoPanel] Mounted.');
        // Potentially load analysis/coding tools based on itemPath/itemType here
    });
</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden transition-all duration-300 ease-in-out p-2"
      class:w-full={!$panelStateStore.rightCollapsed}
      class:w-12={$panelStateStore.rightCollapsed} >
    <h2 class="text-sm font-semibold border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center h-7"
        class:mb-3={!$panelStateStore.rightCollapsed}
        class:mb-0={$panelStateStore.rightCollapsed}
        class:justify-start={!$panelStateStore.rightCollapsed}
        class:justify-center={$panelStateStore.rightCollapsed} >
        <button
            on:click={panelStateStore.toggleRightPanel}
            class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            title={$panelStateStore.rightCollapsed ? 'Expand Highlights Panel' : 'Collapse Highlights Panel'}
            on:mouseenter={showTooltip}
            on:mouseleave={hideTooltip}
            on:focus={showTooltip}
            on:blur={hideTooltip}
        >
            <StickiesIcon class="w-4 h-4"/>
        </button>
        {#if !$panelStateStore.rightCollapsed}
            <span class="ml-2">Highlights</span>
        {/if}
    </h2>
    {#if !$panelStateStore.rightCollapsed}
    <div class="flex-grow overflow-y-auto min-h-0" transition:slide={{ duration: 300, easing: sineInOut }}>
         <p class="text-xs text-gray-500 dark:text-gray-400 italic px-1 py-2">
            Coding tools or analysis features will appear here. (Placeholder)
        </p>
        <!-- Example: {#if itemType === 'document'} Show coding tools {/if} -->
        <!-- Example: {#if itemType === 'table'} Show chart options {/if} -->
    </div>
    {/if}
</div>

<CategoryTooltip
    bind:visible={tooltipVisible}
    categoryName={tooltipContentName}
    files={tooltipContentFiles}
    x={tooltipX}
    y={tooltipY}
/>

 <style lang="postcss">
    .min-h-0 { min-height: 0; }
    /* Add scrollbar styles if needed, similar to other panels */
    .overflow-y-auto::-webkit-scrollbar { @apply w-[6px] h-[6px]; }
    .overflow-y-auto::-webkit-scrollbar-track { @apply bg-transparent; }
    .overflow-y-auto::-webkit-scrollbar-thumb { @apply rounded bg-gray-400/50 dark:bg-gray-500/50; }
    .overflow-y-auto::-webkit-scrollbar-thumb:hover { @apply bg-gray-500/70 dark:bg-gray-400/70; }
    .overflow-y-auto { scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
    :root { --scrollbar-thumb: rgba(156, 163, 175, 0.5); --scrollbar-track: transparent; }
    html.dark { --scrollbar-thumb: rgba(107, 114, 128, 0.5); }
</style>