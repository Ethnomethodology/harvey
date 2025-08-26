<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get, writable } from 'svelte/store';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import {
        loadTableData,
        saveTableData,
        saveTableLayoutPrefs,
        loadTableLayoutPrefs,
        renameTableHeader,
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
    let searchMatches = [];
    let currentMatchIndex = -1;
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };
    let tableClipboard = null;

    const saveCurrentTableLayout = debounce(async () => {
        if (!tabulatorInstance || !currentLoadedPath) return;
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
        try {
            await column.delete();
            await saveTableChanges();
            await saveCurrentTableLayoutImmediately();
        } catch (err) {
            console.error("Error deleting column:", err);
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
            await saveCurrentTableLayoutImmediately();
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
                    const row = cell.getRow();
                    const rowIndex = row.getData().harvey_internal_id;
                    const colField = cell.getField();
                    const cellKey = `cell-${rowIndex}-${colField}`;
                    const cellElement = cell.getElement();
                    const cellColor = tableStyles.cellStyles[cellKey];
                    cellElement.style.backgroundColor = cellColor || "";
                    cell.getElement().style.whiteSpace = "pre-wrap";
                    return cell.getValue();
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
                    row.getElement().style.backgroundColor = rowColor || "";
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
            tabulatorInstance.on("columnMoved", saveCurrentTableLayout);
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
        const term = searchTerm.trim();
        tabulatorInstance.setFilter(term ? columnFields.map(field => ({ field, type: 'like', value: term })) : []);
        searchMatches = tabulatorInstance.getRows("active");
        currentMatchIndex = -1;
        if (searchMatches.length > 0) navigateToMatch(0);
    }

    async function navigateToMatch(index) {
        if (!tabulatorInstance || !searchMatches[index]) return;
        currentMatchIndex = index;
        await searchMatches[index].scrollTo().catch(err => console.error("Scroll failed", err));
        tabulatorInstance.deselectRow();
        searchMatches[index].select();
    }

    function goToNextMatch() {
        if (currentMatchIndex < searchMatches.length - 1) navigateToMatch(currentMatchIndex + 1);
    }

    function goToPreviousMatch() {
        if (currentMatchIndex > 0) navigateToMatch(currentMatchIndex - 1);
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
    <div class="bg-white dark:bg-gray-700 p-4 rounded-md shadow-lg">
        <h3 class="text-lg font-bold mb-4">Edit Header</h3>
        <label for="header-name-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Header Name</label>
        <input
            id="header-name-input"
            type="text"
            bind:value={editingHeader.newName}
            class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
        />
        <div class="mt-4 flex justify-end space-x-2">
            <button class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 rounded-md" on:click={() => showEditHeaderModal = false}>
                Cancel
            </button>
            <button class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md" on:click|preventDefault|stopPropagation={handleSaveHeader}>
                Save
            </button>
        </div>
    </div>
</div>
{/if}

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
     <div class="flex items-center justify-between px-2 h-9 border-b border-gray-200 dark:border-gray-600 dark:bg-slate-600 flex-shrink-0">
        <div class="flex items-center space-x-2">
            <button id="history-undo" class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500" title="Undo">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.919.5.5 0 0 0-.908-.418A6 6 0 1 0 8 2z"/>
                    <path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L.694 6.438a.5.5 0 0 0 0 .724l6.896 6.896A.5.5 0 0 0 8 13.466V9.534a.25.25 0 0 0-.41-.192L1.194 6.706a.5.5 0 0 0 0-.724l6.396-6.396A.25.25 0 0 0 8 4.466"/>
                </svg>
            </button>
            <button id="history-redo" class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500" title="Redo">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.919.5.5 0 0 1 .908-.418A6 6 0 1 1 8 2z"/>
                    <path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L.694 6.438a.5.5 0 0 0 0 .724l6.896 6.896A.5.5 0 0 0 8 13.466V9.534a.25.25 0 0 0-.41-.192L1.194 6.706a.5.5 0 0 0 0-.724l6.396-6.396A.25.25 0 0 0 8 4.466"/>
                </svg>
            </button>
        </div>
         {#if !isLoading && !error}
         <div class="flex items-center space-x-2">
            <input
              type="search"
              bind:value={searchTerm}
              on:input={handleSearch}
              placeholder="Search table..."
              class="text-xs border border-gray-300 dark:border-gray-600 rounded px-2 py-1 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500"
              autocomplete="off"
            >
            <button
              title="Previous Match"
              class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToPreviousMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex <= 0}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-left" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M11.354 1.646a.5.5 0 0 1 0 .708L5.707 8l5.647 5.646a.5.5 0 0 1-.708.708l-6-6a.5.5 0 0 1 0-.708l6-6a.5.5 0 0 1 .708 0"/></svg>
            </button>
            <button
              title="Next Match"
              class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToNextMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex >= searchMatches.length - 1}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
            </button>
            <button class="text-xs px-2 py-1 border rounded bg-blue-500 hover:bg-blue-600 text-white disabled:opacity-50" on:click={() => { console.log('TODO: Add new row action'); alert('Add Row'); }} title="Add New Row">
                Add Row
            </button>
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
         @apply p-1 text-xs border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 box-border w-auto;
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
            word-break: break-all;
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
</style>