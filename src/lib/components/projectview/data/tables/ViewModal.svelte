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

    import { FileText } from 'lucide-svelte';

    const viewTypes = [
        { value: 'partial', name: 'Partial Table View', description: 'Select specific columns and apply filters', icon: 'Table2' },
        { value: 'pivot', name: 'Pivot Table', description: 'Summarize data with cross-tabulation', icon: 'LayoutGrid' },
        { value: 'survey', name: 'Survey Data Table', description: 'Generate documents from survey data', icon: 'FileText' }
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
    let pivotRowFields = [];
    let pivotColFields = [];
    let pivotValueFields = [{ field: '', aggregation: 'Sum' }];

    // Survey View fields
    let surveyGroupByType = 'Participants'; // 'Participants' or 'Questions'
    let surveyUniqueIdentifierField = '';
    let surveyParticipantIncludedFields = [];
    let surveySelectedQuestions = [];
    let surveyIncludedOtherFields = [];

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

    $: requiredOrPrimaryColumns = allColumns.filter(c => {
        const colSchema = activeSchema[c.value];
        return colSchema && (colSchema.primary === true || colSchema.required === true);
    }).map(c => c.value);

    // Force inclusion of required/primary columns in the partial view selection
    $: if (selectedViewType === 'partial' && partialSelectedColumns) {
        const missingRequired = requiredOrPrimaryColumns.filter(f => !partialSelectedColumns.includes(f));
        if (missingRequired.length > 0) {
            // Re-assign to trigger reactivity and keep required columns selected
            partialSelectedColumns = [...new Set([...partialSelectedColumns, ...requiredOrPrimaryColumns])];
        }
    }

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
        // Default to activeViewName if it's a partial view, otherwise Base Table
        const isActiveViewPartial = views.some(v => v.view_name === activeViewName && v.view_type === 'partial');
        dataSource = isActiveViewPartial ? activeViewName : 'Base Table';
        partialSelectedColumns = allColumns.map(c => c.value);
        partialFilterField = '';
        partialFilterValue = '';
        partialFilterOperator = 'contains';
        pivotRowFields = [];
        pivotColFields = [];
        pivotValueFields = [{ field: '', aggregation: 'Sum' }];

        surveyGroupByType = 'Participants';
        surveyUniqueIdentifierField = '';
        surveyParticipantIncludedFields = allColumns.map(c => c.value);
        surveySelectedQuestions = [];
        surveyIncludedOtherFields = [];
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

        // Survey views generate actual documents on save.
        // We do not want to auto-save and trigger generation on initial creation
        // before the user has configured the survey options.
        if (selectedViewType !== 'survey') {
            await saveView(true); // Don't trigger explicit non-autosave logic on initial UI transition
        }
    }

    function getCurrentConfig() {
        let config = { description: viewDescription, dataSource };
        if (selectedViewType === 'partial') {
            config.selectedColumns = partialSelectedColumns;
            config.filterField = partialFilterField;
            config.filterValue = partialFilterValue;
            config.filterOperator = partialFilterOperator;
        } else if (selectedViewType === 'pivot') {
            config.rowFields = pivotRowFields;
            config.colFields = pivotColFields;
            config.valueFields = pivotValueFields.filter(vf => vf.field); // filter out empties
        } else if (selectedViewType === 'survey') {
            config.surveyGroupByType = surveyGroupByType;
            config.surveyUniqueIdentifierField = surveyUniqueIdentifierField;
            if (surveyGroupByType === 'Participants') {
                config.surveyParticipantIncludedFields = surveyParticipantIncludedFields;
            } else {
                config.surveySelectedQuestions = surveySelectedQuestions;
                config.surveyIncludedOtherFields = surveyIncludedOtherFields;
            }
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
                // Handle backwards compatibility with old single-field config
                if (config.rowField) pivotRowFields = [config.rowField];
                else pivotRowFields = config.rowFields || [];

                if (config.colField) pivotColFields = [config.colField];
                else pivotColFields = config.colFields || [];

                if (config.valueFields && config.valueFields.length > 0) {
                    pivotValueFields = config.valueFields;
                } else if (config.valueField) {
                    pivotValueFields = [{ field: config.valueField, aggregation: config.aggregation || 'Sum' }];
                } else {
                    pivotValueFields = [{ field: '', aggregation: 'Sum' }];
                }
            } else if (selectedViewType === 'survey') {
                surveyGroupByType = config.surveyGroupByType || 'Participants';
                surveyUniqueIdentifierField = config.surveyUniqueIdentifierField || '';
                surveyParticipantIncludedFields = config.surveyParticipantIncludedFields || allColumns.map(c => c.value);
                surveySelectedQuestions = config.surveySelectedQuestions || [];
                surveyIncludedOtherFields = config.surveyIncludedOtherFields || [];
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
                if (selectedViewType === 'survey') {
                    // For survey, generate the actual documents on explicit save
                    try {
                        notificationStore.add('Generating documents, this may take a moment...', 'info');
                        await invoke('generate_survey_documents_command', {
                            projectId: projectStoreState.id,
                            tablePath: normalizedTablePath,
                            viewName: viewName,
                            configJson: JSON.stringify(config),
                            projectXmlPathStr: projectStoreState.xmlPath
                        });
                        notificationStore.add('Survey documents generated. Check the Attachments Panel.', 'success');
                    } catch (genError) {
                        console.error('Failed to generate survey documents:', genError);
                        notificationStore.add('View saved, but failed to generate documents.', 'error');
                    }
                } else {
                    notificationStore.add('View saved successfully.', 'success');
                }
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
        const projectStoreState = get(project);
        let promptMessage = `Are you sure you want to delete view ${targetName}?`;

        // Find if it's a survey view to warn about documents
        const viewToDelete = views.find(v => v.view_name === targetName);
        if (viewToDelete && viewToDelete.view_type === 'survey') {
            promptMessage += `\n\nWARNING: Deleting this Survey Data Table view will also permanently delete ALL generated .json documents associated with it. This action cannot be undone.`;
        }

        const confirmed = await ask(promptMessage, { title: 'Delete View', type: 'warning' });
        if (!confirmed) return;

        try {
            await invoke('delete_table_view_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath,
                viewName: targetName,
                projectXmlPathStr: projectStoreState.xmlPath
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

    function generatePivotData(data, rowFields, colFields, valueField, aggregation, valueFieldsObj = []) {
        let actualValueFields = valueFieldsObj.filter(vf => vf.field);
        if (actualValueFields.length === 0 && valueField) {
            actualValueFields.push({ field: valueField, aggregation: aggregation || 'Sum' });
        }

        if (!rowFields || rowFields.length === 0 || actualValueFields.length === 0) return { colHeaders: [], rows: [] };

        let rowTree = {};
        let allColLeaves = new Set();

        data.forEach(row => {
            let currentLevel = rowTree;
            for (let i = 0; i < rowFields.length; i++) {
                const field = rowFields[i];
                const val = String(row[field] || '(Blank)');
                if (!currentLevel[val]) {
                    currentLevel[val] = {
                        _val: val,
                        _field: field,
                        _children: i === rowFields.length - 1 ? null : {},
                        _data: []
                    };
                }
                currentLevel = currentLevel[val]._children || currentLevel[val];
                if (i === rowFields.length - 1) {
                    currentLevel._data.push(row);
                }
            }

            let cVals = colFields ? colFields.map(f => String(row[f] || '(Blank)')) : [];
            actualValueFields.forEach(vf => {
                const keyParts = [...cVals, `${vf.field} (${vf.aggregation})`];
                allColLeaves.add(JSON.stringify(keyParts));
            });
        });

        const colLeaves = Array.from(allColLeaves).map(c => JSON.parse(c)).sort();

        function aggregateRows(rows, vfParts, colFieldsArray) {
            const matchColParts = vfParts.slice(0, -1);
            const vfPart = vfParts[vfParts.length - 1];
            const match = vfPart.match(/(.+) \((Sum|Count|Average|Min|Max)\)$/);
            if (!match) return null;
            const vField = match[1];
            const aggType = match[2];

            let filteredRows = rows;
            if (colFieldsArray && colFieldsArray.length > 0) {
                filteredRows = rows.filter(r => {
                    return colFieldsArray.every((cf, i) => String(r[cf] || '(Blank)') === matchColParts[i]);
                });
            }

            if (filteredRows.length === 0) return null;

            let vals = filteredRows.map(r => parseFloat(r[vField]) || 0);
            if (aggType === 'Sum') return vals.reduce((a,b)=>a+b, 0);
            if (aggType === 'Count') return vals.length;
            if (aggType === 'Average') return vals.reduce((a,b)=>a+b, 0) / vals.length;
            if (aggType === 'Min') return Math.min(...vals);
            if (aggType === 'Max') return Math.max(...vals);
            return null;
        }

        let flatRows = [];

        function traverseRowTree(nodeMap, currentDepth) {
            let totalRowSpan = 0;
            let childRows = [];

            const keys = Object.keys(nodeMap).sort();
            for (const k of keys) {
                const node = nodeMap[k];
                let rowSpan = 1;
                let descendants = [];

                if (node._children) {
                    const res = traverseRowTree(node._children, currentDepth + 1);
                    rowSpan = res.totalRowSpan;
                    descendants = res.childRows;
                } else {
                    let rowData = {};
                    colLeaves.forEach((colLeafParts, i) => {
                        const aggVal = aggregateRows(node._data, colLeafParts, colFields);
                        rowData[`val_${i}`] = aggVal !== null ? (Number.isInteger(aggVal) ? aggVal : parseFloat(aggVal.toFixed(2))) : '';
                    });
                    descendants = [{ data: rowData, headers: [] }];
                }

                totalRowSpan += rowSpan;

                descendants.forEach((d, i) => {
                    d.headers.unshift({ val: k, rowspan: i === 0 ? rowSpan : 0 });
                });

                childRows.push(...descendants);
            }

            return { totalRowSpan, childRows };
        }

        let { childRows } = traverseRowTree(rowTree, 0);

        const colDepth = (colFields ? colFields.length : 0) + 1;
        let colHeaders = Array.from({length: colDepth}, () => []);

        for (let level = 0; level < colDepth; level++) {
            let currentVal = null;
            let colspan = 0;

            colLeaves.forEach((leafParts, idx) => {
                const val = leafParts[level];
                if (val !== currentVal) {
                    if (colspan > 0) colHeaders[level].push({ val: currentVal, colspan });
                    currentVal = val;
                    colspan = 1;
                } else {
                    colspan++;
                }

                if (idx === colLeaves.length - 1) {
                    colHeaders[level].push({ val: currentVal, colspan });
                }
            });
        }

        return {
            colHeaders,
            rows: childRows,
            rowFieldsCount: rowFields.length,
            colLeavesCount: colLeaves.length
        };
    }

    // Reactive statements for auto-saving config
    $: if (isEditingExisting && viewName && selectedViewType !== 'survey') {
        // Track dependencies to trigger autosave
        let _ = partialSelectedColumns;
        let __ = partialFilterField;
        let ___ = partialFilterValue;
        let ____ = partialFilterOperator;
        let _____ = pivotRowFields;
        let ______ = pivotColFields;
        let _______ = JSON.stringify(pivotValueFields);
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
        // Pivot tables are now rendered natively with HTML/Tailwind, so we destroy any active tabulator instance
        if (previewTabulatorInstance) {
            previewTabulatorInstance.destroy();
            previewTabulatorInstance = null;
        }
    }

    // Reactive pivot data generation for native HTML rendering
    let generatedPivotResult = { colHeaders: [], rows: [], rowFieldsCount: 0, colLeavesCount: 0 };

    $: if (open && isEditingExisting && selectedViewType === 'pivot') {
        const validValueFields = pivotValueFields.filter(vf => vf.field);
        if (pivotRowFields && pivotRowFields.length > 0 && validValueFields.length > 0) {
            generatedPivotResult = generatePivotData(activeData, pivotRowFields, pivotColFields, validValueFields[0].field, validValueFields[0].aggregation, validValueFields);
        } else {
            generatedPivotResult = { colHeaders: [], rows: [], rowFieldsCount: 0, colLeavesCount: 0 };
        }
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
            <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                {#if activeTab === 'create' && isEditingExisting && selectedViewType === 'pivot'}
                    <LayoutGrid size={20} class="text-blue-600 dark:text-blue-400" />
                {:else if activeTab === 'create' && isEditingExisting && selectedViewType === 'survey'}
                    <FileText size={20} class="text-blue-600 dark:text-blue-400" />
                {:else}
                    <Table2 size={20} class="text-blue-600 dark:text-blue-400" />
                {/if}
            </div>
            <div>
                <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                    {#if activeTab === 'create' && isEditingExisting}
                        {#if selectedViewType === 'pivot'}
                            Edit Pivot Table: {viewName || 'New View'}
                        {:else if selectedViewType === 'survey'}
                            Edit Survey Data Table: {viewName || 'New View'}
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
        <!-- For survey configuration, we expand the sidebar to fill or take more space since there's no live preview -->
        <div class="border-r border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-800 transition-all duration-300 {activeTab === 'create' && isEditingExisting && selectedViewType === 'survey' ? 'w-[500px]' : 'w-80'}">
            {#if !(activeTab === 'create' && isEditingExisting)}
                <div class="flex border-b border-gray-200 dark:border-gray-700">
                    <button
                        class="flex-1 py-3 text-sm font-medium border-b-2 {activeTab === 'create' ? 'border-blue-600 text-blue-600 dark:border-blue-500 dark:text-blue-500 bg-white dark:bg-gray-900' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 hover:dark:text-gray-300'}"
                        on:click={() => handleTabChange('create')}
                    >
                        <div class="flex items-center justify-center gap-2"><Plus size={16}/> Create</div>
                    </button>
                    <button
                        class="flex-1 py-3 text-sm font-medium border-b-2 {activeTab === 'existing' ? 'border-blue-600 text-blue-600 dark:border-blue-500 dark:text-blue-500 bg-white dark:bg-gray-900' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 hover:dark:text-gray-300'}"
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
                                                    {#if view.view_name !== viewName && view.view_type === 'partial'}
                                                        <option value={view.view_name}>{view.view_name}</option>
                                                    {/if}
                                                {/each}
                                            </Select>
                                        </div>
                                        <div>
                                            <Label class="mb-2">Select Visible Columns</Label>
                                            <MultiSelect items={allColumns} bind:value={partialSelectedColumns} placeholder="Select fields to display" />
                                            <Helper class="mt-2">Columns not selected will be hidden in this view. <span class="font-semibold text-blue-600 dark:text-blue-400">Primary and required fields cannot be hidden.</span></Helper>
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
                                                    {#if view.view_name !== viewName && view.view_type === 'partial'}
                                                        <option value={view.view_name}>{view.view_name}</option>
                                                    {/if}
                                                {/each}
                                            </Select>
                                        </div>
                                        <div>
                                            <Label for="pivotRow" class="mb-2">Row Fields (Group By)</Label>
                                            <MultiSelect id="pivotRow" items={allColumns} bind:value={pivotRowFields} placeholder="Select row fields" />
                                        </div>
                                        <div>
                                            <Label for="pivotCol" class="mb-2">Column Fields (Pivot Across)</Label>
                                            <MultiSelect id="pivotCol" items={allColumns} bind:value={pivotColFields} placeholder="Select column fields" />
                                        </div>
                                        <div>
                                            <Label class="mb-2">Values to Aggregate</Label>
                                            {#each pivotValueFields as vf, index}
                                                <div class="flex items-center gap-2 mb-2">
                                                    <Select class="flex-1" items={pivotValueOptions} bind:value={vf.field} placeholder="Select value field" />
                                                    <Select class="w-32" items={[{value:'Sum', name:'Sum'}, {value:'Count', name:'Count'}, {value:'Average', name:'Average'}, {value:'Min', name:'Min'}, {value:'Max', name:'Max'}]} bind:value={vf.aggregation} />
                                                    <Button color="red" size="sm" outline class="p-2" on:click={() => pivotValueFields = pivotValueFields.filter((_, i) => i !== index)} disabled={pivotValueFields.length === 1}>
                                                        <Trash2 class="w-4 h-4" />
                                                    </Button>
                                                </div>
                                            {/each}
                                            <Button color="light" size="sm" class="mt-1" on:click={() => pivotValueFields = [...pivotValueFields, { field: '', aggregation: 'Sum' }]}>
                                                <Plus class="w-4 h-4 mr-2" /> Add Value Field
                                            </Button>
                                        </div>
                                    </div>
                                </AccordionItem>
                            {:else if selectedViewType === 'survey'}
                                <AccordionItem open>
                                    <span slot="header" class="flex items-center"><FileText class="w-4 h-4 mr-2" />Survey Configuration</span>
                                    <div class="space-y-4">
                                        <div>
                                            <Label for="surveyGroupByType" class="mb-2">Organize Survey Data By</Label>
                                            <Select id="surveyGroupByType" items={[{value: 'Participants', name: 'Participants (One document per participant)'}, {value: 'Questions', name: 'Questions (One document per question)'}]} bind:value={surveyGroupByType} />
                                        </div>

                                        {#if surveyGroupByType === 'Participants'}
                                            <div>
                                                <Label for="participantUniqueId" class="mb-2">Participant Unique Identifier</Label>
                                                <Select id="participantUniqueId" items={allColumns} bind:value={surveyUniqueIdentifierField} placeholder="Select unique ID field" />
                                            </div>
                                            <div>
                                                <Label class="mb-2">Include Fields in Document</Label>
                                                <MultiSelect items={allColumns} bind:value={surveyParticipantIncludedFields} placeholder="Select fields to include" />
                                                <Helper class="mt-2">Selected fields will be included in each participant's document.</Helper>
                                            </div>
                                        {:else if surveyGroupByType === 'Questions'}
                                            <div>
                                                <Label for="questionUniqueId" class="mb-2">Participant Unique Identifier</Label>
                                                <Select id="questionUniqueId" items={allColumns} bind:value={surveyUniqueIdentifierField} placeholder="Select unique ID field" />
                                            </div>
                                            <div>
                                                <Label class="mb-2">Select Questions to Import</Label>
                                                <MultiSelect items={allColumns} bind:value={surveySelectedQuestions} placeholder="Select questions" />
                                                <Helper class="mt-2">A document will be created for each selected question.</Helper>
                                            </div>
                                            <div>
                                                <Label class="mb-2">Include Other Fields (Optional)</Label>
                                                <MultiSelect items={allColumns} bind:value={surveyIncludedOtherFields} placeholder="Select other fields to include" />
                                            </div>
                                        {/if}
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
                                class="flex items-start gap-4 p-6 border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 rounded-lg hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group relative overflow-hidden text-left"
                                class:border-blue-500={selectedViewType === type.value}
                                class:ring-2={selectedViewType === type.value}
                                class:ring-blue-500={selectedViewType === type.value}
                                on:click={() => selectViewType(type.value)}
                            >
                                <div class="w-16 h-16 bg-blue-50 dark:bg-blue-900/30 rounded-xl flex items-center justify-center text-blue-600 dark:text-blue-400 flex-shrink-0">
                                    {#if type.icon === 'Table2'}<Table2 size={32} strokeWidth={1.5} />{/if}
                                    {#if type.icon === 'LayoutGrid'}<LayoutGrid size={32} strokeWidth={1.5} />{/if}
                                    {#if type.icon === 'FileText'}<FileText size={32} strokeWidth={1.5} />{/if}
                                </div>
                                <div class="flex flex-col pt-1">
                                    <span class="font-bold text-lg text-gray-900 dark:text-white mb-1">{type.name}</span>
                                    <span class="text-sm text-gray-500 dark:text-gray-400 leading-relaxed">{type.description}</span>
                                </div>
                            </button>
                        {/each}
                    </div>
                    <div class="absolute bottom-6 right-6 z-10">
                        <Button color="blue" disabled={!selectedViewType} on:click={initialCreate} class="px-6 shadow-md">
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
                        <div class="flex-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-sm overflow-auto">
                            <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400 border-collapse">
                                <thead class="text-xs text-gray-700 uppercase bg-gray-100 dark:bg-gray-700 dark:text-gray-400 sticky top-0 z-10 shadow-sm">
                                    <!-- Render Multi-level Column Headers -->
                                    {#if generatedPivotResult && generatedPivotResult.colHeaders.length > 0}
                                        {#each generatedPivotResult.colHeaders as headerRow, levelIndex}
                                            <tr>
                                                <!-- Only render row field labels in the bottom-most header row -->
                                                {#if levelIndex === generatedPivotResult.colHeaders.length - 1}
                                                    {#each pivotRowFields as rowField}
                                                        <th scope="col" class="px-6 py-3 whitespace-nowrap font-bold border border-gray-200 dark:border-gray-600 bg-gray-200 dark:bg-gray-600 align-bottom">
                                                            {rowField}
                                                        </th>
                                                    {/each}
                                                {:else if pivotRowFields.length > 0}
                                                    <!-- Spacer for multi-level columns above row fields -->
                                                    <th colspan={pivotRowFields.length} class="border border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-700"></th>
                                                {/if}

                                                {#each headerRow as h}
                                                    <th scope="col" colspan={h.colspan} class="px-6 py-3 whitespace-nowrap text-center border border-gray-200 dark:border-gray-600 {levelIndex === generatedPivotResult.colHeaders.length - 1 ? 'bg-gray-100 dark:bg-gray-700' : 'bg-gray-200 dark:bg-gray-600'}">
                                                        {h.val}
                                                    </th>
                                                {/each}
                                            </tr>
                                        {/each}
                                    {/if}
                                </thead>
                                <tbody>
                                    {#if generatedPivotResult && generatedPivotResult.rows.length > 0}
                                        {#each generatedPivotResult.rows as row, i}
                                            <tr class="bg-white border-b dark:bg-gray-800 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors">
                                                <!-- Render row headers with rowspan -->
                                                {#each row.headers as header}
                                                    {#if header.rowspan > 0}
                                                        <td rowspan={header.rowspan} class="px-6 py-4 whitespace-nowrap font-bold text-gray-900 dark:text-white border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 align-top">
                                                            {header.val}
                                                        </td>
                                                    {/if}
                                                {/each}

                                                <!-- Render data cells -->
                                                {#each Array(generatedPivotResult.colLeavesCount) as _, colIndex}
                                                    <td class="px-6 py-4 whitespace-nowrap text-right border border-gray-200 dark:border-gray-700">
                                                        {row.data[`val_${colIndex}`] !== undefined ? row.data[`val_${colIndex}`] : ''}
                                                    </td>
                                                {/each}
                                            </tr>
                                        {/each}
                                    {:else}
                                        <tr>
                                            <td colspan="100%" class="px-6 py-8 text-center text-gray-500">
                                                Select row, column, and value fields to generate pivot preview.
                                            </td>
                                        </tr>
                                    {/if}
                                </tbody>
                            </table>
                        </div>
                    </div>
                {:else if selectedViewType === 'survey'}
                    <div class="flex-1 w-full h-full p-8 bg-gray-50/50 dark:bg-gray-900/50 border-l border-gray-100 dark:border-gray-800 flex flex-col items-center justify-center">
                        <div class="max-w-md text-center space-y-4">
                            <div class="w-16 h-16 mx-auto bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 rounded-full flex items-center justify-center">
                                <FileText size={32} />
                            </div>
                            <h4 class="text-xl font-bold text-gray-900 dark:text-white">Survey Data Documents</h4>
                            <p class="text-sm text-gray-500 dark:text-gray-400">
                                This view generates individual Lexical JSON documents based on your configuration.
                                Click <strong class="text-gray-700 dark:text-gray-200">Switch to this view</strong> below to generate and save these files.
                                They will be accessible via the <strong class="text-gray-700 dark:text-gray-200">Attachments Panel</strong>.
                            </p>
                        </div>
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
            <Button color="blue" on:click={switchToView}>Switch to this view</Button>
        </div>
        {/if}
    </svelte:fragment>
</Modal>