<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { Modal, Button, Tabs, TabItem, Label, Select, Input, Textarea, Toggle, Helper } from 'flowbite-svelte';
    import { PieChart, Download, Save, Image as ImageIcon, Trash2, X, Plus, FolderOpen } from 'lucide-svelte';
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
    export let initialChart = null;

    // Chart Options
    const chartTypes = [
        { value: 'bar', name: 'Bar Chart', icon: 'BarChart' },
        { value: 'line', name: 'Line Chart', icon: 'LineChart' },
        { value: 'scatter', name: 'Scatter Plot', icon: 'ScatterChart' },
        { value: 'pie', name: 'Pie Chart', icon: 'PieChart' },
        { value: 'gantt', name: 'Gantt Chart', icon: 'AlignLeft' } // Using Gantt approximation for now
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
    $: numericColumns = columns.filter(c => {
        return tableData.some(row => !isNaN(parseFloat(row[c.field])) && isFinite(row[c.field]));
    }).map(c => ({ value: c.field, name: c.title || c.field }));

    $: dateColumns = columns.filter(c => {
        return tableData.some(row => {
            const val = row[c.field];
            return val && !isNaN(Date.parse(val));
        });
    }).map(c => ({ value: c.field, name: c.title || c.field }));

    $: allColumns = columns.map(c => ({ value: c.field, name: c.title || c.field }));

    $: {
        if (open) {
            loadExistingCharts().then(() => {
                if (initialChart) {
                    selectExistingChart(initialChart);
                    initialChart = null; // Reset after loading once
                }
            });
        }
    }

    // Re-render chart when data or config changes
    $: if (open && chartInstance && selectedChartType) {
        renderChart();
    }

    async function loadExistingCharts() {
        try {
            const projectStoreState = get(project);
            if (!projectStoreState.id) return;
            existingCharts = await invoke('load_chart_configs_command', {
                projectId: projectStoreState.id,
                tablePath: tablePath
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
        }
    }

    function resetForm() {
        chartName = '';
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

    function selectChartType(type) {
        selectedChartType = type;
        if (chartInstance) {
            setTimeout(renderChart, 50);
        }
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

    async function saveChart() {
        if (!chartName) {
            notificationStore.add('Chart name is required.', 'error');
            return;
        }
        if (!selectedChartType) {
            notificationStore.add('Chart type must be selected.', 'error');
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
                tablePath: tablePath,
                chartName: chartName,
                chartType: selectedChartType,
                configJson: JSON.stringify(config)
            });
            notificationStore.add('Chart saved successfully.', 'success');
            await loadExistingCharts();
            dispatch('chartSaved');
        } catch (error) {
            console.error('Failed to save chart:', error);
            notificationStore.add('Failed to save chart.', 'error');
        }
    }

    async function deleteChart() {
        if (!isEditingExisting) return;
        const projectStoreState = get(project);
        try {
            await invoke('delete_chart_config_command', {
                projectId: projectStoreState.id,
                tablePath: tablePath,
                chartName: chartName
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
            if (selectedChartType === 'bar' || selectedChartType === 'line' || selectedChartType === 'scatter') {
                if (!xAxisCol || !yAxisCol) { chartInstance.clear(); return; }
                const xData = tableData.map(row => row[xAxisCol]);
                const yData = tableData.map(row => parseFloat(row[yAxisCol]) || 0);

                option.xAxis = { type: 'category', data: xData };
                option.yAxis = { type: 'value' };

                if (selectedChartType === 'scatter') {
                    option.xAxis = { type: 'value' };
                    // Scatter needs [x, y] data pairs
                    const scatterData = tableData.map(row => [parseFloat(row[xAxisCol]) || 0, parseFloat(row[yAxisCol]) || 0]);
                    option.series = [{ type: 'scatter', data: scatterData }];
                } else {
                    option.series = [{ type: selectedChartType, data: yData }];
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

<Modal bind:open size="xl" on:close={handleModalClose} title="Insert Chart" headerClass="bg-gray-100 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-700" bodyClass="p-0 flex h-[600px] bg-white dark:bg-gray-900" footerClass="bg-gray-100 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-700 justify-between">
    <svelte:fragment slot="header">
        <div class="flex items-center gap-2">
            <PieChart class="w-5 h-5 text-gray-700 dark:text-gray-300" />
            <h3 class="text-lg font-medium text-gray-900 dark:text-white">Insert Chart</h3>
        </div>
    </svelte:fragment>

    <div class="flex w-full h-full">
        <!-- Left Sidebar: Create / Open Existing -->
        <div class="w-64 border-r border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-800">
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

            <div class="flex-1 overflow-y-auto p-4">
                {#if activeTab === 'create'}
                    <div class="space-y-4">
                        <div>
                            <Label for="chartName" class="mb-2">Chart Name</Label>
                            <Input id="chartName" bind:value={chartName} placeholder="Enter chart name" />
                        </div>
                        <div>
                            <Label for="chartDescription" class="mb-2">Description</Label>
                            <Textarea id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
                        </div>

                        {#if !selectedChartType}
                            <div class="text-sm text-gray-500 dark:text-gray-400 italic mt-4">Select a chart type from the right panel.</div>
                        {:else}
                            <div class="text-sm font-medium text-gray-700 dark:text-gray-300 border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
                                Data Configuration
                            </div>

                            {#if selectedChartType === 'bar' || selectedChartType === 'line' || selectedChartType === 'scatter'}
                                <div>
                                    <Label for="xAxisCol" class="mb-2">X-Axis Column</Label>
                                    <Select id="xAxisCol" items={allColumns} bind:value={xAxisCol} />
                                </div>
                                <div>
                                    <Label for="yAxisCol" class="mb-2">Y-Axis Column (Numeric)</Label>
                                    <Select id="yAxisCol" items={numericColumns} bind:value={yAxisCol} />
                                </div>
                            {:else if selectedChartType === 'pie'}
                                <div>
                                    <Label for="categoryCol" class="mb-2">Category Column</Label>
                                    <Select id="categoryCol" items={allColumns} bind:value={categoryCol} />
                                </div>
                                <div>
                                    <Label for="valueCol" class="mb-2">Value Column (Numeric)</Label>
                                    <Select id="valueCol" items={numericColumns} bind:value={valueCol} />
                                </div>
                            {:else if selectedChartType === 'gantt'}
                                <div>
                                    <Label for="taskCol" class="mb-2">Task Name Column</Label>
                                    <Select id="taskCol" items={allColumns} bind:value={taskCol} />
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
                        {/if}
                    </div>
                {:else if activeTab === 'existing'}
                    {#if existingCharts.length === 0}
                        <div class="text-sm text-gray-500 dark:text-gray-400 italic">No existing charts found for this table.</div>
                    {:else}
                        <ul class="space-y-2">
                            {#each existingCharts as chart}
                                <li>
                                    <button
                                        class="w-full text-left p-3 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                                        on:click={() => selectExistingChart(chart)}
                                    >
                                        <div class="flex justify-between items-center">
                                            <div class="font-medium text-gray-900 dark:text-white truncate">{chart.chart_name}</div>
                                            <div class="text-xs text-gray-400 dark:text-gray-500 ml-2 whitespace-nowrap">{new Date(chart.updated_at).toLocaleDateString()}</div>
                                        </div>
                                        <div class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide mt-1">{chart.chart_type}</div>
                                    </button>
                                </li>
                            {/each}
                        </ul>
                    {/if}
                {/if}
            </div>
        </div>

        <!-- Right Content Area -->
        <div class="flex-1 flex flex-col overflow-hidden bg-white dark:bg-gray-900 relative">
            {#if activeTab === 'create' && !selectedChartType}
                <div class="p-8 h-full overflow-y-auto">
                    <h4 class="text-xl font-medium mb-6 text-gray-800 dark:text-gray-200">Select Chart Type</h4>
                    <div class="grid grid-cols-2 lg:grid-cols-3 gap-6">
                        {#each chartTypes as type}
                            <button
                                class="flex flex-col items-center p-6 border-2 border-dashed border-gray-300 dark:border-gray-700 rounded-xl hover:border-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all"
                                on:click={() => selectChartType(type.value)}
                            >
                                <div class="w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4 text-gray-600 dark:text-gray-400">
                                    <!-- Use PieChart as a generic placeholder if others aren't imported -->
                                    <PieChart size={32} />
                                </div>
                                <span class="font-medium text-gray-900 dark:text-white">{type.name}</span>
                            </button>
                        {/each}
                    </div>
                </div>
            {:else if activeTab === 'create' && selectedChartType}
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

    <svelte:fragment slot="footer">
        <div>
            {#if isEditingExisting}
                <Button color="red" on:click={deleteChart} size="sm">
                    <Trash2 class="w-4 h-4 mr-2" /> Delete
                </Button>
            {/if}
        </div>
        <div class="flex gap-2">
            <Button color="alternative" on:click={() => { if(activeTab==='create' && chartName) saveChart(); open = false; }} size="sm">Cancel</Button>
            <Button color="blue" on:click={saveChart} size="sm">
                <Save class="w-4 h-4 mr-2" /> Save
            </Button>
        </div>
    </svelte:fragment>
</Modal>
