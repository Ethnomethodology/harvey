const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// In ChartModal.svelte, `columns` is `tableColumnsForModal` which is `tabulatorInstance.getColumnDefinitions()`.
// This array contains objects like { field: 'Date', title: [HTMLDivElement] ... }
// Svelte Flowbite `<Select items={categoricalColumns}>` expects `{ name: 'String', value: 'String' }`.
// If `name` is an HTMLDivElement or empty, it won't render the text.
// If we look at how `columns` is built in `TableViewerPanel.svelte`, `title` is generated via a function returning an HTML container with an icon.
// BUT `c.title` in `TableViewerPanel` for Tabulator column definitions is `(() => { const container = document.createElement("div"); ... return container; })()`.
// So `c.title` IS A DOM NODE!
// When we map over `columns` to create the dropdown options:
// const title = c.title || fieldName;
// `title` becomes an HTMLDivElement!
// Svelte's `<Select>` will stringify it to `"[object HTMLDivElement]"` or just fail.
// So we must use `c.field` for the name, OR extract the text from the DOM node!

content = content.replace(
    /const title = c\.title \|\| fieldName;/g,
    `const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel`
);

fs.writeFileSync(file, content);
