const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// 1. Fix Infinite Loop
content = content.replace(
    /let isEditingExisting = false;/g,
    `let isEditingExisting = false;\n    let prevOpen = false;`
);

content = content.replace(
    /\$: \{\n\s*if \(open\) \{\n\s*loadExistingCharts\(\)\.then\(\(\) => \{\n\s*if \(initialChart\) \{\n\s*selectExistingChart\(initialChart\);\n\s*initialChart = null; \/\/ Reset after loading once\n\s*\} else \{\n\s*\/\/ Force open to the fresh create screen\n\s*resetForm\(\);\n\s*activeTab = 'create';\n\s*isEditingExisting = false;\n\s*chartName = `Chart-\$\{existingCharts\.length \+ 1\}`;\n\s*\}\n\s*\}\);\n\s*\}\n\s*\}/g,
    `$: {
        if (open !== prevOpen) {
            prevOpen = open;
            if (open) {
                loadExistingCharts().then(() => {
                    if (initialChart) {
                        selectExistingChart(initialChart);
                        initialChart = null; // Reset after loading once
                    } else {
                        // Force open to the fresh create screen
                        resetForm();
                        activeTab = 'create';
                        isEditingExisting = false;
                        chartName = \`Chart-\${existingCharts.length + 1}\`;
                    }
                });
            }
        }
    }`
);

// 2. Remove old reactive statement
content = content.replace(
    /\/\/ Set initial chartName automatically when creation tab opens, but don't save yet\n\s*\$: if \(open && activeTab === 'create' && !isEditingExisting && !chartName\) \{\n\s*chartName = `Chart-\$\{existingCharts\.length \+ 1\}`;\n\s*\}/g,
    ``
);

// 3. Fix Left Panel UI
// Let's replace the whole `{#if activeTab === 'create'}` block carefully using substring splitting.
// We want to replace everything from `{#if activeTab === 'create'}` up to `<div class="text-sm font-medium` (which is Data Configuration)

const startStr = `{#if activeTab === 'create'}`;
const endStr = `<div class="text-sm font-medium`;
const startIndex = content.indexOf(startStr);
const endIndex = content.indexOf(endStr);

if (startIndex !== -1 && endIndex !== -1) {
    const before = content.substring(0, startIndex);
    const after = content.substring(endIndex);

    const newBlock = `{#if activeTab === 'create'}
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
                            <div class="text-sm text-gray-500 dark:text-gray-400 italic pt-4 border-t border-gray-200 dark:border-gray-700">
                                Select a chart type from the right panel and click Create to begin configuring data.
                            </div>
                        {:else}
                            `;

    content = before + newBlock + after;
}

fs.writeFileSync(file, content);
