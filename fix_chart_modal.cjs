const fs = require('fs');

const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The issue of reopening the last opened chart is because `initialChart` and internal state is not resetting properly when `open` becomes true.
// The user also mentioned:
// "on the first screen when the Insert Chart modal opens show chart name, description, and Data Configuration on the left. these are not mandatory but show them on the first screen."
// "1. You're correct, lets just show Name, description on the left panel on first screen. Name, description and Data Configuration should come in the 2nd screen. "
// "No, left them for next screen. Once the chart type is selected show create button. if clicked create, create the chart and in the next screen show the dropdowns based on the created chart. "

// So the requirement: Left panel should not have Name, Description, and Data Config on the first screen.
// ONLY after clicking "Create Chart" they should appear.
// My code already has:
// {#if !isEditingExisting} <div class="...">Select a chart type...</div> {:else} <form>...
// And the "Create Chart" button is on the right. When clicked, it calls `initialCreate()`, setting `isEditingExisting = true`, which shows the form!
// Wait, the user said: "on the first screen when the Insert Chart modal opens show chart name, description, and Data Configuration on the left. these are not mandatory but show them on the first screen."
// That was their ORIGINAL request.
// Then I asked: "First Screen UI (Create mode without chart type selected): Left panel will show the Chart Name input and Description textarea immediately. The Data Configuration section will NOT be visible yet. ... Confirmation: Only once a chart type is clicked on the right will the Data Configuration section appear on the left, correct?"
// The user replied: "No, left them for next screen. Once the chart type is selected show create button. if clicked create, create the chart and in the next screen show the dropdowns based on the created chart. "

// So the user is telling me to keep Name, Description, and Data Config ALL on the second screen, exactly like `isEditingExisting` works now.
// The issue is likely just that the dropdowns are empty, the delete bug, and the reopen bug.
// Let's fix the Reopen Bug (Plan Step 1 / 4).
// Let's modify the `$: { if (open) {` block.

content = content.replace(
    `$: {
        if (open) {
            loadExistingCharts().then(() => {
                if (initialChart) {
                    selectExistingChart(initialChart);
                    initialChart = null; // Reset after loading once
                }
            });
        }
    }`,
    `$: {
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
    }`
);

fs.writeFileSync(file, content);
