<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get, writable } from 'svelte/store';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import {
        loadTableData,
        saveTableData,
        saveTableLayoutPrefs,
        loadTableLayoutPrefs,
        renameTableHeader,
        deleteTableColumn,
        saveTableStyles,
        loadTableStyles,
        saveTableHighlights,
        loadTableHighlights
    } from '$lib/services/projectService.js';
    import { project, setTableHighlights, setLoadedTableHighlights } from '$lib/stores/projectStore.js';
    import { sep } from '@tauri-apps/api/path';
    import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';

    export let tablePath = '';
    export let hasHeaders = true;

    let tableContainer;
    let tabulatorInstance = null;
    let tableData = [];
    let isLoading = true;
    let error = null;
    let currentLoadedPath = null;

    const highlightOptions = HIGHLIGHT_OPTIONS;

    let tableStyles = { rowStyles: {}, cellStyles: {} }; // This will be derived from highlights
    let searchTerm = '';
    let cellMatches = []; // Changed from searchMatches to store cell components
    let currentMatchIndex = -1;
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };
    let tableClipboard = null;
    let searchInputRef = null;

    let showOptionsMenu = false;
    let areFiltersVisible = true; // Start with the assumption that filters are visible

    async function toggleFilters() {
        if (!tabulatorInstance) return;
        areFiltersVisible = !areFiltersVisible;
        const columns = tabulatorInstance.getColumns();

        // Use Promise.all to apply all updates concurrently.
        await Promise.all(
            columns.map(column => {
                const definition = column.getDefinition();
                // Only update columns that actually have a header filter defined.
                if (definition.field && definition.headerFilter) {
                    return tabulatorInstance.updateColumnDefinition(definition.field, {
                        headerFilterVisible: areFiltersVisible
                    });
                }
                return Promise.resolve(); // No action needed for this column.
            })
        );
        showOptionsMenu = false; // Hide menu after action
    }

    const saveCurrentTableLayout = debounce(async () => {
        if (!tabulatorInstance || !currentLoadedPath) return;

        // Redraw rows to recalculate height after resize
        tabulatorInstance.redraw(true);

        const baseDirForSave = get(project)?.baseDirectory;
        const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
        if (!baseDirForSave || !relativePathForSave) return;
        updateTableLayoutSnapshot();
        await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
    }, 750);

    async function saveCurrentTableLayoutImmediately() {
        if (!tabulatorInstance || !currentLoadedPath) return;
        const baseDirForSave = get(project)?.baseDirectory;
        const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
        if (!baseDirForSave || !relativePathForSave) return;
        updateTableLayoutSnapshot();
        await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
    }

    async function saveTableChanges() {
        if (!tabulatorInstance) return;
        const updatedData = tabulatorInstance.getData();
        tableData = updatedData;

        // Strip the internal ID before saving
        const dataToSave = JSON.parse(JSON.stringify(updatedData));
        dataToSave.forEach(row => delete row.harvey_internal_id);

        await saveTableData(tablePath, dataToSave);
    }

    const debouncedSave = debounce(saveTableChanges, 750);

    function getUniqueColumnName(baseName) {
        if (!tabulatorInstance) return baseName;
        let newName = baseName;
        let i = 1;
        while (tabulatorInstance.getColumn(newName)) {
            newName = `${baseName}_${i}`;
            i++;
        }
        return newName;
    }

    // Column Actions
    async function copyColumn(column) {
        const field = column.getField();
        const values = tabulatorInstance.getRows().map(row => row.getData()[field]);
        tableClipboard = {
            header: column.getDefinition().title,
            values: values,
            type: 'column'
        };
    }

    async function cutColumn(column) {
        await copyColumn(column);
        await deleteColumn(column);
    }

    async function deleteColumn(column) {
        const columnName = column.getField();
        try {
            // Step 1: Capture the current layout BEFORE doing anything else.
            updateTableLayoutSnapshot();
            const layoutBeforeDelete = tableLayoutSnapshot;

            // Step 2: Call the backend to delete the column from the file.
            await deleteTableColumn(tablePath, columnName);

            // Step 3: Manually remove the deleted column from our captured layout.
            if (layoutBeforeDelete.columns[columnName]) {
                delete layoutBeforeDelete.columns[columnName];
            }

            // Step 4: Save the now-modified layout object.
            const projectBaseDir = get(project)?.baseDirectory;
            const relativeTablePath = getRelativePath(tablePath, projectBaseDir);
            if (relativeTablePath) {
                await saveTableLayoutPrefs(relativeTablePath, layoutBeforeDelete);
            }

            // Step 5: Reload the table. It will now load the correct data AND the correct layout.
            await initializeTable(tablePath, null, true);

        } catch (err) {
            console.error(`Error deleting column "${columnName}":`, err);
        }
    }

    async function insertColumn(column, position) {
        const newFieldName = getUniqueColumnName("NewColumn");
        const newColumnDef = { title: newFieldName, field: newFieldName, editor: "textarea", headerFilter: "input" };
        try {
            await tabulatorInstance.addColumn(newColumnDef, position === 'before', column);

            // Force Tabulator to update the data model for the new column
            const rows = tabulatorInstance.getRows();
            rows.forEach(row => {
                const cell = row.getCell(newFieldName);
                if (cell) {
                    cell.setValue("", true); // Set empty string, suppress cellEdited event
                }
            });

            await saveTableChanges();
            await saveCurrentTableLayoutImmediately();
        } catch (err) {
            console.error(`Error inserting column ${position} ${column.getField()}:`, err);
        }
    }

    async function pasteColumn(column, position) {
        if (!tableClipboard || tableClipboard.type !== 'column') {
            alert("No column data on clipboard.");
            return;
        }
        const newFieldName = getUniqueColumnName(tableClipboard.header);
        const newColumnDef = { title: tableClipboard.header, field: newFieldName, editor: "textarea", headerFilter: "input" };
        try {
            await tabulatorInstance.addColumn(newColumnDef, position === 'before', column);
            const rows = tabulatorInstance.getRows();
            rows.forEach((row, index) => {
                if (tableClipboard.values[index] !== undefined) {
                    row.getCell(newFieldName).setValue(tableClipboard.values[index], true);
                }
            });
            await saveTableChanges();
        } catch (err) {
            console.error(`Error pasting column ${position} ${column.getField()}:`, err);
        }
    }

    // Row Actions
    async function copyRow(row) {
        tableClipboard = { type: 'row', data: row.getData() };
    }

    async function cutRow(row) {
        await copyRow(row);
        await deleteRow(row);
    }

    async function deleteRow(row) {
        try {
            await row.delete();
            await saveTableChanges();
        } catch (err) {
            console.error("Error deleting row:", err);
        }
    }

    async function insertRow(row, position) {
        const newRowData = {};
        tabulatorInstance.getColumns().forEach(column => {
            if (column.getField()) {
                newRowData[column.getField()] = "";
            }
        });
        try {
            const addedRow = await tabulatorInstance.addRow(newRowData, position === 'before', row);

            // Workaround for suspected backend bug: "dirty" a cell to ensure the new row is saved.
            const cells = addedRow.getCells();
            if (cells.length > 0) {
                cells[0].setValue(" ", true); // Set a single space, suppress cellEdited event
            }

            await saveTableChanges();
        } catch (err) {
            console.error("Error inserting row:", err);
        }
    }

    async function pasteRow(row, position) {
        if (!tableClipboard || tableClipboard.type !== 'row') {
            alert("No row data on clipboard.");
            return;
        }
        try {
            await tabulatorInstance.addRow(tableClipboard.data, position === 'before', row);
            await saveTableChanges();
        } catch (err) {
            console.error("Error pasting row:", err);
        }
    }

    function updateTableLayoutSnapshot() {
        if (!tabulatorInstance) return;
        const currentColumnDefs = tabulatorInstance.getColumnDefinitions();
        const newSnapshotColumns = {};
        currentColumnDefs.forEach((colDef, index) => {
            if (colDef.field) {
                const columnComponent = tabulatorInstance.getColumn(colDef.field);
                if (columnComponent) {
                    newSnapshotColumns[colDef.field] = {
                        order: index,
                        visible: columnComponent.isVisible(),
                        width: columnComponent.getWidth(),
                    };
                }
            }
        });
        tableLayoutSnapshot.columns = newSnapshotColumns;
    }

    function getRelativePath(absolutePath, baseDir) {
        if (!absolutePath || !baseDir) return null;
        let relativePath = absolutePath;
        if (absolutePath.startsWith(baseDir)) {
            relativePath = absolutePath.substring(baseDir.length);
            if (relativePath.startsWith(sep) || relativePath.startsWith('/') || relativePath.startsWith('\\')) {
                relativePath = relativePath.substring(1);
            }
        }
        return relativePath.replaceAll('\\', '/');
    }

    function debounce(func, delay) {
        let timeout;
        return function(...args) {
            const context = this;
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(context, args), delay);
        };
    }

    async function applyHighlightToCells(color, cellsToModify) {
        if (!tabulatorInstance || !cellsToModify || cellsToModify.length === 0) return;
        const rowsToReformat = new Set();
        cellsToModify.forEach(cell => {
            const row = cell.getRow();
            const rowIndex = row.getData().harvey_internal_id;
            const colField = cell.getField();
            const cellKey = `cell-${rowIndex}-${colField}`;
            if (color) {
                tableStyles.cellStyles[cellKey] = color;
            } else {
                delete tableStyles.cellStyles[cellKey];
            }
            rowsToReformat.add(row);
        });
        rowsToReformat.forEach(row => row.reformat());
        const ranges = tabulatorInstance.getRanges();
        if (ranges) {
            ranges.forEach(range => range.remove());
        }
        try {
            await saveTableStyles(tablePath, {
                rowStyles: tableStyles.rowStyles,
                cellStyles: tableStyles.cellStyles
            });
        } catch (err) {
            console.error("Failed to save table styles:", err);
        }
    }

    async function applyHighlightToRows(color, rows) {
        if (!tabulatorInstance || !rows || rows.length === 0) return;

        let currentHighlights = get(project).currentTableHighlights || [];

        rows.forEach(row => {
            const rowData = row.getData();
            const rowIndex = rowData.harvey_internal_id;

            // Remove existing highlight for this row
            currentHighlights = currentHighlights.filter(h => h.id !== `row-${rowIndex}`);

            if (color) {
                // Add new highlight
                const text = Object.values(rowData).filter(val => val !== null && val !== undefined).join(' | ');
                const newHighlight = {
                    id: `row-${rowIndex}`,
                    color: color,
                    text: text,
                    tags: [],
                    comments: []
                };
                currentHighlights.push(newHighlight);
            }
        });

        setTableHighlights(currentHighlights);
        await saveTableHighlights();

        tableStyles = { rowStyles: {}, cellStyles: {} };
        currentHighlights.forEach(h => {
            if (h.id.startsWith('row-')) {
                const rowIndex = h.id.substring(4);
                tableStyles.rowStyles[rowIndex] = h.color;
            }
        });
        rows.forEach(row => row.reformat());

        const ranges = tabulatorInstance.getRanges();
        if (ranges) {
            ranges.forEach(range => range.remove());
        }
    }

    // Custom header filter editor to prevent Enter key propagation
    function customHeaderFilterEditor(cell, onRendered, success, cancel, editorParams){
        var editor = document.createElement("input");
        editor.setAttribute("type", "text");
        editor.setAttribute("placeholder", "Filter...");
        editor.style.width = "100%";
        editor.style.boxSizing = "border-box";
        editor.style.padding = "4px";
        editor.style.border = "1px solid #ccc";
        editor.style.borderRadius = "3px";

        editor.value = cell.getValue();

        onRendered(function(){
            editor.focus();
            editor.style.css = "100%";
        });

        function successFunc(){
            success(editor.value);
        }

        editor.addEventListener("change", successFunc);
        editor.addEventListener("blur", successFunc);

        // Prevent Enter key from propagating
        editor.addEventListener("keydown", function(e){
            if(e.key === "Enter"){
                e.preventDefault();
                e.stopPropagation();
                successFunc(); // Apply filter on Enter
            }
            if(e.key === "Escape"){
                cancel();
            }
        });

        return editor;
    }

    function generateColumns(data, headers, savedLayoutObj) {
        if (!headers || headers.length === 0) return [{title: "No Data", field: "placeholder"}];
        let dataColumnDefs = headers.map(header => {
            const colDef = {
                title: header,
                field: header,
                headerFilter: customHeaderFilterEditor, // Use custom editor
                headerFilterPlaceholder: "Filter...", // Add a placeholder
                headerFilterFunc: function(headerValue, rowValue, rowData, filterParams){
                    // headerValue is the value from the header filter input
                    // rowValue is the value of the cell in the current row for this column
                    if (headerValue === null || headerValue === undefined || String(headerValue).trim() === "") {
                        return true; // Show all rows if filter is empty
                    }
                    if (rowValue === null || rowValue === undefined) {
                        return false; // Don't show rows with empty cell values if filter is not empty
                    }
                    return String(rowValue).toLowerCase().includes(String(headerValue).toLowerCase());
                },
                sorter: "string",
                editor: "textarea",
                editorParams:{ verticalNavigation:"editor", shiftEnterSubmit:true },
                formatter: (cell) => {
                    const rowIndex = cell.getRow().getData().harvey_internal_id;
                    const colField = cell.getField();
                    const cellKey = `cell-${rowIndex}-${colField}`;
                    const cellElement = cell.getElement();
                    const cellColor = tableStyles.cellStyles[cellKey];
                    cellElement.style.backgroundColor = cellColor || "";
                    if (cellColor) {
                        cellElement.classList.add('highlighted-cell');
                    } else {
                        cellElement.classList.remove('highlighted-cell');
                    }
                    cell.getElement().style.whiteSpace = "pre-wrap";

                    const term = searchTerm.trim();
                    const cellValue = cell.getValue();
                    if (term && cellValue !== null && cellValue !== undefined) {
                        const escapedTerm = term.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
                        const regex = new RegExp(`(${escapedTerm})`, 'gi');
                        return String(cellValue).replace(regex, '<span class="search-match-highlight">$1</span>');
                    }
                    return cellValue;
                },
                headerContextMenu: (column) => {
                    const menu = [
                        { label: "Edit Header", action: (e, column) => openHeaderEditor(column) },
                        { separator: true },
                        { label: "Sort Ascending", action: (e, column) => tabulatorInstance.setSort(column.getField(), 'asc') },
                        { label: "Sort Descending", action: (e, column) => tabulatorInstance.setSort(column.getField(), 'desc') },
                        { separator: true },
                        { label: "Cut Column", action: (e, column) => cutColumn(column) },
                        { label: "Copy Column", action: (e, column) => copyColumn(column) },
                    ];
                    if (tableClipboard && tableClipboard.type === 'column') {
                        menu.push({ label: "Paste Column Before", action: (e, column) => pasteColumn(column, 'before') });
                        menu.push({ label: "Paste Column After", action: (e, column) => pasteColumn(column, 'after') });
                    }
                    menu.push({ separator: true });
                    menu.push({ label: "Insert Column Before", action: (e, column) => insertColumn(column, 'before') });
                    menu.push({ label: "Insert Column After", action: (e, column) => insertColumn(column, 'after') });
                    menu.push({ separator: true });
                    menu.push({ label: "Delete Column", action: (e, column) => deleteColumn(column) });
                    return menu;
                },
                contextMenu: (e, cell) => {
                    e.preventDefault();
                    return [];
                }
            };
            if (savedLayoutObj?.columns?.[header]) {
                const savedCol = savedLayoutObj.columns[header];
                if (typeof savedCol.width === 'number' && savedCol.width > 0) colDef.width = savedCol.width;
                colDef.visible = savedCol.visible;
            }
            return colDef;
        });
        if (savedLayoutObj?.columns) {
            dataColumnDefs.sort((a, b) => (savedLayoutObj.columns[a.field]?.order ?? Infinity) - (savedLayoutObj.columns[b.field]?.order ?? Infinity));
        }
        return dataColumnDefs;
    }

    async function initializeTable(pathForTable, newHasHeaders = null, force = false) {
        if (newHasHeaders !== null) hasHeaders = newHasHeaders;
        if (!pathForTable || !tableContainer) return;
        if (isLoading && currentLoadedPath === pathForTable) return;
        if (!force && !isLoading && tabulatorInstance && currentLoadedPath === pathForTable) return;

        currentLoadedPath = pathForTable;
        isLoading = true;
        error = null;
        tableData = [];

        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }

        try {
            const loadedHighlightsOrStyles = await loadTableStyles(pathForTable);

            let highlightsForStore = [];
            tableStyles = { rowStyles: {}, cellStyles: {} };

            if (loadedHighlightsOrStyles) {
                if (Array.isArray(loadedHighlightsOrStyles)) {
                    // New format
                    highlightsForStore = loadedHighlightsOrStyles;
                    loadedHighlightsOrStyles.forEach(h => {
                        if (h.id.startsWith('row-')) {
                            const rowIndex = h.id.substring(4);
                            tableStyles.rowStyles[rowIndex] = h.color;
                        }
                    });
                } else if (typeof loadedHighlightsOrStyles === 'object' && loadedHighlightsOrStyles.rowStyles) {
                    // Old format, for backward compatibility
                    tableStyles.rowStyles = loadedHighlightsOrStyles.rowStyles || {};
                    tableStyles.cellStyles = loadedHighlightsOrStyles.cellStyles || {};
                    // Highlights panel will be empty for this table until new highlights are added.
                }
            }

            setLoadedTableHighlights(highlightsForStore);

            const response = await loadTableData(pathForTable, hasHeaders);
            tableData = response.data;
            tableData.forEach((d, i) => d.harvey_internal_id = i);
            const tableHeaders = response.headers;
            await tick();
            if (!tableContainer) {
                 error = 'Failed to initialize table viewer: container lost.';
                 isLoading = false;
                 return;
            }
            const projectBaseDir = get(project)?.baseDirectory;
            if (!projectBaseDir) {
                error = "Project configuration error: base directory missing.";
                isLoading = false;
                return;
            }
            const relativeTablePath = getRelativePath(pathForTable, projectBaseDir);
            if (!relativeTablePath) {
                error = "Error determining asset relative path.";
                isLoading = false;
                return;
            }
            let savedLayout = await loadTableLayoutPrefs(relativeTablePath).catch(e => console.error(`Error loading layout for ${relativeTablePath}:`, e));
            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                index: "harvey_internal_id",
                layout: "fitData",
                columns: generateColumns(tableData, tableHeaders, savedLayout, !savedLayout),
                height: "100%",
                placeholder: "No Data Available",
                selectableRange: 1,
                selectableRangeColumns: true,
                selectableRangeRows: true,
                history:true,
                editTriggerEvent:"dblclick",
                movableColumns: true,
                resizableColumnFit: false,
                rowFormatter: (row) => {
                    const rowIndex = row.getData().harvey_internal_id;
                    const rowColor = tableStyles.rowStyles[rowIndex];
                    const rowElement = row.getElement();
                    rowElement.style.backgroundColor = rowColor || "";
                    if (rowColor) {
                        rowElement.classList.add('highlighted-row');
                    } else {
                        rowElement.classList.remove('highlighted-row');
                    }
                },
                rowContextMenu: (e, row) => {
                    const ranges = tabulatorInstance.getRanges();
                    let selectedRowsForMenu = [row];

                    if (ranges.length > 0) {
                        const activeRange = ranges[0];
                        const rangeRows = activeRange.getRows();
                        if (rangeRows.some(r => r.getIndex() === row.getIndex())) {
                            selectedRowsForMenu = rangeRows;
                        }
                    }

                    const highlightAction = (color) => {
                        applyHighlightToRows(color, selectedRowsForMenu);
                    };

                    const highlightColorOptions = highlightOptions.map(option => ({
                        label: `<span style='display:inline-block; width:15px; height:15px; background-color:${option.value}; margin-right: 8px; vertical-align: middle;'></span>${option.label}`,
                        action: () => highlightAction(option.value)
                    }));

                    const menu = [
                        { label: "Cut Row", action: (e, row) => cutRow(row) },
                        { label: "Copy Row", action: (e, row) => copyRow(row) },
                    ];

                    if (tableClipboard && tableClipboard.type === 'row') {
                        menu.push({ label: "Paste Row Above", action: (e, row) => pasteRow(row, 'before') });
                        menu.push({ label: "Paste Row Below", action: (e, row) => pasteRow(row, 'after') });
                    }

                    menu.push({ separator: true });
                    menu.push({ label: "Insert Row Above", action: (e, row) => insertRow(row, 'before') });
                    menu.push({ label: "Insert Row Below", action: (e, row) => insertRow(row, 'after') });
                    menu.push({ separator: true });
                    menu.push({ label: "Delete Row", action: (e, row) => deleteRow(row) });
                    menu.push({ separator: true });
                    menu.push({ label: "Highlight Row", menu: highlightColorOptions });
                    menu.push({ label: "Clear Row Highlight", action: () => highlightAction(null) });

                    return menu;
                },
                columnDefaults: {
                    headerSort:false,
                    headerHozAlign:"center",
                    editor:"textarea",
                    editorParams:{ verticalNavigation:"editor", shiftEnterSubmit:true },
                    resizable:"header",
                    width:100,
                    minWidth: 50,
                },
                rowHeader:{
                    resizable: false,
                    frozen: true,
                    headerSort:false,
                    hozAlign:"center",
                    formatter: "rownum",
                    width: 50,
                    cssClass:"range-header-col"
                },
                clipboard: true,
                clipboardCopyStyled:false,
                clipboardCopyConfig:{ rowHeaders:false, columnHeaders:false },
                clipboardCopyRowRange:"range",
                clipboardPasteParser:"range",
                clipboardPasteAction:"range",
            });
            const saveCurrentTableLayout = debounce(async () => {
                if (!tabulatorInstance || !currentLoadedPath) return;
                const baseDirForSave = get(project)?.baseDirectory;
                const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
                if (!baseDirForSave || !relativePathForSave) return;
                updateTableLayoutSnapshot();
                await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
            }, 750);
            tabulatorInstance.on("columnResized", saveCurrentTableLayout);

            // Event-driven layout saving for structural changes
            tabulatorInstance.on("columnMoved", saveCurrentTableLayoutImmediately);
            tabulatorInstance.on("columnAdded", saveCurrentTableLayoutImmediately);
            tabulatorInstance.on("columnDeleted", saveCurrentTableLayoutImmediately);

            tabulatorInstance.on("cellEdited", (cell) => {
                debouncedSave();
                const row = cell.getRow();
                const rowData = row.getData();
                const rowIndex = rowData.harvey_internal_id;
                const highlightId = `row-${rowIndex}`;

                let currentHighlights = get(project).currentTableHighlights || [];
                const highlightIndex = currentHighlights.findIndex(h => h.id === highlightId);

                if (highlightIndex !== -1) {
                    const newText = Object.values(rowData).filter(val => val !== null && val !== undefined).join(' | ');
                    const updatedHighlights = [
                        ...currentHighlights.slice(0, highlightIndex),
                        { ...currentHighlights[highlightIndex], text: newText },
                        ...currentHighlights.slice(highlightIndex + 1)
                    ];
                    setTableHighlights(updatedHighlights);
                    saveTableHighlights();
                }
            });
            columnFields = tabulatorInstance.getColumnDefinitions().map(c => c.field).filter(Boolean);
        } catch (err) {
            error = `Failed to load table: ${err.message || err}`;
        } finally {
            isLoading = false;
        }
    }

    function handleSearch() {
        if (!tabulatorInstance) return;

        // Clear existing highlights and reset matches
        cellMatches.forEach(cell => {
            const el = cell.getElement();
            if (el) el.classList.remove('search-match-focus');
        });
        cellMatches = [];
        currentMatchIndex = -1;
        tabulatorInstance.redraw(true); // Redraw to clear old highlights from formatter

        const term = searchTerm.trim().toLowerCase();

        if (!term) {
            tabulatorInstance.clearFilter();
            return;
        }

        // Filter rows first
        tabulatorInstance.setFilter((data) => {
            for (const key in data) {
                if (key === 'harvey_internal_id') continue;
                const value = data[key];
                if (value !== null && value !== undefined && String(value).toLowerCase().includes(term)) {
                    return true;
                }
            }
            return false;
        });

        // After filtering, find all matching cells in the active (visible) rows
        const activeRows = tabulatorInstance.getRows('active');
        activeRows.forEach(row => {
            row.getCells().forEach(cell => {
                const cellValue = cell.getValue();
                if (cellValue !== null && cellValue !== undefined && String(cellValue).toLowerCase().includes(term)) {
                    cellMatches.push(cell);
                }
            });
        });

        // Redraw the table to apply the search term highlighting via the formatter
        tabulatorInstance.redraw(true);

        if (cellMatches.length > 0) {
            navigateToMatch(0);
        }

        // Restore focus to the search input after the search is complete
        if (searchInputRef) {
            searchInputRef.focus();
        }
    }

    async function navigateToMatch(index) {
        if (!tabulatorInstance || !cellMatches[index]) return;

        // Clear any previous programmatically created ranges to ensure only one cell is selected
        const ranges = tabulatorInstance.getRanges();
        if (ranges) {
            ranges.forEach(range => range.remove());
        }

        currentMatchIndex = index;
        const currentCell = cellMatches[currentMatchIndex];

        // Scroll to the row of the current cell first to ensure it is visible
        await currentCell.getRow().scrollTo().catch(err => console.error("Scroll to row failed", err));

        // Use Tabulator's built-in range selection to highlight the active cell
        tabulatorInstance.addRange(currentCell, currentCell);
    }

    function goToNextMatch() {
        if (cellMatches.length === 0) return;
        const nextIndex = (currentMatchIndex + 1) % cellMatches.length;
        navigateToMatch(nextIndex);
    }

    function goToPreviousMatch() {
        if (cellMatches.length === 0) return;
        const prevIndex = (currentMatchIndex - 1 + cellMatches.length) % cellMatches.length;
        navigateToMatch(prevIndex);
    }

    let showEditHeaderModal = false;
    let editingHeader = { oldName: '', newName: '' };
    let currentColumnComponent = null;

    function openHeaderEditor(column) {
        currentColumnComponent = column;
        editingHeader = { oldName: column.getDefinition().field, newName: column.getDefinition().field };
        showEditHeaderModal = true;
    }

    onMount(() => {
        if (tablePath) initializeTable(tablePath);
        const undoBtn = document.getElementById("history-undo");
        const redoBtn = document.getElementById("history-redo");
        const undo = () => tabulatorInstance?.undo();
        const redo = () => tabulatorInstance?.redo();
        undoBtn?.addEventListener("click", undo);
        redoBtn?.addEventListener("click", redo);

        const handleKeyDown = (e) => {
            if (e.metaKey && e.key === 'c') {
                e.preventDefault();
                tabulatorInstance?.copyToClipboard("range");
            }
        };
        tableContainer?.addEventListener('keydown', handleKeyDown);

        const handleHeaderFilterKeydown = (e) => {
            if (e.target.tagName === 'INPUT' && e.target.closest('.tabulator-header-filter') && e.key === 'Enter') {
                e.preventDefault();
                e.stopPropagation();
            }
        };
        tableContainer?.addEventListener('keydown', handleHeaderFilterKeydown);


		return () => {
			tabulatorInstance?.destroy();
            undoBtn?.removeEventListener("click", undo);
            redoBtn?.removeEventListener("click", redo);
            tableContainer?.removeEventListener('keydown', handleKeyDown);
            tableContainer?.removeEventListener('keydown', handleHeaderFilterKeydown);
		}
    });

    $: if (tablePath && tablePath !== currentLoadedPath) {
        initializeTable(tablePath);
    }

    $: if ($panelStateStore.tagsLeftPanelCollapsed !== undefined && tabulatorInstance) {
        // Debounce this to avoid excessive redraws during rapid toggling
        debounce(() => {
            tabulatorInstance.redraw(true);
        }, 100)();
    }

    async function handleSaveHeader() {
        if (!currentColumnComponent || !editingHeader.newName.trim()) return;
        if (!tablePath) {
            console.error("[TableViewerPanel] handleSaveHeader: tablePath is missing. Aborting save.");
            showEditHeaderModal = false;
            return;
        }
        const { oldName, newName } = editingHeader;
        try {
            await renameTableHeader(tablePath, oldName, newName);
            const projectBaseDir = get(project)?.baseDirectory;
            if (projectBaseDir) {
                const relativeTablePath = getRelativePath(tablePath, projectBaseDir);
                if (relativeTablePath) {
                    let savedLayout = await loadTableLayoutPrefs(relativeTablePath);
                    if (savedLayout?.columns?.[oldName]) {
                        savedLayout.columns[newName] = savedLayout.columns[oldName];
                        delete savedLayout.columns[oldName];
                        await saveTableLayoutPrefs(relativeTablePath, savedLayout);
                    }
                }
            }
            await initializeTable(tablePath, null, true);
        } catch (error) {
            console.error("Failed to rename header:", error);
        } finally {
            showEditHeaderModal = false;
        }
    }
</script>

{#if showEditHeaderModal}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-700 p-4 shadow-lg">
        <h3 class="text-lg font-bold mb-4">Edit Header</h3>
        <label for="header-name-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Header Name</label>
        <input
            id="header-name-input"
            type="text"
            bind:value={editingHeader.newName}
            class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
        />
        <div class="mt-4 flex justify-end space-x-2">
            <button class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500" on:click={() => showEditHeaderModal = false}>
                Cancel
            </button>
            <button class="px-4 py-2 text-sm font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 text-white bg-blue-600 hover:bg-blue-700" on:click|preventDefault|stopPropagation={handleSaveHeader}>
                Save
            </button>
        </div>
    </div>
</div>
{/if}

<div class="flex flex-col h-full w-full bg-white dark:bg-dark-bg-form-field shadow overflow-hidden">
     <div class="flex items-center justify-between h-9 px-2 border-b border-gray-200 dark:border-dark-bg-tertiary bg-gray-100 dark:bg-surface-3">
        <div class="flex items-center space-x-2">
            <button id="history-undo" class="ui-button-icon" title="Undo">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/>
                    <path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/>
                </svg>
            </button>
            <button id="history-redo" class="ui-button-icon" title="Redo">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/>
                    <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/>
                </svg>
            </button>
        </div>
         {#if !isLoading && !error}
         <div class="flex items-center space-x-2">
            <input
              type="search"
              bind:this={searchInputRef}
              bind:value={searchTerm}
              on:input={handleSearch}
              on:keydown={e => {
                  if (e.key === 'Enter') {
                      e.preventDefault();
                      e.stopPropagation();
                      goToNextMatch();
                  }
              }}
              placeholder="Search table..."
              class="text-xs border border-gray-300 dark:border-dark-bg-tertiary px-2 py-1 bg-white dark:bg-dark-bg-form-field text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500"
              autocomplete="off"
            >
            <button
              title="Previous Match"
              class="ui-button-icon disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToPreviousMatch}
              disabled={cellMatches.length === 0}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-left" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M11.354 1.646a.5.5 0 0 1 0 .708L5.707 8l5.647 5.646a.5.5 0 0 1-.708.708l-6-6a.5.5 0 0 1 0-.708l6-6a.5.5 0 0 1 .708 0"/></svg>
            </button>
            <span class="text-xs text-gray-500 dark:text-gray-400">
                {#if cellMatches.length > 0}
                    {currentMatchIndex + 1} of {cellMatches.length}
                {:else if searchTerm}
                    0 of 0
                {/if}
            </span>
            <button
              title="Next Match"
              class="ui-button-icon disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToNextMatch}
              disabled={cellMatches.length === 0}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
            </button>
             <div class="relative">
                <button
                  title="More Options"
                  class="ui-button-icon"
                  on:click={() => showOptionsMenu = !showOptionsMenu}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16">
                    <path d="M9.5 3a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0zm0 5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0zm0 5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0z"/>
                  </svg>
                </button>
                {#if showOptionsMenu}
                  <div
                    class="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-md shadow-lg z-20"
                    on:mouseleave={() => showOptionsMenu = false}
                  >
                    <button
                      class="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700"
                      on:click={toggleFilters}
                    >
                      {areFiltersVisible ? 'Hide' : 'Show'} Filters
                    </button>
                  </div>
                {/if}
              </div>
         </div>
         {/if}
    </div>

    <div class="flex-grow overflow-auto min-h-0 relative">
        {#if isLoading}
            <div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400 z-10">Loading table data...</div>
        {:else if error}
             <div class="absolute inset-0 flex items-center justify-center text-red-600 dark:text-red-400 p-4 text-center z-10">Error: {error}</div>
        {/if}
        <div bind:this={tableContainer} class="w-full h-full">
             {#if !isLoading && !error && tableData.length === 0 && tablePath}
                 <div class="p-4 text-center text-gray-500 dark:text-gray-400">Table is empty or data could not be loaded.</div>
             {/if}
        </div>
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
     :global(.tabulator) {
        height: 100% !important;
         border: none;
    }
     :global(.tabulator .tabulator-header .tabulator-col .tabulator-col-content .tabulator-col-title-holder .tabulator-col-title) {
         white-space: normal !important;
     }
     :global(.tabulator-header-filter input) {
         @apply p-1 text-xs border border-gray-300 dark:border-border rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 box-border w-auto;
     }
     :global(.tabulator .tabulator-row .tabulator-cell.cell-highlighted-placeholder) {
         background-color: rgba(255, 255, 0, 0.3) !important;
     }
    :global(.tabulator-row-number-column) {
         background-color: #f0f0f0; /* Light gray background */
         font-weight: bold;
         color: #555;
         border-right: 1px solid #ddd;
         padding-right: 5px; /* Adjust padding */
         text-align: center; /* Center the number */
     }

    .flex-grow {
        position: relative;
    }
    .flex-grow > div[bind\:this={tableContainer}] {
        position: absolute;
        inset: 0;
    }

:global(.tabulator .tabulator-header .tabulator-col) {
    padding-left: 0px !important;
}

        :global(.tabulator-cell) {
            overflow: hidden;
            word-break: break-word;
            border-right: 1px solid #ddd;
        }

        :global(.tabulator-cell textarea) {
            width: 100%;
            height: 100%;
            box-sizing: border-box;
            overflow: auto;
            white-space: pre-wrap;
            word-break: break-all;
            border: none;
            resize: none;
            padding: 2px 4px;
            margin: 0;
            background-color: transparent;
            color: inherit;
            font-family: inherit;
            font-size: inherit;
        }

        :global(html.dark .tabulator-row.highlighted-row .tabulator-cell) {
            color: #111827 !important;
        }
        :global(html.dark .tabulator-cell.highlighted-cell) {
            color: #111827 !important;
        }
        :global(.search-match-highlight) {
            background-color: #ffdd77;
            font-weight: bold;
        }
        :global(html.dark .search-match-highlight) {
            background-color: #ffdd77;
            color: #111827;
        }
</style>
