const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The user wants Name & Description on the first screen (before clicking Create), and Data Configuration on the second screen.
// Let's modify the Left panel `activeTab === 'create'` block.

content = content.replace(
    /{#if activeTab === 'create'}\n\s*{#if !isEditingExisting}\n\s*<div class="text-sm text-gray-500 dark:text-gray-400 italic">Select a chart type from the right panel.<\/div>\n\s*{:else}/g,
    `{#if activeTab === 'create'}
                    <div class="space-y-4">
                        <div>
                            <Label for="chartName" class="mb-2">Chart Name</Label>
                            <Input id="chartName" bind:value={chartName} placeholder="Enter chart name" />
                        </div>
                        <div>
                            <Label for="chartDescription" class="mb-2">Description</Label>
                            <Textarea id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
                        </div>

                        {#if !isEditingExisting}
                            <div class="text-sm text-gray-500 dark:text-gray-400 italic pt-4 border-t border-gray-200 dark:border-gray-700">Select a chart type from the right panel and click Create to begin.</div>
                        {:else}`
);

// We need to also remove the duplicate Name/Description inputs that were originally inside the `{:else}` block.
// Those inputs are:
//                            <div>
//                                <Label for="chartName" class="mb-2">Chart Name</Label>
//                                <Input id="chartName" bind:value={chartName} placeholder="Enter chart name" />
//                            </div>
//                            <div>
//                                <Label for="chartDescription" class="mb-2">Description</Label>
//                                <Textarea id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
//                            </div>
// Let's do string replacement for the duplicate lines we just moved.

content = content.replace(
    /<div class="space-y-4">\n\s*<div>\n\s*<Label for="chartName" class="mb-2">Chart Name<\/Label>\n\s*<Input id="chartName" bind:value=\{chartName\} placeholder="Enter chart name" \/>\n\s*<\/div>\n\s*<div>\n\s*<Label for="chartDescription" class="mb-2">Description<\/Label>\n\s*<Textarea id="chartDescription" bind:value=\{chartDescription\} placeholder="Optional description" rows="2" \/>\n\s*<\/div>/,
    `<div class="space-y-4">` // Wait, this matches the one we JUST added!
);

fs.writeFileSync(file, content);
