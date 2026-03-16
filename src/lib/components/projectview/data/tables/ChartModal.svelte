<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { Modal, Button, Tabs, TabItem, Label, Select, Input, Textarea, Toggle, Helper, Accordion, AccordionItem, Range, Checkbox } from 'flowbite-svelte';
    import { PieChart, ChartBar, ChartColumn, LineChart, ScatterChart, SquareChartGantt, Download, Save, Image as ImageIcon, ImagePlus, Share, Trash2, X, Plus, FolderOpen } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import notificationStore from '$lib/stores/notificationStore.js';
    import * as echarts from 'echarts';
    import ImageExportModal from '$lib/components/projectview/modals/ImageExportModal.svelte';
    import { writeFile } from '@tauri-apps/plugin-fs';

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
    let prevOpen = false;

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

    // Bar/Column Chart Extra Configuration
    let aggregationType = 'Sum'; // Sum, Average, Count, Min, Max
    let breakdownCol = ''; // Group By
    let barType = 'Clustered'; // Clustered, Stacked, 100% Stacked
    let sortOrder = 'None'; // None, Highest, Lowest, A-Z

    // Line Chart Extra Configuration
    let lineType = 'Line'; // Line, Stacked Line, 100% Stacked Line
    let lineStyleOption = 'With Markers'; // With Markers, Area Filled

    // Labels Configuration
    let xAxisLabel = '';
    let yAxisLabel = '';
    let titlePosition = 'Top'; // Top, Bottom
    let yAxisWidthLimit = 150; // 50-300px
    let longTextHandling = 'Truncate'; // Truncate, Wrap
    let showValueLabels = false;
    let valueLabelPosition = 'Inside End'; // Inside End, Outside End

    // Appearance Configuration
    let colorPalette = 'Modern'; // Modern, Soft Pastels, Warm Pastels, Warm Sunset
    let legendPosition = 'Bottom'; // Top, Bottom, Left, Right

    let chartContainer;
    let chartInstance;
    let showImageExportModal = false;

    // Derived dropdown options
    $: numericColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (colSchema && colSchema.type === 'Numeric') return true;
        // Fallback if schema not well defined
        return tableData.some(row => row[c.field] !== null && row[c.field] !== undefined && row[c.field] !== '' && !isNaN(parseFloat(row[c.field])) && isFinite(row[c.field]));
    }).map(c => ({ value: c.field, name: c.title }));

    $: dateColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel
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
        const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel
        return { field: fieldName, title };
    }).filter(c => {
        const colSchema = schema[c.field];
        if (!colSchema) return true; // Fallback if no schema
        // short text, numeric and datetype fields
        if (colSchema.type === 'Text' && colSchema.subType === 'Small Text') return true;
        if (colSchema.type === 'Numeric') return true;
        if (colSchema.type === 'DateTime') return true;
        // Optionally include categorical selectboxes as well
        if (colSchema.type === 'Misc' && (colSchema.subType === 'Selectbox' || colSchema.subType === 'Multiselect')) return true;
        return false;
    }).map(c => ({ value: c.field, name: c.title }));

    // Also keep allColumns for backwards compatibility or fallback if needed
    $: allColumns = columns.map(c => {
        const fieldName = typeof c.getField === 'function' ? c.getField() : c.field;
        const title = fieldName; // Ignore c.title because it is an HTML element in TableViewerPanel
        return { value: fieldName, name: title };
    });

    $: {
        if (open !== prevOpen) {
            prevOpen = open;
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
    }

    // Re-render chart when data or config changes
    $: if (open && chartContainer && selectedChartType) {
        // Trigger render when any of these change
        const _deps = [xAxisCol, yAxisCol, categoryCol, valueCol, startDateCol, endDateCol, taskCol, showLegend, chartName, chartDescription, tableData, aggregationType, breakdownCol, barType, sortOrder, xAxisLabel, yAxisLabel, titlePosition, yAxisWidthLimit, longTextHandling, showValueLabels, valueLabelPosition, colorPalette, legendPosition, lineType, lineStyleOption];
        if (typeof window !== 'undefined') {
            setTimeout(() => {
                if (chartContainer) {
                    renderChart();
                }
            }, 50);
        }
    }

    let saveTimeout;
    $: {
        if (open && activeTab === 'create' && selectedChartType && isEditingExisting) {
            // Reactive dependencies for auto-saving
            const state = [chartName, chartDescription, xAxisCol, yAxisCol, categoryCol, valueCol, startDateCol, endDateCol, taskCol, showLegend, aggregationType, breakdownCol, barType, sortOrder, xAxisLabel, yAxisLabel, titlePosition, yAxisWidthLimit, longTextHandling, showValueLabels, valueLabelPosition, colorPalette, legendPosition, lineType, lineStyleOption];
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
        aggregationType = 'Sum';
        breakdownCol = '';
        barType = 'Clustered';
        sortOrder = 'None';
        lineType = 'Line';
        lineStyleOption = 'With Markers';
        xAxisLabel = '';
        yAxisLabel = '';
        titlePosition = 'Top';
        yAxisWidthLimit = 150;
        longTextHandling = 'Truncate';
        showValueLabels = false;
        valueLabelPosition = 'Inside End';
        colorPalette = 'Modern';
        legendPosition = 'Bottom';
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

            aggregationType = config.aggregationType || 'Sum';
            breakdownCol = config.breakdownCol || '';
            barType = config.barType || 'Clustered';
            sortOrder = config.sortOrder || 'None';
            lineType = config.lineType || 'Line';
            lineStyleOption = config.lineStyleOption || 'With Markers';
            xAxisLabel = config.xAxisLabel || '';
            yAxisLabel = config.yAxisLabel || '';
            titlePosition = config.titlePosition || 'Top';
            yAxisWidthLimit = config.yAxisWidthLimit || 150;
            longTextHandling = config.longTextHandling || 'Truncate';
            showValueLabels = config.showValueLabels || false;
            valueLabelPosition = config.valueLabelPosition || 'Inside End';
            colorPalette = config.colorPalette || 'Modern';
            legendPosition = config.legendPosition || 'Bottom';
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
            showLegend,
            aggregationType,
            breakdownCol,
            barType,
            sortOrder,
            lineType,
            lineStyleOption,
            xAxisLabel,
            yAxisLabel,
            titlePosition,
            yAxisWidthLimit,
            longTextHandling,
            showValueLabels,
            valueLabelPosition,
            colorPalette,
            legendPosition
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
            if (selectedChartType === 'bar' || selectedChartType === 'column' || selectedChartType === 'line') {
                // Determine whether required axis cols are present depending on aggregation.
                // If Count, we technically only need xAxisCol, but UI enforces yAxisCol selection.
                if (!xAxisCol || !yAxisCol) { chartInstance.clear(); return; }
                let validData = tableData.filter(row => row[xAxisCol] !== null && row[xAxisCol] !== undefined && row[xAxisCol] !== '');
                if (aggregationType !== 'Count') {
                    validData = validData.filter(row => row[yAxisCol] !== null && row[yAxisCol] !== undefined && row[yAxisCol] !== '');
                }

                // Map the data into structured groups
                // Grouping by X-Axis Categories
                let groupedData = {};
                // If breakdown is selected, we have a second level of grouping
                // Structure: groupedData = { category_x: { breakdown_1: [y_val, y_val], breakdown_2: [y_val] } }

                validData.forEach(row => {
                    const cat = String(row[xAxisCol]);
                    const bKey = breakdownCol && row[breakdownCol] ? String(row[breakdownCol]) : 'All';
                    let yVal = 0;
                    if (aggregationType !== 'Count') {
                        yVal = parseFloat(row[yAxisCol]) || 0;
                    } else {
                        yVal = 1; // For count, every row is 1
                    }

                    if (!groupedData[cat]) groupedData[cat] = {};
                    if (!groupedData[cat][bKey]) groupedData[cat][bKey] = [];
                    groupedData[cat][bKey].push(yVal);
                });

                // Aggregate
                let aggData = []; // [{ category: 'cat', total: val, series: { 'Breakdown 1': val, ... } }]
                for (const cat in groupedData) {
                    let catObj = { category: cat, total: 0, series: {} };
                    for (const bKey in groupedData[cat]) {
                        const vals = groupedData[cat][bKey];
                        let val = 0;
                        if (aggregationType === 'Sum' || aggregationType === 'Count') {
                            val = vals.reduce((a, b) => a + b, 0);
                        } else if (aggregationType === 'Average') {
                            val = vals.reduce((a, b) => a + b, 0) / vals.length;
                        } else if (aggregationType === 'Max') {
                            val = Math.max(...vals);
                        } else if (aggregationType === 'Min') {
                            val = Math.min(...vals);
                        }
                        catObj.series[bKey] = val;
                        catObj.total += val; // Total for sorting and 100% stack normalization
                    }
                    aggData.push(catObj);
                }

                // Sort
                if (sortOrder === 'Highest') {
                    aggData.sort((a, b) => b.total - a.total);
                } else if (sortOrder === 'Lowest') {
                    aggData.sort((a, b) => a.total - b.total);
                } else if (sortOrder === 'A-Z') {
                    aggData.sort((a, b) => a.category.localeCompare(b.category));
                }

                // Extract all unique breakdown keys (series names)
                const breakdownKeys = new Set();
                aggData.forEach(d => Object.keys(d.series).forEach(k => breakdownKeys.add(k)));
                const seriesNames = Array.from(breakdownKeys);

                const xData = aggData.map(d => d.category);

                // Color Palettes
                const palettes = {
                    'Modern': ['#3b82f6', '#8b5cf6', '#ec4899', '#f43f5e', '#f97316', '#eab308', '#22c55e', '#06b6d4'],
                    'Soft Pastels': ['#fca5a5', '#fcd34d', '#86efac', '#93c5fd', '#c4b5fd', '#f9a8d4'],
                    'Warm Pastels': ['#ffb3ba', '#ffdfba', '#ffffba', '#baffc9', '#bae1ff'],
                    'Warm Sunset': ['#f87171', '#fb923c', '#fbbf24', '#a3e635', '#34d399', '#2dd4bf']
                };
                option.color = palettes[colorPalette] || palettes['Modern'];

                // Series config
                const seriesArray = seriesNames.map(sName => {
                    const dataPoints = aggData.map(d => {
                        let val = d.series[sName] || 0;
                        if ((barType === '100% Stacked' || lineType === '100% Stacked Line') && d.total !== 0) {
                            val = (val / d.total) * 100;
                        }
                        return val;
                    });

                    let valType = barType;
                    if (selectedChartType === 'line') {
                        valType = lineType;
                    }

                    let sConfig = {
                        name: breakdownCol ? sName : (aggregationType === 'Count' ? 'Count' : yAxisCol),
                        type: selectedChartType === 'line' ? 'line' : 'bar',
                        data: dataPoints
                    };

                    if (valType === 'Stacked' || valType === '100% Stacked' || valType === 'Stacked Line' || valType === '100% Stacked Line') {
                        sConfig.stack = 'total';
                    }

                    if (selectedChartType === 'line') {
                        if (lineStyleOption === 'Area Filled') {
                            sConfig.areaStyle = {};
                            sConfig.showSymbol = false; // Hide markers to emphasize area
                        } else {
                            sConfig.showSymbol = true; // Emphasize markers
                        }
                    }

                    if (showValueLabels) {
                        sConfig.label = {
                            show: true,
                            position: selectedChartType === 'line' ? 'top' : (valueLabelPosition === 'Inside End' ? (selectedChartType === 'bar' ? 'insideRight' : 'insideTop') : (selectedChartType === 'bar' ? 'right' : 'top')),
                            color: selectedChartType === 'line' ? '#000' : (valueLabelPosition === 'Inside End' ? '#fff' : '#000'),
                            formatter: (valType === '100% Stacked' || valType === '100% Stacked Line') ? '{c}%' : '{c}'
                        };
                    }

                    return sConfig;
                });

                // Title
                let titleConfig = { text: chartName || 'New Chart' };
                if (chartDescription && chartDescription.trim() !== '') {
                    titleConfig.subtext = chartDescription;
                }
                const hasSubtext = !!titleConfig.subtext;

                if (titlePosition === 'Top') {
                    titleConfig.top = 0;
                    titleConfig.left = 'center';
                } else {
                    titleConfig.bottom = 0;
                    titleConfig.left = 'center';
                }
                option.title = titleConfig;

                // Legend Pos
                let legendConfig = { show: showLegend, type: 'scroll' };
                if (legendPosition === 'Top') {
                    legendConfig.top = titlePosition === 'Top' ? (hasSubtext ? 50 : 30) : 0;
                }
                if (legendPosition === 'Bottom') {
                    legendConfig.bottom = titlePosition === 'Bottom' ? (hasSubtext ? 50 : 30) : 0;
                }
                if (legendPosition === 'Left') {
                    legendConfig.left = 0;
                    legendConfig.orient = 'vertical';
                    legendConfig.top = 'middle';
                    // Set a max width so it doesn't bleed endlessly into chart
                    legendConfig.width = 120;
                }
                if (legendPosition === 'Right') {
                    legendConfig.right = 0;
                    legendConfig.orient = 'vertical';
                    legendConfig.top = 'middle';
                    legendConfig.width = 120;
                }
                option.legend = legendConfig;

                // Axis Grid offsets so it doesn't clip
                // Base grid with default padding
                option.grid = { containLabel: true, left: 30, right: 30, top: 40, bottom: 30 };

                // Adjust Grid for Title
                if (titlePosition === 'Top') {
                    option.grid.top = hasSubtext ? 70 : 50;
                } else {
                    option.grid.bottom = hasSubtext ? 70 : 50;
                }

                // Adjust Grid for Legend
                if (showLegend) {
                    if (legendPosition === 'Left') option.grid.left = 140;
                    if (legendPosition === 'Right') option.grid.right = 140;
                    if (legendPosition === 'Top') option.grid.top = Math.max(option.grid.top, (titlePosition === 'Top' ? (hasSubtext ? 90 : 70) : 40));
                    if (legendPosition === 'Bottom') option.grid.bottom = Math.max(option.grid.bottom, (titlePosition === 'Bottom' ? (hasSubtext ? 90 : 70) : 40));
                }

                // ECharts with `containLabel: true` usually automatically adds space for labels.
                // However, setting `grid.left = 140` hardcodes the left offset, bypassing `containLabel`'s
                // dynamic calculation for the *remaining* axis labels inside the grid.
                // To force the chart to squeeze instead of clip, we compute `left` as a dynamically sized percentage
                // if it exceeds standard bounds, or we add `yAxisWidthLimit` to `grid.left` for horizontal bars.
                if (selectedChartType === 'bar') {
                    option.grid.left = (option.grid.left || 30) + (yAxisWidthLimit || 50) + 20;
                } else if (selectedChartType === 'column') {
                    option.grid.bottom = (option.grid.bottom || 30) + 40; // Space for X-Axis labels
                }


                const isCurrency = schema[yAxisCol] && schema[yAxisCol].subType === 'Currency';
                option.tooltip = {
                    trigger: 'axis',
                    axisPointer: { type: 'shadow' },
                    confine: true, // Prevents tooltip clipping by modal overflow
                    appendToBody: true, // Appends DOM outside relative bounds
                    formatter: (params) => {
                        let html = `<div class="font-bold mb-1">${params[0].axisValue}</div>`;
                        params.forEach(p => {
                            let valStr;
                            if (barType === '100% Stacked' || lineType === '100% Stacked Line') {
                                valStr = `${p.value.toFixed(1)}%`;
                            } else if (aggregationType === 'Count') {
                                valStr = p.value;
                            } else {
                                const rounded = Math.round(p.value);
                                valStr = isCurrency ? `${rounded.toLocaleString()}` : rounded.toLocaleString();
                            }
                            html += `<div>${p.marker} ${p.seriesName}: <b>${valStr}</b></div>`;
                        });
                        return html;
                    }
                };

                // Apply axes
                if (selectedChartType === 'bar') {
                    // Horizontal bar
                    option.xAxis = { type: 'value', name: xAxisLabel, nameLocation: 'middle', nameGap: 30 };
                    if (barType === '100% Stacked') {
                        option.xAxis.max = 100;
                        option.xAxis.axisLabel = { formatter: '{value}%' };
                    }
                    option.yAxis = {
                        type: 'category',
                        data: xData,
                        name: yAxisLabel,
                        nameLocation: 'middle',
                        nameGap: yAxisWidthLimit + 20,
                        axisLabel: {
                            width: yAxisWidthLimit,
                            overflow: longTextHandling === 'Wrap' ? 'break' : 'truncate'
                        }
                    };
                    option.series = seriesArray;
                } else if (selectedChartType === 'column' || selectedChartType === 'line') {
                    // Vertical column or Line
                    option.xAxis = {
                        type: 'category',
                        data: xData,
                        name: xAxisLabel,
                        nameLocation: 'middle',
                        nameGap: 40,
                        axisLabel: {
                            width: selectedChartType === 'line' ? undefined : yAxisWidthLimit,
                            overflow: selectedChartType === 'line' ? undefined : (longTextHandling === 'Wrap' ? 'break' : 'truncate')
                        }
                    };
                    // ECharts line charts typically prefer boundaryGap: false to start the line strictly from the Y-axis edge
                    if (selectedChartType === 'line') {
                        option.xAxis.boundaryGap = false;
                    }

                    option.yAxis = { type: 'value', name: yAxisLabel, nameLocation: 'middle', nameGap: 40 };
                    if ((selectedChartType === 'column' && barType === '100% Stacked') || (selectedChartType === 'line' && lineType === '100% Stacked Line')) {
                        option.yAxis.max = 100;
                        option.yAxis.axisLabel = { formatter: '{value}%' };
                    }
                    option.series = seriesArray;
                }
            } else if (selectedChartType === 'line' || selectedChartType === 'scatter') {
                if (!xAxisCol || !yAxisCol) { chartInstance.clear(); return; }
                const validData = tableData.filter(row => row[xAxisCol] !== null && row[xAxisCol] !== undefined && row[xAxisCol] !== '' && row[yAxisCol] !== null && row[yAxisCol] !== undefined && row[yAxisCol] !== '');
                const xData = validData.map(row => row[xAxisCol]);
                const yData = validData.map(row => parseFloat(row[yAxisCol]) || 0);

                option.xAxis = { type: 'category', data: xData };
                option.yAxis = { type: 'value' };

                if (selectedChartType === 'scatter') {
                    option.xAxis = { type: 'value' };
                    // Scatter needs [x, y] data pairs
                    const scatterData = validData.map(row => [parseFloat(row[xAxisCol]) || 0, parseFloat(row[yAxisCol]) || 0]);
                    option.series = [{ type: 'scatter', name: yAxisCol, data: scatterData }];
                } else {
                    option.series = [{ type: selectedChartType, name: yAxisCol, data: yData }];
                }
            } else if (selectedChartType === 'pie') {
                if (!categoryCol || !valueCol) { chartInstance.clear(); return; }
                const validData = tableData.filter(row => row[categoryCol] !== null && row[categoryCol] !== undefined && row[categoryCol] !== '' && row[valueCol] !== null && row[valueCol] !== undefined && row[valueCol] !== '');
                const pieData = validData.map(row => ({
                    name: String(row[categoryCol]),
                    value: parseFloat(row[valueCol]) || 0
                }));
                option.tooltip.trigger = 'item';
                option.series = [{ type: 'pie', radius: '50%', data: pieData }];
            } else if (selectedChartType === 'gantt') {
                if (!taskCol || !startDateCol || !endDateCol) { chartInstance.clear(); return; }

                const validData = tableData.filter(row =>
                    row[taskCol] !== null && row[taskCol] !== undefined && row[taskCol] !== '' &&
                    row[startDateCol] !== null && row[startDateCol] !== undefined && row[startDateCol] !== '' &&
                    row[endDateCol] !== null && row[endDateCol] !== undefined && row[endDateCol] !== ''
                );

                // Extremely basic gantt implementation using custom series or stacked bar
                const tasks = validData.map(row => row[taskCol]);
                const starts = validData.map(row => new Date(row[startDateCol]).getTime());
                const ends = validData.map(row => new Date(row[endDateCol]).getTime());
                const durations = ends.map((end, i) => end - starts[i]);

                option.xAxis = { type: 'time' };
                option.yAxis = { type: 'category', data: tasks };
                option.series = [
                    {
                        type: 'bar',
                        name: 'Start',
                        stack: 'Total',
                        itemStyle: { borderColor: 'transparent', color: 'transparent' },
                        data: starts
                    },
                    {
                        type: 'bar',
                        name: 'Duration',
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

    function openExportModal() {
        if (!chartInstance) return;
        showImageExportModal = true;
    }

    async function handleExportConfirm(event) {
        const { filePath } = event.detail;
        if (!chartInstance) return;

        const ext = filePath.split('.').pop().toLowerCase();
        const exportType = ext === 'jpg' || ext === 'jpeg' ? 'jpeg' : 'png';

        const base64 = chartInstance.getDataURL({ type: exportType, backgroundColor: '#fff' });
        const base64Data = base64.split(',')[1];

        try {
            const binaryString = atob(base64Data);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            await writeFile(filePath, bytes);
            notificationStore.add('Chart exported successfully.', 'success');
        } catch (error) {
            console.error('Failed to export chart:', error);
            notificationStore.add('Failed to export chart.', 'error');
        }
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
            dispatch('chartSavedToImages');
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
    size="6xl"
    on:close={handleModalClose}
    outsideclose
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col h-[92vh] max-h-[1200px] relative bg-white dark:bg-gray-900"
>
    <div slot="header" class="flex items-center justify-between w-full pr-4">
        <div class="flex items-center space-x-3">
            <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                <PieChart size={20} class="text-blue-600 dark:text-blue-400" />
            </div>
            <div>
                <h3 class="text-lg font-bold text-gray-900 dark:text-white">Insert Chart</h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">Create or open visualizations from table data</p>
            </div>
        </div>
        <div class="flex gap-2">
            <Button size="sm" color="light" on:click={openExportModal} title="Export" disabled={!chartInstance}>
                <Share class="w-4 h-4 mr-2" /> Export
            </Button>
            <Button size="sm" color="light" on:click={saveChartToImages} title="Save to Images" disabled={!chartInstance}>
                <ImagePlus class="w-4 h-4 mr-2" /> Save to Images
            </Button>
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
                    <div class="space-y-4">
                        {#if !(isEditingExisting && (selectedChartType === 'bar' || selectedChartType === 'column' || selectedChartType === 'line'))}
                            <div>
                                <Label for="chartName" class="mb-2">Chart Name</Label>
                                <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="chartName" bind:value={chartName} placeholder="Enter chart name" />
                            </div>
                            <div>
                                <Label for="chartDescription" class="mb-2">Description</Label>
                                <Textarea autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
                            </div>
                        {/if}

                        {#if !isEditingExisting}
                            <div class="text-sm text-gray-500 dark:text-gray-400 italic pt-4 border-t border-gray-200 dark:border-gray-700">
                                Select a chart type from the right panel and click Create to begin configuring data.
                            </div>
                        {:else}
                            <h3 class="text-center font-bold text-lg text-gray-800 dark:text-gray-200 pb-2">
                                {chartTypes.find(t => t.value === selectedChartType)?.name || 'Chart Type'} Configuration
                            </h3>

                            {#if selectedChartType === 'bar' || selectedChartType === 'column' || selectedChartType === 'line'}
                                <Accordion flush>
                                    <AccordionItem open>
                                        <span slot="header">Data Mapping</span>
                                        <div class="space-y-4">
                                            <div>
                                                <Label for="xAxisCol" class="mb-2">Categories ({selectedChartType === 'bar' ? 'Y' : 'X'}-Axis)</Label>
                                                <Select id="xAxisCol" items={categoricalColumns} bind:value={xAxisCol} />
                                            </div>
                                            <div>
                                                <Label for="yAxisCol" class="mb-2">Values ({selectedChartType === 'bar' ? 'X' : 'Y'}-Axis)</Label>
                                                <div class="flex gap-2">
                                                    <Select id="yAxisCol" items={numericColumns} bind:value={yAxisCol} class="flex-1" />
                                                    <Select id="aggregationType" items={[{value:'Sum', name:'Sum'}, {value:'Average', name:'Average'}, {value:'Count', name:'Count'}, {value:'Min', name:'Min'}, {value:'Max', name:'Max'}]} bind:value={aggregationType} class="w-28" />
                                                </div>
                                            </div>
                                            <div>
                                                <Label for="breakdownCol" class="mb-2">Breakdown (Group By)</Label>
                                                <Select id="breakdownCol" items={[{value:'', name:'-- None --'}, ...categoricalColumns]} bind:value={breakdownCol} />
                                            </div>
                                        </div>
                                    </AccordionItem>
                                    <AccordionItem>
                                        <span slot="header">Appearance</span>
                                        <div class="space-y-4">
                                            {#if selectedChartType === 'bar' || selectedChartType === 'column'}
                                                <div>
                                                    <Label for="barType" class="mb-2">Bar Type</Label>
                                                    <Select id="barType" items={[{value:'Clustered', name:'Clustered'}, {value:'Stacked', name:'Stacked'}, {value:'100% Stacked', name:'100% Stacked'}]} bind:value={barType} />
                                                </div>
                                                <div>
                                                    <Label for="sortOrder" class="mb-2">Sorting</Label>
                                                    <Select id="sortOrder" items={[{value:'None', name:'None'}, {value:'Highest', name:'Highest values first (Leaderboard)'}, {value:'Lowest', name:'Lowest values first'}, {value:'A-Z', name:'Alphabetical (A-Z)'}]} bind:value={sortOrder} />
                                                </div>
                                            {:else if selectedChartType === 'line'}
                                                <div>
                                                    <Label for="lineType" class="mb-2">Line Type</Label>
                                                    <Select id="lineType" items={[{value:'Line', name:'Line'}, {value:'Stacked Line', name:'Stacked Line'}, {value:'100% Stacked Line', name:'100% Stacked Line'}]} bind:value={lineType} />
                                                </div>
                                                <div class="flex gap-4">
                                                    <Label class="flex items-center gap-2">
                                                        <input type="radio" value="With Markers" bind:group={lineStyleOption} class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600" />
                                                        With Markers
                                                    </Label>
                                                    <Label class="flex items-center gap-2">
                                                        <input type="radio" value="Area Filled" bind:group={lineStyleOption} class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600" />
                                                        Area Filled
                                                    </Label>
                                                </div>
                                            {/if}
                                            <div>
                                                <Label for="colorPalette" class="mb-2">Color Palette</Label>
                                                <Select id="colorPalette" items={[{value:'Modern', name:'Modern'}, {value:'Soft Pastels', name:'Soft Pastels'}, {value:'Warm Pastels', name:'Warm Pastels'}, {value:'Warm Sunset', name:'Warm Sunset'}]} bind:value={colorPalette} />
                                            </div>
                                            <div class="pt-2">
                                                <Checkbox bind:checked={showLegend}>Show Legend</Checkbox>
                                            </div>
                                            {#if showLegend}
                                                <div>
                                                    <Label for="legendPosition" class="mb-2">Legend Position</Label>
                                                    <Select id="legendPosition" items={[{value:'Top', name:'Top'}, {value:'Bottom', name:'Bottom'}, {value:'Left', name:'Left'}, {value:'Right', name:'Right'}]} bind:value={legendPosition} />
                                                </div>
                                            {/if}
                                        </div>
                                    </AccordionItem>
                                    <AccordionItem>
                                        <span slot="header">Labels</span>
                                        <div class="space-y-4">
                                            <div>
                                                <Label for="titlePosition" class="mb-2">Main Chart Title Position</Label>
                                                <Select id="titlePosition" items={[{value:'Top', name:'Top'}, {value:'Bottom', name:'Bottom'}]} bind:value={titlePosition} />
                                            </div>
                                            <div>
                                                <Label for="chartName" class="mb-2">Chart Name</Label>
                                                <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="chartName" bind:value={chartName} placeholder="Enter chart name" />
                                            </div>
                                            <div>
                                                <Label for="chartDescription" class="mb-2">Description</Label>
                                                <Textarea autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="chartDescription" bind:value={chartDescription} placeholder="Optional description" rows="2" />
                                            </div>
                                            <div>
                                                <Label for="xAxisLabel" class="mb-2">{selectedChartType === 'bar' ? 'Y' : 'X'}-Axis Label (Categories)</Label>
                                                <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="xAxisLabel" bind:value={xAxisLabel} placeholder="Optional title" />
                                            </div>
                                            <div>
                                                <Label for="yAxisLabel" class="mb-2">{selectedChartType === 'bar' ? 'X' : 'Y'}-Axis Label (Values)</Label>
                                                <Input autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" id="yAxisLabel" bind:value={yAxisLabel} placeholder="Optional title" />
                                            </div>
                                            {#if selectedChartType === 'bar' || selectedChartType === 'column'}
                                                <div>
                                                    <Label for="yAxisWidth" class="mb-2">{selectedChartType === 'bar' ? 'Y' : 'X'}-Axis Label Width: {yAxisWidthLimit}px</Label>
                                                    <Range id="yAxisWidth" min="50" max="300" bind:value={yAxisWidthLimit} />
                                                </div>
                                                <div>
                                                    <Label for="longTextHandling" class="mb-2">Long Text Handling</Label>
                                                    <Select id="longTextHandling" items={[{value:'Truncate', name:'Truncate'}, {value:'Wrap', name:'Wrap to next line'}]} bind:value={longTextHandling} />
                                                </div>
                                            {/if}
                                            <div class="pt-2">
                                                <Checkbox bind:checked={showValueLabels}>Show Value Labels</Checkbox>
                                            </div>
                                            {#if showValueLabels}
                                                <div>
                                                    <Label for="valueLabelPosition" class="mb-2">Label Position</Label>
                                                    <Select id="valueLabelPosition" items={[{value:'Inside End', name:'Inside End (White text)'}, {value:'Outside End', name:'Outside End (Dark text)'}]} bind:value={valueLabelPosition} />
                                                </div>
                                            {/if}
                                        </div>
                                    </AccordionItem>
                                </Accordion>

                            {:else if selectedChartType === 'scatter'}
                                <div>
                                    <Label for="xAxisCol" class="mb-2">X-Axis Column</Label>
                                    <Select id="xAxisCol" items={categoricalColumns} bind:value={xAxisCol} />
                                </div>
                                <div>
                                    <Label for="yAxisCol" class="mb-2">Y-Axis Column (Numeric)</Label>
                                    <Select id="yAxisCol" items={numericColumns} bind:value={yAxisCol} />
                                </div>
                                <div class="pt-2">
                                    <Toggle bind:checked={showLegend}>Show Legend</Toggle>
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
                                <div class="pt-2">
                                    <Toggle bind:checked={showLegend}>Show Legend</Toggle>
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
                                <div class="pt-2">
                                    <Toggle bind:checked={showLegend}>Show Legend</Toggle>
                                </div>
                            {/if}
                           {/if}
                    </div>
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
                    <div class="absolute bottom-6 right-6 z-10">
                        <Button color="blue" disabled={!selectedChartType} on:click={initialCreate} class="px-6 shadow-md">
                            Create Chart
                        </Button>
                    </div>
                </div>
            {:else if activeTab === 'create' && isEditingExisting}
                <!-- Chart Preview & Dashboard -->
                <div class="flex-1 w-full h-full p-4" bind:this={chartContainer}></div>
            {:else if activeTab === 'existing'}
                 <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400 italic">
                    Select a chart from the list to view or edit.
                 </div>
            {/if}
        </div>
    </div>

</Modal>

{#if showImageExportModal}
    <ImageExportModal
        bind:showModal={showImageExportModal}
        defaultFileName={chartName || 'chart'}
        exportTypeLabel="Chart"
        showAnnotations={false}
        on:export={handleExportConfirm}
        on:close={() => showImageExportModal = false}
    />
{/if}
