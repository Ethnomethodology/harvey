<!-- src/lib/components/projectview/notes/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { loadTableData } from '$lib/services/projectService.js';

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

    // Placeholder functions (Unchanged)
    function openRowForm(row) {
        const rowData = row.getData();
        console.log("Placeholder: Open form view for row:", rowData);
        alert(`Open Form View (Placeholder)\n\nRow Data:\n${JSON.stringify(rowData, null, 2)}`);
    }
    function addComment(cell) {
        const cellValue = cell.getValue();
        const columnName = cell.getColumn().getField();
        const rowData = cell.getRow().getData();
        console.log(`Placeholder: Add comment to cell (${columnName}: ${cellValue})`, rowData);
        const comment = prompt(`Add comment for "${columnName}" in this row:`, "");
        if (comment !== null) {
            alert(`Comment Added (Placeholder):\n"${comment}"`);
        }
    }
     function addHighlight(cell) {
        const cellValue = cell.getValue();
        const columnName = cell.getColumn().getField();
        const rowData = cell.getRow().getData();
        console.log(`Placeholder: Add highlight to cell (${columnName}: ${cellValue})`, rowData);
         const highlight = confirm(`Highlight this cell?`);
         if (highlight) {
             alert(`Cell Highlighted (Placeholder)`);
            cell.getElement().classList.toggle('cell-highlighted-placeholder');
         }
     }

    async function initializeTable(pathForTable) {
        if (!pathForTable || !tableContainer) {
            console.log('[TableViewerPanel initializeTable] Skipping: no path or container.', { pathForTable, tableContainerExists: !!tableContainer });
            isLoading = false; // Ensure loading is false if we skip
            return;
        }

        // Prevent re-initialization if already loading or loaded for this path
        if (isLoading && currentLoadedPath === pathForTable) {
            console.log(`[TableViewerPanel initializeTable] Already loading ${pathForTable}, skipping.`);
            return;
        }
        if (!isLoading && tabulatorInstance && currentLoadedPath === pathForTable) {
            console.log(`[TableViewerPanel initializeTable] Table for ${pathForTable} already initialized and loaded.`);
            return;
        }


        console.log(`[TableViewerPanel initializeTable] Initializing for path: ${pathForTable}`);
        currentLoadedPath = pathForTable; // Set path being processed
        isLoading = true;
        error = null;
        tableData = [];

        if (tabulatorInstance) {
            console.log('[TableViewerPanel initializeTable] Destroying previous Tabulator instance.');
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }

        try {
            tableData = await loadTableData(pathForTable);

            if (tableData.length === 0) {
                console.log('[TableViewerPanel initializeTable] No data returned from loadTableData.');
            }

            await tick(); // Ensure DOM is ready if container was re-rendered

            if (!tableContainer) {
                 console.error('[TableViewerPanel initializeTable] Table container element became null during data load for path:', pathForTable);
                 error = 'Failed to initialize table viewer: container lost.';
                 isLoading = false;
                 currentLoadedPath = null; // Reset if failed
                 return;
            }

            console.log(`[TableViewerPanel initializeTable] Creating Tabulator instance for ${pathForTable}`);
            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                layout: "fitDataTable", // Changed back from fitColumns
                columns: generateColumns(tableData),
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

            tabulatorInstance.on("rowClick", function(e, row){
                 console.log("Row Clicked:", row.getData());
            });

            // Disable macOS autocorrect/autocomplete on column header filters
            tabulatorInstance.on("renderComplete", () => {
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
                    console.log('[TableViewerPanel initializeTable] Triggering a gentle redraw after short delay.');
                    tabulatorInstance.redraw(); // Using redraw() without 'true' for a less disruptive redraw
                }
            }, 100); // 100ms delay, can be adjusted

            const columns = tabulatorInstance.getColumnDefinitions();
            columnFields = columns.map(col => col.field).filter(field => field && field !== 'placeholder'); // Store actual field names

            console.log(`[TableViewerPanel initializeTable] Tabulator initialized for ${pathForTable}.`);

        } catch (err) {
            console.error(`[TableViewerPanel initializeTable] Error for path ${pathForTable}:`, err);
            error = `Failed to load table: ${err.message || err}`;
            currentLoadedPath = null; // Reset if failed
        } finally {
            // Only set isLoading to false if we are still on the same path.
            // This prevents a quick flash if path changes rapidly.
            if (currentLoadedPath === pathForTable) {
                isLoading = false;
            }
            console.log(`[TableViewerPanel initializeTable] Finished for ${pathForTable}. isLoading: ${isLoading}`);
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
        console.log(`Found ${searchMatches.length} rows matching "${term}"`);
    }

    function clearHighlights() {
        if (tabulatorInstance) {
            tabulatorInstance.deselectRow(); // Deselect all rows
        }
        console.log("clearHighlights called: deselected all rows.");
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
                console.error("Error navigating to match:", err);
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

    // Function to generate column definitions (Unchanged)
    function generateColumns(data) {
        if (!data || data.length === 0) return [{title: "No Data", field: "placeholder"}]; // Return a placeholder if no data
        const headers = Object.keys(data[0]);
        return headers.map(header => ({
            title: header,
            field: header,
            headerFilter: "input",
            sorter: inferSorter(data, header),
        }));
    }

     // Very basic type inference for better default sorting (Unchanged)
     function inferSorter(data, field) {
         if (!data || data.length === 0 || !data[0].hasOwnProperty(field)) return "string";
         const firstValue = data[0][field];
         if (typeof firstValue === 'number') return "number";
         if (typeof firstValue === 'boolean') return "boolean";
         return "string";
     }





    onMount(() => {
        console.log('[TableViewerPanel] Mounted. Initial Path:', tablePath);
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
            console.log(`[TableViewerPanel reactive] tablePath changed to '${tablePath}'`);
            if (tableContainer) {
                initializeTable(tablePath);
            } else {
                console.log(`[TableViewerPanel reactive] Container not ready, deferring init for ${tablePath}`);
                if (!isLoading) isLoading = true;
            }
        } else if (!tablePath && tabulatorInstance) {
            console.log(`[TableViewerPanel reactive] tablePath cleared, destroying table`);
            tabulatorInstance.destroy();
            tabulatorInstance = null;
            tableData = [];
            isLoading = false;
            error = null;
            currentLoadedPath = null;
        }
    }

    onDestroy(() => {
        console.log('[TableViewerPanel] Destroyed.');
        if (tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }
    });

</script>

<!-- Import Tabulator CSS -->
<svelte:head>
	<link href="/tabulator_themes/tabulator_modern.min.css" rel="stylesheet">
</svelte:head>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
     <div class="flex items-center justify-between p-2 border-b border-gray-200 dark:border-gray-600 flex-shrink-0">
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
                    onclick={() => { console.log('TODO: Add new row'); alert('Add Row (Placeholder)'); }}
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

:global(.tabulator .tabulator-row .tabulator-cell:first-child) {
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