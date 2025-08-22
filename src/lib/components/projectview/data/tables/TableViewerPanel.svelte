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
        loadTableStyles
    } from '$lib/services/projectService.js';
    import { project } from '$lib/stores/projectStore.js';
    import { sep } from '@tauri-apps/api/path';
    import ColorPickerModal from '$lib/components/modals/ColorPickerModal.svelte';
    import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';

    export let tablePath = '';
    export let hasHeaders = true;

    let tableContainer;
    let tabulatorInstance = null;
    let tableData = [];
    let isLoading = true;
    let error = null;
    let currentLoadedPath = null;

    const colorPickerStateStore = writable({ show: false, callback: null });
    const highlightOptions = HIGHLIGHT_OPTIONS;

    let tableStyles = { rowStyles: {} };
    let searchTerm = '';
    let searchMatches = [];
    let currentMatchIndex = -1;
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };

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
            const loadedStyles = await loadTableStyles(pathForTable);
            if (loadedStyles && loadedStyles.rowStyles) {
                tableStyles = loadedStyles;
            } else {
                tableStyles = { rowStyles: {} };
            }

            const response = await loadTableData(pathForTable, hasHeaders);
            tableData = response.data;
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
                layout: "fitData",
                columns: generateColumns(tableData, tableHeaders, savedLayout, !savedLayout),
                height: "100%",
                placeholder: "No Data Available",
                selectableRange: 1,
                rowFormatter: function(row) {
                    const rowIndex = row.getPosition();
                    const color = tableStyles.rowStyles[rowIndex];
                    if (color) {
                        row.getElement().style.backgroundColor = color;
                    } else {
                        row.getElement().style.backgroundColor = "";
                    }
                },
                history:true,
                editTriggerEvent:"dblclick",
                movableColumns: false,
                resizableColumnFit: false,
                columnDefaults: {
                    headerSort:false,
                    headerHozAlign:"center",
                    editor:"textarea",
                    editorParams:{ verticalNavigation:"editor", shiftEnterSubmit:true },
                    resizable:"header",
                    width:100,
                    minWidth: 50,
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

            tabulatorInstance.on("cellEdited", function(cell) {
                const updatedData = tabulatorInstance.getData();
                tableData = updatedData;
                saveTableData(tablePath, tableData);
            });

            tabulatorInstance.on("renderComplete", updateTableLayoutSnapshot);

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

    async function applyRowColor(cell, color = null) {
        if (!tabulatorInstance) return;

        const ranges = tabulatorInstance.getRanges();
        const selectedCells = ranges.flatMap(r => r.getCells());
        const rowsToUpdate = (selectedCells.length > 0)
            ? [...new Set(selectedCells.map(c => c.getRow()))]
            : [cell.getRow()];

        rowsToUpdate.forEach(row => {
            const rowIndex = row.getPosition();
            const rowElement = row.getElement();

            if (color) {
                tableStyles.rowStyles[rowIndex] = color;
                if (rowElement) rowElement.style.backgroundColor = color;
            } else {
                delete tableStyles.rowStyles[rowIndex];
                if (rowElement) rowElement.style.backgroundColor = "";
            }
        });

        try {
            await saveTableStyles(tablePath, tableStyles);
        } catch (err) {
            console.error("Failed to save table styles:", err);
        }
    }

    function generateColumns(data, headers, savedLayoutObj) {
        if (!headers || headers.length === 0) return [{title: "No Data", field: "placeholder"}];

        const rowNumColumn = { title: "#", formatter: "rownum", width: 50, minWidth: 30, hozAlign: "center", resizable: false, headerSort: false, cssClass: "tabulator-row-number-column", editor:false };

        let dataColumnDefs = headers.map(header => {
            const colDef = {
                title: header,
                field: header,
                headerFilter: "input",
                sorter: "string", // Simpler default
                editor: "textarea",
                editorParams:{ verticalNavigation:"editor", shiftEnterSubmit:true },
                formatter: (cell) => { cell.getElement().style.whiteSpace = "pre-wrap"; return cell.getValue(); },
                headerContextMenu: [
                    { label: "Edit Header", action: (e, column) => openHeaderEditor(column) },
                ],
                contextMenu: (e, cell) => {
                    return [
                        { label: "Copy", action: () => navigator.clipboard.writeText(cell.getValue()) },
                        { label: "Cut", action: () => { navigator.clipboard.writeText(cell.getValue()); cell.setValue(""); } },
                        { label: "Paste", action: () => navigator.clipboard.readText().then(text => cell.setValue(text)) },
                        { label: "Delete", action: () => cell.setValue("") },
                        { separator: true },
                        {
                            label: "Set Row Background Color",
                            action: () => {
                                colorPickerStateStore.set({ show: true, callback: (color) => applyRowColor(cell, color) });
                            }
                        },
                        {
                            label: "Clear Row Background Color",
                            action: () => applyRowColor(cell, null)
                        }
                    ];
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

        return [rowNumColumn, ...dataColumnDefs];
    }

    onMount(() => {
        if (tablePath) initializeTable(tablePath);

        const undoBtn = document.getElementById("history-undo");
        const redoBtn = document.getElementById("history-redo");

        const undo = () => tabulatorInstance?.undo();
        const redo = () => tabulatorInstance?.redo();

        undoBtn?.addEventListener("click", undo);
        redoBtn?.addEventListener("click", redo);

		return () => {
			tabulatorInstance?.destroy();
            undoBtn?.removeEventListener("click", undo);
            redoBtn?.removeEventListener("click", redo);
		}
    });

    $: if (tablePath && tablePath !== currentLoadedPath) {
        initializeTable(tablePath);
    }

    async function handleSaveHeader() {
        if (!currentColumnComponent || !editingHeader.newName.trim()) return;
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
            <button class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md" on:click={handleSaveHeader}>
                Save
            </button>
        </div>
    </div>
</div>
{/if}

{#if $colorPickerStateStore.show}
<ColorPickerModal
    bind:show={$colorPickerStateStore.show}
    title="Set Row Background Color"
    on:confirm={(e) => {
        const state = get(colorPickerStateStore);
        if (state.callback) state.callback(e.detail.color);
        colorPickerStateStore.set({ show: false, callback: null });
    }}
    on:close={() => colorPickerStateStore.set({ show: false, callback: null })}
/>
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
