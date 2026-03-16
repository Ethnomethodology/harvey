const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// Replace the reactive $: if (open) block with an onMount or an explicit watch on open
// Actually we can just track `prevOpen` in Svelte script:
// let prevOpen = false;
// $: if (open && !prevOpen) { ... } prevOpen = open;
//
// But let's look at what I did:
// $: {
//     if (open) {
//         loadExistingCharts().then(() => {
//             if (initialChart) {
//                 selectExistingChart(initialChart);
//                 initialChart = null; // Reset after loading once
//             } else {
//                 resetForm();
//                 activeTab = 'create';
//                 isEditingExisting = false;
//                 chartName = `Chart-${existingCharts.length + 1}`;
//             }
//         });
//     }
// }
// `chartName = Chart-${existingCharts.length + 1}` changes state, but wait: `existingCharts` is updated by `loadExistingCharts`.
// Because Svelte reactive blocks `$: { ... }` re-run ANY TIME ANY VARIABLE INSIDE CHANGES.
// So when `existingCharts` changes, it re-runs.
// When `activeTab` changes, it re-runs!
// When `isEditingExisting` changes, it re-runs!
// We MUST track `prevOpen` to break the cycle.

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

// We also have this other block causing a loop:
// $: if (open && activeTab === 'create' && !isEditingExisting && !chartName) {
//     chartName = `Chart-${existingCharts.length + 1}`;
// }
// We can just remove it since we do it in the open block now!

content = content.replace(
    /\/\/ Set initial chartName automatically when creation tab opens, but don't save yet\n\s*\$: if \(open && activeTab === 'create' && !isEditingExisting && !chartName\) \{\n\s*chartName = `Chart-\$\{existingCharts\.length \+ 1\}`;\n\s*\}/g,
    ``
);

fs.writeFileSync(file, content);
