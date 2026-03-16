const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// Replace double declarations
content = content.replace(/let prevOpen = false;\n\s*let prevOpen = false;/g, 'let prevOpen = false;');

fs.writeFileSync(file, content);
