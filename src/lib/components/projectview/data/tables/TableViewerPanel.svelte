<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { get, writable } from 'svelte/store';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import {
        loadTableData,
        saveTableData,
        saveTableLayoutPrefs,
        loadTableLayoutPrefs,
        renameTableHeader,
        deleteTableColumn,
        saveTableStyles,
        loadTableStyles,
        saveTableHighlights,
        loadTableHighlights,
        loadTableSchema,
        saveTableSchema
    } from '$lib/services/projectService.js';
    import { project, setTableHighlights, setLoadedTableHighlights } from '$lib/stores/projectStore.js';
    import { sep } from '@tauri-apps/api/path';
    import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';
    import EditEntryModal from '$lib/components/projectview/modals/EditEntryModal.svelte';
    import EditFieldModal from '$lib/components/projectview/modals/EditFieldModal.svelte';
    import TableHeaderIcon from './TableHeaderIcon.svelte';
    import TableIcon from './TableIcon.svelte';
    import { 
        Pencil, 
        Undo2, 
        Redo2, 
        ChevronLeft, 
        ChevronRight, 
        MoreVertical,
        Plus,
        ExternalLink,
        Check,
        Mail,
        FolderSearch,
        FolderOpen
    } from 'lucide-svelte';
    import { mount, createEventDispatcher } from 'svelte';
    import { openUrl } from '@tauri-apps/plugin-opener';
    import { invoke } from '@tauri-apps/api/core';
    import { 
        Input, 
        Button, 
        Dropdown, 
        DropdownItem, 
        Tooltip,
        Search,
        Badge
    } from 'flowbite-svelte';
    import { Datepicker } from 'flowbite-datepicker';

    export let tablePath = '';
    export let hasHeaders = true;

    const dispatch = createEventDispatcher();

    let tableContainer;
    let tabulatorInstance = null;
    let tableData = [];
    let tableSchema = {};
    let isLoading = true;
    let error = null;
    let currentLoadedPath = null;

    const highlightOptions = HIGHLIGHT_OPTIONS;

    let tableReady = false;
    let tableStyles = { rowStyles: {}, cellStyles: {} }; // This will be derived from highlights

    let showEditEntryModal = false;
    let editingEntryData = null;
    let editingEntryIndex = -1;
    let tableColumnsForModal = [];

    let currentPrimaryField = null;
    let duplicateIds = new Set(); // Stores harvey_internal_id of rows with duplicate primary values
    
    let invalidCells = new Set(); // Stores cell keys "rowIndex-colField"
    let tableHasValidationErrors = false;

    function reformatAllRows() {
        if (tabulatorInstance && tableReady) {
            tabulatorInstance.getRows().forEach(row => row.reformat());
        }
    }

    // Reactive mapping of store highlights to Tabulator styles
    $: if ($project.currentTableHighlights) {
        const hls = $project.currentTableHighlights;
        const newRowStyles = {};
        const newCellStyles = {};
        
        if (Array.isArray(hls)) {
            hls.forEach(h => {
                if (h.id?.startsWith('row-')) {
                    const rowIndex = h.id.substring(4);
                    newRowStyles[rowIndex] = h.color;
                } else if (h.id?.startsWith('cell-')) {
                    // Cell IDs are in format "cell-rowIndex-colField"
                    newCellStyles[h.id] = h.color;
                }
            });
        }
        
        tableStyles = { rowStyles: newRowStyles, cellStyles: newCellStyles };
        
        // Trigger a reformat of entries to apply new styles
        reformatAllRows();
    }

    let searchTerm = '';
    let cellMatches = []; // Changed from searchMatches to store cell components
    let currentMatchIndex = -1;
    let columnFields = [];
    let tableLayoutSnapshot = { columns: {} };
    let tableClipboard = null;
    let searchInputRef = null;

    let showOptionsMenu = false;
    let areFiltersVisible = false; // Start with the assumption that filters are hidden

    let showUrlPopover = false;
    let popoverUrl = '';
    let popoverX = 0;
    let popoverY = 0;
    let isUrlCopied = false;

    async function handleOpenUrl() {
        if (!popoverUrl) return;
        try {
            let targetUrl = popoverUrl.trim();
            if (!targetUrl.toLowerCase().startsWith('http://') && !targetUrl.toLowerCase().startsWith('https://')) {
                targetUrl = 'https://' + targetUrl;
            }
            await openUrl(targetUrl);
            showUrlPopover = false;
        } catch (e) {
            console.error("Failed to open URL:", e);
        }
    }

    function handleCopyUrl() {
        if (!popoverUrl) return;
        navigator.clipboard.writeText(popoverUrl);
        isUrlCopied = true;
        setTimeout(() => {
            showUrlPopover = false;
            isUrlCopied = false;
        }, 2000);
    }

    let showEmailPopover = false;
    let popoverEmail = '';
    let popoverEmailX = 0;
    let popoverEmailY = 0;
    let isEmailCopied = false;

    async function handleOpenEmail() {
        if (!popoverEmail) return;
        try {
            await openUrl('mailto:' + popoverEmail.trim());
            showEmailPopover = false;
        } catch (e) {
            console.error("Failed to open email client:", e);
        }
    }

    function handleCopyEmail() {
        if (!popoverEmail) return;
        navigator.clipboard.writeText(popoverEmail);
        isEmailCopied = true;
        setTimeout(() => {
            showEmailPopover = false;
            isEmailCopied = false;
        }, 2000);
    }

    let showProjectLinkPopover = false;
    let popoverProjectLink = '';
    let popoverProjectLinkCategory = '';
    let popoverProjectLinkX = 0;
    let popoverProjectLinkY = 0;

    let revealButtonLabel = 'Show in Finder';
    import { type as getOsType } from '@tauri-apps/plugin-os';

    onMount(async () => {
        try {
            const currentOs = await getOsType();
            if (currentOs === 'windows') revealButtonLabel = 'Reveal in Explorer';
            else if (currentOs === 'macos') revealButtonLabel = 'Reveal in Finder';
            else revealButtonLabel = 'Open File Location';
        } catch (e) {
            console.error("Error getting OS type:", e);
        }
    });

    async function handleRevealProjectLink() {
        if (!popoverProjectLink) return;
        try {
            const proj = get(project);
            let absolutePath = popoverProjectLink;
            // If the path is relative, prepend the base directory
            if (!absolutePath.startsWith('/') && !absolutePath.startsWith('\\') && !absolutePath.includes(':') && proj?.baseDirectory) {
                absolutePath = `${proj.baseDirectory}/${absolutePath}`;
            }
            await invoke('locate_in_finder', { projectXmlPath: absolutePath });
            showProjectLinkPopover = false;
        } catch (e) {
            console.error("Failed to reveal project link in file manager:", e);
        }
    }

    async function handleOpenProjectLink() {
        if (!popoverProjectLink) return;
        const category = popoverProjectLinkCategory;
        let viewType = 'document';
        let originalDocType = '';

        if (category === 'Audios' || category === 'Videos') {
            viewType = 'media';
            originalDocType = category === 'Audios' ? 'audio' : 'video';
        } else if (category === 'Audio Transcripts') {
            viewType = 'media';
            originalDocType = 'audio_transcript';
        } else if (category === 'Video Transcripts') {
            viewType = 'media';
            originalDocType = 'video_transcript';
        } else if (category === 'Transcripts') {
            viewType = 'transcript';
            originalDocType = 'imported_transcript';
        } else if (category === 'Tables') {
            viewType = 'table';
            originalDocType = 'csv';
        } else if (category === 'Images') {
            viewType = 'image';
            originalDocType = 'image';
        }

        if (originalDocType === 'audio_transcript' || originalDocType === 'video_transcript') {
            // Find parent media to load it in the transcription UI properly
            const proj = get(project);
            let mediaNode = null;

            function findMediaByTranscriptPathRecursive(nodes, transcriptPath) {
                if (!Array.isArray(nodes)) return null;
                for (const node of nodes) {
                    if (node.file_type === 'media' && node.associated_transcripts?.some(t => t.path === transcriptPath || t.relativePath === transcriptPath || proj.baseDirectory + '/' + t.relativePath === transcriptPath)) return node;
                    const found = findMediaByTranscriptPathRecursive(node.children || [], transcriptPath);
                    if (found) return found;
                }
                return null;
            }

            mediaNode = findMediaByTranscriptPathRecursive(proj.files, popoverProjectLink);

            if (mediaNode) {
                dispatch('requestviewchange', {
                    tabName: 'data',
                    loadNotePath: popoverProjectLink,
                    viewType: viewType,
                    originalDocType: originalDocType
                });
                showProjectLinkPopover = false;
                return;
            }
        }

        dispatch('requestviewchange', {
            tabName: 'data',
            loadNotePath: popoverProjectLink,
            viewType: viewType,
            originalDocType: originalDocType || category
        });
        showProjectLinkPopover = false;
    }

    function scrollToHighlight(id) {
        if (!id || !tabulatorInstance) return;
        
        // Clear immediately to prevent infinite loops
        project.update(p => ({ ...p, requestedHighlightId: null }));

        console.log(`[TableViewerPanel] Scrolling to highlight: ${id}`);
        if (id.startsWith('row-')) {
            const rowIndex = parseInt(id.substring(4), 10);
            
            // Small delay to ensure Tabulator has finished internal layout
            setTimeout(() => {
                const row = tabulatorInstance.getRow(rowIndex);
                if (row) {
                    row.scrollTo().then(() => {
                        const el = row.getElement();
                        el.style.transition = 'outline 0.3s ease';
                        el.style.outline = '4px solid #3b82f6';
                        el.style.outlineOffset = '-4px';
                        setTimeout(() => {
                            el.style.outline = 'none';
                        }, 2000);
                    }).catch(err => console.error(`[TableViewerPanel] Scroll failed for entry ${rowIndex}:`, err));
                } else {
                    console.warn(`[TableViewerPanel] Entry ${rowIndex} not found for highlight ${id}`);
                }
            }, 100);
        }
    }

    $: if ($project.requestedHighlightId && tableReady) {
        scrollToHighlight($project.requestedHighlightId);
    }

    async function toggleFilters() {
        if (!tabulatorInstance) return;
        areFiltersVisible = !areFiltersVisible;
        const columns = tabulatorInstance.getColumns();

        await Promise.all(
            columns.map(async (column) => {
                const definition = column.getDefinition();
                if (definition.field) { // Ensure it's a data field
                    if (!areFiltersVisible) {
                        // Clear the filter value before hiding
                        tabulatorInstance.setHeaderFilterValue(definition.field, "");
                    }
                    await tabulatorInstance.updateColumnDefinition(definition.field, {
                        headerFilter: areFiltersVisible ? customHeaderFilterEditor : null
                    });
                }
            })
        );
        showOptionsMenu = false; // Hide menu after action
    }

    const saveCurrentTableLayout = debounce(async () => {
        if (!tabulatorInstance || !currentLoadedPath) return;

        // Use reformat instead of full redraw to avoid focus loss
        reformatAllRows();

        const baseDirForSave = get(project)?.baseDirectory;
        const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
        if (!baseDirForSave || !relativePathForSave) return;
        updateTableLayoutSnapshot();
        await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
    }, 750);

    async function saveCurrentTableLayoutImmediately() {
        if (!tabulatorInstance || !currentLoadedPath) return;
        const baseDirForSave = get(project)?.baseDirectory;
        const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
        if (!baseDirForSave || !relativePathForSave) return;
        updateTableLayoutSnapshot();
        await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
    }

    async function saveTableChanges() {
        if (!tabulatorInstance) return;
        const updatedData = tabulatorInstance.getData();
        
        // DO NOT update tableData = updatedData; here. 
        // Updating reactive tableData causes Tabulator to re-init/re-render, which steals focus.

        const dataToSave = JSON.parse(JSON.stringify(updatedData));
        dataToSave.forEach(row => {
            delete row.harvey_internal_id;
            // Convert Multiselect arrays back to CSV strings for persistence
            for (const field in tableSchema) {
                if (tableSchema[field].type === 'Misc' && tableSchema[field].subType === 'Multiselect') {
                    if (Array.isArray(row[field])) {
                        row[field] = row[field].join(', ');
                    }
                }
            }
        });

        const columns = tabulatorInstance.getColumns();
        const orderedHeaders = columns
            .filter(column => column.getField()) // Ensure we only get data fields
            .map(column => column.getField());

        await saveTableData(tablePath, dataToSave, orderedHeaders);
    }

    const debouncedSave = debounce(saveTableChanges, 750);

    function detectDuplicates() {
        if (!currentPrimaryField || !tabulatorInstance) {
            duplicateIds = new Set();
            return;
        }

        const data = tabulatorInstance.getData();
        const valueMap = new Map(); // value -> [internal_ids]
        
        data.forEach(row => {
            const val = String(row[currentPrimaryField] || "").trim();
            if (val === "") return; // Skip empty
            if (!valueMap.has(val)) {
                valueMap.set(val, []);
            }
            valueMap.get(val).push(row.harvey_internal_id);
        });

        const newDuplicateIds = new Set();
        let foundDuplicates = false;
        valueMap.forEach((ids, val) => {
            if (ids.length > 1) {
                ids.forEach(id => newDuplicateIds.add(id));
                foundDuplicates = true;
            }
        });

        if (foundDuplicates && duplicateIds.size === 0) {
            import('@tauri-apps/plugin-dialog').then(d => {
                d.message(`Duplicate values found in primary field "${currentPrimaryField}". Duplicates are highlighted in red.`, { title: 'Duplicate Primary Key', type: 'warning' });
            });
        }

        duplicateIds = newDuplicateIds;
        reformatAllRows();
    }

    function getUniqueColumnName(baseName) {
        if (!tabulatorInstance) return baseName;
        let newName = baseName;
        let i = 1;
        while (tabulatorInstance.getColumn(newName)) {
            newName = `${baseName}_${i}`;
            i++;
        }
        return newName;
    }

    // Field Actions
    async function copyColumn(column) {
        const field = column.getField();
        const values = tabulatorInstance.getRows().map(row => row.getData()[field]);
        tableClipboard = {
            header: column.getDefinition().title,
            values: values,
            type: 'column'
        };
    }

    async function cutColumn(column) {
        await copyColumn(column);
        await deleteColumn(column);
    }

    async function deleteColumn(column) {
        const columnName = column.getField();
        try {
            await deleteTableColumn(tablePath, columnName);
            await column.delete();
            await saveCurrentTableLayoutImmediately();
        } catch (err) {
            console.error(`Error deleting field "${columnName}":`, err);
            // If the backend fails, we should probably reload to be safe
            await initializeTable(tablePath, null, true);
        }
    }

    function getColumnContextMenu(column) {
        const menu = [
            { label: "Edit Field", action: (e, column) => openFieldEditor(column) },
            { separator: true },
            { label: "Sort Ascending", action: (e, column) => tabulatorInstance.setSort(column.getField(), 'asc') },
            { label: "Sort Descending", action: (e, column) => tabulatorInstance.setSort(column.getField(), 'desc') },
            { separator: true },
            { label: "Cut Field", action: (e, column) => cutColumn(column) },
            { label: "Copy Field", action: (e, column) => copyColumn(column) },
        ];
        if (tableClipboard && tableClipboard.type === 'column') {
            menu.push({ label: "Paste Field Before", action: (e, column) => pasteColumn(column, 'before') });
            menu.push({ label: "Paste Field After", action: (e, column) => pasteColumn(column, 'after') });
        }
        menu.push({ separator: true });
        menu.push({ label: "Insert Field Before", action: (e, column) => insertColumn(column, 'before') });
        menu.push({ label: "Insert Field After", action: (e, column) => insertColumn(column, 'after') });
        menu.push({ separator: true });
        menu.push({ label: "Delete Field", action: (e, column) => deleteColumn(column) });
        return menu;
    }

    let showEditFieldModal = false;
    let editingFieldData = { name: '', schema: {} };
    let isAddingNewField = false;
    let newFieldPosition = 'after';
    let newFieldTargetColumn = null;

    function openFieldEditor(column) {
        const field = column.getField();
        editingFieldData = { 
            name: field, 
            schema: tableSchema[field] || { type: 'Text', subType: 'Small Text' } 
        };
        showEditFieldModal = true;
    }

    async function handleSaveField(event) {
        const { oldName, newName, schema } = event.detail;
        if (!tablePath) return;

        try {
            if (isAddingNewField) {
                // Handle new field addition
                // 1. Update local schema
                tableSchema[newName] = { ...schema };

                // 2. Prepare data with empty values for the new field
                const updatedData = tabulatorInstance.getData();
                updatedData.forEach(row => {
                    row[newName] = "";
                });

                // 3. Determine ordered headers
                const columns = tabulatorInstance.getColumns();
                let orderedHeaders = columns
                    .filter(c => c.getField())
                    .map(c => c.getField());
                
                if (newFieldTargetColumn) {
                    const targetField = newFieldTargetColumn.getField();
                    const index = orderedHeaders.indexOf(targetField);
                    if (index !== -1) {
                        if (newFieldPosition === 'before') {
                            orderedHeaders.splice(index, 0, newName);
                        } else {
                            orderedHeaders.splice(index + 1, 0, newName);
                        }
                    } else {
                        orderedHeaders.push(newName);
                    }
                } else {
                    orderedHeaders.push(newName);
                }

                // 4. Save data and schema
                await saveTableData(tablePath, updatedData, orderedHeaders);
                await saveTableSchema(tablePath, tableSchema);

                // 5. Reload
                await initializeTable(tablePath, null, true);
            } else {
                // Handle existing field rename/update
                if (oldName !== newName) {
                    await renameTableHeader(tablePath, oldName, newName);
                    
                    // Update local schema reference
                    tableSchema[newName] = { ...schema };
                    delete tableSchema[oldName];

                    // Update layout prefs if they exist
                    const projectBaseDir = get(project)?.baseDirectory;
                    if (projectBaseDir) {
                        const relativeTablePath = getRelativePath(tablePath, projectBaseDir);
                        if (relativeTablePath) {
                            let savedLayout = await loadTableLayoutPrefs(relativeTablePath);
                            if (savedLayout?.columns?.[oldName]) {
                                savedLayout.columns[newName] = savedLayout.columns[oldName];
                                delete savedLayout.columns[oldName];
                                await saveTableLayoutPrefs(relativeTablePath, savedLayout);
                            }
                        }
                    }
                } else {
                    // Just update schema
                    tableSchema[oldName] = { ...schema };
                }

                // Save the updated schema
                await saveTableSchema(tablePath, tableSchema);

                // Reload table to reflect structural and schema changes
                await initializeTable(tablePath, null, true);
            }
        } catch (error) {
            console.error("Failed to save field:", error);
        } finally {
            showEditFieldModal = false;
            isAddingNewField = false;
            newFieldTargetColumn = null;
        }
    }

    async function insertColumn(column, position) {
        isAddingNewField = true;
        newFieldPosition = position;
        newFieldTargetColumn = column;
        
        editingFieldData = {
            name: getUniqueColumnName("NewField"),
            schema: { type: 'Text', subType: 'Small Text' }
        };
        showEditFieldModal = true;
    }

    async function pasteColumn(column, position) {
        if (!tableClipboard || tableClipboard.type !== 'column') {
            alert("No field data on clipboard.");
            return;
        }
        const newFieldName = getUniqueColumnName(tableClipboard.header);
        const newColumnDef = {
            title: tableClipboard.header,
            field: newFieldName,
            editor: "textarea",
            headerFilter: areFiltersVisible ? customHeaderFilterEditor : null,
        };
        try {
            await tabulatorInstance.addColumn(newColumnDef, position === 'before', column);
            await tabulatorInstance.updateColumnDefinition(newFieldName, {
                headerContextMenu: getColumnContextMenu
            });
            const rows = tabulatorInstance.getRows();
            rows.forEach((row, index) => {
                if (tableClipboard.values[index] !== undefined) {
                    row.getCell(newFieldName).setValue(tableClipboard.values[index], true);
                }
            });
            await saveTableChanges();
            await saveCurrentTableLayoutImmediately();
        } catch (err) {
            console.error(`Error pasting field ${position} ${column.getField()}:`, err);
        }
    }

    // Entry Actions
    async function copyRow(row) {
        tableClipboard = { type: 'row', data: row.getData() };
    }

    async function cutRow(row) {
        await copyRow(row);
        await deleteRow(row);
    }

    async function deleteRow(row) {
        try {
            await row.delete();
            await saveTableChanges();
        } catch (err) {
            console.error("Error deleting entry:", err);
        }
    }

    async function insertRow(row, position) {
        const newRowData = {};
        tabulatorInstance.getColumns().forEach(column => {
            if (column.getField()) {
                newRowData[column.getField()] = "";
            }
        });

        // Calculate a new unique internal ID
        const allData = tabulatorInstance.getData();
        const maxId = allData.reduce((max, r) => Math.max(max, r.harvey_internal_id || 0), -1);
        newRowData.harvey_internal_id = maxId + 1;

        try {
            const addedRow = await tabulatorInstance.addRow(newRowData, position === 'before', row);

            // Workaround for suspected backend bug: "dirty" a cell to ensure the new entry is saved.
            const cells = addedRow.getCells();
            if (cells.length > 0 && cells[0].getField() !== 'harvey_internal_id') {
                cells[0].setValue(" ", true); // Set a single space, suppress cellEdited event
            }

            await saveTableChanges();
        } catch (err) {
            console.error("Error inserting entry:", err);
        }
    }

    async function pasteRow(row, position) {
        if (!tableClipboard || tableClipboard.type !== 'row') {
            alert("No entry data on clipboard.");
            return;
        }
        
        const newRowData = { ...tableClipboard.data };
        
        // Calculate a new unique internal ID
        const allData = tabulatorInstance.getData();
        const maxId = allData.reduce((max, r) => Math.max(max, r.harvey_internal_id || 0), -1);
        newRowData.harvey_internal_id = maxId + 1;

        try {
            await tabulatorInstance.addRow(newRowData, position === 'before', row);
            await saveTableChanges();
        } catch (err) {
            console.error("Error pasting entry:", err);
        }
    }

    function openEditEntryModal(row) {
        editingEntryData = { ...row.getData() };
        editingEntryIndex = row.getData().harvey_internal_id;
        tableColumnsForModal = tabulatorInstance.getColumnDefinitions().filter(c => c.field && c.field !== 'harvey_internal_id');
        showEditEntryModal = true;
    }

    async function handleSaveEntry(event) {
        const { rowData, rowIndex } = event.detail;
        if (tabulatorInstance) {
            const row = tabulatorInstance.getRow(rowIndex);
            if (row) {
                await row.update(rowData);
                debouncedSave();
            }
        }
        showEditEntryModal = false;
        editingEntryData = null;
        editingEntryIndex = -1;
    }

    function updateTableLayoutSnapshot() {
        if (!tabulatorInstance) return;
        const columns = tabulatorInstance.getColumns(); // This gets fields in their current display order
        const newSnapshotColumns = {};
        columns.forEach((column, index) => {
            const definition = column.getDefinition();
            if (definition.field) {
                newSnapshotColumns[definition.field] = {
                    order: index,
                    visible: column.isVisible(),
                    width: column.getWidth(),
                };
            }
        });
        tableLayoutSnapshot.columns = newSnapshotColumns;
    }

    function getRelativePath(absolutePath, baseDir) {
        if (!absolutePath || !baseDir) return null;
        let relativePath = absolutePath;
        if (absolutePath.startsWith(baseDir)) {
            relativePath = absolutePath.substring(baseDir.length);
            if (relativePath.startsWith(sep) || relativePath.startsWith('/') || relativePath.startsWith('\\')) {
                relativePath = relativePath.substring(1);
            }
        }
        return relativePath.replaceAll('\\', '/');
    }

    function debounce(func, delay) {
        let timeout;
        return function(...args) {
            const context = this;
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(context, args), delay);
        };
    }

    async function applyHighlightToCells(color, cellsToModify) {
        if (!tabulatorInstance || !cellsToModify || cellsToModify.length === 0) return;
        
        let currentHighlights = get(project).currentTableHighlights || [];
        const orderedColumns = tabulatorInstance.getColumns().filter(c => c.getField());

        cellsToModify.forEach(cell => {
            const row = cell.getRow();
            const rowData = row.getData();
            const rowIndex = rowData.harvey_internal_id;
            const colField = cell.getField();
            const cellKey = `cell-${rowIndex}-${colField}`;
            
            // Remove existing highlight for this cell
            currentHighlights = currentHighlights.filter(h => h.id !== cellKey);
            
            if (color) {
                const cellValue = rowData[colField];
                const text = `Cell [Entry ${rowIndex + 1}, ${colField}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ""}`;
                
                currentHighlights.push({
                    id: cellKey,
                    color: color,
                    text: text,
                    tags: [],
                    comments: []
                });
            }
        });
        
        setTableHighlights(currentHighlights);
        await saveTableHighlights();
    }

    async function applyHighlightToRows(color, rows) {
        if (!tabulatorInstance || !rows || rows.length === 0) return;

        let currentHighlights = get(project).currentTableHighlights || [];
        const orderedColumns = tabulatorInstance.getColumns().filter(c => c.getField());

        rows.forEach(row => {
            const rowData = row.getData();
            const rowIndex = rowData.harvey_internal_id;

            // Remove existing highlight for this entry
            currentHighlights = currentHighlights.filter(h => h.id !== `row-${rowIndex}`);

            if (color) {
                // Construct the text in the correct order, starting with the 1-indexed entry number.
                const rowNumber = rowIndex + 1;
                const textParts = [rowNumber.toString()];
                orderedColumns.forEach(column => {
                    const value = rowData[column.getField()];
                    textParts.push(value !== null && value !== undefined ? value : "");
                });
                const text = textParts.join(' | ');

                const newHighlight = {
                    id: `row-${rowIndex}`,
                    color: color,
                    text: text,
                    tags: [],
                    comments: []
                };
                currentHighlights.push(newHighlight);
            }
        });

        setTableHighlights(currentHighlights);
        await saveTableHighlights();

        const ranges = tabulatorInstance.getRanges();
        if (ranges) {
            ranges.forEach(range => range.remove());
        }
    }

    // Custom header filter editor to prevent Enter key propagation
    function customHeaderFilterEditor(cell, onRendered, success, cancel, editorParams){
        var editor = document.createElement("input");
        editor.setAttribute("type", "text");
        editor.setAttribute("placeholder", "Filter...");
        editor.style.width = "100%";
        editor.style.boxSizing = "border-box";
        editor.style.padding = "4px";
        editor.style.border = "1px solid #ccc";
        editor.style.borderRadius = "3px";

        editor.value = cell.getValue();

        onRendered(function(){
            editor.focus();
            editor.style.css = "100%";
        });

        function successFunc(){
            success(editor.value);
        }

        editor.addEventListener("change", successFunc);
        editor.addEventListener("blur", successFunc);

        // Prevent Enter key from propagating
        editor.addEventListener("keydown", function(e){
            if(e.key === "Enter"){
                e.preventDefault();
                e.stopPropagation();
                successFunc(); // Apply filter on Enter
            }
            if(e.key === "Escape"){
                cancel();
            }
        });

        return editor;
    }

    function checkValidationErrors() {
        if (!tabulatorInstance) return;
        
        const rows = tabulatorInstance.getRows();
        let foundError = false;
        const newInvalidCells = new Map(); // Store message instead of just a boolean flag

        rows.forEach(row => {
            const rowIndex = row.getData().harvey_internal_id;
            row.getCells().forEach(cell => {
                const colField = cell.getField();
                const value = cell.getValue();
                const schema = tableSchema[colField];
                if (schema) {
                    const validationResult = performSoftValidationDetailed(value, schema);
                    if (!validationResult.valid) {
                        foundError = true;
                        newInvalidCells.set(`${rowIndex}-${colField}`, validationResult.message);
                    }
                }
            });
        });

        invalidCells = newInvalidCells;
        tableHasValidationErrors = foundError;
        
        // Extract filename from path (handle both / and \ separators)
        const filename = tablePath.split(/[\\/]/).pop() || 'Table';
        
        if (tableHasValidationErrors) {
            project.update(p => ({ ...p, statusMessage: `${filename} contains validation errors.` }));
        } else if (foundError === false && tabulatorInstance) {
            // Restore default message if errors cleared
            project.update(p => ({ ...p, statusMessage: `Ready: ${filename}` }));
        }
        
        reformatAllRows();
    }

    function performSoftValidationDetailed(value, schema) {
        if (!schema) return { valid: true };
        const type = schema.type;
        const subType = schema.subType;

        if (schema.required && (value === null || value === undefined || value === "")) {
            return { valid: false, message: 'This field is required.' };
        } 
        
        if (value !== null && value !== undefined && value !== "") {
            if (type === 'Numeric') {
                const num = parseFloat(value);
                if (isNaN(num) || !isFinite(value)) return { valid: false, message: 'Must be a valid number.' };
                if (schema.min !== null && num < schema.min) return { valid: false, message: `Value cannot be less than ${schema.min}.` };
                if (schema.max !== null && num > schema.max) return { valid: false, message: `Value cannot be greater than ${schema.max}.` };
            } else if (type === 'Contact' && subType === 'Email') {
                if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return { valid: false, message: 'Must be a valid email address.' };
            } else if (type === 'Contact' && subType === 'Hyperlink') {
                if (!/^(https?:\/\/)?([\da-z\.-]+)\.([a-z\.]{2,6})([\/\w \.-]*)*\/?$/i.test(value)) return { valid: false, message: 'Must be a valid URL.' };
            } else if (type === 'Contact' && subType === 'Phone') {
                if (!/^\+?[\d\s-]{7,20}$/.test(value)) return { valid: false, message: 'Must be a valid phone number.' };
            } else if (type === 'DateTime') {
                if (subType === 'Time') {
                    if (schema.format === 'HH:mm') return /^([01]\d|2[0-3]):([0-5]\d)$/.test(value) ? { valid: true } : { valid: false, message: 'Must match format HH:mm.' };
                    if (schema.format === 'HH:mm:ss') return /^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/.test(value) ? { valid: true } : { valid: false, message: 'Must match format HH:mm:ss.' };
                    if (schema.format === 'hh:mm A') return /^(0[1-9]|1[0-2]):([0-5]\d)\s?(AM|PM)$/i.test(value) ? { valid: true } : { valid: false, message: 'Must match format hh:mm A.' };
                    return /^([01]\d|2[0-3]):?([0-5]\d)$/.test(value) ? { valid: true } : { valid: false, message: 'Must be a valid time.' };
                } else if (subType === 'Date') {
                    if (schema.format === 'YYYY-MM-DD') return /^\d{4}-(0[1-9]|1[0-2])-(0[12]|[12]\d|3[01])$/.test(value) ? { valid: true } : { valid: false, message: 'Must match format YYYY-MM-DD.' };
                    if (schema.format === 'DD/MM/YYYY') return /^(0[1-9]|[12]\d|3[01])\/(0[1-9]|1[0-2])\/\d{4}$/.test(value) ? { valid: true } : { valid: false, message: 'Must match format DD/MM/YYYY.' };
                    if (schema.format === 'MM/DD/YYYY') return /^(0[1-9]|1[0-2])\/(0[1-9]|[12]\d|3[01])\/\d{4}$/.test(value) ? { valid: true } : { valid: false, message: 'Must match format MM/DD/YYYY.' };
                    if (schema.format === 'YYYY') return /^\d{4}$/.test(value) ? { valid: true } : { valid: false, message: 'Must be a valid 4-digit year.' };
                    if (schema.format === 'MMMM') return ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"].includes(value.toLowerCase()) ? { valid: true } : { valid: false, message: 'Must be a valid month name.' };
                    if (schema.format === 'MMMM YYYY') {
                        const parts = value.split(' ');
                        const isValid = parts.length === 2 && ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"].includes(parts[0].toLowerCase()) && /^\d{4}$/.test(parts[1]);
                        return isValid ? { valid: true } : { valid: false, message: 'Must match format MMMM YYYY.' };
                    }
                    return !isNaN(Date.parse(value)) ? { valid: true } : { valid: false, message: 'Must be a valid date.' };
                } else {
                    return !isNaN(Date.parse(value)) ? { valid: true } : { valid: false, message: 'Must be a valid date and time.' };
                }
            }
        }
        return { valid: true };
    }

    function performSoftValidation(value, schema) {
        return performSoftValidationDetailed(value, schema).valid;
    }

    // Custom soft validator wrapper for Tabulator
    function softValidator(cell, value, parameters) {
        // Validation check is now handled by checkValidationErrors which redraws and triggers formatting
        setTimeout(checkValidationErrors, 10);
        return true; // Always allow editing
    }

    async function getAllProjectAssets() {
        const currentProject = get(project);
        if (!currentProject?.id) return [];
        const { getProjectAssetsForLink } = await import('$lib/services/projectService.js');
        return await getProjectAssetsForLink(currentProject.id);
    }

    // Custom editors for Date, Time, and DateTime
    function dateEditor(cell, onRendered, success, cancel, editorParams) {
        const editor = document.createElement("input");
        editor.setAttribute("type", "text");
        editor.style.padding = "4px";
        editor.style.width = "100%";
        editor.style.boxSizing = "border-box";
        editor.value = cell.getValue() || "";

        let picker;

        onRendered(function() {
            editor.focus();
            picker = new Datepicker(editor, {
                format: 'yyyy-mm-dd',
                autohide: true,
                orientation: 'auto',
                todayBtn: true,
                clearBtn: true,
                container: 'body'
            });
            picker.show(); // Ensure picker appears immediately

            editor.addEventListener('changeDate', (e) => {
                success(picker.getDate('yyyy-mm-dd'));
            });

            // Handle outside click specifically for Tabulator inline
            const handleOutside = (e) => {
                const path = e.composedPath();
                if (picker && picker.active && !path.includes(editor) && !path.includes(picker.pickerElement)) {
                    picker.hide();
                    cancel();
                }
            };
            document.addEventListener('mousedown', handleOutside, true);

            editor.onremove = () => {
                document.removeEventListener('mousedown', handleOutside, true);
                if (picker) picker.destroy();
            };
        });

        return editor;
    }

    function timeEditor(cell, onRendered, success, cancel, editorParams) {
        const container = document.createElement("div");
        container.style.position = "relative";
        container.style.width = "100%";
        container.style.height = "100%";

        const input = document.createElement("input");
        input.type = "text";
        input.value = cell.getValue() || "00:00";
        input.style.width = "100%";
        input.style.height = "100%";
        input.style.padding = "4px";
        input.readOnly = true;
        container.appendChild(input);

        onRendered(() => {
            const dropdownEl = document.createElement("div");
            dropdownEl.className = "z-[10000] w-24 bg-white dark:bg-gray-800 shadow-xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden";
            document.body.appendChild(dropdownEl);

            const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
            const minutes = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

            const content = document.createElement("div");
            content.className = "flex h-48";
            
            const hCol = document.createElement("div");
            hCol.className = "flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800";
            hours.forEach(h => {
                const btn = document.createElement("button");
                btn.className = `w-full py-1 text-xs hover:bg-blue-100 dark:hover:bg-blue-900/30 ${input.value.startsWith(h) ? 'bg-blue-500 text-white font-bold' : ''}`;
                btn.textContent = h;
                btn.onclick = (e) => {
                    e.stopPropagation();
                    const m = input.value.split(':')[1] || "00";
                    input.value = `${h}:${m}`;
                    updateSelected();
                };
                hCol.appendChild(btn);
            });

            const mCol = document.createElement("div");
            mCol.className = "flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700";
            minutes.forEach(m => {
                const btn = document.createElement("button");
                btn.className = `w-full py-1 text-xs hover:bg-blue-100 dark:hover:bg-blue-900/30 ${input.value.endsWith(m) ? 'bg-blue-500 text-white font-bold' : ''}`;
                btn.textContent = m;
                btn.onclick = (e) => {
                    e.stopPropagation();
                    const h = input.value.split(':')[0] || "00";
                    input.value = `${h}:${m}`;
                    success(input.value);
                    cleanup();
                };
                mCol.appendChild(btn);
            });

            function updateSelected() {
                const [h, m] = input.value.split(':');
                Array.from(hCol.children).forEach(b => b.classList.toggle('bg-blue-500', b.textContent === h));
                Array.from(mCol.children).forEach(b => b.classList.toggle('bg-blue-500', b.textContent === m));
            }

            content.appendChild(hCol);
            content.appendChild(mCol);
            dropdownEl.appendChild(content);

            const rect = input.getBoundingClientRect();
            dropdownEl.style.position = "fixed";
            dropdownEl.style.top = `${rect.bottom}px`;
            dropdownEl.style.left = `${rect.left}px`;

            function cleanup() {
                document.removeEventListener('mousedown', handleOutside, true);
                if (dropdownEl.parentNode) dropdownEl.parentNode.removeChild(dropdownEl);
            }

            function handleOutside(e) {
                if (!dropdownEl.contains(e.target) && e.target !== input) {
                    cancel();
                    cleanup();
                }
            }

            document.addEventListener('mousedown', handleOutside, true);
        });

        return container;
    }

    function datetimeEditor(cell, onRendered, success, cancel, editorParams) {
        const container = document.createElement("div");
        container.className = "flex items-center gap-1 w-full h-full p-1";
        
        const val = cell.getValue() || "";
        let [datePart, timePart] = val.includes('T') ? val.split('T') : [val, "00:00"];
        if (!datePart) datePart = "2026-03-07";

        const dateInput = document.createElement("input");
        dateInput.type = "text";
        dateInput.value = datePart;
        dateInput.className = "flex-1 min-w-0 h-full border-none p-0 text-xs";
        
        const timeInput = document.createElement("input");
        timeInput.type = "text";
        timeInput.value = timePart;
        timeInput.className = "w-12 h-full border-none p-0 text-xs";
        timeInput.readOnly = true;

        container.appendChild(dateInput);
        container.appendChild(timeInput);

        let datePicker;

        onRendered(() => {
            dateInput.focus(); // Focus date input first
            datePicker = new Datepicker(dateInput, {
                format: 'yyyy-mm-dd',
                autohide: true,
                container: 'body'
            });
            datePicker.show(); // Show picker immediately

            const finish = () => {
                success(`${dateInput.value}T${timeInput.value}`);
                cleanup();
            };

            dateInput.addEventListener('changeDate', () => {
                // Don't finish yet, let them pick time
            });

            timeInput.onclick = (e) => {
                e.stopPropagation();
                // Simple implementation for now
            };

            const handleOutside = (e) => {
                const path = e.composedPath();
                if (!path.includes(container) && (!datePicker || !path.includes(datePicker.pickerElement))) {
                    finish();
                }
            };

            document.addEventListener('mousedown', handleOutside, true);

            function cleanup() {
                document.removeEventListener('mousedown', handleOutside, true);
                if (datePicker) datePicker.destroy();
            }
        });

        return container;
    }

    async function generateColumns(data, headers, savedLayoutObj, schema) {
        if (!headers || headers.length === 0) return [{title: "No Data", field: "placeholder"}];
        
        currentPrimaryField = Object.keys(schema).find(key => schema[key].primary) || null;
        const projectAssetOptions = await getAllProjectAssets();
        
        let dataColumnDefs = headers.map(header => {
            const colSchema = schema[header] || { type: 'Text', subType: 'Small Text' };
            const isPrimary = colSchema.primary === true;

            const colDef = {
                title: (() => {
                    const container = document.createElement("div");
                    mount(TableHeaderIcon, {
                        target: container,
                        props: { colSchema, header }
                    });
                    return container;
                })(),
                field: header,
                headerFilter: areFiltersVisible ? customHeaderFilterEditor : null,
                headerFilterPlaceholder: "Filter...",
                headerFilterFunc: function(headerValue, rowValue, rowData, filterParams){
                    if (headerValue === null || headerValue === undefined || String(headerValue).trim() === "") return true;
                    if (rowValue === null || rowValue === undefined) return false;
                    return String(rowValue).toLowerCase().includes(String(headerValue).toLowerCase());
                },
                sorter: colSchema.type === 'Numeric' ? 'number' : (colSchema.type === 'DateTime' ? 'datetime' : 'string'),
                validator: softValidator,
                headerContextMenu: getColumnContextMenu,
                headerTooltip: colSchema.description || null,
                frozen: isPrimary,
            };

            // Set editor based on schema
            if (colSchema.type === 'Misc') {
                if (colSchema.subType === 'Checkbox') {
                    colDef.editor = false; // Disable editor to prevent tickCross "cross" icons
                    colDef.formatter = (cell) => {
                        const val = cell.getValue();
                        const isChecked = val === true || val === 'true' || val === 1 || val === "1";
                        return `<div class="flex items-center justify-center h-full">
                            <input type="checkbox" ${isChecked ? 'checked' : ''} class="h-4 w-4 text-blue-600 border-gray-300 rounded cursor-pointer" onclick="event.preventDefault()" />
                        </div>`;
                    };
                    colDef.cellClick = function(e, cell) {
                        // Immediate toggle on single click
                        const currentVal = cell.getValue();
                        const isCurrentlyChecked = currentVal === true || currentVal === 'true' || currentVal === 1 || currentVal === "1";
                        cell.setValue(!isCurrentlyChecked);
                    };
                    colDef.hozAlign = "center";
                    colDef.headerHozAlign = "center";
                    colDef.width = 50;
                    colDef.resizable = false;
                } else if (colSchema.subType === 'Selectbox' || colSchema.subType === 'Multiselect') {
                    colDef.editor = "list";
                    colDef.editorParams = {
                        values: colSchema.options || [],
                        multiselect: colSchema.subType === 'Multiselect'
                    };
                    colDef.formatter = (cell) => {
                        const val = cell.getValue();
                        if (colSchema.subType === 'Multiselect' && Array.isArray(val)) {
                            return `<div class="flex flex-wrap gap-1">
                                ${val.map(v => `<span class="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-[10px] font-medium rounded-full border border-blue-200 dark:border-blue-800/50 whitespace-nowrap">${v}</span>`).join('')}
                            </div>`;
                        }
                        return val || "";
                    };
                } else if (colSchema.subType === 'Project Link') {
                    colDef.editor = "list";
                    colDef.editorParams = {
                        values: projectAssetOptions
                    };
                }
            } else if (colSchema.type === 'Numeric') {
                colDef.editor = "number";
                if (colSchema.subType === 'Currency') {
                    colDef.formatter = (cell) => {
                        const val = cell.getValue();
                        if (val === null || val === undefined || val === "") return "";
                        const currencyCode = (colSchema.currency || 'USD').toUpperCase();
                        try {
                            // Try native formatting first (supports standard ISO codes)
                            return new Intl.NumberFormat('en-US', { 
                                style: 'currency', 
                                currency: currencyCode
                            }).format(val);
                        } catch (e) {
                            // Fallback for custom/unsupported codes (e.g. "BTC", "XXX")
                            const formattedNum = new Intl.NumberFormat('en-US', { 
                                style: 'decimal',
                                minimumFractionDigits: 2,
                                maximumFractionDigits: 2
                            }).format(val);
                            return `${currencyCode} ${formattedNum}`;
                        }
                    };
                } else if (colSchema.subType === 'Percent') {
                    colDef.formatter = (cell) => {
                        const val = cell.getValue();
                        return (val !== null && val !== undefined && val !== "") ? val + '%' : '';
                    };
                }
            } else if (colSchema.type === 'DateTime') {
                if (colSchema.subType === 'Time') {
                    colDef.editor = timeEditor;
                } else if (colSchema.subType === 'Date') {
                    colDef.editor = dateEditor;
                } else {
                    colDef.editor = datetimeEditor;
                }
            } else if (colSchema.type === 'Contact' && colSchema.subType === 'Hyperlink') {
                colDef.editor = "input";
            } else if (colSchema.type === 'Text') {
                if (colSchema.subType === 'Small Text') {
                    colDef.editor = "input";
                } else {
                    colDef.editor = "textarea";
                    colDef.editorParams = { verticalNavigation:"editor", shiftEnterSubmit:true };
                }
            } else {
                colDef.editor = "textarea";
                colDef.editorParams = { verticalNavigation:"editor", shiftEnterSubmit:true };
            }

            // Apply custom styling/highlighting formatter logic
            const baseFormatter = colDef.formatter;
            colDef.formatter = (cell, formatterParams, onRendered) => {
                const rowData = cell.getRow().getData();
                const rowIndex = rowData.harvey_internal_id;
                const colField = cell.getField();
                const cellKey = `cell-${rowIndex}-${colField}`;
                const cellElement = cell.getElement();
                const cellColor = tableStyles.cellStyles[cellKey];
                
                cellElement.style.backgroundColor = cellColor || "";
                if (cellColor) {
                    cellElement.classList.add('highlighted-cell');
                } else {
                    cellElement.classList.remove('highlighted-cell');
                }

                // Validation border
                if (invalidCells.has(`${rowIndex}-${colField}`)) {
                    cellElement.classList.add('invalid-cell');
                } else {
                    cellElement.classList.remove('invalid-cell');
                }

                // Primary duplicate highlighting
                if (colField === currentPrimaryField && duplicateIds.has(rowIndex)) {
                    cellElement.classList.add('duplicate-primary-cell');
                } else {
                    cellElement.classList.remove('duplicate-primary-cell');
                }
                
                if (colSchema.type === 'Text' || colSchema.type === 'Misc' || !colSchema.type) {
                    cellElement.style.whiteSpace = "pre-wrap";
                }

                // Call base formatter if it exists
                let value = cell.getValue();
                if (typeof baseFormatter === 'function') {
                    value = baseFormatter(cell, formatterParams, onRendered);
                } else if (typeof baseFormatter === 'string') {
                    if (baseFormatter === 'tickCross') {
                        const icon = value === true || value === 'true' || value === 1 ? '✔' : '✖';
                        value = `<div style="text-align:center">${icon}</div>`;
                    } else if (baseFormatter === 'money') {
                        if (value !== null && value !== undefined && value !== "") {
                            value = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(value);
                        }
                    }
                }

                // Append validation error message to cell attribute
                if (invalidCells.has(`${rowIndex}-${colField}`)) {
                    const errorMsg = invalidCells.get(`${rowIndex}-${colField}`);
                    if (errorMsg) cellElement.setAttribute('title', errorMsg);
                } else {
                    cellElement.removeAttribute('title');
                }

                const term = searchTerm.trim();
                let outputValue = value;
                if (term && value !== null && value !== undefined && typeof value === 'string') {
                    const escapedTerm = term.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
                    const regex = new RegExp(`(${escapedTerm})`, 'gi');
                    outputValue = String(value).replace(regex, '<span class="search-match-highlight">$1</span>');
                }

                if (colSchema.type === 'Contact' && colSchema.subType === 'Hyperlink' && value) {
                    cellElement.classList.add('interactive-contact-cell');
                    // Add the value directly to the element dataset for the global click handler to access
                    cellElement.dataset.urlValue = value;
                    return `
                        <div class="flex items-center justify-between w-full h-full">
                            <span class="truncate mr-2">${outputValue}</span>
                            <div class="hyperlink-icon-container hidden cursor-pointer text-blue-500 hover:text-blue-600 shrink-0" title="Link Options">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-external-link"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>
                            </div>
                        </div>
                    `;
                }

                if (colSchema.type === 'Contact' && colSchema.subType === 'Email' && value) {
                    cellElement.classList.add('interactive-contact-cell');
                    cellElement.dataset.emailValue = value;
                    return `
                        <div class="flex items-center justify-between w-full h-full">
                            <span class="truncate mr-2">${outputValue}</span>
                            <div class="email-icon-container hidden cursor-pointer text-blue-500 hover:text-blue-600 shrink-0" title="Email Options">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-mail"><rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/></svg>
                            </div>
                        </div>
                    `;
                }

                if (colSchema.type === 'Misc' && colSchema.subType === 'Project Link' && value) {
                    cellElement.classList.add('interactive-contact-cell');
                    cellElement.dataset.projectLinkValue = value;
                    return `
                        <div class="flex items-center justify-between w-full h-full">
                            <span class="truncate mr-2">${outputValue}</span>
                            <div class="project-link-icon-container hidden cursor-pointer text-blue-500 hover:text-blue-600 shrink-0" title="Link Options">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-folder-open"><path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.5 5.96A2 2 0 0 1 18.5 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v5.22"/></svg>
                            </div>
                        </div>
                    `;
                }

                return outputValue;
            };

            if (savedLayoutObj?.columns?.[header]) {
                const savedCol = savedLayoutObj.columns[header];
                if (typeof savedCol.width === 'number' && savedCol.width > 0) colDef.width = savedCol.width;
                colDef.visible = savedCol.visible;
            }
            return colDef;
        });
        
        // Ensure primary field is first in data columns if frozen
        if (currentPrimaryField) {
            const primaryIdx = dataColumnDefs.findIndex(c => c.field === currentPrimaryField);
            if (primaryIdx > 0) {
                const [primaryCol] = dataColumnDefs.splice(primaryIdx, 1);
                dataColumnDefs.unshift(primaryCol);
            }
        }

        if (savedLayoutObj?.columns) {
            dataColumnDefs.sort((a, b) => {
                if (a.frozen) return -1;
                if (b.frozen) return 1;
                return (savedLayoutObj.columns[a.field]?.order ?? Infinity) - (savedLayoutObj.columns[b.field]?.order ?? Infinity)
            });
        }

        // Add the "Add Field" column at the end
        dataColumnDefs.push({
            title: (() => {
                const button = document.createElement("button");
                button.className = "flex items-center justify-center w-full h-full text-blue-500 hover:text-blue-600 transition-colors";
                button.title = "Add New Field";
                mount(TableIcon, {
                    target: button,
                    props: { icon: Plus, size: 16 }
                });
                return button;
            })(),
            headerClick: (e, column) => {
                insertColumn(column, 'after'); // Generic button at end
            },
            width: 40,
            minWidth: 40,
            headerSort: false,
            resizable: false,
            frozen: false,
            cssClass: "add-column-header",
            formatter: (cell) => {
                // Important: returning empty/standard to avoid custom row highlights applying here
                return "";
            }
        });

        return dataColumnDefs;
    }

    async function initializeTable(pathForTable, newHasHeaders = null, force = false) {
        if (newHasHeaders !== null) hasHeaders = newHasHeaders;
        if (!pathForTable || !tableContainer) return;
        if (isLoading && currentLoadedPath === pathForTable) return;
        if (!force && !isLoading && tabulatorInstance && currentLoadedPath === pathForTable) return;

        currentLoadedPath = pathForTable;
        isLoading = true;
        error = null;
        tableData = [];

        if (tabulatorInstance) {
            if (addRowButtonEl) {
                addRowButtonEl.remove();
                addRowButtonEl = null;
            }
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }

        try {
            // 1. Load Table Data
            const response = await loadTableData(pathForTable, hasHeaders);
            tableData = response.data;
            tableData.forEach((d, i) => d.harvey_internal_id = i);
            const tableHeaders = response.headers;

            // 2. Load Schema
            tableSchema = await loadTableSchema(pathForTable) || {};

            // 3. Transform Multiselect to arrays for UI consistency
            tableData.forEach((row) => {
                for (const field in tableSchema) {
                    if (tableSchema[field].type === 'Misc' && tableSchema[field].subType === 'Multiselect') {
                        if (typeof row[field] === 'string') {
                            row[field] = row[field].split(',').map(s => s.trim()).filter(Boolean);
                        } else if (!row[field]) {
                            row[field] = [];
                        }
                    }
                }
            });

            // 4. Load Highlights/Styles
            const loadedHighlightsOrStyles = await loadTableStyles(pathForTable);

            let highlightsForStore = [];

            if (loadedHighlightsOrStyles) {
                if (Array.isArray(loadedHighlightsOrStyles)) {
                    // New format: Array of highlights
                    highlightsForStore = loadedHighlightsOrStyles;
                } else if (typeof loadedHighlightsOrStyles === 'object' && loadedHighlightsOrStyles.rowStyles) {
                    // Old format: Object with rowStyles and cellStyles. Convert to new format.
                    console.log('[TableViewerPanel] Converting old style format to new highlights format');
                    
                    // Convert rowStyles
                    if (loadedHighlightsOrStyles.rowStyles) {
                        for (const [rowIndexStr, color] of Object.entries(loadedHighlightsOrStyles.rowStyles)) {
                            const rowIndex = parseInt(rowIndexStr, 10);
                            const rowData = tableData[rowIndex];
                            if (rowData) {
                                const rowNumber = rowIndex + 1;
                                const textParts = [rowNumber.toString()];
                                // We don't have ordered fields yet, so we just use all values
                                Object.keys(rowData).forEach(key => {
                                    if (key !== 'harvey_internal_id') {
                                        const value = rowData[key];
                                        textParts.push(value !== null && value !== undefined ? value : "");
                                    }
                                });
                                const text = textParts.join(' | ');
                                highlightsForStore.push({
                                    id: `row-${rowIndex}`,
                                    color: color,
                                    text: text,
                                    tags: [],
                                    comments: []
                                });
                            }
                        }
                    }
                    
                    // Convert cellStyles
                    if (loadedHighlightsOrStyles.cellStyles) {
                        for (const [cellKey, color] of Object.entries(loadedHighlightsOrStyles.cellStyles)) {
                            // cellKey format: "cell-rowIndex-colField"
                            const parts = cellKey.split('-');
                            if (parts.length >= 3) {
                                const rowIndex = parseInt(parts[1], 10);
                                const colField = parts.slice(2).join('-');
                                const rowData = tableData[rowIndex];
                                if (rowData) {
                                    const cellValue = rowData[colField];
                                    const text = `Cell [Entry ${rowIndex + 1}, ${colField}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ""}`;
                                    highlightsForStore.push({
                                        id: cellKey,
                                        color: color,
                                        text: text,
                                        tags: [],
                                        comments: []
                                    });
                                }
                            }
                        }
                    }
                }
            }

            setLoadedTableHighlights(highlightsForStore);

            const filename = pathForTable.split(/[\\/]/).pop() || 'Table';
            project.update(p => ({ ...p, statusMessage: `Ready: ${filename}` }));

            await tick();
            if (!tableContainer) {
                 error = 'Failed to initialize table viewer: container lost.';
                 isLoading = false;
                 return;
            }
            const projectBaseDir = get(project)?.baseDirectory;
            if (!projectBaseDir) {
                error = "Project configuration error: base directory missing.";
                isLoading = false;
                return;
            }
            const relativeTablePath = getRelativePath(pathForTable, projectBaseDir);
            if (!relativeTablePath) {
                error = "Error determining asset relative path.";
                isLoading = false;
                return;
            }
            let savedLayout = await loadTableLayoutPrefs(relativeTablePath).catch(e => console.error(`Error loading layout for ${relativeTablePath}:`, e));
            
            // Reset duplicates state for new table
            duplicateIds = new Set();
            const generatedColumns = await generateColumns(tableData, tableHeaders, savedLayout, tableSchema);

            tabulatorInstance = new Tabulator(tableContainer, {
                data: tableData,
                index: "harvey_internal_id",
                layout: "fitData",
                columns: generatedColumns,
                height: "100%",
                placeholder: "No Data Available",
                selectableRange: 1,
                selectableRangeColumns: true,
                selectableRangeRows: true,
                history:true,
                editTriggerEvent:"dblclick",
                movableColumns: true,
                resizableColumnFit: false,
                rowFormatter: (row) => {
                    const rowIndex = row.getData().harvey_internal_id;
                    const rowColor = tableStyles.rowStyles[rowIndex];
                    const rowElement = row.getElement();
                    rowElement.style.backgroundColor = rowColor || "";
                    if (rowColor) {
                        rowElement.classList.add('highlighted-row');
                    } else {
                        rowElement.classList.remove('highlighted-row');
                    }
                },
                rowContextMenu: (e, row) => {
                    const ranges = tabulatorInstance.getRanges();
                    let selectedRowsForMenu = [row];

                    if (ranges.length > 0) {
                        const activeRange = ranges[0];
                        const rangeRows = activeRange.getRows();
                        if (rangeRows.some(r => r.getIndex() === row.getIndex())) {
                            selectedRowsForMenu = rangeRows;
                        }
                    }

                    const highlightAction = (color) => {
                        applyHighlightToRows(color, selectedRowsForMenu);
                    };

                    const highlightColorOptions = highlightOptions.map(option => ({
                        label: `<span style='display:inline-block; width:15px; height:15px; background-color:${option.value}; margin-right: 8px; vertical-align: middle;'></span>${option.label}`,
                        action: () => highlightAction(option.value)
                    }));

                    const menu = [
                        { label: "Edit Entry", action: (e, row) => openEditEntryModal(row) },
                        { separator: true },
                        { label: "Cut Entry", action: (e, row) => cutRow(row) },
                        { label: "Copy Entry", action: (e, row) => copyRow(row) },
                    ];

                    if (tableClipboard && tableClipboard.type === 'row') {
                        menu.push({ label: "Paste Entry Above", action: (e, row) => pasteRow(row, 'before') });
                        menu.push({ label: "Paste Entry Below", action: (e, row) => pasteRow(row, 'after') });
                    }

                    menu.push({ separator: true });
                    menu.push({ label: "Insert Entry Above", action: (e, row) => insertRow(row, 'before') });
                    menu.push({ label: "Insert Entry Below", action: (e, row) => insertRow(row, 'after') });
                    menu.push({ separator: true });
                    menu.push({ label: "Delete Entry", action: (e, row) => deleteRow(row) });
                    menu.push({ separator: true });
                    menu.push({ label: "Highlight Entry", menu: highlightColorOptions });
                    menu.push({ label: "Clear Entry Highlight", action: () => highlightAction(null) });

                    return menu;
                },
                columnDefaults: {
                    headerSort:false,
                    headerHozAlign:"center",
                    headerVAlign:"middle",
                    editor:"textarea",
                    editorParams:{ verticalNavigation:"editor", shiftEnterSubmit:true },
                    resizable:"header",
                    width:200,
                    minWidth: 100,
                },
                rowHeader:{
                    resizable: false,
                    frozen: true,
                    headerSort:false,
                    hozAlign:"center",
                    formatter: function(cell) {
                        const rowNum = cell.getRow().getPosition(true);
                        const container = document.createElement("div");
                        container.className = "row-number-container group relative flex items-center justify-center h-full w-full";
                        
                        const span = document.createElement("span");
                        span.className = "row-number-text group-hover:hidden";
                        span.textContent = rowNum;
                        
                        const button = document.createElement("button");
                        button.className = "edit-icon-placeholder hidden group-hover:flex items-center justify-center h-full w-full text-blue-500 hover:text-blue-600 transition-colors";
                        button.title = "Edit Entry";
                        
                        mount(TableIcon, {
                            target: button,
                            props: { icon: Pencil, size: 14 }
                        });
                        
                        container.appendChild(span);
                        container.appendChild(button);
                        return container;
                    },
                    cellClick: (e, cell) => {
                        if (e.target.closest('.edit-icon-placeholder')) {
                            e.preventDefault();
                            e.stopPropagation();
                            openEditEntryModal(cell.getRow());
                        }
                    },
                    width: 50,
                    minWidth: 40,
                    cssClass:"range-header-col tabulator-row-number-column"
                },
                clipboard: true,
                clipboardCopyStyled:false,
                clipboardCopyConfig:{ rowHeaders:false, columnHeaders:false },
                clipboardCopyRowRange:"range",
                clipboardPasteParser:"range",
                clipboardPasteAction:"range",
            });
            tabulatorInstance.on("tableBuilt", () => {
                tableReady = true;
                addFloatingAddRowButton();
                detectDuplicates();
                checkValidationErrors();
            });
            tabulatorInstance.on("renderComplete", () => {
                updateFloatingAddRowButtonPosition();
            });
            tabulatorInstance.on("scrollVertical", () => {
                updateFloatingAddRowButtonPosition();
            });
            const saveCurrentTableLayout = debounce(async () => {
                if (!tabulatorInstance || !currentLoadedPath) return;
                const baseDirForSave = get(project)?.baseDirectory;
                const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
                if (!baseDirForSave || !relativePathForSave) return;
                updateTableLayoutSnapshot();
                await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch(err => console.error(`Failed to save layout:`, err));
            }, 750);
            tabulatorInstance.on("columnResized", saveCurrentTableLayout);

            // Event-driven layout saving for structural changes
            tabulatorInstance.on("columnMoved", saveCurrentTableLayoutImmediately);
            tabulatorInstance.on("columnAdded", saveCurrentTableLayoutImmediately);
            tabulatorInstance.on("columnDeleted", saveCurrentTableLayoutImmediately);

            tabulatorInstance.on("cellEdited", (cell) => {
                debouncedSave();
                if (cell.getField() === currentPrimaryField) {
                    detectDuplicates();
                }
                const row = cell.getRow();
                const rowData = row.getData();
                const rowIndex = rowData.harvey_internal_id;
                const highlightId = `row-${rowIndex}`;

                let currentHighlights = get(project).currentTableHighlights || [];
                const highlightIndex = currentHighlights.findIndex(h => h.id === highlightId);

                if (highlightIndex !== -1) {
                    const newText = Object.values(rowData).filter(val => val !== null && val !== undefined).join(' | ');
                    const updatedHighlights = [
                        ...currentHighlights.slice(0, highlightIndex),
                        { ...currentHighlights[highlightIndex], text: newText },
                        ...currentHighlights.slice(highlightIndex + 1)
                    ];
                    setTableHighlights(updatedHighlights);
                    saveTableHighlights();
                }
            });
            columnFields = tabulatorInstance.getColumnDefinitions().map(c => c.field).filter(Boolean);
        } catch (err) {
            error = `Failed to load table: ${err.message || err}`;
        } finally {
            isLoading = false;
        }
    }

    function handleSearch() {
        if (!tabulatorInstance) return;

        // Clear existing highlights and reset matches
        cellMatches.forEach(cell => {
            const el = cell.getElement();
            if (el) el.classList.remove('search-match-focus');
        });
        cellMatches = [];
        currentMatchIndex = -1;
        reformatAllRows(); // Redraw to clear old highlights from formatter

        const term = searchTerm.trim().toLowerCase();

        if (!term) {
            tabulatorInstance.clearFilter();
            return;
        }

        // Filter entries first
        tabulatorInstance.setFilter((data) => {
            for (const key in data) {
                if (key === 'harvey_internal_id') continue;
                const value = data[key];
                if (value !== null && value !== undefined && String(value).toLowerCase().includes(term)) {
                    return true;
                }
            }
            return false;
        });

        // After filtering, find all matching cells in the active (visible) entries
        const activeRows = tabulatorInstance.getRows('active');
        activeRows.forEach(row => {
            row.getCells().forEach(cell => {
                const cellValue = cell.getValue();
                if (cellValue !== null && cellValue !== undefined && String(cellValue).toLowerCase().includes(term)) {
                    cellMatches.push(cell);
                }
            });
        });

        // Use reformat instead of full redraw
        reformatAllRows();

        if (cellMatches.length > 0) {
            navigateToMatch(0);
        }

        // Restore focus to the search input after the search is complete
        if (searchInputRef) {
            searchInputRef.focus();
        }
    }

    async function navigateToMatch(index) {
        if (!tabulatorInstance || !cellMatches[index]) return;

        // Clear any previous programmatically created ranges to ensure only one cell is selected
        const ranges = tabulatorInstance.getRanges();
        if (ranges) {
            ranges.forEach(range => range.remove());
        }

        currentMatchIndex = index;
        const currentCell = cellMatches[currentMatchIndex];

        // Scroll to the entry of the current cell first to ensure it is visible
        await currentCell.getRow().scrollTo().catch(err => console.error("Scroll to entry failed", err));

        // Use Tabulator's built-in range selection to highlight the active cell
        tabulatorInstance.addRange(currentCell, currentCell);
    }

    let addRowButtonEl = null;

    function addFloatingAddRowButton() {
        if (!tableContainer || addRowButtonEl) return;
        
        addRowButtonEl = document.createElement("button");
        addRowButtonEl.className = "absolute z-30 flex items-center justify-center w-8 h-8 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-full shadow-md text-blue-500 hover:text-blue-600 transition-all hover:scale-110";
        addRowButtonEl.style.left = "9px"; // Centered under the 50px width row number column
        addRowButtonEl.title = "Add New Entry";
        
        mount(TableIcon, {
            target: addRowButtonEl,
            props: { icon: Plus, size: 18 }
        });

        addRowButtonEl.onclick = () => {
            const rows = tabulatorInstance.getRows();
            const lastRow = rows.length > 0 ? rows[rows.length - 1] : null;
            insertRow(lastRow, 'after');
        };

        tableContainer.appendChild(addRowButtonEl);
        updateFloatingAddRowButtonPosition();
    }

    function updateFloatingAddRowButtonPosition() {
        if (!tabulatorInstance || !addRowButtonEl) return;
        
        const rows = tabulatorInstance.getRows("active");
        if (rows.length === 0) {
            addRowButtonEl.style.top = "45px"; // Just below header
            return;
        }

        const lastRow = rows[rows.length - 1];
        const lastRowEl = lastRow.getElement();
        const tableHeaderHeight = tableContainer.querySelector(".tabulator-header")?.offsetHeight || 0;
        
        // Position it just below the last row
        const topPos = lastRowEl.offsetTop + lastRowEl.offsetHeight + tableHeaderHeight - tabulatorInstance.rowManager.element.scrollTop;
        
        addRowButtonEl.style.top = `${topPos + 5}px`;
        
        // Hide if it would be outside the visible area of the holder
        const holderHeight = tableContainer.querySelector(".tabulator-tableholder")?.offsetHeight || 0;
        if (topPos > holderHeight + tableHeaderHeight || topPos < tableHeaderHeight) {
            addRowButtonEl.style.opacity = "0";
            addRowButtonEl.style.pointerEvents = "none";
        } else {
            addRowButtonEl.style.opacity = "1";
            addRowButtonEl.style.pointerEvents = "auto";
        }
    }

    function goToNextMatch() {
        if (cellMatches.length === 0) return;
        const nextIndex = (currentMatchIndex + 1) % cellMatches.length;
        navigateToMatch(nextIndex);
    }

    function goToPreviousMatch() {
        if (cellMatches.length === 0) return;
        const prevIndex = (currentMatchIndex - 1 + cellMatches.length) % cellMatches.length;
        navigateToMatch(prevIndex);
    }

    onMount(() => {
        if (tablePath) initializeTable(tablePath);
        const undoBtn = document.getElementById("history-undo");
        const redoBtn = document.getElementById("history-redo");
        const undo = () => tabulatorInstance?.undo();
        const redo = () => tabulatorInstance?.redo();
        undoBtn?.addEventListener("click", undo);
        redoBtn?.addEventListener("click", redo);

        const handleKeyDown = (e) => {
            if (e.metaKey && e.key === 'c') {
                e.preventDefault();
                tabulatorInstance?.copyToClipboard("range");
            }
        };
        tableContainer?.addEventListener('keydown', handleKeyDown);

        const handleHeaderFilterKeydown = (e) => {
            if (e.target.tagName === 'INPUT' && e.target.closest('.tabulator-header-filter') && e.key === 'Enter') {
                e.preventDefault();
                e.stopPropagation();
            }
        };
        tableContainer?.addEventListener('keydown', handleHeaderFilterKeydown);

        const handleTableClick = async (e) => {
            const urlIconContainer = e.target.closest('.hyperlink-icon-container');
            const emailIconContainer = e.target.closest('.email-icon-container');
            const projectLinkIconContainer = e.target.closest('.project-link-icon-container');

            if (urlIconContainer) {
                e.preventDefault();
                e.stopPropagation();
                const cellElement = urlIconContainer.closest('.tabulator-cell');
                if (cellElement && cellElement.dataset.urlValue) {
                    const rect = urlIconContainer.getBoundingClientRect();
                    popoverUrl = cellElement.dataset.urlValue;
                    popoverX = rect.left + window.scrollX - 60; // offset a bit to center the popover
                    popoverY = rect.bottom + window.scrollY + 5;
                    showUrlPopover = true;
                    showEmailPopover = false;
                    showProjectLinkPopover = false;
                    isUrlCopied = false;
                }
            } else if (emailIconContainer) {
                e.preventDefault();
                e.stopPropagation();
                const cellElement = emailIconContainer.closest('.tabulator-cell');
                if (cellElement && cellElement.dataset.emailValue) {
                    const rect = emailIconContainer.getBoundingClientRect();
                    popoverEmail = cellElement.dataset.emailValue;
                    popoverEmailX = rect.left + window.scrollX - 60; // offset a bit to center the popover
                    popoverEmailY = rect.bottom + window.scrollY + 5;
                    showEmailPopover = true;
                    showUrlPopover = false;
                    showProjectLinkPopover = false;
                    isEmailCopied = false;
                }
            } else if (projectLinkIconContainer) {
                e.preventDefault();
                e.stopPropagation();
                const cellElement = projectLinkIconContainer.closest('.tabulator-cell');
                if (cellElement && cellElement.dataset.projectLinkValue) {
                    const rect = projectLinkIconContainer.getBoundingClientRect();
                    popoverProjectLink = cellElement.dataset.projectLinkValue;
                    popoverProjectLinkX = rect.left + window.scrollX - 60; // offset a bit to center the popover
                    popoverProjectLinkY = rect.bottom + window.scrollY + 5;

                    const projectAssets = await getAllProjectAssets();
                    const asset = projectAssets.find(a => a.value === popoverProjectLink);
                    popoverProjectLinkCategory = asset ? asset.category : '';

                    showProjectLinkPopover = true;
                    showEmailPopover = false;
                    showUrlPopover = false;
                }
            } else {
                if (showUrlPopover && !e.target.closest('.url-popover-container')) {
                    showUrlPopover = false;
                }
                if (showEmailPopover && !e.target.closest('.email-popover-container')) {
                    showEmailPopover = false;
                }
                if (showProjectLinkPopover && !e.target.closest('.project-link-popover-container')) {
                    showProjectLinkPopover = false;
                }
            }
        };
        document.addEventListener('click', handleTableClick);

		return () => {
			tabulatorInstance?.destroy();
            undoBtn?.removeEventListener("click", undo);
            redoBtn?.removeEventListener("click", redo);
            tableContainer?.removeEventListener('keydown', handleKeyDown);
            tableContainer?.removeEventListener('keydown', handleHeaderFilterKeydown);
            document.removeEventListener('click', handleTableClick);
		}
    });

    $: if (tablePath && tablePath !== currentLoadedPath) {
        initializeTable(tablePath);
    }

    $: if ($panelStateStore.tagsLeftPanelCollapsed !== undefined && tabulatorInstance) {
        // Debounce this to avoid excessive redraws during rapid toggling
        debounce(() => {
            reformatAllRows();
        }, 100)();
    }
</script>

{#if showEditFieldModal}
    <EditFieldModal
        fieldName={editingFieldData.name}
        colSchema={editingFieldData.schema}
        currentPrimaryField={currentPrimaryField}
        on:save={handleSaveField}
        on:cancel={() => {
            showEditFieldModal = false;
            isAddingNewField = false;
            newFieldTargetColumn = null;
        }}
    />
{/if}

{#if showEditEntryModal}
    <EditEntryModal
        rowData={editingEntryData}
        rowIndex={editingEntryIndex}
        columns={tableColumnsForModal}
        schema={tableSchema}
        on:save={handleSaveEntry}
        on:cancel={() => showEditEntryModal = false}
    />
{/if}

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-900 shadow overflow-hidden">
     <div class="flex items-center justify-between h-12 px-4 border-b border-gray-200 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50 backdrop-blur-md">
        <div class="flex items-center gap-1">
            <Button size="xs" color="alternative" id="history-undo" class="px-2">
                <Undo2 size={16} />
            </Button>
            <Tooltip triggeredBy="#history-undo">Undo</Tooltip>
            
            <Button size="xs" color="alternative" id="history-redo" class="px-2">
                <Redo2 size={16} />
            </Button>
            <Tooltip triggeredBy="#history-redo">Redo</Tooltip>
        </div>

         {#if !isLoading && !error}
         <div class="flex items-center gap-3">
            <div class="flex items-center gap-1">
                <Search
                    size="sm"
                    bind:this={searchInputRef}
                    bind:value={searchTerm}
                    on:input={handleSearch}
                    on:keydown={e => {
                        if (e.key === 'Enter') {
                            e.preventDefault();
                            e.stopPropagation();
                            goToNextMatch();
                        }
                    }}
                    placeholder="Search table..."
                    class="w-64"
                />
                
                <div class="flex items-center">
                    <Button size="xs" color="alternative" on:click={goToPreviousMatch} disabled={cellMatches.length === 0} class="px-2 rounded-r-none border-r-0" title="Previous match">
                        <ChevronLeft size={16} />
                    </Button>
                    <Button size="xs" color="alternative" on:click={goToNextMatch} disabled={cellMatches.length === 0} class="px-2 rounded-l-none" title="Next match">
                        <ChevronRight size={16} />
                    </Button>
                </div>
            </div>

            <div class="h-6 w-[1px] bg-gray-300 dark:bg-gray-700 mx-1"></div>

             <div class="relative">
                <Button size="xs" color="alternative" id="table-options-btn" class="px-2">
                  <MoreVertical size={16} />
                </Button>
                <Dropdown triggeredBy="#table-options-btn" placement="bottom-end">
                    <DropdownItem on:click={toggleFilters}>
                        {areFiltersVisible ? 'Hide' : 'Show'} Column Filters
                    </DropdownItem>
                    <DropdownItem on:click={() => tabulatorInstance?.download("csv", "table_export.csv")}>
                        Export as CSV
                    </DropdownItem>
                </Dropdown>
              </div>
         </div>
         {/if}
    </div>

    <div class="flex-grow overflow-auto min-h-0 relative">
        {#if showUrlPopover}
            <div class="url-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1" style="left: {popoverX}px; top: {popoverY}px; min-width: 140px;">
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleOpenUrl}>
                    <ExternalLink size={14} /> Open in browser
                </button>
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleCopyUrl}>
                    {#if isUrlCopied}
                        <Check size={14} class="text-green-500" /> Copied
                    {:else}
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-copy"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg> Copy
                    {/if}
                </button>
            </div>
        {/if}

        {#if showProjectLinkPopover}
            <div class="project-link-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1" style="left: {popoverProjectLinkX}px; top: {popoverProjectLinkY}px; min-width: 140px;">
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleOpenProjectLink}>
                    <FolderOpen size={14} /> Open File
                </button>
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleRevealProjectLink}>
                    <FolderSearch size={14} /> {revealButtonLabel}
                </button>
            </div>
        {/if}

        {#if showEmailPopover}
            <div class="email-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1" style="left: {popoverEmailX}px; top: {popoverEmailY}px; min-width: 140px;">
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleOpenEmail}>
                    <Mail size={14} /> Send Email
                </button>
                <button class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2" on:click={handleCopyEmail}>
                    {#if isEmailCopied}
                        <Check size={14} class="text-green-500" /> Copied
                    {:else}
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-copy"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg> Copy Email
                    {/if}
                </button>
            </div>
        {/if}

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
         @apply p-1 text-xs border border-gray-300 dark:border-gray-700 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 box-border w-auto;
     }
     :global(.tabulator .tabulator-row .tabulator-cell.cell-highlighted-placeholder) {
         background-color: rgba(255, 255, 0, 0.3) !important;
     }
    :global(.tabulator-row-number-column) {
         background-color: #f0f0f0; /* Light gray background */
         font-weight: bold;
         color: #555;
         border-right: 1px solid #ddd;
         padding: 0 !important;
         text-align: center; /* Center the number */
     }
     :global(.row-number-container:hover .row-number-text) {
         display: none;
     }
     :global(.row-number-container:hover .edit-icon-placeholder) {
         display: flex !important;
     }
     :global(.tabulator-row:hover .tabulator-row-number-column) {
         background-color: #e5e7eb !important;
     }
     :global(html.dark .tabulator-row-number-column) {
         background-color: #1f2937;
         color: #9ca3af;
         border-right: 1px solid #374151;
     }
     :global(html.dark .tabulator-row:hover .tabulator-row-number-column) {
         background-color: #374151 !important;
     }

     :global(.add-column-header) {
         border-left: 1px dashed #3b82f6 !important;
         background-color: rgba(59, 130, 246, 0.05) !important;
     }
     :global(html.dark .add-column-header) {
         border-left: 1px dashed #3b82f6 !important;
         background-color: rgba(59, 130, 246, 0.1) !important;
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

        :global(.tabulator-cell) {
            overflow: hidden;
            word-break: break-word;
            border-right: 1px solid #ddd;
        }

        :global(.tabulator-cell textarea) {
            width: 100%;
            height: 100%;
            box-sizing: border-box;
            overflow: auto;
            white-space: pre-wrap;
            word-break: break-all;
            border: none;
            resize: none;
            padding: 2px 4px;
            margin: 0;
            background-color: transparent;
            color: inherit;
            font-family: inherit;
            font-size: inherit;
        }

        :global(html.dark .tabulator-row.highlighted-row .tabulator-cell) {
            color: #111827 !important;
        }
        :global(html.dark .tabulator-cell.highlighted-cell) {
            color: #111827 !important;
        }
        
        /* Ensure the Add Field column doesn't inherit row highlight backgrounds */
        :global(.tabulator-row .tabulator-cell.add-column-header) {
            background-color: white !important;
        }
        :global(html.dark .tabulator-row .tabulator-cell.add-column-header) {
            background-color: #030712 !important; /* gray-950 */
        }
        :global(.tabulator-row:hover .tabulator-cell.add-column-header) {
            background-color: #f9fafb !important; /* gray-50 */
        }
        :global(html.dark .tabulator-row:hover .tabulator-cell.add-column-header) {
            background-color: #111827 !important; /* gray-900 */
        }

        :global(.search-match-highlight) {
            background-color: #ffdd77;
            font-weight: bold;
        }
        :global(html.dark .search-match-highlight) {
            background-color: #ffdd77;
            color: #111827;
        }
        :global(.invalid-cell) {
            box-shadow: inset 0 0 0 2px #ef4444 !important;
        }
        :global(.duplicate-primary-cell) {
            box-shadow: inset 0 0 0 2px #ef4444 !important;
        }

        :global(.tabulator-cell.interactive-contact-cell:hover .hyperlink-icon-container, .tabulator-cell.interactive-contact-cell:hover .email-icon-container, .tabulator-cell.interactive-contact-cell:hover .project-link-icon-container) {
            display: flex !important;
        }
</style>
