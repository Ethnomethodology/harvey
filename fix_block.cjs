const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// There is one more issue:
// The `{:else}` block was originally matching `{#if !isEditingExisting}` inside `{#if activeTab === 'create'}`.
// So we now have:
// {#if !isEditingExisting}
//   ...
// {:else}
//   <div class="text-sm font-medium ...>Data Configuration</div>
//   {#if selectedChartType === 'bar' ...}
//   ...
//   {/if}
//   <div class="pt-2"><Toggle bind:checked={showLegend}>Show Legend</Toggle></div>
// </div> <-- This closes `<div class="space-y-4">`
// {/if} <-- This used to close `{#if !isEditingExisting}`
//
// Wait, in my replacement string, I did:
// `{#if !isEditingExisting} <div ...>...</div> {:else} <div class="text-sm font-medium...`
// Then `after` starts with `<div class="text-sm font-medium...`
// The original code had:
// {#if !isEditingExisting}
//    <div class="text-sm text-gray-500 dark:text-gray-400 italic">Select a chart type from the right panel.</div>
// {:else}
//    <div class="space-y-4">
//        <div><Label...><Input...></div>
//        <div><Label...><Textarea...></div>
//        <div class="text-sm font-medium text-gray-700 dark:text-gray-300 border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
//             Data Configuration
//        </div>
//
// My replacement replaced from `{#if activeTab === 'create'}` to `<div class="text-sm font-medium...`
// Let's verify the `{#if !isEditingExisting}` is properly closed in the current file.
