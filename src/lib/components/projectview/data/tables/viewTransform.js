export function applyViewConfigToData(tableData, columns, schema, viewConfig, viewType) {
  let transformedData = [...tableData];
  let transformedColumns = [...columns];
  let transformedSchema = { ...schema };

  if (viewType === 'partial') {
    if (viewConfig.filterField && viewConfig.filterValue) {
      const field = viewConfig.filterField;
      const op = viewConfig.filterOperator || 'contains';
      const val = String(viewConfig.filterValue).toLowerCase();

      transformedData = transformedData.filter((row) => {
        const rowVal = row[field];
        if (rowVal === null || rowVal === undefined) return false;
        const strVal = String(rowVal).toLowerCase();

        switch (op) {
          case '=':
            return strVal === val;
          case '!=':
            return strVal !== val;
          case 'like':
          case 'contains':
            return strVal.includes(val);
          default:
            return strVal.includes(val);
        }
      });
    }
    if (viewConfig.selectedColumns && viewConfig.selectedColumns.length > 0) {
      // Ensure primary or required fields are always included, even if omitted from old saved configs
      const requiredOrPrimaryFields = Object.keys(schema).filter(
        (k) => schema[k] && (schema[k].primary === true || schema[k].required === true)
      );
      const enforcedColumns = new Set([...viewConfig.selectedColumns, ...requiredOrPrimaryFields]);

      transformedColumns = transformedColumns.filter((col) => {
        const field = typeof col.getField === 'function' ? col.getField() : col.field;
        return field === 'harvey_internal_id' || enforcedColumns.has(field);
      });
    }
  } else if (viewType === 'pivot') {
    const { rowField, colField, rowFields, colFields, valueField, aggregation, valueFields } =
      viewConfig;

    // Backwards compatibility with old config shape
    let actualRowFields = rowFields || (rowField ? [rowField] : []);
    let actualColFields = colFields || (colField ? [colField] : []);

    // Handle migration from single valueField to multiple valueFields
    let actualValueFields = valueFields || [];
    if (actualValueFields.length === 0 && valueField) {
      actualValueFields.push({ field: valueField, aggregation: aggregation || 'Sum' });
    }

    if (actualRowFields.length > 0 && actualValueFields.length > 0) {
      let groupedData = {};
      let allColKeys = new Set();
      let rowKeyToValuesMap = new Map();

      transformedData.forEach((row) => {
        const rVals = actualRowFields.map((f) => String(row[f] || '(Blank)'));
        const rKey = rVals.join(' | ');

        if (!rowKeyToValuesMap.has(rKey)) {
          let rowValsObj = {};
          actualRowFields.forEach((f, i) => (rowValsObj[f] = rVals[i]));
          rowKeyToValuesMap.set(rKey, rowValsObj);
        }

        let baseCKey = 'Total';
        if (actualColFields.length > 0) {
          baseCKey = actualColFields.map((f) => String(row[f] || '(Blank)')).join(' | ');
        }

        if (!groupedData[rKey]) groupedData[rKey] = {};

        actualValueFields.forEach((vf) => {
          // Create a unique column key per value field to avoid collision
          // For charts, we explicitly flatten the multi-dimensional structure so eCharts can read it easily
          const cKey =
            actualColFields.length > 0
              ? `${baseCKey} | ${vf.field} (${vf.aggregation})`
              : `${vf.field} (${vf.aggregation})`;

          const vVal = parseFloat(row[vf.field]) || 0;

          if (!groupedData[rKey][cKey]) groupedData[rKey][cKey] = [];
          groupedData[rKey][cKey].push(vVal);
          allColKeys.add(cKey);
        });
      });

      let pivotCols = [];

      // To make chart selection UX reasonable, we construct a single "Row Fields" string
      // representation since eCharts prefers a flat 1D X-axis category
      const compositeRowField = actualRowFields.join(' | ');
      pivotCols.push({
        field: compositeRowField,
        title: compositeRowField,
        frozen: true,
        editor: false
      });

      let sortedColKeys = Array.from(allColKeys).sort();
      sortedColKeys.forEach((ck) => {
        pivotCols.push({ field: ck, title: ck, hozAlign: 'right', editor: false });
        transformedSchema[ck] = { type: 'Numeric', subType: 'Decimal' };
      });

      let pivotData = [];
      for (const [rKey, cData] of Object.entries(groupedData)) {
        let rowData = { [compositeRowField]: rKey }; // Just the composite string

        // Keep track of aggregation mapping per column key for final processing
        sortedColKeys.forEach((ck) => {
          const vals = cData[ck] || [];

          // Extract aggregation type from column key name (e.g. "Sales (Sum)" -> "Sum")
          const match = ck.match(/\((Sum|Count|Average|Min|Max)\)$/);
          const aggType = match ? match[1] : 'Sum';

          let aggVal = 0;
          if (vals.length > 0) {
            if (aggType === 'Sum') aggVal = vals.reduce((a, b) => a + b, 0);
            else if (aggType === 'Count') aggVal = vals.length;
            else if (aggType === 'Average') aggVal = vals.reduce((a, b) => a + b, 0) / vals.length;
            else if (aggType === 'Min') aggVal = Math.min(...vals);
            else if (aggType === 'Max') aggVal = Math.max(...vals);
          } else {
            aggVal = null;
          }

          rowData[ck] =
            aggVal !== null
              ? Number.isInteger(aggVal)
                ? aggVal
                : parseFloat(aggVal.toFixed(2))
              : '';
        });
        pivotData.push(rowData);
      }

      transformedData = pivotData;
      transformedColumns = pivotCols;
    }
  }

  return { transformedData, transformedColumns, transformedSchema };
}
