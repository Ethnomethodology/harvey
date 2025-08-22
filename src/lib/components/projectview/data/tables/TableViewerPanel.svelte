<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { loadTableData, saveTableData, saveTableLayoutPrefs, loadTableLayoutPrefs, renameTableHeader } from '$lib/services/projectService.js';
    import { project } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';
    import { sep } from '@tauri-apps/api/path';
    import { getRelativePath } from '$lib/utils/pathUtils.js';
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

    let showCustomRowMenu = false;
    let customMenuX = 0;
    let customMenuY = 0;
    let clickedRowComponent = null;

    let showColorPicker = false;
    let colorPickerCallback = null;

    const highlightOptions = HIGHLIGHT_OPTIONS;

    let searchTerm = '';
    let searchMatches = [];
    let currentMatchIndex = -1;
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };
    let selectedCellCache = [];

    function styledCellFormatter(cell, formatterParams, onRendered) {
        const data = cell.getRow().getData();
        const field = cell.getField();
        const element = cell.getElement();

        if (data._cellStyles && data._cellStyles[field] && data._cellStyles[field].backgroundColor) {
            element.style.backgroundColor = data._cellStyles[field].backgroundColor;
        } else {
            element.style.backgroundColor = '';
        }

        element.style.whiteSpace = "pre-wrap";
        return cell.getValue();
    }

    function generateColumns(data, headers, savedLayoutObj) {
        if (!headers || headers.length === 0) return [{ title: "No Data", field: "placeholder" }];

        const rowNumColumn = {
            title: "#",
            formatter: "rownum",
            width: 50,
            hozAlign: "center",
            resizable: false,
            headerSort: false,
        };

        let dataColumnDefs = headers.map(header => {
            const colDef = {
                title: header,
                field: header,
                headerFilter: "input",
                sorter: "string",
                editor: "textarea",
                formatter: styledCellFormatter,
                headerContextMenu: [
                    { label: "Edit Header", action: (e, column) => openHeaderEditor(column) },
                ],
                contextMenu: (cell) => {
                    return [
                        { label: "Copy", action: () => navigator.clipboard.writeText(cell.getValue()) },
                        { label: "Cut", action: () => { navigator.clipboard.writeText(cell.getValue()); cell.setValue(""); } },
                        { label: "Paste", action: () => navigator.clipboard.readText().then(text => cell.setValue(text)) },
                        { label: "Delete", action: () => cell.setValue("") },
                        { separator: true },
                        {
                            label: "Color Cell...",
                            action: () => {
                                colorPickerCallback = (color) => {
                                    const ranges = tabulatorInstance.getRanges();
                                    if (ranges && ranges.length > 0) {
                                        ranges.forEach(range => {
                                            const cells = range.getCells();
                                            cells.forEach(c => {
                                                const row = c.getRow();
                                                let rowData = row.getData();
                                                let field = c.getField();
                                                if (!rowData._cellStyles) rowData._cellStyles = {};
                                                if (!rowData._cellStyles[field]) rowData._cellStyles[field] = {};
                                                rowData._cellStyles[field].backgroundColor = color;
                                                row.update({ _cellStyles: rowData._cellStyles });
                                            });
                                        });
                                    } else {
                                        const row = cell.getRow();
                                        let rowData = row.getData();
                                        let field = cell.getField();
                                        if (!rowData._cellStyles) rowData._cellStyles = {};
                                        if (!rowData._cellStyles[field]) rowData._cellStyles[field] = {};
                                        rowData._cellStyles[field].backgroundColor = color;
                                        row.update({ _cellStyles: rowData._cellStyles });
                                    }
                                    saveTableData(tablePath, tableData);
                                };
                                showColorPicker = true;
                            }
                        },
                        {
                            label: "Clear Cell Color",
                            action: () => {
                                const ranges = tabulatorInstance.getRanges();
                                if (ranges && ranges.length > 0) {
                                    ranges.forEach(range => {
                                        const cells = range.getCells();
                                        cells.forEach(c => {
                                            const row = c.getRow();
                                            let rowData = row.getData();
                                            let field = c.getField();
                                            if (rowData._cellStyles && rowData._cellStyles[field]) {
                                                delete rowData._cellStyles[field].backgroundColor;
                                                if (Object.keys(rowData._cellStyles[field]).length === 0) {
                                                    delete rowData._cellStyles[field];
                                                }
                                                row.update({ _cellStyles: rowData._cellStyles });
                                            }
                                        });
                                    });
                                } else {
                                    const row = cell.getRow();
                                    let rowData = row.getData();
                                    let field = cell.getField();
                                    if (rowData._cellStyles && rowData._cellStyles[field]) {
                                        delete rowData._cellStyles[field].backgroundColor;
                                        if (Object.keys(rowData._cellStyles[field]).length === 0) {
                                            delete rowData._cellStyles[field];
                                        }
                                        row.update({ _cellStyles: rowData._cellStyles });
                                    }
                                }
                                saveTableData(tablePath, tableData);
                            }
                        }
                    ];
                }
            };
            if (savedLayoutObj && savedLayoutObj.columns && savedLayoutObj.columns[header]) {
                const savedCol = savedLayoutObj.columns[header];
                if (savedCol.width) colDef.width = savedCol.width;
                colDef.visible = savedCol.visible;
            }
            return colDef;
        });

        if (savedLayoutObj && savedLayoutObj.columns) {
            dataColumnDefs.sort((a, b) => (savedLayoutObj.columns[a.field]?.order ?? Infinity) - (savedLayoutObj.columns[b.field]?.order ?? Infinity));
        }

        return [rowNumColumn, ...dataColumnDefs];
    }

    async function initializeTable(path, headersExist) {
        if (!path || !tableContainer) return;
        currentLoadedPath = path;
        isLoading = true;
        error = null;

        if (tabulatorInstance) tabulatorInstance.destroy();

        try {
            const response = await loadTableData(path, headersExist);
            tableData = response.data;
            const headers = response.headers;

            const relativePath = getRelativePath(path, get(project)?.baseDirectory);
            const savedLayout = relativePath ? await loadTableLayoutPrefs(relativePath) : null;

            await tick();

            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                columns: generateColumns(tableData, headers, savedLayout),
                height: "100%",
                layout: "fitData",
                history: true,
                selectableRange: true,
                columnDefaults: {
                    resizable: "header",
                    headerSort: false,
                },
            });

            tabulatorInstance.on("cellEdited", (cell) => {
                saveTableData(tablePath, tableData);
            });

        } catch (err) {
            error = `Failed to load table: ${err.message || err}`;
        } finally {
            isLoading = false;
        }
    }

    onMount(() => {
        if (tablePath) initializeTable(tablePath, hasHeaders);
    });

    $: if (tablePath && tablePath !== currentLoadedPath) {
        initializeTable(tablePath, hasHeaders);
    }
</script>

<div class="flex flex-col h-full w-full">
    <div class="flex-grow overflow-auto min-h-0 relative">
        {#if isLoading}
            <div class="p-4 text-center">Loading...</div>
        {:else if error}
            <div class="p-4 text-center text-red-500">{error}</div>
        {/if}
        <div bind:this={tableContainer} class="w-full h-full"></div>
    </div>
</div>

<ColorPickerModal
    bind:show={showColorPicker}
    on:save={(e) => {
        if (colorPickerCallback) colorPickerCallback(e.detail.color);
        colorPickerCallback = null;
    }}
    on:cancel={() => colorPickerCallback = null}
/>

<style>
    :global(.tabulator) {
        border: none;
    }
</style>
