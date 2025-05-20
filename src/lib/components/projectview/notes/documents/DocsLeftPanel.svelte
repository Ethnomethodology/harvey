<!-- src/lib/components/projectview/documents/DocsLeftPanel.svelte -->
<script>
    import { onMount } from 'svelte';

    onMount(() => {
        console.log('[DocsLeftPanel] Mounted.');
    });
</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow p-3 flex flex-col overflow-hidden">
    <h2 class="text-sm font-semibold mb-3 border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 flex-shrink-0">
        Document Outline/Info
    </h2>
    <div class="flex-grow overflow-y-auto min-h-0">
        <p class="text-xs text-gray-500 dark:text-gray-400 italic px-1 py-2">
            Document structure or metadata will appear here. (Placeholder)
        </p>
    </div>
</div>

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