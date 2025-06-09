<!-- src/lib/components/projectview/notes/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { loadTableData, saveTableLayoutPrefs, loadTableLayoutPrefs } from '$lib/services/projectService.js'; // Added new functions
    import { project } from '$lib/stores/projectStore.js'; // For baseDirectory
    import { get } from 'svelte/store'; // To read store value
    import { sep } from '@tauri-apps/api/path'; // For path manipulation

    export let tablePath = '';

    let tableContainer;
    let tabulatorInstance = null;
    let tableData = [];
    let isLoading = true;
    let error = null;
    let currentLoadedPath = null; // Track what path is currently loaded/being loaded

    let searchTerm = '';
    let searchMatches = []; // To store Tabulator RowComponents that match
    let currentMatchIndex = -1;
    // It might be useful to also store the actual column fields for easy access
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };

    // Placeholder functions (Unchanged)
    function openRowForm(row) {
        const rowData = row.getData();
        console.log("Placeholder: Open form view for row:", rowData); // Keep as is (placeholder)
        alert(`Open Form View (Placeholder)\n\nRow Data:\n${JSON.stringify(rowData, null, 2)}`);
    }
    function addComment(cell) {
        const cellValue = cell.getValue();
        const columnName = cell.getColumn().getField();
        const rowData = cell.getRow().getData();
        console.log(`Placeholder: Add comment to cell (${columnName}: ${cellValue})`, rowData); // Keep as is (placeholder)
        const comment = prompt(`Add comment for "${columnName}" in this row:`, "");
        if (comment !== null) {
            alert(`Comment Added (Placeholder):\n"${comment}"`);
        }
    }
     function addHighlight(cell) {
        const cellValue = cell.getValue();
        const columnName = cell.getColumn().getField();
        const rowData = cell.getRow().getData();
        console.log(`Placeholder: Add highlight to cell (${columnName}: ${cellValue})`, rowData); // Keep as is (placeholder)
         const highlight = confirm(`Highlight this cell?`);
         if (highlight) {
             alert(`Cell Highlighted (Placeholder)`);
            cell.getElement().classList.toggle('cell-highlighted-placeholder');
         }
     }

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
        // console.debug('[TableViewerPanel updateTableLayoutSnapshot] Snapshot updated:', JSON.stringify(tableLayoutSnapshot, null, 2));
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
        return relativePath.replace(/\\/g, '/'); // Normalize to forward slashes
    }

    function debounce(func, delay) {
        let timeout;
        return function(...args) {
            const context = this;
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(context, args), delay);
        };
    }

    async function initializeTable(pathForTable) {
        if (!pathForTable || !tableContainer) {
            console.debug('[TableViewerPanel initializeTable] Skipping: no path or container.', { pathForTable, tableContainerExists: !!tableContainer }); // DEBUG
            isLoading = false; // Ensure loading is false if we skip
            return;
        }

        // Prevent re-initialization if already loading or loaded for this path
        if (isLoading && currentLoadedPath === pathForTable) {
            console.debug(`[TableViewerPanel initializeTable] Already loading ${pathForTable}, skipping.`); // DEBUG
            return;
        }
        if (!isLoading && tabulatorInstance && currentLoadedPath === pathForTable) {
            console.debug(`[TableViewerPanel initializeTable] Table for ${pathForTable} already initialized and loaded.`); // DEBUG
            return;
        }


        console.info(`[TableViewerPanel initializeTable] Initializing for path: ${pathForTable}`); // INFO
        currentLoadedPath = pathForTable; // Set path being processed
        isLoading = true;
        error = null;
        tableData = [];

        if (tabulatorInstance) {
            console.debug('[TableViewerPanel initializeTable] Destroying previous Tabulator instance.'); // DEBUG
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }

        try {
            tableData = await loadTableData(pathForTable);

            if (tableData.length === 0) {
                console.warn('[TableViewerPanel initializeTable] No data returned from loadTableData.'); // WARN
            }

            await tick(); // Ensure DOM is ready if container was re-rendered

            if (!tableContainer) {
                 console.error('[TableViewerPanel initializeTable] Table container element became null during data load for path:', pathForTable); // ERROR
                 error = 'Failed to initialize table viewer: container lost.';
                 isLoading = false;
                 currentLoadedPath = null; // Reset if failed
                 return;
            }

            const projectBaseDir = get(project)?.baseDirectory;
            if (!projectBaseDir) {
                console.error("[TableViewerPanel] Project baseDirectory not available from store. Cannot determine relative path for layout prefs.");
                // Proceed without layout prefs, or handle error more gracefully
                error = "Project configuration error: base directory missing.";
                isLoading = false;
                currentLoadedPath = null;
                return;
            }
            const relativeTablePath = getRelativePath(pathForTable, projectBaseDir);

            if (!relativeTablePath) {
                console.error(`[TableViewerPanel] Could not determine relative path for ${pathForTable} against base ${projectBaseDir}`);
                error = "Error determining asset relative path.";
                isLoading = false;
                currentLoadedPath = null; // Reset if failed
                return;
            }
            console.debug(`[TableViewerPanel] Absolute path: ${pathForTable}, Relative path for DB: ${relativeTablePath}`);


            let savedLayout = null;
            try {
                savedLayout = await loadTableLayoutPrefs(relativeTablePath);
                if (savedLayout) {
                    console.debug(`[TableViewerPanel] Loaded saved layout for ${relativeTablePath}:`, JSON.stringify(savedLayout, null, 2)); // Log the full object
                } else {
                    console.debug(`[TableViewerPanel] No saved layout found for ${relativeTablePath}.`);
                }
            } catch (e) {
                console.error(`[TableViewerPanel] Error loading saved layout for ${relativeTablePath}:`, e); // Corrected path variable for error log
                // Continue with default layout
            }

            let initialLayoutMode = "fitDataTable"; // Default
            if (!savedLayout) {
                if (relativeTablePath && relativeTablePath.toLowerCase().endsWith('.csv')) {
                    initialLayoutMode = "fitDataStretch"; // Stretch for CSVs if no saved layout
                    console.debug(`[TableViewerPanel] No saved layout for CSV ${relativeTablePath}, using ${initialLayoutMode}.`);
                } else if (relativeTablePath) {
                    console.debug(`[TableViewerPanel] No saved layout for ${relativeTablePath}, using default ${initialLayoutMode}.`);
                }
            } else {
                initialLayoutMode = "fitData"; // Use fitData if layout is loaded to respect saved widths
                console.debug(`[TableViewerPanel] Saved layout found for ${relativeTablePath}, using ${initialLayoutMode}.`);
            }

            console.debug(`[TableViewerPanel initializeTable] Creating Tabulator instance for ${pathForTable} (relative: ${relativeTablePath})`); // DEBUG
            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                layout: initialLayoutMode,
                columns: generateColumns(tableData, savedLayout),
                height: "100%",
                placeholder: "No Data Available",
                selectable: 1,
                movableColumns: true,
                resizableColumnFit: true,
                // Pagination options
                pagination: true,
                paginationSize: 20,
                paginationMode: 'local',
            });

            const saveCurrentTableLayout = debounce(async () => {
                if (!tabulatorInstance || !currentLoadedPath) return;

                const baseDirForSave = get(project)?.baseDirectory;
                if (!baseDirForSave) {
                    console.error("[TableViewerPanel saveLayout] baseDirectory not available. Cannot save layout.");
                    return;
                }
                const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
                if (!relativePathForSave) {
                    console.error(`[TableViewerPanel saveCurrentTableLayout] Could not determine relative path for DB key. Absolute path: ${currentLoadedPath}`);
                    return;
                }

                console.debug(`[TableViewerPanel saveCurrentTableLayout] Attempting to save layout for ${relativePathForSave} using snapshot:`, JSON.stringify(tableLayoutSnapshot, null, 2));
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

                    console.debug(`[TableViewerPanel saveCurrentTableLayout] Saving layout for ${relativePathForSave}:`, JSON.stringify(layoutToSave, null, 2));
                    await saveTableLayoutPrefs(relativePathForSave, JSON.stringify(layoutToSave));
                    console.info(`[TableViewerPanel] Layout saved for ${relativePathForSave}`);
                } catch (error) {
                    console.error(`[TableViewerPanel] Failed to save layout for ${relativePathForSave}:`, error);
                }
            }, 750);

            tabulatorInstance.on("columnResized", (column) => {
                // console.debug(`[TableViewerPanel columnResized] Column: ${column.getField()}, Width: ${column.getWidth()}`);
                if (tableLayoutSnapshot.columns[column.getField()]) {
                    tableLayoutSnapshot.columns[column.getField()].width = column.getWidth();
                } else { // Should ideally not happen if snapshot is initialized correctly
                    tableLayoutSnapshot.columns[column.getField()] = {
                        width: column.getWidth(),
                        order: column.getPosition(true) -1, // Be cautious with order here
                        visible: column.isVisible()
                    };
                }
                // Update orders for all columns as resizing one might affect others in some layouts
                updateTableLayoutSnapshot(); // This will re-capture all orders and widths
                saveCurrentTableLayout(); // Call debounced save
            });

            tabulatorInstance.on("columnMoved", (column, columns) => { // columns is array of all column components in new order
                // console.debug(`[TableViewerPanel columnMoved] Column: ${column.getField()} moved.`);
                updateTableLayoutSnapshot(); // This will re-capture all new orders and existing widths
                saveCurrentTableLayout(); // Call debounced save
            });
            // tabulatorInstance.on("columnVisibilityChanged", (column, visible) => {
            //     updateTableLayoutSnapshot();
            //     saveCurrentTableLayout();
            // });


            tabulatorInstance.on("rowClick", function(e, row){
                 console.debug("Row Clicked:", row.getData()); // DEBUG
            });

            // Disable macOS autocorrect/autocomplete on column header filters and init snapshot
            tabulatorInstance.on("renderComplete", () => {
                updateTableLayoutSnapshot(); // Initial snapshot after table is fully rendered
                const filters = tableContainer.querySelectorAll(".tabulator-header-filter input");
                filters.forEach(input => {
                    input.setAttribute("autocomplete", "off");
                    input.setAttribute("autocorrect", "off");
                    input.setAttribute("autocapitalize", "none");
                    input.setAttribute("spellcheck", "false");
                });
            });

            // After setting up event handlers:
            setTimeout(() => {
                if (tabulatorInstance && typeof tabulatorInstance.redraw === 'function') {
                    console.debug('[TableViewerPanel initializeTable] Triggering a gentle redraw after short delay.'); // DEBUG
                    tabulatorInstance.redraw(); // Using redraw() without 'true' for a less disruptive redraw
                }
            }, 100); // 100ms delay, can be adjusted

            const columns = tabulatorInstance.getColumnDefinitions();
            columnFields = columns.map(col => col.field).filter(field => field && field !== 'placeholder'); // Store actual field names

            console.info(`[TableViewerPanel initializeTable] Tabulator initialized for ${pathForTable}.`); // INFO

        } catch (err) {
            console.error(`[TableViewerPanel initializeTable] Error for path ${pathForTable}:`, err); // ERROR
            error = `Failed to load table: ${err.message || err}`;
            currentLoadedPath = null; // Reset if failed
        } finally {
            // Only set isLoading to false if we are still on the same path.
            // This prevents a quick flash if path changes rapidly.
            if (currentLoadedPath === pathForTable) {
                isLoading = false;
            }
            console.debug(`[TableViewerPanel initializeTable] Finished for ${pathForTable}. isLoading: ${isLoading}`); // DEBUG
        }
    }

    function handleSearch() {
        if (!tabulatorInstance) return;
        const term = searchTerm.trim();

        clearHighlights(); // Clear highlights before new search or clearing filter

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
            // currentMatchIndex = 0; // Set before navigateToMatch
            navigateToMatch(0); // Navigate to the first match
        } else {
            currentMatchIndex = -1;
        }
        console.debug(`Found ${searchMatches.length} rows matching "${term}"`); // DEBUG
    }

    function clearHighlights() {
        if (tabulatorInstance) {
            tabulatorInstance.deselectRow(); // Deselect all rows
        }
        console.debug("clearHighlights called: deselected all rows."); // DEBUG
    }

    async function navigateToMatch(index) {
        if (!tabulatorInstance || searchMatches.length === 0 || index < 0 || index >= searchMatches.length) {
            currentMatchIndex = -1;
            // If no valid match, ensure nothing is selected from search
            if (tabulatorInstance) tabulatorInstance.deselectRow();
            return;
        }

        // It's good practice to clear any programmatically set selections before new action
        if (tabulatorInstance) {
            tabulatorInstance.deselectRow();
        }

        currentMatchIndex = index;
        const rowComponent = searchMatches[currentMatchIndex];

        if (rowComponent) {
            try {
                await rowComponent.scrollTo();
                await rowComponent.select(); // Select the current row
            } catch (err) {
                console.error("Error navigating to match:", err); // ERROR
            }
        }
    }

    function goToNextMatch() {
        if (searchMatches.length > 0 && currentMatchIndex < searchMatches.length - 1) {
            navigateToMatch(currentMatchIndex + 1);
        } else if (searchMatches.length > 0 && currentMatchIndex === searchMatches.length - 1) {
            // Optional: loop back to the first match
            // navigateToMatch(0);
        }
    }

    function goToPreviousMatch() {
        if (searchMatches.length > 0 && currentMatchIndex > 0) {
            navigateToMatch(currentMatchIndex - 1);
        } else if (searchMatches.length > 0 && currentMatchIndex === 0) {
            // Optional: loop back to the last match
            // navigateToMatch(searchMatches.length - 1);
        }
    }

    // Function to generate column definitions
    function generateColumns(data, savedLayoutObj) { // Added savedLayoutObj
        console.debug('[TableViewerPanel generateColumns] Received savedLayoutObj:', JSON.stringify(savedLayoutObj, null, 2));
        if (!data || data.length === 0) return [{title: "No Data", field: "placeholder"}]; // Return a placeholder if no data
        const headers = Object.keys(data[0]);
        let columnDefs = headers.map(header => {
            const colDef = {
                title: header,
                field: header,
                headerFilter: "input",
                sorter: inferSorter(data, header),
                formatter: "textarea",
            };

            if (savedLayoutObj && savedLayoutObj.columns && savedLayoutObj.columns[header]) {
                const savedCol = savedLayoutObj.columns[header];
                console.debug(`[TableViewerPanel generateColumns] Applying saved layout for column '${header}': width=${savedCol.width}, order=${savedCol.order}, visible=${savedCol.visible}`);
                if (typeof savedCol.width === 'number' && savedCol.width > 0) { // Ensure width is positive number
                    colDef.width = savedCol.width;
                } else {
                    console.debug(`[TableViewerPanel generateColumns] No valid saved width for column '${header}', default will be used by Tabulator.`);
                }
                colDef.visible = savedCol.visible; // Keep this as it was
            } else {
                console.debug(`[TableViewerPanel generateColumns] No saved layout found for column '${header}'.`);
            }
            return colDef;
        });

        if (savedLayoutObj && savedLayoutObj.columns) {
            columnDefs.sort((a, b) => {
                const orderA = savedLayoutObj.columns[a.field]?.order ?? Infinity;
                const orderB = savedLayoutObj.columns[b.field]?.order ?? Infinity;
                return orderA - orderB;
            });
             // Apply visibility based on saved layout *after* sorting by order
            // This is important if Tabulator doesn't automatically hide columns based on `visible: false` in the definition
            // upon initial load. However, Tabulator usually respects `visible: false` in column definitions.
            // If direct manipulation is needed:
            // columnDefs = columnDefs.filter(colDef => {
            //     const savedCol = savedLayoutObj.columns[colDef.field];
            //     return savedCol ? savedCol.visible !== false : true; // Default to visible if not in saved layout
            // });
            // Or, to set the visible property for Tabulator to interpret:
            columnDefs.forEach(colDef => {
                if (savedLayoutObj.columns[colDef.field] && typeof savedLayoutObj.columns[colDef.field].visible === 'boolean') {
                    colDef.visible = savedLayoutObj.columns[colDef.field].visible;
                }
            });
        }
        console.debug('[TableViewerPanel generateColumns] Final column definitions after applying layout and sort:', JSON.stringify(columnDefs.map(c => ({ field: c.field, width: c.width, visible: c.visible, order: savedLayoutObj?.columns[c.field]?.order })), null, 2));
        return columnDefs;
    }

     // Very basic type inference for better default sorting
     function inferSorter(data, field) {
         if (!data || data.length === 0 || !data[0].hasOwnProperty(field)) return "string";
         const firstValue = data[0][field];
         if (typeof firstValue === 'number') return "number";
         if (typeof firstValue === 'boolean') return "boolean";
         return "string";
     }





    onMount(() => {
        console.debug('[TableViewerPanel] Mounted. Initial Path:', tablePath); // DEBUG
        // tableContainer should be available here due to bind:this
        if (tablePath && tableContainer) {
             initializeTable(tablePath);
        } else {
            isLoading = false; // No path on mount, so not loading
        }
    });

    // React to tablePath changes
    $: {
        if (tablePath && tablePath !== currentLoadedPath) {
            console.debug(`[TableViewerPanel reactive] tablePath changed to '${tablePath}'`); // DEBUG
            if (tableContainer) {
                initializeTable(tablePath);
            } else {
                console.debug(`[TableViewerPanel reactive] Container not ready, deferring init for ${tablePath}`); // DEBUG
                if (!isLoading) isLoading = true;
            }
        } else if (!tablePath && tabulatorInstance) {
            console.info(`[TableViewerPanel reactive] tablePath cleared, destroying table`); // INFO
            tabulatorInstance.destroy();
            tabulatorInstance = null;
            tableData = [];
            isLoading = false;
            error = null;
            currentLoadedPath = null;
        }
    }

    onDestroy(() => {
        console.debug('[TableViewerPanel] Destroyed.'); // DEBUG
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }
    });

</script>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
     <div class="flex items-center justify-between p-2 border-b border-gray-200 dark:border-gray-600 dark:bg-slate-600 flex-shrink-0">
        <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 truncate pr-2" title={tablePath}>
            Data: {tablePath ? tablePath.split(/[\\/]/).pop() : 'No table selected'}
        </h3>
         { #if !isLoading && !error }
         <div class="flex items-center space-x-2">
            <input
              type="search"
              bind:value={searchTerm}
              oninput={handleSearch}
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
              onclick={goToPreviousMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex <= 0}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-left" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M11.354 1.646a.5.5 0 0 1 0 .708L5.707 8l5.647 5.646a.5.5 0 0 1-.708.708l-6-6a.5.5 0 0 1 0-.708l6-6a.5.5 0 0 1 .708 0"/>
              </svg>
            </button>
            <button
              title="Next Match"
              class="p-1 border rounded bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={goToNextMatch}
              disabled={searchMatches.length === 0 || currentMatchIndex >= searchMatches.length - 1}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/>
              </svg>
            </button>
            <button class="text-xs px-2 py-1 border rounded bg-blue-500 hover:bg-blue-600 text-white disabled:opacity-50"
                    onclick={() => { console.log('TODO: Add new row placeholder action'); alert('Add Row (Placeholder)'); }}
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

:global(.tabulator-footer .tabulator-paginator .tabulator-page.active) {
    background-color: #0d6efd !important; /* Using a common Bootstrap primary blue */
    color: white !important;
    font-weight: bold !important;
    border-color: #0d6efd !important; /* Ensure border matches */
}

:global(.tabulator-footer .tabulator-paginator .tabulator-page[aria-current="page"]) {
    background-color: #0d6efd !important; /* Using a common Bootstrap primary blue */
    color: white !important;
    font-weight: bold !important;
    border-color: #0d6efd !important; /* Ensure border matches */
}
</style>