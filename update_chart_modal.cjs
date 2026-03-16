const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The issue currently: the user wants the left panel to just show nothing, or a placeholder "Select a chart type from the right panel and click Create to begin"
// Currently it shows this:
//                     {#if !isEditingExisting}
//                         <div class="text-sm text-gray-500 dark:text-gray-400 italic">Select a chart type from the right panel.</div>

// When `isEditingExisting` is true (after Create), the Name, Description, and Data Configuration fields are shown.

// Wait, the user said:
// "on the first screen when the Insert Chart modal opens show chart name, description, and Data Configuration on the left. these are not mandatory but show them on the first screen."
// "No, left them for next screen. Once the chart type is selected show create button. if clicked create, create the chart and in the next screen show the dropdowns based on the created chart."

// This means my CURRENT code is already doing exactly what they want in the second message, or maybe not exactly?
// Ah! Wait. Look at the code. `isEditingExisting` controls the display of the form.
// Does the form currently show the dropdowns empty? Yes, the bug the user reported was "see why the dropdowns under Data Configuration always empty. they should show the numeric fields available in the underlying table for numeric dropdowns and short text field for the categorical dropdown. see why the modal is failing to populate these dropdowns."
// AND the user said: "After creating a plot if I close or after opening a plot from the attachments panel if I close, I cant get to the first screen when I click on Insert Charts again. It reopens the last opened chart. fix that."
