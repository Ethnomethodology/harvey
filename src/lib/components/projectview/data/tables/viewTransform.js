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
        const { rowField, colField, valueField, aggregation } = viewConfig;
        if (rowField && valueField) {
            let groupedData = {};
            let allColKeys = new Set();

            transformedData.forEach(row => {
                const rVal = String(row[rowField] || '(Blank)');
                const cVal = colField ? String(row[colField] || '(Blank)') : 'Total';
                const vVal = parseFloat(row[valueField]) || 0;

                if (!groupedData[rVal]) groupedData[rVal] = {};
                if (!groupedData[rVal][cVal]) groupedData[rVal][cVal] = [];
                groupedData[rVal][cVal].push(vVal);
                allColKeys.add(cVal);
            });

            let pivotCols = [
                { field: rowField, title: rowField, frozen: true, editor: false }
            ];

            let sortedColKeys = Array.from(allColKeys).sort();
            sortedColKeys.forEach(ck => {
                pivotCols.push({ field: ck, title: ck, hozAlign: 'right', editor: false });
                transformedSchema[ck] = { type: 'Numeric', subType: 'Decimal' };
            });

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
