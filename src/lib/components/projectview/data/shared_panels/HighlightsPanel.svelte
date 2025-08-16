<!-- src/lib/components/projectview/data/shared_panels/HighlightsPanel.svelte -->
<script>
    import { onMount } from 'svelte';
    import { get } from 'svelte/store';
    import { project, highlightsLastUpdated } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';

    export let itemPath = null;
    export let itemType = null;

    let highlights = [];

    async function loadHighlights() {
        if (!itemPath || itemType !== 'doc') {
            highlights = [];
            return;
        }

        try {
            const highlightsJson = await invoke('load_lexical_highlights', {
                args: {
                    projectId: get(project).id,
                    documentPath: itemPath,
                }
            });

            if (highlightsJson) {
                highlights = JSON.parse(highlightsJson);
            } else {
                highlights = [];
            }
        } catch (error) {
            console.error('Error loading highlights:', error);
            highlights = [];
        }
    }

    onMount(() => {
        loadHighlights();
    });

    $: if (itemPath) {
        loadHighlights();
    }

    $: if ($highlightsLastUpdated) {
        loadHighlights();
    }
</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden py-2">
    <div class="text-sm font-semibold border-b pb-1 px-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0 flex items-center justify-between h-8 mb-2">
        <span class="ml-1">Highlights</span>
    </div>

    <div class="flex-grow overflow-y-auto overflow-x-hidden min-h-0 text-xs relative px-2">
        {#if highlights.length > 0}
            <ul class="space-y-2">
                {#each highlights as highlight}
                    <li class="p-2 rounded-md" style="background-color: {highlight.color}; color: {highlight.color === '#111827' ? '#ffffff' : '#000000'};">
                        <p class="font-semibold">{highlight.text}</p>
                    </li>
                {/each}
            </ul>
        {:else if itemType === 'doc'}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                No highlights present for this document.
            </p>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">
                Highlights are only available for document files.
            </p>
        {/if}
    </div>
</div>
