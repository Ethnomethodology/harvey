<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { loadTableData, saveTableData, saveTableLayoutPrefs, loadTableLayoutPrefs, renameTableHeader } from '$lib/services/projectService.js'; // Added new functions
    import { project } from '$lib/stores/projectStore.js'; // For baseDirectory
    import { get } from 'svelte/store'; // To read store value
    import { sep } from '@tauri-apps/api/path'; // For path manipulation

    export let tablePath = '';
    export let hasHeaders = true;

    let tableContainer;
    let tabulatorInstance = null;
    let tableData = [];
    let isLoading = true;
    let error = null;
    let currentLoadedPath = null; // Track what path is currently loaded/being loaded

    // Custom Context Menu State
    let showCustomRowMenu = false;
    let customMenuX = 0;
    let customMenuY = 0;
    let clickedRowComponent = null; // To store the Tabulator RowComponent that was right-clicked

    import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';
    const highlightOptions = HIGHLIGHT_OPTIONS;

    let searchTerm = '';
    let searchMatches = []; // To store Tabulator RowComponents that match
    let currentMatchIndex = -1;
    // It might be useful to also store the actual column fields for easy access
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };
    
    let selectedCellCache = [];

    function updateTableLayoutSnapshot() {
        if (!tabulatorInstance) return;
        const currentColumnDefs = tabulatorInstance.getColumnDefinitions();
        const newSnapshotColumns = {};
        currentColumnDefs.forEach((colDef, index) => {
            if (colDef.field) {
                const columnComponent = tabulatorInstance.getColumn(colDef.field); // Get column component
                if (columnComponent) {
                    newSnapshotColumns[colDef.field] = {
                        order: index, // Or columnComponent.getPosition(true) - 1 if more robust for order
                        visible: columnComponent.isVisible(),
                        width: columnComponent.getWidth(), // Get live width
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
        return relativePath.replaceAll('\\', '/'); // Normalize to forward slashes
    }

    function debounce(func, delay) {
        let timeout;
        return function(...args) {
            const context = this;
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(context, args), delay);
        };
    }

    async function initializeTable(pathForTable, newHasHeaders = null, force = false) {
        if (newHasHeaders !== null) {
            hasHeaders = newHasHeaders;
        }
        if (!pathForTable || !tableContainer) {
            return;
        }

        if (isLoading && currentLoadedPath === pathForTable) {
            return;
        }
        if (!force && !isLoading && tabulatorInstance && currentLoadedPath === pathForTable) {
            return;
        }

        currentLoadedPath = pathForTable;
        isLoading = true;
        error = null;
        tableData = [];
        selectedCellCache = [];

        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }

        try {
            const response = await loadTableData(pathForTable, hasHeaders);
            const tableHeaders = response.headers;
            tableData = response.data;

            if (tableData.length === 0) {
                console.warn('[TableViewerPanel initializeTable] No data returned from loadTableData.');
            }

            await tick();

            if (!tableContainer) {
                 error = 'Failed to initialize table viewer: container lost.';
                 isLoading = false;
                 currentLoadedPath = null;
                 return;
            }

            const projectBaseDir = get(project)?.baseDirectory;
            if (!projectBaseDir) {
                error = "Project configuration error: base directory missing.";
                isLoading = false;
                currentLoadedPath = null;
                return;
            }
            const relativeTablePath = getRelativePath(pathForTable, projectBaseDir);

            if (!relativeTablePath) {
                error = "Error determining asset relative path.";
                isLoading = false;
                currentLoadedPath = null;
                return;
            }

            let savedLayout = null;
            try {
                const layoutData = await loadTableLayoutPrefs(relativeTablePath);
                if (layoutData) {
                    savedLayout = layoutData;
                }
            } catch (e) {
                console.error(`[TableViewerPanel] Error loading layout for ${relativeTablePath}:`, e);
            }

            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                layout: "fitData",
                columns: generateColumns(tableData, tableHeaders, savedLayout, !savedLayout),
                height: "100%",
                placeholder: "No Data Available",
                selectableRange: 1,
                history:true,
                editTriggerEvent:"dblclick",
                movableColumns: false,
                resizableColumnFit: false,
                columnDefaults: {
                    headerSort:false,
                    headerHozAlign:"center",
                    editor:"textarea",
                    editorParams:{
                        verticalNavigation:"editor",
                        shiftEnterSubmit:true,
                    },
                    resizable:"header",
                    width:100,
                    minWidth: 50,
                },
                clipboard: true,
                clipboardCopyStyled:false,
                clipboardCopyConfig:{
                    rowHeaders:false,
                    columnHeaders:false,
                },
                clipboardCopyRowRange:"range",
                clipboardPasteParser:"range",
                clipboardPasteAction:"range",
            });

            const saveCurrentTableLayout = debounce(async () => {
                if (!tabulatorInstance || !currentLoadedPath) return;

                const baseDirForSave = get(project)?.baseDirectory;
                if (!baseDirForSave) {
                    return;
                }
                const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
                if (!relativePathForSave) {
                    return;
                }

                try {
                    const layoutToSave = { columns: {} };
                    for (const field in tableLayoutSnapshot.columns) {
                        const colData = tableLayoutSnapshot.columns[field];
                        const columnSaveData = {
                            order: colData.order,
                            visible: colData.visible,
                        };
                        if (typeof colData.width === 'number' && colData.width > 0) {
                            columnSaveData.width = colData.width;
                        }
                        layoutToSave.columns[field] = columnSaveData;
                    }
                    await saveTableLayoutPrefs(relativePathForSave, layoutToSave);
                } catch (error) {
                    console.error(`[TableViewerPanel] Failed to save layout for ${relativePathForSave}:`, error);
                }
            }, 750);

            tabulatorInstance.on("columnResized", (column) => {
                if (tableLayoutSnapshot.columns[column.getField()]) {
                    tableLayoutSnapshot.columns[column.getField()].width = column.getWidth();
                } else {
                    tableLayoutSnapshot.columns[column.getField()] = {
                        width: column.getWidth(),
                        order: column.getPosition(true) -1,
                        visible: column.isVisible()
                    };
                }
                updateTableLayoutSnapshot();
                saveCurrentTableLayout();
            });

            tabulatorInstance.on("columnMoved", (column, columns) => {
                updateTableLayoutSnapshot();
                saveCurrentTableLayout();
            });

            tabulatorInstance.on("cellEdited", function(cell) {
                const rowIndex = cell.getRow().getPosition() - 1;
                const field = cell.getField();
                const newValue = cell.getValue().replace(/\r/g, '');
                tableData[rowIndex][field] = newValue;
                project.update(p => ({ ...p, isDocumentDirty: true, tableData: tableData }));
                saveTableData(tablePath, tableData);
            });

            tabulatorInstance.on("rangeAdded", (range) => {
                selectedCellCache = range.getCells();
            });

            tabulatorInstance.on("rangeRemoved", (range) => {
                selectedCellCache = [];
            });

            tabulatorInstance.on("renderComplete", () => {
                updateTableLayoutSnapshot();
                const filters = tableContainer.querySelectorAll(".tabulator-header-filter input");
                filters.forEach(input => {
                    input.setAttribute("autocomplete", "off");
                    input.setAttribute("autocorrect", "off");
                    input.setAttribute("autocapitalize", "none");
                    input.setAttribute("spellcheck", "false");
                });
            });

            setTimeout(() => {
                if (tabulatorInstance && typeof tabulatorInstance.redraw === 'function') {
                    tabulatorInstance.redraw();
                }
            }, 100);

            const columns = tabulatorInstance.getColumnDefinitions();
            columnFields = columns.map(col => col.field).filter(field => field && field !== 'placeholder');

        } catch (err) {
            error = `Failed to load table: ${err.message || err}`;
            currentLoadedPath = null;
        } finally {
            if (currentLoadedPath === pathForTable) {
                isLoading = false;
            }
        }
    }

    function handleSearch() {
        if (!tabulatorInstance) return;
        const term = searchTerm.trim();

        clearHighlights();

        if (!term) {
            tabulatorInstance.clearFilter();
            searchMatches = [];
            currentMatchIndex = -1;
            return;
        }

        const filters = columnFields.map(field => ({ field: field, type: 'like', value: term }));
        tabulatorInstance.setFilter(filters);
        searchMatches = tabulatorInstance.getRows("active");

        if (searchMatches.length > 0) {
            navigateToMatch(0);
        } else {
            currentMatchIndex = -1;
        }
    }

    function clearHighlights() {
        if (tabulatorInstance) {
            tabulatorInstance.deselectRow();
        }
    }

    async function navigateToMatch(index) {
        if (!tabulatorInstance || searchMatches.length === 0 || index < 0 || index >= searchMatches.length) {
            currentMatchIndex = -1;
            if (tabulatorInstance) tabulatorInstance.deselectRow();
            return;
        }

        if (tabulatorInstance) {
            tabulatorInstance.deselectRow();
        }

        currentMatchIndex = index;
        const rowComponent = searchMatches[currentMatchIndex];

        if (rowComponent) {
            try {
                await rowComponent.scrollTo();
                await rowComponent.select();
            } catch (err) {
                console.error("Error navigating to match:", err);
            }
        }
    }

    function goToNextMatch() {
        if (searchMatches.length > 0 && currentMatchIndex < searchMatches.length - 1) {
            navigateToMatch(currentMatchIndex + 1);
        }
    }

    function goToPreviousMatch() {
        if (searchMatches.length > 0 && currentMatchIndex > 0) {
            navigateToMatch(currentMatchIndex - 1);
        }
    }

    let showEditHeaderModal = false;
    let editingHeader = { oldName: '', newName: '' };
    let currentColumnComponent = null;

    function openHeaderEditor(column) {
        currentColumnComponent = column;
        editingHeader = {
            oldName: column.getDefinition().field,
            newName: column.getDefinition().field
        };
        showEditHeaderModal = true;
    }

    function styledCellFormatter(cell, formatterParams, onRendered) {
        const data = cell.getRow().getData();
        const field = cell.getField();

        if (data._cellStyles && data._cellStyles[field] && data._cellStyles[field].backgroundColor) {
            cell.getElement().style.backgroundColor = data._cellStyles[field].backgroundColor;
        } else {
            cell.getElement().style.backgroundColor = '';
        }

        return cell.getValue();
    }


    function generateColumns(data, headers, savedLayoutObj, isFirstLoad) {
        if (!headers || headers.length === 0) return [{title: "No Data", field: "placeholder"}];

        const rowNumColumn = {
            title: "#",
            formatter: "rownum",
            width: 50,
            minWidth: 30,
            hozAlign: "center",
            resizable: false,
            headerSort: false,
            cssClass: "tabulator-row-number-column",
            editor:false
        };
        
        let dataColumnDefs = headers.map(header => {
            const colDef = {
                title: header,
                field: header,
                headerFilter: "input",
                sorter: inferSorter(data, header),
                editor: "textarea",
                editorParams:{
                    verticalNavigation:"editor",
                    shiftEnterSubmit:true,
                },
                formatter: styledCellFormatter,
                formatterParams: {},
                headerContextMenu: [
                    {
                        label: "Edit Header",
                        action: function(e, column) {
                            openHeaderEditor(column);
                        }
                    },
                    {
                        label: "Copy",
                        action: function(e, column) {
                            navigator.clipboard.writeText(column.getField()).catch(err => {
                                console.error('Could not copy header to clipboard: ', err);
                            });
                        }
                    },
                    {
                        label: "Cut",
                        action: function(e, column) {
                            console.log(`Cut header ${column.getField()}`);
                        }
                    },
                    {
                        label: "Paste",
                        action: function(e, column) {
                            console.log(`Paste header at ${column.getField()}`);
                        }
                    },
                    {
                        label: "Delete Column",
                        action: function(e, column) {
                            if (confirm(`Are you sure you want to delete the column '${column.getField()}'? This action cannot be undone.`)) {
                                tabulatorInstance.deleteColumn(column);
                            }
                        }
                    },
                    {
                        label: "Insert Column Left",
                        action: function(e, column) {
                            console.log(`Insert column left of ${column.getField()}`);
                        }
                    },
                    {
                        label: "Insert Column Right",
                        action: function(e, column) {
                            console.log(`Insert column right of ${column.getField()}`);
                        }
                    }
                ],
                cellContextMenu: [
                    {
                        label: "Copy",
                        action: (e, cell) => {
                            navigator.clipboard.writeText(cell.getValue()).catch(err => {
                                console.error('Could not copy cell value to clipboard: ', err);
                            });
                        }
                    },
                    {
                        label: "Cut",
                        action: (e, cell) => {
                            navigator.clipboard.writeText(cell.getValue()).then(() => {
                                cell.setValue("");
                            }).catch(err => {
                                console.error('Could not cut cell value: ', err);
                            });
                        }
                    },
                    {
                        label: "Paste",
                        action: (e, cell) => {
                            navigator.clipboard.readText().then(text => {
                                cell.setValue(text);
                            }).catch(err => {
                                console.error('Could not paste into cell: ', err);
                            });
                        }
                    },
                    {
                        label: "Delete",
                        action: (e, cell) => {
                            cell.setValue("");
                        }
                    },
                    {
                        separator: true,
                    },
                    {
                        label: "Color Cell",
                        menu: highlightOptions.map(option => ({
                            label: `<span style="display:inline-block; width:10px; height:10px; background-color:${option.value}; margin-right:5px;"></span> ${option.label}`,
                            action: (e, cell) => {
                                const selectedCells = tabulatorInstance.getSelectedCells();
                                const cellsToUpdate = selectedCells.length > 0 ? selectedCells : [cell];

                                cellsToUpdate.forEach(c => {
                                    const row = c.getRow();
                                    let rowData = row.getData();
                                    let field = c.getField();

                                    const newCellStyles = { ...(rowData._cellStyles || {}) };
                                    if (!newCellStyles[field]) {
                                        newCellStyles[field] = {};
                                    }
                                    newCellStyles[field].backgroundColor = option.value;
                                    row.update({ _cellStyles: newCellStyles });
                                });
                            }
                        }))
                    },
                    {
                        label: "Clear Cell Color",
                        action: (e, cell) => {
                            const selectedCells = tabulatorInstance.getSelectedCells();
                            const cellsToUpdate = selectedCells.length > 0 ? selectedCells : [cell];

                            cellsToUpdate.forEach(c => {
                                const row = c.getRow();
                                let rowData = row.getData();
                                let field = c.getField();

                                if (rowData._cellStyles && rowData._cellStyles[field]) {
                                    const newCellStyles = { ...rowData._cellStyles };
                                    delete newCellStyles[field].backgroundColor;

                                    if (Object.keys(newCellStyles[field]).length === 0) {
                                        delete newCellStyles[field];
                                    }

                                    row.update({ _cellStyles: newCellStyles });
                                }
                            });
                        }
                    }
                ]
            };

            if (savedLayoutObj && savedLayoutObj.columns && savedLayoutObj.columns[header]) {
                const savedCol = savedLayoutObj.columns[header];
                if (typeof savedCol.width === 'number' && savedCol.width > 0) {
                    colDef.width = savedCol.width;
                }
                colDef.visible = savedCol.visible;
            }
            return colDef;
        });

        if (savedLayoutObj && savedLayoutObj.columns) {
            dataColumnDefs.sort((a, b) => {
                const orderA = savedLayoutObj.columns[a.field]?.order ?? Infinity;
                const orderB = savedLayoutObj.columns[b.field]?.order ?? Infinity;
                return orderA - orderB;
            });
            dataColumnDefs.forEach(colDef => {
                if (savedLayoutObj.columns[colDef.field] && typeof savedLayoutObj.columns[colDef.field].visible === 'boolean') {
                    colDef.visible = savedLayoutObj.columns[colDef.field].visible;
                }
            });
        }

        return [rowNumColumn].concat(dataColumnDefs);
    }

     function inferSorter(data, field) {
         if (!data || data.length === 0 || !data[0].hasOwnProperty(field)) return "string";
         const firstValue = data[0][field];
         if (typeof firstValue === 'number') return "number";
         if (typeof firstValue === 'boolean') return "boolean";
         return "string";
     }

    onMount(() => {
        if (tablePath && tableContainer) {
             initializeTable(tablePath);
        }

        const handleContextMenu = (event) => {
            const target = event.target;
            if (target.closest('.tabulator-row-number-column') && !target.closest('.tabulator-header')) {
                event.preventDefault();
                event.stopPropagation();
                showCustomRowMenu = true;
                customMenuX = event.clientX;
                customMenuY = event.clientY;

                const rowElement = target.closest('.tabulator-row');
                if (rowElement) {
                    if (tabulatorInstance) {
                        clickedRowComponent = tabulatorInstance.getRow(rowElement);
                    }
                }
            }
        };

        const hideCustomRowMenu = (event) => {
            if (showCustomRowMenu && event.target.closest('.custom-row-menu') === null) {
                showCustomRowMenu = false;
            }
        };

        tableContainer.addEventListener('contextmenu', handleContextMenu);
        document.addEventListener('click', hideCustomRowMenu);

        document.getElementById("history-undo").addEventListener("click", function(){
            if (tabulatorInstance) {
                tabulatorInstance.undo();
            }
        });

        document.getElementById("history-redo").addEventListener("click", function(){
            if (tabulatorInstance) {
                tabulatorInstance.redo();
            }
        });

		return () => {
			if (tabulatorInstance) {
				tabulatorInstance.destroy();
				tabulatorInstance = null;
			}
            tableContainer.removeEventListener('contextmenu', handleContextMenu);
            document.removeEventListener('click', hideCustomRowMenu);
		}
    });

    $: if (tablePath && tableContainer) {
        if (tablePath !== currentLoadedPath) {
            initializeTable(tablePath);
        }
    } else if (!tablePath && tabulatorInstance) {
        tabulatorInstance.destroy();
        tabulatorInstance = null;
        tableData = [];
        isLoading = false;
        error = null;
        currentLoadedPath = null;
    }

    onDestroy(() => {
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }
    });

    async function handleSaveHeader() {
        if (!currentColumnComponent || editingHeader.newName.trim() === '') return;

        const oldName = editingHeader.oldName;
        const newName = editingHeader.newName;

        try {
            await renameTableHeader(tablePath, oldName, newName);

            const projectBaseDir = get(project)?.baseDirectory;
            if (projectBaseDir) {
                const relativeTablePath = getRelativePath(tablePath, projectBaseDir);
                if (relativeTablePath) {
                    let savedLayout = await loadTableLayoutPrefs(relativeTablePath);
                    if (savedLayout && savedLayout.columns && savedLayout.columns[oldName]) {
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
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 rounded-md"
                on:click={() => showEditHeaderModal = false}
            >
                Cancel
            </button>
            <button
                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
                on:click={handleSaveHeader}
            >
                Save
            </button>
        </div>
    </div>
</div>
{/if}

{#if showCustomRowMenu}
<div
    class="absolute z-50 bg-white dark:bg-gray-800 shadow-lg rounded-md py-1"
    style="left: {customMenuX}px; top: {customMenuY}px;"
>
    <ul class="text-sm text-gray-700 dark:text-gray-200">
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={copyRow}>Copy</button></li>
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={cutRow}>Cut</button></li>
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={pasteRow}>Paste</button></li>
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={deleteRow}>Delete</button></li>
        <hr class="my-1 border-gray-200 dark:border-gray-700">
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={insertRowAbove}>Insert Row Above</button></li>
        <li><button class="block w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700" on:click={insertRowBelow}>Insert Row Below</button></li>
    </ul>
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
         { #if !isLoading && !error }
         <div class="flex items-center space-x-2">
            <input
              type="search"
              bind:value={searchTerm}
              on:input={handleSearch}
              placeholder="Search table..."
              class="text-xs border border-gray-300 dark:border-gray-600 rounded px-2 py-1 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="none"
              spellcheck="false"
            >
            <button
              title="Previous Match"
              class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToPreviousMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex <= 0}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-left" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M11.354 1.646a.5.5 0 0 1 0 .708L5.707 8l5.647 5.646a.5.5 0 0 1-.708.708l-6-6a.5.5 0 0 1 0-.708l6-6a.5.5 0 0 1 .708 0"/>
              </svg>
            </button>
            <button
              title="Next Match"
              class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 disabled:cursor-not-allowed"
              on:click={goToNextMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex >= searchMatches.length - 1}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/>
              </svg>
            </button>
            <button class="text-xs px-2 py-1 border rounded bg-blue-500 hover:bg-blue-600 text-white disabled:opacity-50"
                    on:click={() => { console.log('TODO: Add new row action'); alert('Add Row'); }}
                    title="Add New Row">
                Add Row
            </button>
         </div>
         { /if }
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
            overflow: hidden; /* Prevent cell itself from showing overflow if textarea somehow fails */
            word-break: break-all; /* Help break very long words at cell level */
        }

        :global(.tabulator-cell textarea) {
            width: 100%;
            height: 100%;
            box-sizing: border-box;
            overflow: auto; /* Important: allow scrollbars within the textarea */
            white-space: pre-wrap; /* Respect newlines, wrap text */
            word-break: break-all; /* Break long words within textarea */
            border: none;
            resize: none;
            padding: 2px 4px; /* Adjust to match Tabulator's default cell padding or desired look */
            margin: 0;
            background-color: transparent; /* Inherit cell background */
            color: inherit; /* Inherit cell text color */
            font-family: inherit; /* Inherit cell font */
            font-size: inherit; /* Inherit cell font size */
        }
</style>
