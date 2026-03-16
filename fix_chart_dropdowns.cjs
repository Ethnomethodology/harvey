const fs = require('fs');

const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The dropdowns are empty. `numericColumns`, `dateColumns`, `allColumns`.
// They map over `columns`. `columns` is passed from TableViewerPanel.
// `columns` is an array of Tabulator ColumnComponents.
// BUT Tabulator ColumnComponents `getField()` only works when the table is alive. In modal it might not.
// Wait, TableViewerPanel calls `<ChartModal columns={tableColumnsForModal} ... />`
// `tableColumnsForModal` is `tabulatorInstance.getColumnDefinitions().filter(c => c.field && c.field !== 'harvey_internal_id');`
// So it's an array of objects with `.field`, NOT components!
// This explains `c.getField()` failing or returning undefined.
// `c.getField === 'function' ? c.getField() : c.field` should be `c.field`.
// Let's replace `allColumns` with `categoricalColumns`.

content = content.replace(
    /\$: numericColumns = columns\.map\(c => \{\s*const fieldName = typeof c\.getField === 'function' \? c\.getField\(\) : c\.field;\s*const title = c\.title \|\| fieldName;\s*return \{ field: fieldName, title \};\s*\}\)\.filter\(c => \{\s*const colSchema = schema\[c\.field\];\s*if \(colSchema && colSchema\.type === 'Numeric'\) return true;\s*\/\/ Fallback if schema not well defined\s*return tableData\.some\(row => row\[c\.field\] !== null && row\[c\.field\] !== undefined && row\[c\.field\] !== '' && !isNaN\(parseFloat\(row\[c\.field\]\)\) && isFinite\(row\[c\.field\]\)\);\s*\}\)\.map\(c => \(\{ value: c\.field, name: c\.title \}\)\);/g,
    `$: numericColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = c.title || fieldName;
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (colSchema && colSchema.type === 'Numeric') return true;
        // Fallback if schema not well defined
        return tableData.some(row => row[c.field] !== null && row[c.field] !== undefined && row[c.field] !== '' && !isNaN(parseFloat(row[c.field])) && isFinite(row[c.field]));
    }).map(c => ({ value: c.field, name: c.title }));`
);

content = content.replace(
    /\$: dateColumns = columns\.map\(c => \{\s*const fieldName = typeof c\.getField === 'function' \? c\.getField\(\) : c\.field;\s*const title = c\.title \|\| fieldName;\s*return \{ field: fieldName, title \};\s*\}\)\.filter\(c => \{\s*const colSchema = schema\[c\.field\];\s*if \(colSchema && colSchema\.type === 'DateTime'\) return true;\s*\/\/ Fallback if schema not well defined\s*return tableData\.some\(row => \{\s*const val = row\[c\.field\];\s*return val && !isNaN\(Date\.parse\(val\)\);\s*\}\);\s*\}\)\.map\(c => \(\{ value: c\.field, name: c\.title \}\)\);/g,
    `$: dateColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = c.title || fieldName;
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (colSchema && colSchema.type === 'DateTime') return true;
        // Fallback if schema not well defined
        return tableData.some(row => {
            const val = row[c.field];
            return val && !isNaN(Date.parse(val));
        });
    }).map(c => ({ value: c.field, name: c.title }));`
);

content = content.replace(
    /\$: allColumns = columns\.map\(c => \{\s*const fieldName = typeof c\.getField === 'function' \? c\.getField\(\) : c\.field;\s*const title = c\.title \|\| fieldName;\s*return \{ value: fieldName, name: title \};\s*\}\);/g,
    `$: categoricalColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = c.title || fieldName;
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (!colSchema) return true; // Fallback if no schema
        if (colSchema.type === 'Text' && colSchema.subType === 'Small Text') return true;
        if (colSchema.type === 'Misc' && colSchema.subType === 'Selectbox') return true;
        if (colSchema.type === 'Numeric') return true;
        if (colSchema.type === 'DateTime') return true;
        return false;
    }).map(c => ({ value: c.field, name: c.title }));

    // Also keep allColumns for backwards compatibility or fallback if needed
    $: allColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = c.title || fieldName;
        return { value: fieldName, name: title };
    });`
);

// Replace usages of allColumns for Category dropdowns with categoricalColumns
content = content.replace(
    /<Select id="xAxisCol" items=\{allColumns\} bind:value=\{xAxisCol\} \/>/g,
    `<Select id="xAxisCol" items={categoricalColumns} bind:value={xAxisCol} />`
);
content = content.replace(
    /<Select id="categoryCol" items=\{allColumns\} bind:value=\{categoryCol\} \/>/g,
    `<Select id="categoryCol" items={categoricalColumns} bind:value={categoryCol} />`
);
content = content.replace(
    /<Select id="taskCol" items=\{allColumns\} bind:value=\{taskCol\} \/>/g,
    `<Select id="taskCol" items={categoricalColumns} bind:value={taskCol} />`
);

fs.writeFileSync(file, content);
