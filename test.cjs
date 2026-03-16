const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

if (content.includes('categoricalColumns')) {
    console.log("Categorical columns injected correctly.");
} else {
    console.log("Categorical columns NOT injected!");
}

if (content.includes('items={categoricalColumns}')) {
    console.log("HTML replaced correctly.");
} else {
    console.log("HTML NOT replaced!");
}
