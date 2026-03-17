<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { Modal, Button, Tabs, TabItem, Label, Select, Input, Textarea, Toggle, Helper, Accordion, AccordionItem, Range, Checkbox, MultiSelect } from 'flowbite-svelte';
    import { Table2, LayoutGrid, Database, Palette, Plus, FolderOpen, Trash2, X, Filter } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import notificationStore from '$lib/stores/notificationStore.js';

    const dispatch = createEventDispatcher();

    export let open = false;
    import { TabulatorFull as Tabulator } from 'tabulator-tables';

    export let tablePath = '';
    export let columns = [];
    export let tableData = [];
    export let schema = {};
    export let initialView = null; // Used when opening a saved view from attachments
    export let views = []; // Array of existing views
    export let activeViewName = null; // Current active view from parent

    import { applyViewConfigToData } from './viewTransform.js';

    let previewContainer;
    let previewTabulatorInstance;

    // Normalize path to match DB and Attachments Panel
    $: normalizedTablePath = (() => {
        const projectStoreState = get(project);
        if (!tablePath || !projectStoreState || !projectStoreState.baseDirectory) return tablePath;
        let relative = tablePath.startsWith(projectStoreState.baseDirectory) ? tablePath.substring(projectStoreState.baseDirectory.length) : tablePath;
        return relative.replace(/\\/g, '/').replace(/^\//, '');
    })();

    const viewTypes = [
        { value: 'partial', name: 'Partial Table View', description: 'Select specific columns and apply filters', icon: 'Table2' },
        { value: 'pivot', name: 'Pivot Table', description: 'Summarize data with cross-tabulation', icon: 'LayoutGrid' }
    ];

    let activeTab = 'create';
    let selectedViewType = null;
    let existingViews = [];
    let isEditingExisting = false;
    let prevOpen = false;

    // Common fields
    let viewName = '';
    let viewDescription = '';
    let dataSource = 'Base Table';

    // Partial View fields
    let partialSelectedColumns = [];
    let partialFilterField = '';
    let partialFilterValue = '';
    let partialFilterOperator = 'contains';

    // Pivot View fields
    let pivotRowField = '';
    let pivotColField = '';
    let pivotValueField = '';
    let pivotAggregation = 'Sum'; // Sum, Count, Average, Min, Max

    let activeData = [];
    let activeColumns = [];
    let activeSchema = {};

    $: {
        activeData = tableData;
        activeColumns = columns;
        activeSchema = schema;

        if (dataSource && dataSource !== 'Base Table') {
            const selectedView = views.find(v => v.view_name === dataSource);
            if (selectedView) {
                try {
                    const config = JSON.parse(selectedView.config_json);
                    const { transformedData, transformedColumns, transformedSchema } = applyViewConfigToData(tableData, columns, schema, config, selectedView.view_type);
                    activeData = transformedData;
                    activeColumns = transformedColumns;
                    activeSchema = transformedSchema;
                } catch(e) {
                    console.error("Failed to transform data for view preview using parent view:", e);
                }
            }
        }
    }

    // Derived dropdown options
    $: allColumns = activeColumns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        return { value: fieldName, name: fieldName };
    }).filter(c => c.value && c.value !== 'harvey_internal_id');

    $: numericColumns = allColumns.filter(c => {
        const colSchema = activeSchema[c.value];
        if (colSchema && colSchema.type === 'Numeric') return true;
        return false;
    });
    // Fallback if no numeric schema
    $: pivotValueOptions = numericColumns.length > 0 ? numericColumns : allColumns;

    $: {
        if (open !== prevOpen) {
            prevOpen = open;
            if (open) {
                loadExistingViews().then(() => {
                    if (initialView) {
                        selectExistingView(initialView);
                        initialView = null;
                    } else {
                        resetForm();
                        activeTab = 'create';
                        isEditingExisting = false;
                        viewName = `View-${existingViews.length + 1}`;
                    }
                });
            }
        }
    }

    async function loadExistingViews() {
        try {
            const projectStoreState = get(project);
            if (!projectStoreState.id) return;
            existingViews = await invoke('load_table_views_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath
            });
        } catch (error) {
            console.error('Failed to load existing views:', error);
        }
    }

    function handleTabChange(tab) {
        activeTab = tab;
        if (tab === 'create') {
            resetForm();
            isEditingExisting = false;
            viewName = `View-${existingViews.length + 1}`;
        }
    }

    function resetForm() {
        viewDescription = '';
        selectedViewType = null;
        dataSource = activeViewName || 'Base Table';
        partialSelectedColumns = allColumns.map(c => c.value);
        partialFilterField = '';
        partialFilterValue = '';
        partialFilterOperator = 'contains';
        pivotRowField = '';
        pivotColField = '';
        pivotValueField = '';
        pivotAggregation = 'Sum';
    }

    function selectViewType(type) {
        selectedViewType = type;
        if (type === 'partial' && partialSelectedColumns.length === 0) {
            partialSelectedColumns = allColumns.map(c => c.value);
        }
    }

    async function initialCreate() {
        if (!viewName) viewName = `View-${existingViews.length + 1}`;
        isEditingExisting = true;
        await saveView(true); // Don't trigger explicit non-autosave logic on initial UI transition
    }

    function getCurrentConfig() {
        let config = { description: viewDescription, dataSource };
        if (selectedViewType === 'partial') {
            config.selectedColumns = partialSelectedColumns;
            config.filterField = partialFilterField;
            config.filterValue = partialFilterValue;
            config.filterOperator = partialFilterOperator;
        } else if (selectedViewType === 'pivot') {
            config.rowField = pivotRowField;
            config.colField = pivotColField;
            config.valueField = pivotValueField;
            config.aggregation = pivotAggregation;
        }
        return config;
    }

    async function switchToView() {
        await saveView(true);
        dispatch('viewApplied', { viewName, viewType: selectedViewType, config: getCurrentConfig() });
        open = false;
    }

    function selectExistingView(view) {
        viewName = view.view_name;
        selectedViewType = view.view_type;
        try {
            const config = JSON.parse(view.config_json);
            viewDescription = config.description || '';
            dataSource = config.dataSource || 'Base Table';

            if (selectedViewType === 'partial') {
                partialSelectedColumns = config.selectedColumns || [];
                partialFilterField = config.filterField || '';
                partialFilterValue = config.filterValue || '';
                partialFilterOperator = config.filterOperator || 'contains';
            } else if (selectedViewType === 'pivot') {
                pivotRowField = config.rowField || '';
                pivotColField = config.colField || '';
                pivotValueField = config.valueField || '';
                pivotAggregation = config.aggregation || 'Sum';
            }
        } catch (e) {
            console.error('Failed to parse view config:', e);
        }
        isEditingExisting = true;
        activeTab = 'create';
    }

    async function saveView(isAutoSave = false) {
        if (!viewName) {
            if (!isAutoSave) notificationStore.add('View name is required.', 'error');
            return;
        }
        if (!selectedViewType) {
            if (!isAutoSave) notificationStore.add('View type must be selected.', 'error');
            return;
        }

        const projectStoreState = get(project);
        if (!projectStoreState.id) return;

        let config = getCurrentConfig();

        try {
            await invoke('save_table_view_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath,
                viewName: viewName,
                viewType: selectedViewType,
                configJson: JSON.stringify(config)
            });
            if (!isAutoSave) {
                notificationStore.add('View saved successfully.', 'success');
                dispatch('viewSaved', { viewName, viewType: selectedViewType, config });
            } else {
                dispatch('viewSaved', { viewName, viewType: selectedViewType, config, isAutoSave: true });
            }
            await loadExistingViews();
        } catch (error) {
            console.error('Failed to save view:', error);
            if (!isAutoSave) notificationStore.add('Failed to save view.', 'error');
        }
    }

    async function deleteView(name) {
        const targetName = typeof name === 'string' ? name : viewName;
        if (!targetName) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(`Are you sure you want to delete view ${targetName}?`, { title: 'Delete View', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);
        try {
            await invoke('delete_table_view_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath,
                viewName: targetName
            });
            notificationStore.add('View deleted.', 'success');
            resetForm();
            isEditingExisting = false;
            await loadExistingViews();
            dispatch('viewDeleted', { viewName: targetName });
        } catch (error) {
            console.error('Failed to delete view:', error);
            notificationStore.add('Failed to delete view.', 'error');
        }
    }

    function handleModalClose() {
        if (previewTabulatorInstance) {
            previewTabulatorInstance.destroy();
            previewTabulatorInstance = null;
        }
        open = false;
    }

    function generatePivotData(data, rowField, colField, valueField, aggregation) {
        if (!rowField || !valueField) return { pivotCols: [], pivotData: [] };

        let groupedData = {};
        let allColKeys = new Set();

        // 1. Group Data
        data.forEach(row => {
            const rVal = String(row[rowField] || '(Blank)');
            const cVal = colField ? String(row[colField] || '(Blank)') : 'Total';
            const vVal = parseFloat(row[valueField]) || 0;

            if (!groupedData[rVal]) groupedData[rVal] = {};
            if (!groupedData[rVal][cVal]) groupedData[rVal][cVal] = [];
            groupedData[rVal][cVal].push(vVal);
            allColKeys.add(cVal);
        });

        // 2. Build Columns for Tabulator
        let pivotCols = [
            { field: rowField, title: rowField, frozen: true }
        ];

        let sortedColKeys = Array.from(allColKeys).sort();
        sortedColKeys.forEach(ck => {
            pivotCols.push({ field: ck, title: ck, hozAlign: 'right' });
        });

        // 3. Aggregate Values
        let pivotData = [];
        for (const [rKey, cData] of Object.entries(groupedData)) {
            let rowData = { [rowField]: rKey };
            sortedColKeys.forEach(ck => {
                const vals = cData[ck] || [];
                let aggVal = 0;
                if (vals.length > 0) {
                    if (aggregation === 'Sum') aggVal = vals.reduce((a,b)=>a+b, 0);
                    else if (aggregation === 'Count') aggVal = vals.length;
                    else if (aggregation === 'Average') aggVal = vals.reduce((a,b)=>a+b, 0) / vals.length;
                    else if (aggregation === 'Min') aggVal = Math.min(...vals);
                    else if (aggregation === 'Max') aggVal = Math.max(...vals);
                } else {
                    aggVal = null; // empty cell
                }

                rowData[ck] = aggVal !== null ? (Number.isInteger(aggVal) ? aggVal : parseFloat(aggVal.toFixed(2))) : '';
            });
            pivotData.push(rowData);
        }

        return { pivotCols, pivotData };
    }

    // Reactive statements for auto-saving config
    $: if (isEditingExisting && viewName) {
        // Track dependencies to trigger autosave
        let _ = partialSelectedColumns;
        let __ = partialFilterField;
        let ___ = partialFilterValue;
        let ____ = partialFilterOperator;
        let _____ = pivotRowField;
        let ______ = pivotColField;
        let _______ = pivotValueField;
        let ________ = pivotAggregation;
        let _________ = viewDescription;
        let __________ = dataSource;

        saveView(true);
    }

    // Reactive statement to render the preview table
    $: if (open && isEditingExisting && selectedViewType === 'partial' && previewContainer) {
        // Debounce to allow container to render
        setTimeout(() => {
            if (!previewContainer) return; // Verify still mounted
            if (!previewTabulatorInstance) {
                // Initialize clean instance
                const previewCols = allColumns.map(c => ({
                    field: c.value,
                    title: c.name,
                    visible: partialSelectedColumns.includes(c.value)
                }));

                previewTabulatorInstance = new Tabulator(previewContainer, {
                    data: activeData,
                    columns: previewCols,
                    layout: "fitDataFill",
                    height: "100%",
                    reactiveData: false, // Read only preview
                    selectable: false,
                    nestedFieldSeparator: false
                });
            } else {
                // Update existing instance
                previewTabulatorInstance.replaceData(activeData);
                const allCols = previewTabulatorInstance.getColumns();
                allCols.forEach(col => {
                    const field = col.getField();
                    if (partialSelectedColumns.includes(field)) {
                        col.show();
                    } else {
                        col.hide();
                    }
                });

                previewTabulatorInstance.clearFilter();
                if (partialFilterField && partialFilterValue) {
                    previewTabulatorInstance.setFilter(partialFilterField, partialFilterOperator || 'like', partialFilterValue);
                }
            }
        }, 50);
    } else if (open && isEditingExisting && selectedViewType === 'pivot' && previewContainer) {
        setTimeout(() => {
            if (!previewContainer) return; // Verify still mounted
            if (!pivotRowField || !pivotValueField) {
                if (previewTabulatorInstance) {
                    previewTabulatorInstance.destroy();
                    previewTabulatorInstance = null;
                }
                return;
            }

            const { pivotCols, pivotData } = generatePivotData(activeData, pivotRowField, pivotColField, pivotValueField, pivotAggregation);

            if (pivotCols.length > 0) {
                if (previewTabulatorInstance) {
                    previewTabulatorInstance.setColumns(pivotCols);
                    previewTabulatorInstance.replaceData(pivotData);
                } else {
                    previewTabulatorInstance = new Tabulator(previewContainer, {
                        data: pivotData,
                        columns: pivotCols,
                        layout: "fitDataFill",
                        height: "100%",
                        reactiveData: false,
                        selectable: false,
                        nestedFieldSeparator: false
                    });
                }
            } else if (previewTabulatorInstance) {
                previewTabulatorInstance.destroy();
                previewTabulatorInstance = null;
            }
        }, 50);
    }

</script>

<Modal
    bind:open={open}
    size="5xl"
    on:close={handleModalClose}
    outsideclose
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col h-[85vh] max-h-[900px] relative bg-white dark:bg-gray-900"
>
    <div slot="header" class="flex items-center justify-between w-full pr-4">
        <div class="flex items-center space-x-3">
            <div class="p-2 bg-purple-100 dark:bg-purple-900/30 rounded-lg">
                {#if activeTab === 'create' && isEditingExisting && selectedViewType === 'pivot'}
                    <LayoutGrid size={20} class="text-purple-600 dark:text-purple-400" />
                {:else}
                    <Table2 size={20} class="text-purple-600 dark:text-purple-400" />
                {/if}
            </div>
            <div>
                <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                    {#if activeTab === 'create' && isEditingExisting}
                        {#if selectedViewType === 'pivot'}
                            Edit Pivot Table: {viewName || 'New View'}
                        {:else}
                            Edit Partial Table: {viewName || 'New View'}
                        {/if}
                    {:else}
                        Create Views
                    {/if}
                </h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">
                    Create customized lenses into your table data.
                </p>
            </div>
        </div>
    </div>

    <div class="flex-1 flex overflow-hidden -m-6 h-full border-t border-gray-200 dark:border-gray-700">
        <!-- Left Sidebar: Create / Open Existing -->
        <div class="w-80 border-r border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-800">
            {#if !(activeTab === 'create' && isEditingExisting)}
                <div class="flex border-b border-gray-200 dark:border-gray-700">
                    <button
                        class="flex-1 py-3 text-sm font-medium border-b-2 {activeTab === 'create' ? 'border-purple-600 text-purple-600 dark:border-purple-500 dark:text-purple-500 bg-white dark:bg-gray-900' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 hover:dark:text-gray-300'}"
                        on:click={() => handleTabChange('create')}
                    >
                        <div class="flex items-center justify-center gap-2"><Plus size={16}/> Create</div>
                    </button>
                    <button
                        class="flex-1 py-3 text-sm font-medium border-b-2 {activeTab === 'existing' ? 'border-purple-600 text-purple-600 dark:border-purple-500 dark:text-purple-500 bg-white dark:bg-gray-900' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 hover:dark:text-gray-300'}"
                        on:click={() => handleTabChange('existing')}
                    >
                        <div class="flex items-center justify-center gap-2"><FolderOpen size={16}/> Existing</div>
                    </button>
                </div>
            {/if}

            <div class="flex-1 overflow-y-auto p-4">
                {#if activeTab === 'create'}
                    <div class="space-y-4">
                        {#if !isEditingExisting}
                            <div>
                                <Label for="viewName" class="mb-2">View Name</Label>
                                <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="viewName" bind:value={viewName} placeholder="Enter view name" />
                            </div>
                            <div>
                                <Label for="viewDescription" class="mb-2">Description</Label>
                                <Textarea autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="viewDescription" bind:value={viewDescription} placeholder="Optional description" rows="2" />
                            </div>
                            <div class="text-sm text-gray-500 dark:text-gray-400 italic pt-4 border-t border-gray-200 dark:border-gray-700">
                                Select a view type from the right panel and click Create to begin configuring.
                            </div>
                        {:else}
                            <h3 class="text-center font-bold text-lg text-gray-800 dark:text-gray-200 pb-2">
                                {viewTypes.find(t => t.value === selectedViewType)?.name || 'View Type'} Configuration
                            </h3>

                            <Accordion flush>
                                <AccordionItem>
                                    <span slot="header" class="flex items-center"><Table2 class="w-4 h-4 mr-2" />General Details</span>
                                    <div class="space-y-4">
                                        <div>
                                            <Label for="viewName" class="mb-2">View Name</Label>
                                            <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="viewName" bind:value={viewName} placeholder="Enter view name" />
                                        </div>
                                        <div>
                                            <Label for="viewDescription" class="mb-2">Description</Label>
                                            <Textarea autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="viewDescription" bind:value={viewDescription} placeholder="Optional description" rows="2" />
                                        </div>
                                    </div>
                                </AccordionItem>

                            {#if selectedViewType === 'partial'}
                                <AccordionItem open>
                                    <span slot="header" class="flex items-center"><Table2 class="w-4 h-4 mr-2" />Data Mapping</span>
                                    <div class="space-y-4">
                                        <div>
                                            <Label for="dataSource" class="mb-2">Data Source</Label>
                                            <Select id="dataSource" bind:value={dataSource}>
                                                <option value="Base Table">Base Table</option>
                                                {#each views as view}
                                                    {#if view.view_name !== viewName}
                                                        <option value={view.view_name}>{view.view_name}</option>
                                                    {/if}
                                                {/each}
                                            </Select>
                                        </div>
                                        <div>
                                            <Label class="mb-2">Select Visible Columns</Label>
                                            <MultiSelect items={allColumns} bind:value={partialSelectedColumns} placeholder="Select fields to display" />
                                            <Helper class="mt-2">Columns not selected will be hidden in this view.</Helper>
                                        </div>
                                    </div>
                                </AccordionItem>
                                <AccordionItem>
                                    <span slot="header" class="flex items-center"><Filter class="w-4 h-4 mr-2" />Default Filter</span>
                                    <div class="space-y-4">
                                        <div>
                                            <Label for="filterField" class="mb-2">Filter Column</Label>
                                            <Select id="filterField" items={[{value:'', name:'-- No Filter --'}, ...allColumns]} bind:value={partialFilterField} />
                                        </div>
                                        {#if partialFilterField}
                                            <div>
                                                <Label for="filterOperator" class="mb-2">Operator</Label>
                                                <Select id="filterOperator" items={[{value:'=', name:'Equals'}, {value:'!=', name:'Not Equals'}, {value:'like', name:'Contains'}, {value:'>', name:'Greater Than'}, {value:'<', name:'Less Than'}]} bind:value={partialFilterOperator} />
                                            </div>
                                            <div>
                                                <Label for="filterValue" class="mb-2">Value</Label>
                                                <Input id="filterValue" autocomplete="off" bind:value={partialFilterValue} />
                                            </div>
                                        {/if}
                                    </div>
                                </AccordionItem>
                            {:else if selectedViewType === 'pivot'}
                                <AccordionItem open>
                                    <span slot="header" class="flex items-center"><LayoutGrid class="w-4 h-4 mr-2" />Data Mapping</span>
                                    <div class="space-y-4">
                                        <div>
                                            <Label for="dataSource" class="mb-2">Data Source</Label>
                                            <Select id="dataSource" bind:value={dataSource}>
                                                <option value="Base Table">Base Table</option>
                                                {#each views as view}
                                                    {#if view.view_name !== viewName}
                                                        <option value={view.view_name}>{view.view_name}</option>
                                                    {/if}
                                                {/each}
                                            </Select>
                                        </div>
                                        <div>
                                            <Label for="pivotRow" class="mb-2">Row Field (Group By)</Label>
                                            <Select id="pivotRow" items={allColumns} bind:value={pivotRowField} />
                                        </div>
                                        <div>
                                            <Label for="pivotCol" class="mb-2">Column Field (Pivot Across)</Label>
                                            <Select id="pivotCol" items={[{value:'', name:'-- None --'}, ...allColumns]} bind:value={pivotColField} />
                                        </div>
                                        <div>
                                            <Label for="pivotValue" class="mb-2">Value Field (To Aggregate)</Label>
                                            <Select id="pivotValue" items={pivotValueOptions} bind:value={pivotValueField} />
                                        </div>
                                        <div>
                                            <Label for="pivotAgg" class="mb-2">Aggregation Type</Label>
                                            <Select id="pivotAgg" items={[{value:'Sum', name:'Sum'}, {value:'Count', name:'Count'}, {value:'Average', name:'Average'}, {value:'Min', name:'Min'}, {value:'Max', name:'Max'}]} bind:value={pivotAggregation} />
                                        </div>
                                    </div>
                                </AccordionItem>
                            {/if}
                            </Accordion>

                        {/if}
                    </div>
                {:else if activeTab === 'existing'}
                    {#if existingViews.length === 0}
                        <div class="text-sm text-gray-500 dark:text-gray-400 italic">No existing views found for this table.</div>
                    {:else}
                        <ul class="space-y-2">
                            {#each existingViews as view}
                                <li>
                                    <div class="w-full flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors group">
                                        <button
                                            class="flex-1 text-left flex flex-col items-start pr-2 overflow-hidden"
                                            on:click={() => selectExistingView(view)}
                                        >
                                            <div class="font-medium text-gray-900 dark:text-white truncate w-full">{view.view_name}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide mt-1">{view.view_type} View</div>
                                        </button>
                                        <div class="flex flex-col items-end gap-1 flex-shrink-0">
                                            <Button size="xs" color="light" class="p-1 border-0 shadow-none opacity-0 group-hover:opacity-100 transition-opacity bg-transparent hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-900/30" on:click={() => deleteView(view.view_name)} title="Delete">
                                                <Trash2 class="w-3.5 h-3.5" />
                                            </Button>
                                        </div>
                                    </div>
                                </li>
                            {/each}
                        </ul>
                    {/if}
                {/if}
            </div>
        </div>

        <!-- Right Content Area (Preview / Selection) -->
        <div class="flex-1 flex flex-col overflow-hidden bg-white dark:bg-gray-900 relative">
            {#if activeTab === 'create' && !isEditingExisting}
                <div class="p-8 h-full overflow-y-auto bg-gray-50/30 dark:bg-gray-900/30 flex flex-col">
                    <h4 class="text-xl font-semibold mb-6 text-gray-900 dark:text-white">Select View Type</h4>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {#each viewTypes as type}
                            <button
                                class="flex flex-col items-start p-6 border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 rounded-lg hover:border-purple-500 dark:hover:border-purple-500 hover:shadow-md transition-all group relative overflow-hidden text-left"
                                class:border-purple-500={selectedViewType === type.value}
                                class:ring-2={selectedViewType === type.value}
                                class:ring-purple-500={selectedViewType === type.value}
                                on:click={() => selectViewType(type.value)}
                            >
                                <div class="w-12 h-12 bg-purple-50 dark:bg-purple-900/30 rounded-lg flex items-center justify-center mb-4 text-purple-600 dark:text-purple-400">
                                    {#if type.icon === 'Table2'}<Table2 size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'LayoutGrid'}<LayoutGrid size={24} strokeWidth={2} />{/if}
                                </div>
                                <span class="font-bold text-lg text-gray-900 dark:text-white mb-2">{type.name}</span>
                                <span class="text-sm text-gray-500 dark:text-gray-400">{type.description}</span>
                            </button>
                        {/each}
                    </div>
                    <div class="absolute bottom-6 right-6 z-10">
                        <Button color="purple" disabled={!selectedViewType} on:click={initialCreate} class="px-6 shadow-md">
                            Create View
                        </Button>
                    </div>
                </div>
            {:else if activeTab === 'create' && isEditingExisting}
                {#if selectedViewType === 'partial'}
                    <div class="flex-1 w-full h-full p-4 bg-gray-50/50 dark:bg-gray-900/50 border-l border-gray-100 dark:border-gray-800 flex flex-col">
                        <div class="mb-2 text-sm font-semibold text-gray-600 dark:text-gray-400 flex items-center gap-2">
                            <Table2 size={16} /> Live Preview: {viewName}
                        </div>
                        <div class="flex-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-sm overflow-hidden" bind:this={previewContainer}></div>
                    </div>
                {:else if selectedViewType === 'pivot'}
                    <div class="flex-1 w-full h-full p-4 bg-gray-50/50 dark:bg-gray-900/50 border-l border-gray-100 dark:border-gray-800 flex flex-col">
                        <div class="mb-2 text-sm font-semibold text-gray-600 dark:text-gray-400 flex items-center gap-2">
                            <LayoutGrid size={16} /> Live Preview: {viewName}
                        </div>
                        <div class="flex-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-sm overflow-hidden" bind:this={previewContainer}></div>
                    </div>
                {/if}
            {:else if activeTab === 'existing'}
                 <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400 italic">
                    Select a view from the list to edit its configuration.
                 </div>
            {/if}
        </div>
    </div>

    <svelte:fragment slot="footer">
        {#if activeTab === 'create' && isEditingExisting}
        <div class="flex justify-end w-full space-x-2">
            <Button color="alternative" on:click={handleModalClose}>Close</Button>
            <Button color="purple" on:click={switchToView}>Switch to this view</Button>
        </div>
        {/if}
    </svelte:fragment>
</Modal>