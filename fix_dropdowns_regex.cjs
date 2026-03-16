const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// I also need to make sure `c.title` isn't mapping `c.name: c.title`.
// The mappings end with `.map(c => ({ value: c.field, name: c.title }))`.
// If `c.title` is a string (fieldName), it's fine.
// I replaced `const title = c.title || fieldName;` with `const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel`.
// So `c.title` inside the map output `{ value: c.field, name: c.title }` refers to the `title` variable inside `columns.map`. Wait.

content = content.replace(
    /\.map\(c => \(\{ value: c\.field, name: c\.title \}\)\)/g,
    `.map(c => ({ value: c.field, name: c.title }))`
);

// Wait, the first map returns `{ field: fieldName, title }`. So `c.title` in the LAST map IS the string `title` from the first map!
// `columns.map(c => { ... return { field: fieldName, title: title}; }).filter(...).map(c => ({ value: c.field, name: c.title }))`
// This perfectly passes the string down to the Flowbite Select!
