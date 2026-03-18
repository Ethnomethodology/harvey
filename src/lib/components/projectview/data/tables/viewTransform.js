export function applyViewConfigToData(tableData, columns, schema, viewConfig, viewType) {
    let transformedData = [...tableData];
    let transformedColumns = [...columns];
    let transformedSchema = { ...schema };

    if (viewType === 'partial') {
        if (viewConfig.filterField && viewConfig.filterValue) {
            const field = viewConfig.filterField;
            const op = viewConfig.filterOperator || 'contains';
            const val = String(viewConfig.filterValue).toLowerCase();

            transformedData = transformedData.filter(row => {
                const rowVal = row[field];
                if (rowVal === null || rowVal === undefined) return false;
                const strVal = String(rowVal).toLowerCase();

                switch (op) {
                    case '=': return strVal === val;
                    case '!=': return strVal !== val;
                    case 'like':
                    case 'contains': return strVal.includes(val);
                    default: return strVal.includes(val);
                }
            });
        }
        if (viewConfig.selectedColumns && viewConfig.selectedColumns.length > 0) {
            transformedColumns = transformedColumns.filter(col => {
                const field = typeof col.getField === 'function' ? col.getField() : col.field;
                return field === 'harvey_internal_id' || viewConfig.selectedColumns.includes(field);
            });
        }
    } else if (viewType === 'pivot') {
        const { rowField, colField, rowFields, colFields, valueField, aggregation } = viewConfig;

        // Backwards compatibility with old config shape
        let actualRowFields = rowFields || (rowField ? [rowField] : []);
        let actualColFields = colFields || (colField ? [colField] : []);

        if (actualRowFields.length > 0 && valueField) {
            let groupedData = {};
            let allColKeys = new Set();
            let rowKeyToValuesMap = new Map();

            transformedData.forEach(row => {
                const rVals = actualRowFields.map(f => String(row[f] || '(Blank)'));
                const rKey = rVals.join(' | ');

                if (!rowKeyToValuesMap.has(rKey)) {
                    let rowValsObj = {};
                    actualRowFields.forEach((f, i) => rowValsObj[f] = rVals[i]);
                    rowKeyToValuesMap.set(rKey, rowValsObj);
                }

                let cKey = 'Total';
                if (actualColFields.length > 0) {
                    cKey = actualColFields.map(f => String(row[f] || '(Blank)')).join(' | ');
                }

                const vVal = parseFloat(row[valueField]) || 0;

                if (!groupedData[rKey]) groupedData[rKey] = {};
                if (!groupedData[rKey][cKey]) groupedData[rKey][cKey] = [];
                groupedData[rKey][cKey].push(vVal);
                allColKeys.add(cKey);
            });

            let pivotCols = [];
            actualRowFields.forEach(f => {
                pivotCols.push({ field: f, title: f, frozen: true, editor: false });
            });

            let sortedColKeys = Array.from(allColKeys).sort();
            sortedColKeys.forEach(ck => {
                pivotCols.push({ field: ck, title: ck, hozAlign: 'right', editor: false });
                transformedSchema[ck] = { type: 'Numeric', subType: 'Decimal' };
            });

            let pivotData = [];
            for (const [rKey, cData] of Object.entries(groupedData)) {
                let rowData = { ...rowKeyToValuesMap.get(rKey) };
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
                        aggVal = null;
                    }

                    rowData[ck] = aggVal !== null ? (Number.isInteger(aggVal) ? aggVal : parseFloat(aggVal.toFixed(2))) : '';
                });
                pivotData.push(rowData);
            }

            transformedData = pivotData;
            transformedColumns = pivotCols;
        }
    }

    return { transformedData, transformedColumns, transformedSchema };
}
