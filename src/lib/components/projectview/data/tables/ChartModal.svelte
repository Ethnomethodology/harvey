<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { Modal, Button, Tabs, TabItem, Label, Select, Input, Textarea, Toggle, Helper } from 'flowbite-svelte';
    import { PieChart, ChartBar, ChartColumn, LineChart, ScatterChart, SquareChartGantt, Download, Save, Image as ImageIcon, Trash2, X, Plus, FolderOpen } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import notificationStore from '$lib/stores/notificationStore.js';
    import * as echarts from 'echarts';

    const dispatch = createEventDispatcher();

    export let open = false;
    export let tablePath = '';
    export let columns = [];
    export let tableData = [];
    export let schema = {};
    export let initialChart = null;

    // Normalize path to match DB and Attachments Panel
    $: normalizedTablePath = (() => {
        const projectStoreState = get(project);
        if (!tablePath || !projectStoreState || !projectStoreState.baseDirectory) return tablePath;
        let relative = tablePath.startsWith(projectStoreState.baseDirectory) ? tablePath.substring(projectStoreState.baseDirectory.length) : tablePath;
        return relative.replace(/\\/g, '/').replace(/^\//, '');
    })();

    // Chart Options
    const chartTypes = [
        { value: 'bar', name: 'Bar Chart', icon: 'ChartBar' },
        { value: 'column', name: 'Column Chart', icon: 'ChartColumn' },
        { value: 'line', name: 'Line Chart', icon: 'LineChart' },
        { value: 'scatter', name: 'Scatter Plot', icon: 'ScatterChart' },
        { value: 'pie', name: 'Pie Chart', icon: 'PieChart' },
        { value: 'gantt', name: 'Gantt Chart', icon: 'SquareChartGantt' } // Using Gantt approximation for now
    ];

    let activeTab = 'create';
    let selectedChartType = null;
    let existingCharts = [];
    let isEditingExisting = false;

    // Form fields
    let chartName = '';
    let chartDescription = '';
    let xAxisCol = '';
    let yAxisCol = '';
    let categoryCol = ''; // For Pie
    let valueCol = '';    // For Pie
    let startDateCol = ''; // For Gantt
    let endDateCol = '';   // For Gantt
    let taskCol = '';      // For Gantt
    let showLegend = true;

    let chartContainer;
    let chartInstance;

    // Derived dropdown options
    $: numericColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = c.title || fieldName;
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (colSchema && colSchema.type === 'Numeric') return true;
        // Fallback if schema not well defined
        return tableData.some(row => row[c.field] !== null && row[c.field] !== undefined && row[c.field] !== '' && !isNaN(parseFloat(row[c.field])) && isFinite(row[c.field]));
    }).map(c => ({ value: c.field, name: c.title }));

    $: dateColumns = columns.map(c => {
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
    }).map(c => ({ value: c.field, name: c.title }));

    $: categoricalColumns = columns.map(c => {
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
    });

    $: {
        if (open) {
            loadExistingCharts().then(() => {
                if (initialChart) {
                    selectExistingChart(initialChart);
                    initialChart = null; // Reset after loading once
                } else {
                    // Force open to the fresh create screen
                    resetForm();
                    activeTab = 'create';
                    isEditingExisting = false;
                    chartName = `Chart-${existingCharts.length + 1}`;
                }
            });
        }
    }

    // Re-render chart when data or config changes
    $: if (open && chartInstance && selectedChartType) {
        renderChart();
    }

    let saveTimeout;
    $: {
        if (open && activeTab === 'create' && selectedChartType && isEditingExisting) {
            // Reactive dependencies for auto-saving
            const state = [chartName, chartDescription, xAxisCol, yAxisCol, categoryCol, valueCol, startDateCol, endDateCol, taskCol, showLegend];
            clearTimeout(saveTimeout);
            saveTimeout = setTimeout(() => {
                if (chartName && selectedChartType) {
                    saveChart(true);
                }
            }, 500);
        }
    }

    async function loadExistingCharts() {
        try {
            const projectStoreState = get(project);
            if (!projectStoreState.id) return;
            existingCharts = await invoke('load_chart_configs_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath
            });
        } catch (error) {
            console.error('Failed to load existing charts:', error);
        }
    }

    function handleTabChange(tab) {
        activeTab = tab;
        if (tab === 'create') {
            resetForm();
            isEditingExisting = false;
            chartName = `Chart-${existingCharts.length + 1}`;
        }
    }

    function resetForm() {
        chartDescription = '';
        selectedChartType = null;
        xAxisCol = '';
        yAxisCol = '';
        categoryCol = '';
        valueCol = '';
        startDateCol = '';
        endDateCol = '';
        taskCol = '';
        showLegend = true;
        if (chartInstance) {
            chartInstance.clear();
        }
    }

    async function selectChartType(type) {
        selectedChartType = type;
        if (chartInstance) {
            setTimeout(renderChart, 50);
        }
    }

    // Set initial chartName automatically when creation tab opens, but don't save yet
    $: if (open && activeTab === 'create' && !isEditingExisting && !chartName) {
        chartName = `Chart-${existingCharts.length + 1}`;
    }

    async function initialCreate() {
        if (!chartName) chartName = `Chart-${existingCharts.length + 1}`;
        isEditingExisting = true;
        await saveChart(false);
    }

    function selectExistingChart(chart) {
        chartName = chart.chart_name;
        selectedChartType = chart.chart_type;
        try {
            const config = JSON.parse(chart.config_json);
            chartDescription = config.description || '';
            xAxisCol = config.xAxisCol || '';
            yAxisCol = config.yAxisCol || '';
            categoryCol = config.categoryCol || '';
            valueCol = config.valueCol || '';
            startDateCol = config.startDateCol || '';
            endDateCol = config.endDateCol || '';
            taskCol = config.taskCol || '';
            showLegend = config.showLegend !== false;
        } catch (e) {
            console.error('Failed to parse chart config:', e);
        }
        isEditingExisting = true;
        activeTab = 'create'; // Move to create tab but it acts as edit mode
        if (chartInstance) {
            setTimeout(renderChart, 50);
        }
    }

    async function saveChart(isAutoSave = false) {
        if (!chartName) {
            if (!isAutoSave) notificationStore.add('Chart name is required.', 'error');
            return;
        }
        if (!selectedChartType) {
            if (!isAutoSave) notificationStore.add('Chart type must be selected.', 'error');
            return;
        }

        const projectStoreState = get(project);
        if (!projectStoreState.id) return;

        const config = {
            description: chartDescription,
            xAxisCol,
            yAxisCol,
            categoryCol,
            valueCol,
            startDateCol,
            endDateCol,
            taskCol,
            showLegend
        };

        try {
            await invoke('save_chart_config_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath,
                chartName: chartName,
                chartType: selectedChartType,
                configJson: JSON.stringify(config)
            });
            if (!isAutoSave) notificationStore.add('Chart saved successfully.', 'success');
            await loadExistingCharts();
            dispatch('chartSaved');
        } catch (error) {
            console.error('Failed to save chart:', error);
            if (!isAutoSave) notificationStore.add('Failed to save chart.', 'error');
        }
    }

    async function deleteChart(name) {
        const targetName = typeof name === 'string' ? name : chartName;
        if (!targetName) return;

        const { ask } = await import('@tauri-apps/plugin-dialog');
        const confirmed = await ask(`Are you sure you want to delete ${targetName}?`, { title: 'Delete Chart', type: 'warning' });
        if (!confirmed) return;

        const projectStoreState = get(project);
        try {
            await invoke('delete_chart_config_command', {
                projectId: projectStoreState.id,
                tablePath: normalizedTablePath,
                chartName: targetName
            });
            notificationStore.add('Chart deleted.', 'success');
            resetForm();
            isEditingExisting = false;
            await loadExistingCharts();
            dispatch('chartSaved');
        } catch (error) {
            console.error('Failed to delete chart:', error);
            notificationStore.add('Failed to delete chart.', 'error');
        }
    }

    function renderChart() {
        if (!chartContainer || !selectedChartType) return;
        if (!chartInstance) {
            chartInstance = echarts.init(chartContainer);
        }

        let option = {
            title: { text: chartName || 'New Chart', subtext: chartDescription },
            tooltip: { trigger: 'axis' },
            legend: { show: showLegend, bottom: 0 },
            toolbox: { show: false } // Custom export handles it
        };

        try {
            if (selectedChartType === 'bar' || selectedChartType === 'column' || selectedChartType === 'line' || selectedChartType === 'scatter') {
                if (!xAxisCol || !yAxisCol) { chartInstance.clear(); return; }
                const xData = tableData.map(row => row[xAxisCol]);
                const yData = tableData.map(row => parseFloat(row[yAxisCol]) || 0);

                if (selectedChartType === 'bar') {
                    option.xAxis = { type: 'value' };
                    option.yAxis = { type: 'category', data: xData };
                    option.series = [{ type: 'bar', data: yData }];
                } else {
                    option.xAxis = { type: 'category', data: xData };
                    option.yAxis = { type: 'value' };

                    if (selectedChartType === 'scatter') {
                        option.xAxis = { type: 'value' };
                        // Scatter needs [x, y] data pairs
                        const scatterData = tableData.map(row => [parseFloat(row[xAxisCol]) || 0, parseFloat(row[yAxisCol]) || 0]);
                        option.series = [{ type: 'scatter', data: scatterData }];
                    } else if (selectedChartType === 'column') {
                        option.series = [{ type: 'bar', data: yData }];
                    } else {
                        option.series = [{ type: selectedChartType, data: yData }];
                    }
                }
            } else if (selectedChartType === 'pie') {
                if (!categoryCol || !valueCol) { chartInstance.clear(); return; }
                const pieData = tableData.map(row => ({
                    name: String(row[categoryCol]),
                    value: parseFloat(row[valueCol]) || 0
                }));
                option.tooltip.trigger = 'item';
                option.series = [{ type: 'pie', radius: '50%', data: pieData }];
            } else if (selectedChartType === 'gantt') {
                if (!taskCol || !startDateCol || !endDateCol) { chartInstance.clear(); return; }

                // Extremely basic gantt implementation using custom series or stacked bar
                const tasks = tableData.map(row => row[taskCol]);
                const starts = tableData.map(row => new Date(row[startDateCol]).getTime());
                const ends = tableData.map(row => new Date(row[endDateCol]).getTime());
                const durations = ends.map((end, i) => end - starts[i]);

                option.xAxis = { type: 'time' };
                option.yAxis = { type: 'category', data: tasks };
                option.series = [
                    {
                        type: 'bar',
                        stack: 'Total',
                        itemStyle: { borderColor: 'transparent', color: 'transparent' },
                        data: starts
                    },
                    {
                        type: 'bar',
                        stack: 'Total',
                        data: durations
                    }
                ];
                option.tooltip.formatter = function(params) {
                    if (params[0].dataIndex !== undefined) {
                        const start = new Date(starts[params[0].dataIndex]).toLocaleDateString();
                        const end = new Date(ends[params[0].dataIndex]).toLocaleDateString();
                        return `${tasks[params[0].dataIndex]}<br/>Start: ${start}<br/>End: ${end}`;
                    }
                };
            }

            chartInstance.setOption(option, true);
        } catch (e) {
            console.error('Error rendering chart:', e);
        }
    }

    async function exportChart(type) {
        if (!chartInstance) return;
        const base64 = chartInstance.getDataURL({ type: type, backgroundColor: '#fff' });

        // Trigger download
        const a = document.createElement('a');
        a.href = base64;
        a.download = `${chartName || 'chart'}.${type}`;
        a.click();
    }

    async function saveChartToImages() {
        if (!chartInstance) return;
        const base64 = chartInstance.getDataURL({ type: 'png', backgroundColor: '#fff' });
        // base64 includes 'data:image/png;base64,'
        const imageData = base64.split(',')[1];

        const projectStoreState = get(project);
        if (!projectStoreState.xmlPath) return;

        try {
            await invoke('save_screenshot', {
                projectXmlPathStr: projectStoreState.xmlPath,
                projectId: projectStoreState.id,
                mediaFileName: chartName || 'chart',
                timestamp: 0,
                imageDataBase64: imageData
            });
            notificationStore.add('Chart saved to Images tab.', 'success');
        } catch (error) {
            console.error('Failed to save to images:', error);
            notificationStore.add('Failed to save chart to Images.', 'error');
        }
    }

    function handleModalClose() {
        if (!open) return; // Prevent double trigger
        if (activeTab === 'create' && chartName && selectedChartType) {
            saveChart();
        }
    }

    onMount(() => {
        // Wait for modal transition to finish before initializing echarts if visible
    });

    onDestroy(() => {
        if (chartInstance) {
            chartInstance.dispose();
        }
    });

</script>

<Modal
    bind:open={open}
    size="xl"
    on:close={handleModalClose}
    outsideclose
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col h-[70vh] max-h-[800px] relative bg-white dark:bg-gray-900"
>
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <PieChart size={20} class="text-blue-600 dark:text-blue-400" />
        </div>
        <div>
            <h3 class="text-lg font-bold text-gray-900 dark:text-white">Insert Chart</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Create or open visualizations from table data</p>
        </div>
    </div>

    <div class="flex-1 flex overflow-hidden -m-6 h-full border-t border-gray-200 dark:border-gray-700">
        <!-- Left Sidebar: Create / Open Existing -->
        <div class="w-64 border-r border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-800">
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
                    {#if !isEditingExisting}
                        <div class="text-sm text-gray-500 dark:text-gray-400 italic">Select a chart type from the right panel.</div>
                    {:else}
                        <div class="space-y-4">
                            <div>
                                <Label for="chartName" class="mb-2">Chart Name</Label>
                                <Input id="chartName" bind:value={chartName} placeholder="Enter chart name" />
                            </div>
                            <div>
                                <Label for="chartDescription" class="mb-2">Description</Label>
                                <Textarea id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
                            </div>
                            <div class="text-sm font-medium text-gray-700 dark:text-gray-300 border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
                                Data Configuration
                            </div>

                            {#if selectedChartType === 'bar' || selectedChartType === 'column' || selectedChartType === 'line' || selectedChartType === 'scatter'}
                                <div>
                                    <Label for="xAxisCol" class="mb-2">X-Axis Column</Label>
                                    <Select id="xAxisCol" items={categoricalColumns} bind:value={xAxisCol} />
                                </div>
                                <div>
                                    <Label for="yAxisCol" class="mb-2">Y-Axis Column (Numeric)</Label>
                                    <Select id="yAxisCol" items={numericColumns} bind:value={yAxisCol} />
                                </div>
                            {:else if selectedChartType === 'pie'}
                                <div>
                                    <Label for="categoryCol" class="mb-2">Category Column</Label>
                                    <Select id="categoryCol" items={categoricalColumns} bind:value={categoryCol} />
                                </div>
                                <div>
                                    <Label for="valueCol" class="mb-2">Value Column (Numeric)</Label>
                                    <Select id="valueCol" items={numericColumns} bind:value={valueCol} />
                                </div>
                            {:else if selectedChartType === 'gantt'}
                                <div>
                                    <Label for="taskCol" class="mb-2">Task Name Column</Label>
                                    <Select id="taskCol" items={categoricalColumns} bind:value={taskCol} />
                                </div>
                                <div>
                                    <Label for="startDateCol" class="mb-2">Start Date Column</Label>
                                    <Select id="startDateCol" items={dateColumns} bind:value={startDateCol} />
                                </div>
                                <div>
                                    <Label for="endDateCol" class="mb-2">End Date Column</Label>
                                    <Select id="endDateCol" items={dateColumns} bind:value={endDateCol} />
                                </div>
                            {/if}

                            <div class="pt-2">
                                <Toggle bind:checked={showLegend}>Show Legend</Toggle>
                            </div>
                        </div>
                    {/if}
                {:else if activeTab === 'existing'}
                    {#if existingCharts.length === 0}
                        <div class="text-sm text-gray-500 dark:text-gray-400 italic">No existing charts found for this table.</div>
                    {:else}
                        <ul class="space-y-2">
                            {#each existingCharts as chart}
                                <li>
                                    <div class="w-full flex items-center justify-between p-3 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors group">
                                        <button
                                            class="flex-1 text-left flex flex-col items-start pr-2 overflow-hidden"
                                            on:click={() => selectExistingChart(chart)}
                                        >
                                            <div class="font-medium text-gray-900 dark:text-white truncate w-full">{chart.chart_name}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide mt-1">{chart.chart_type}</div>
                                        </button>
                                        <div class="flex flex-col items-end gap-1 flex-shrink-0">
                                            <div class="text-xs text-gray-400 dark:text-gray-500 whitespace-nowrap">{new Date(chart.updated_at).toLocaleDateString()}</div>
                                            <Button size="xs" color="light" class="p-1 border-0 shadow-none opacity-0 group-hover:opacity-100 transition-opacity bg-transparent hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-900/30" on:click={() => deleteChart(chart.chart_name)} title="Delete">
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

        <!-- Right Content Area -->
        <div class="flex-1 flex flex-col overflow-hidden bg-white dark:bg-gray-900 relative">
            {#if activeTab === 'create' && !isEditingExisting}
                <div class="p-8 h-full overflow-y-auto bg-gray-50/30 dark:bg-gray-900/30 flex flex-col">
                    <h4 class="text-xl font-semibold mb-6 text-gray-900 dark:text-white">Select Chart Type</h4>
                    <div class="grid grid-cols-2 lg:grid-cols-3 gap-4">
                        {#each chartTypes as type}
                            <button
                                class="flex flex-col items-center p-6 border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 rounded-lg hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group relative overflow-hidden"
                                class:border-blue-500={selectedChartType === type.value}
                                class:ring-2={selectedChartType === type.value}
                                class:ring-blue-500={selectedChartType === type.value}
                                on:click={() => selectChartType(type.value)}
                            >
                                <div class="absolute inset-0 bg-blue-50/0 group-hover:bg-blue-50/50 dark:group-hover:bg-blue-900/10 transition-colors z-0"></div>
                                <div class="w-14 h-14 bg-blue-50 dark:bg-blue-900/30 rounded-full flex items-center justify-center mb-3 text-blue-600 dark:text-blue-400 group-hover:scale-110 transition-transform shadow-sm z-10">
                                    {#if type.icon === 'ChartBar'}<ChartBar size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'ChartColumn'}<ChartColumn size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'LineChart'}<LineChart size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'ScatterChart'}<ScatterChart size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'PieChart'}<PieChart size={24} strokeWidth={2} />{/if}
                                    {#if type.icon === 'SquareChartGantt'}<SquareChartGantt size={24} strokeWidth={2} />{/if}
                                </div>
                                <span class="font-medium text-sm text-gray-900 dark:text-white z-10">{type.name}</span>
                            </button>
                        {/each}
                    </div>
                    <div class="mt-auto pt-6 flex justify-end">
                        <Button color="blue" disabled={!selectedChartType} on:click={initialCreate} class="px-6">
                            Create Chart
                        </Button>
                    </div>
                </div>
            {:else if activeTab === 'create' && isEditingExisting}
                <!-- Chart Preview & Dashboard -->
                <div class="absolute top-2 right-2 flex gap-2 z-10">
                    {#if chartInstance}
                        <Button size="xs" color="light" on:click={() => exportChart('png')} title="Export PNG">
                            <Download class="w-4 h-4 mr-1" /> PNG
                        </Button>
                        <Button size="xs" color="light" on:click={saveChartToImages} title="Save to Images">
                            <ImageIcon class="w-4 h-4 mr-1" /> Save to Data
                        </Button>
                    {/if}
                </div>
                <div class="flex-1 w-full h-full p-4" bind:this={chartContainer}></div>
            {:else if activeTab === 'existing'}
                 <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400 italic">
                    Select a chart from the list to view or edit.
                 </div>
            {/if}
        </div>
    </div>

</Modal>
