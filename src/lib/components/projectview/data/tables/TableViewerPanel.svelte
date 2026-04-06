<!-- src/lib/components/projectview/data/tables/TableViewerPanel.svelte -->
<script>
  import { onMount, onDestroy, tick, untrack } from 'svelte';
  import { get, writable } from 'svelte/store';
  import { TabulatorFull as Tabulator, HistoryModule } from 'tabulator-tables';
  Tabulator.registerModule(HistoryModule);
  import { panelState } from '$lib/stores/panelStateStore.svelte.js';
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
  import {
    project,
    setTableHighlights,
    setLoadedTableHighlights,
    setDocumentHighlights
  } from '$lib/stores/projectStore.js';
  import { sep } from '@tauri-apps/api/path';
  import { HIGHLIGHT_OPTIONS } from '$lib/constants/highlightOptions.js';
  import EditEntryModal from '$lib/components/projectview/modals/EditEntryModal.svelte';
  import EditFieldModal from '$lib/components/projectview/modals/EditFieldModal.svelte';
  import TableHeaderIcon from './TableHeaderIcon.svelte';
  import ChartModal from './ChartModal.svelte';
  import ViewModal from './ViewModal.svelte';
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
    FolderOpen,
    Bold,
    Italic,
    Underline,
    Eraser,
    PieChart,
    ChartBar,
    Table2
  } from '@lucide/svelte';
  import { mount, createEventDispatcher } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { invoke } from '@tauri-apps/api/core';
  import { Input, Button, Dropdown, DropdownItem, Search, Badge } from 'flowbite-svelte';
  import { Datepicker } from 'flowbite-datepicker';
  import { mediaEditorStore } from '$lib/stores/mediaEditorStore.svelte.js';
  import LexicalEditor from '$lib/components/projectview/lexical/LexicalEditor.svelte';
  import FloatingTableHighlightToolbar from '../../tables/FloatingTableHighlightToolbar.svelte';

  let {
    tablePath = '',
    hasHeaders = true,
    activeSubItemPath = $bindable(null),
    activeSubItemType = $bindable(null),
    initialChartToLoad = null
  } = $props();

  const dispatch = createEventDispatcher();

  let tableContainer = $state();
  let tabulatorInstance = $state(null);
  let tableData = $state([]);
  let tableSchema = $state({});
  let isLoading = $state(false);
  let error = $state(null);
  let currentLoadedPath = $state(null);
  let availableViews = $state([]);

  let isViewingDocument = $state(false);
  let currentActiveDocumentPath = $state(null);
  let currentActiveDocumentJson = $state(null);
  let currentActiveDocumentHighlights = $state([]);

  let svelteUndoStack = $state([]);
  let svelteRedoStack = $state([]);
  let isUndoRedoActive = $state(false); // Flag to prevent history tracking during undo/redo actions

  const highlightOptions = HIGHLIGHT_OPTIONS;

  let tableReady = $state(false);
  let tableStyles = $state({ rowStyles: {}, cellStyles: {} }); // This will be derived from highlights

  let projectAssetOptions = [];

  let showEditEntryModal = $state(false);
  let editingEntryData = $state(null);
  let editingEntryIndex = $state(-1);
  let tableColumnsForModal = $state([]);

  let currentPrimaryField = $state(null);
  let duplicateIds = $state(new Set()); // Stores harvey_internal_id of rows with duplicate primary values

  let tableHasValidationErrors = $state(false);
  let invalidCells = $state(new Map()); // Stores cell keys "rowIndex-colField" -> errorMessage

  let resizingColField = $state(null);
  let initialMouseX = $state(0);
  let initialColWidth = $state(0);

  let currentActiveViewType = $state('table'); // 'table', 'pivot'

  function handleManualResizeStart(e, field) {
    if (!tabulatorInstance) return;
    const col = tabulatorInstance.getColumn(field);
    if (!col) return;

    resizingColField = field;
    initialMouseX = e.clientX;
    initialColWidth = col.getWidth();

    window.addEventListener('mousemove', handleManualResizeMove);
    window.addEventListener('mouseup', handleManualResizeEnd);
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';
  }

  function handleManualResizeMove(e) {
    if (!resizingColField || !tabulatorInstance) return;

    const deltaX = e.clientX - initialMouseX;
    const newWidth = Math.max(100, initialColWidth + deltaX);

    const col = tabulatorInstance.getColumn(resizingColField);
    if (col && resizingColField !== 'harvey_pseudo_add_col') {
      col.setWidth(newWidth);
    }
  }

  function handleManualResizeEnd() {
    window.removeEventListener('mousemove', handleManualResizeMove);
    window.removeEventListener('mouseup', handleManualResizeEnd);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';

    if (resizingColField) {
      saveCurrentTableLayout();
      resizingColField = null;
    }
  }

  let showTableModifyToolbar = $state(false);
  let tableModifyToolbarPosition = $state({ top: 0, left: 0 });
  let clickedRow = $state(null);
  let selectedRows = $state([]); // Rows from a multi-cell/multi-row selection
  let activeHighlightIdForToolbar = $state(null);

  // Clear all previous scroll/sync logic, no longer needed with pseudo-elements
  let tableHeaderHeight = $state(52);
  let tableInnerWidth = $state(0);
  let tableInnerHeight = $state(0);

  function updateTableDimensions() {
    if (!tableContainer) return;
    const headers = tableContainer.querySelector('.tabulator-headers');
    if (headers) {
      tableHeaderHeight = headers.offsetHeight;
    }
  }
  let lastRangeSelectedTime = 0; // Timestamp to prevent immediate closing of toolbar
  let mainPanelContainer = null;

  function reformatAllRows() {
    if (tabulatorInstance && tableReady) {
      tabulatorInstance.getRows().forEach((row) => row.reformat());
    }
  }

  function handleTableContainerClick(e) {
    if (currentActiveViewType === 'pivot' || !mainPanelContainer || !tabulatorInstance) return;

    const cellEl = e.target.closest('.tabulator-cell');
    const rowEl = e.target.closest('.tabulator-row');

    if (rowEl) {
      try {
        const row = tabulatorInstance.getRow(rowEl);
        if (row) {
          const rowData = row.getData();
          const rowIndex = rowData.harvey_internal_id;
          const highlights = get(project).currentTableHighlights || [];

          // Find the highlight this row/cell belongs to
          const field =
            cellEl?.dataset.field ||
            e.target.closest('.tabulator-cell')?.getAttribute('tabulator-field');
          const cellKey = field ? `cell-${rowIndex}-${field}` : null;

          const existingHighlight = highlights.find((h) => {
            if (cellKey && h.id === cellKey) return true; // Direct cell match
            const indices = h.rowIndices || [parseInt(h.id?.substring(4), 10)];
            return !h.id?.startsWith('cell-') && indices.includes(rowIndex);
          });

          if (existingHighlight) {
            activeHighlightIdForToolbar = existingHighlight.id;

            // If it's a group, load the whole group into selectedRows
            if (existingHighlight.rowIndices && existingHighlight.rowIndices.length > 1) {
              selectedRows = existingHighlight.rowIndices
                .map((idx) => tabulatorInstance.getRow(idx))
                .filter((r) => r !== false);
              clickedRow = null;
            } else {
              // Single row or cell highlight
              clickedRow = row;
              selectedRows = [];
            }

            // Use cell element for positioning if available, otherwise fallback to row
            const targetEl = cellEl || rowEl;
            const rect = targetEl.getBoundingClientRect();
            const showBelow = rect.top < 150;

            tableModifyToolbarPosition = {
              top: showBelow ? rect.bottom + 5 : rect.top - 45,
              left: Math.max(10, rect.left + rect.width / 2 - 60)
            };
            showTableModifyToolbar = true;
          }
        }
      } catch (err) {
        console.error('Error in table container click handler:', err);
      }
    }
  }

  function handleTableMouseUp(e) {
    if (!tabulatorInstance) return;
    // Fallback for cases where rangeSelected doesn't fire
    setTimeout(() => {
      const ranges = tabulatorInstance.getRanges();
      if (ranges && ranges.length > 0 && !showTableModifyToolbar) {
        const range = ranges[0];
        const rows = range.getRows();
        const cols = range.getColumns();

        // Only show on drag end if it's a multi-cell selection (more than 1x1)
        if (rows && cols && (rows.length > 1 || cols.length > 1)) {
          selectedRows = rows;
          clickedRow = null;

          // Identify if this selection matches an existing highlight group
          const firstIdx = rows[0].getData().harvey_internal_id;
          const highlights = get(project).currentTableHighlights || [];
          const existingHighlight = highlights.find(
            (h) => h.rowIndices && h.rowIndices.includes(firstIdx)
          );
          activeHighlightIdForToolbar = existingHighlight ? existingHighlight.id : null;

          lastRangeSelectedTime = Date.now();

          // In a multi-cell selection, anchor to the bottom-right cell
          const lastRow = rows[rows.length - 1];
          const lastCol = cols[cols.length - 1];

          try {
            const cell = lastRow
              .getCells()
              .find((c) => c.getColumn().getField() === lastCol.getField());
            const cellEl = cell ? cell.getElement() : lastRow.getElement();

            if (cellEl) {
              const rect = cellEl.getBoundingClientRect();
              const showBelow = rect.top < 150;

              tableModifyToolbarPosition = {
                top: showBelow ? rect.bottom + 5 : rect.top - 45,
                left: Math.min(
                  window.innerWidth - 130,
                  Math.max(10, rect.left + rect.width / 2 - 60)
                )
              };
              showTableModifyToolbar = true;
            }
          } catch (err) {
            // Fallback to row element if cell target fails
            const lastRowEl = lastRow.getElement();
            if (lastRowEl) {
              const rect = lastRowEl.getBoundingClientRect();
              const showBelow = rect.top < 150;
              tableModifyToolbarPosition = {
                top: showBelow ? rect.bottom + 5 : rect.top - 45,
                left: Math.min(
                  window.innerWidth - 130,
                  Math.max(10, rect.left + rect.width / 2 - 60)
                )
              };
              showTableModifyToolbar = true;
            }
          }
        }
      }
    }, 100);
  }

  // Reactive mapping of store highlights to Tabulator styles
  $effect(() => {
    const highlights = $project.currentTableHighlights; // Primary dependency
    
    untrack(() => {
      if (!highlights) {
        tableStyles = { rowStyles: {}, cellStyles: {} };
        return;
      }

      const newRowStyles = {};
      const newCellStyles = {};

      if (Array.isArray(highlights)) {
        highlights.forEach((h) => {
          if (h.id?.startsWith('row-') || h.rowIndices) {
            const indices = h.rowIndices || [h.id.substring(4)];
            indices.forEach((idx) => {
              newRowStyles[idx] = h.color;
            });
          } else if (h.id?.startsWith('cell-')) {
            newCellStyles[h.id] = {
              color: h.color,
              textColor: h.textColor,
              bold: h.bold,
              italic: h.italic,
              underline: h.underline
            };
          }
        });
      }

      tableStyles = { rowStyles: newRowStyles, cellStyles: newCellStyles };

      // Debounce reformat to avoid rapid successive calls during project state updates
      debounce(() => {
        if (typeof reformatAllRows === 'function') {
          reformatAllRows();
        }
      }, 50)();
    });
  });

  async function toggleStyle(styleType) {
    if (!tabulatorInstance) return;

    // Tabulator uses ranges for cell selection
    const ranges = tabulatorInstance.getRanges();
    if (!ranges || ranges.length === 0) return;

    let cellsToModify = [];
    ranges.forEach((range) => {
      const rows = range.getRows();
      const columns = range.getColumns();

      rows.forEach((row) => {
        columns.forEach((col) => {
          const cell = row.getCell(col.getField());
          if (cell) cellsToModify.push(cell);
        });
      });
    });

    if (cellsToModify.length === 0) return;

    let currentHighlights = get(project).currentTableHighlights || [];

    // Check if all selected cells already have the style, if so, we toggle it off. Otherwise, toggle on.
    let allHaveStyle = true;
    cellsToModify.forEach((cell) => {
      const field = cell.getField();
      const schema = tableSchema[field];
      if (
        schema &&
        schema.type === 'Misc' &&
        (schema.subType === 'Checkbox' ||
          schema.subType === 'Rating' ||
          schema.subType === 'Progress' ||
          schema.subType === 'Selectbox' ||
          schema.subType === 'Multiselect')
      ) {
        return; // Skip these
      }

      const rowIndex = cell.getRow().getData().harvey_internal_id;
      const cellKey = `cell-${rowIndex}-${field}`;
      const existing = currentHighlights.find((h) => h.id === cellKey);
      if (!existing || !existing[styleType]) {
        allHaveStyle = false;
      }
    });

    const targetStyleState = !allHaveStyle;

    cellsToModify.forEach((cell) => {
      const field = cell.getField();
      const schema = tableSchema[field];
      if (
        schema &&
        schema.type === 'Misc' &&
        (schema.subType === 'Checkbox' ||
          schema.subType === 'Rating' ||
          schema.subType === 'Progress' ||
          schema.subType === 'Selectbox' ||
          schema.subType === 'Multiselect')
      ) {
        return;
      }

      const rowIndex = cell.getRow().getData().harvey_internal_id;
      const cellKey = `cell-${rowIndex}-${field}`;
      let existingIndex = currentHighlights.findIndex((h) => h.id === cellKey);

      if (existingIndex !== -1) {
        currentHighlights[existingIndex][styleType] = targetStyleState;
        // If it has no color, no bold, no italic, no underline, maybe remove it entirely?
        // For now, keep the entry as it might hold text or comments later
      } else if (targetStyleState) {
        const cellValue = cell.getValue();
        currentHighlights.push({
          id: cellKey,
          color: null,
          textColor: null,
          [styleType]: true,
          text: `Cell [Entry ${rowIndex + 1}, ${field}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ''}`,
          tags: [],
          comments: []
        });
      }
    });

    setTableHighlights(currentHighlights);
    await saveTableHighlights();
  }

  let isColorDropdownOpen = $state(false);
  let colorDropdownRef = $state();

  function handleOutsideClick(event) {
    if (isColorDropdownOpen && colorDropdownRef && !colorDropdownRef.contains(event.target)) {
      isColorDropdownOpen = false;
    }

    // Prevent immediate closing if a range was just selected (e.g. at the end of a drag)
    const recentlySelected = Date.now() - lastRangeSelectedTime < 300;

    if (
      showTableModifyToolbar &&
      !event.target.closest('.selection-toolbar') &&
      !event.target.closest('[role="menu"]') &&
      !event.target.closest('.z-\\[100001\\]') &&
      !event.target.closest('.z-\\[100000\\]') &&
      !recentlySelected
    ) {
      showTableModifyToolbar = false;
      clickedRow = null;
      selectedRows = [];
      activeHighlightIdForToolbar = null;
    }
  }

  function toggleColorDropdown() {
    if (!tabulatorInstance) return;
    isColorDropdownOpen = !isColorDropdownOpen;
  }

  const colorOptions = [
    { value: '#000000', label: 'Black' },
    { value: '#FF0000', label: 'Red' },
    { value: '#000080', label: 'Navy' },
    { value: '#228B22', label: 'Forest Green' },
    { value: '#FF8C00', label: 'Dark Orange' },
    { value: '#800080', label: 'Purple' },
    { value: '#008B8B', label: 'Teal' },
    { value: 'transparent', label: 'Default' }
  ];

  async function applyTextColor(colorValue) {
    if (!tabulatorInstance) return;

    const ranges = tabulatorInstance.getRanges();
    if (!ranges || ranges.length === 0) return;

    let cellsToModify = [];
    ranges.forEach((range) => {
      const rows = range.getRows();
      const columns = range.getColumns();
      rows.forEach((row) => {
        columns.forEach((col) => {
          const cell = row.getCell(col.getField());
          if (cell) cellsToModify.push(cell);
        });
      });
    });

    if (cellsToModify.length === 0) return;

    let currentHighlights = get(project).currentTableHighlights || [];

    cellsToModify.forEach((cell) => {
      const field = cell.getField();
      const schema = tableSchema[field];
      if (
        schema &&
        schema.type === 'Misc' &&
        (schema.subType === 'Checkbox' ||
          schema.subType === 'Rating' ||
          schema.subType === 'Progress' ||
          schema.subType === 'Selectbox' ||
          schema.subType === 'Multiselect')
      ) {
        return;
      }

      const rowIndex = cell.getRow().getData().harvey_internal_id;
      const cellKey = `cell-${rowIndex}-${field}`;
      let existingIndex = currentHighlights.findIndex((h) => h.id === cellKey);

      if (existingIndex !== -1) {
        currentHighlights[existingIndex].textColor =
          colorValue === 'transparent' ? null : colorValue;
      } else if (colorValue !== 'transparent') {
        const cellValue = cell.getValue();
        currentHighlights.push({
          id: cellKey,
          color: null,
          textColor: colorValue,
          bold: false,
          italic: false,
          underline: false,
          text: `Cell [Entry ${rowIndex + 1}, ${field}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ''}`,
          tags: [],
          comments: []
        });
      }
    });

    setTableHighlights(currentHighlights);
    await saveTableHighlights();
    isColorDropdownOpen = false;
  }

  async function clearFormatting() {
    if (!tabulatorInstance) return;
    const ranges = tabulatorInstance.getRanges();
    if (!ranges || ranges.length === 0) return;

    let cellsToClear = new Set();
    ranges.forEach((range) => {
      const rows = range.getRows();
      const columns = range.getColumns();
      rows.forEach((row) => {
        columns.forEach((col) => {
          const rowIndex = row.getData().harvey_internal_id;
          cellsToClear.add(`cell-${rowIndex}-${col.getField()}`);
        });
      });
    });

    if (cellsToClear.size === 0) return;

    let currentHighlights = get(project).currentTableHighlights || [];
    currentHighlights = currentHighlights.filter(
      (h) => !h.id.startsWith('cell-') || !cellsToClear.has(h.id)
    );
    setTableHighlights(currentHighlights);
    await saveTableHighlights();
  }

  let searchTerm = $state('');
  let cellMatches = $state([]); // Changed from searchMatches to store cell components
  let currentMatchIndex = $state(-1);
  let columnFields = $state([]);
  let tableLayoutSnapshot = $state({ columns: {} });
  let tableClipboard = $state(null);
  let searchInputRef = $state(null);

  let showOptionsMenu = $state(false);
  let areFiltersVisible = $state(false); // Start with the assumption that filters are hidden

  let showUrlPopover = $state(false);
  let popoverUrl = $state('');
  let popoverX = $state(0);
  let popoverY = $state(0);
  let isUrlCopied = $state(false);

  async function handleOpenUrl() {
    if (!popoverUrl) return;
    try {
      let targetUrl = popoverUrl.trim();
      if (
        !targetUrl.toLowerCase().startsWith('http://') &&
        !targetUrl.toLowerCase().startsWith('https://')
      ) {
        targetUrl = 'https://' + targetUrl;
      }
      await openUrl(targetUrl);
      showUrlPopover = false;
    } catch (e) {
      console.error('Failed to open URL:', e);
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

  let showEmailPopover = $state(false);
  let popoverEmail = $state('');
  let popoverEmailX = $state(0);
  let popoverEmailY = $state(0);
  let isEmailCopied = $state(false);

  async function handleOpenEmail() {
    if (!popoverEmail) return;
    try {
      await openUrl('mailto:' + popoverEmail.trim());
      showEmailPopover = false;
    } catch (e) {
      console.error('Failed to open email client:', e);
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

  let showProjectLinkPopover = $state(false);
  let popoverProjectLink = $state('');
  let popoverProjectLinkCategory = $state('');
  let popoverProjectLinkX = $state(0);
  let popoverProjectLinkY = $state(0);

  let revealButtonLabel = $state('Show in Finder');
  import { type as getOsType } from '@tauri-apps/plugin-os';

  onMount(async () => {
    try {
      const currentOs = await getOsType();
      if (currentOs === 'windows') revealButtonLabel = 'Reveal in Explorer';
      else if (currentOs === 'macos') revealButtonLabel = 'Reveal in Finder';
      else revealButtonLabel = 'Open File Location';
    } catch (e) {
      console.error('Error getting OS type:', e);
    }
  });

  function handleTableClick(e) {
    const hyperlinkIcon = e.target.closest('.hyperlink-icon-container');
    const emailIcon = e.target.closest('.email-icon-container');
    const projectLinkIcon = e.target.closest('.project-link-icon-container');

    if (hyperlinkIcon || emailIcon || projectLinkIcon) {
      e.preventDefault();
      e.stopPropagation();

      const cellElement = e.target.closest('.tabulator-cell');
      if (!cellElement) return;

      const rect = cellElement.getBoundingClientRect();

      if (hyperlinkIcon) {
        popoverUrl = cellElement.dataset.urlValue || '';
        popoverX = rect.right;
        popoverY = rect.bottom + window.scrollY;
        showUrlPopover = true;
        showEmailPopover = false;
        showProjectLinkPopover = false;
      } else if (emailIcon) {
        popoverEmail = cellElement.dataset.emailValue || '';
        popoverEmailX = rect.right;
        popoverEmailY = rect.bottom + window.scrollY;
        showEmailPopover = true;
        showUrlPopover = false;
        showProjectLinkPopover = false;
      } else if (projectLinkIcon) {
        const path = cellElement.dataset.projectLinkValue || '';
        popoverProjectLink = path;
        popoverProjectLinkX = rect.right;
        popoverProjectLinkY = rect.bottom + window.scrollY;

        // Find category - resolve path safely, handling both relative and absolute paths
        const proj = get(project);
        const normalizedSearchPath = path.replace(/\\/g, '/');

        const asset = projectAssetOptions.find((a) => {
          const aNormalized = a.value.replace(/\\/g, '/');
          if (aNormalized === normalizedSearchPath) return true;

          // If one is absolute and the other is relative, try to resolve a.value to absolute
          if (proj?.baseDirectory) {
            const baseDir = proj.baseDirectory.replace(/\\/g, '/');
            const aAbsolute = `${baseDir}/${aNormalized.replace(/^\/+/, '')}`;
            const searchAbsolute =
              normalizedSearchPath.startsWith('/') || normalizedSearchPath.includes(':')
                ? normalizedSearchPath
                : `${baseDir}/${normalizedSearchPath.replace(/^\/+/, '')}`;

            return aAbsolute === searchAbsolute;
          }
          return false;
        });

        popoverProjectLinkCategory = asset ? asset.category : 'Other';

        showProjectLinkPopover = true;
        showUrlPopover = false;
        showEmailPopover = false;
      }
    } else {
      // Close popovers if clicking elsewhere
      if (
        !e.target.closest('.url-popover-container') &&
        !e.target.closest('.email-popover-container') &&
        !e.target.closest('.project-link-popover-container')
      ) {
        showUrlPopover = false;
        showEmailPopover = false;
        showProjectLinkPopover = false;
      }
    }
  }

  async function handleRevealProjectLink() {
    if (!popoverProjectLink) return;
    try {
      const proj = get(project);
      let absolutePath = popoverProjectLink;
      // If the path is relative, prepend the base directory
      if (
        !absolutePath.startsWith('/') &&
        !absolutePath.startsWith('\\') &&
        !absolutePath.includes(':') &&
        proj?.baseDirectory
      ) {
        absolutePath = `${proj.baseDirectory}/${absolutePath.replace(/^\/+/, '')}`;
      }
      await invoke('locate_in_finder', { projectXmlPath: absolutePath });
      showProjectLinkPopover = false;
    } catch (e) {
      console.error('Failed to reveal project link in file manager:', e);
    }
  }

  async function handleOpenProjectLink() {
    if (!popoverProjectLink) return;
    const category = popoverProjectLinkCategory;
    const path = popoverProjectLink.toLowerCase();
    let viewType = 'document';
    let originalDocType = '';

    if (category === 'Audios' || category === 'Videos') {
      viewType = 'media';
      originalDocType = category === 'Audios' ? 'audio' : 'video';
    } else if (category === 'Audio Transcripts' || category === 'Video Transcripts') {
      viewType = 'media';
      originalDocType = category === 'Audio Transcripts' ? 'audio_transcript' : 'video_transcript';
    } else if (category === 'Transcripts') {
      // Standalone transcripts (imported)
      viewType = 'transcript';
      originalDocType = 'standalone_transcript';
    } else if (category === 'Tables') {
      viewType = 'table';
      originalDocType = 'csv';
    } else if (category === 'Images') {
      viewType = 'image';
      originalDocType = 'image';
    }

    const proj = get(project);
    let absoluteLinkPath = popoverProjectLink;
    if (
      !absoluteLinkPath.startsWith('/') &&
      !absoluteLinkPath.startsWith('\\') &&
      !absoluteLinkPath.includes(':') &&
      proj?.baseDirectory
    ) {
      absoluteLinkPath = `${proj.baseDirectory}/${popoverProjectLink.replace(/^\/+/, '')}`;
    }

    dispatch('requestviewchange', {
      tabName: 'data',
      itemPath: absoluteLinkPath, // For DataView
      loadNotePath: absoluteLinkPath, // For ProjectView
      viewType: viewType,
      originalDocType: originalDocType || category
    });
    showProjectLinkPopover = false;
  }

  function scrollToHighlight(id) {
    if (!id || !tabulatorInstance) return;

    // Only handle specific highlight formats intended for the base table
    if (!id.startsWith('row-') && !id.startsWith('rows-') && !id.startsWith('cell-')) {
      return;
    }

    // Clear immediately to prevent infinite loops
    project.update((p) => ({ ...p, requestedHighlightId: null }));

    // Clear any existing range/cell selections before focusing new ones
    const ranges = tabulatorInstance.getRanges();
    if (ranges) {
      ranges.forEach((range) => range.remove());
    }

    console.log(`[TableViewerPanel] Scrolling to highlight: ${id}`);

    let rowIndex = null;
    let fieldName = null;

    if (id.startsWith('row-')) {
      rowIndex = parseInt(id.substring(4), 10);
    } else if (id.startsWith('rows-')) {
      let h = get(project).currentTableHighlights?.find((h) => h.id === id);
      if (h && h.rowIndices && h.rowIndices.length > 0) {
        rowIndex = h.rowIndices[0]; // Scroll to the first row of the group
      }
    } else if (id.startsWith('cell-')) {
      // ID format: cell-{rowIndex}-{fieldName}
      const parts = id.split('-');
      if (parts.length >= 3) {
        rowIndex = parseInt(parts[1], 10);
        fieldName = parts.slice(2).join('-');
      }
    }

    if (rowIndex !== null && !isNaN(rowIndex)) {
      // Small delay to ensure Tabulator has finished internal layout
      setTimeout(() => {
        const row = tabulatorInstance.getRow(rowIndex);
        if (row) {
          row
            .scrollTo()
            .then(() => {
              let elToHighlight = null;

              if (fieldName) {
                const cell = row.getCell(fieldName);
                if (cell) {
                  elToHighlight = cell.getElement();
                }
              }

              // Fallback to highlighting the entire row if cell isn't found or it's a row highlight
              if (!elToHighlight) {
                elToHighlight = row.getElement();
              }

              if (elToHighlight) {
                elToHighlight.style.transition = 'outline 0.3s ease';
                elToHighlight.style.outline = '4px solid #3b82f6';
                elToHighlight.style.outlineOffset = '-4px';

                // Let's also actually add the Tabulator range so it is officially "selected"
                if (fieldName) {
                  const cell = row.getCell(fieldName);
                  if (cell) tabulatorInstance.addRange(cell, cell);
                }

                setTimeout(() => {
                  elToHighlight.style.outline = 'none';
                }, 2000);
              }
            })
            .catch((err) =>
              console.error(`[TableViewerPanel] Scroll failed for entry ${rowIndex}:`, err)
            );
        } else {
          console.warn(`[TableViewerPanel] Entry ${rowIndex} not found for highlight ${id}`);
        }
      }, 100);
    }
  }

  $effect(() => {
    if ($project.requestedHighlightId && tableReady && !isViewingDocument) {
      scrollToHighlight($project.requestedHighlightId);
    }
  });

  $effect(() => {
    if (tableReady && mediaEditorStore.isLexicalEditMode !== undefined) {
      // Redraw/Re-run floating layout/buttons when edit mode toggles
      if (tabulatorInstance) {
      }
    }
  });

  async function toggleFilters() {
    if (!tabulatorInstance) return;
    areFiltersVisible = !areFiltersVisible;
    const columns = tabulatorInstance.getColumns();

    await Promise.all(
      columns.map(async (column) => {
        const definition = column.getDefinition();
        if (definition.field) {
          // Ensure it's a data field
          if (!areFiltersVisible) {
            // Clear the filter value before hiding
            tabulatorInstance.setHeaderFilterValue(definition.field, '');
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
    await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch((err) =>
      console.error(`Failed to save layout:`, err)
    );
  }, 750);

  async function saveCurrentTableLayoutImmediately() {
    if (!tabulatorInstance || !currentLoadedPath) return;
    const baseDirForSave = get(project)?.baseDirectory;
    const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
    if (!baseDirForSave || !relativePathForSave) return;
    updateTableLayoutSnapshot();
    await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch((err) =>
      console.error(`Failed to save layout:`, err)
    );
  }

  async function saveTableChanges() {
    if (!tabulatorInstance) return;
    const updatedData = tabulatorInstance.getData();

    // Filter out the pseudo-row before saving data to the backend
    const filteredData = updatedData.filter(row => row.harvey_internal_id !== 'harvey_pseudo_add_row');
    const dataToSave = JSON.parse(JSON.stringify(filteredData));
    dataToSave.forEach((row) => {
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
      .filter((column) => column.getField() && column.getField() !== 'harvey_pseudo_add_col') // Ensure we only get data fields
      .map((column) => column.getField());

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

    data.forEach((row) => {
      const val = String(row[currentPrimaryField] || '').trim();
      if (val === '') return; // Skip empty
      if (!valueMap.has(val)) {
        valueMap.set(val, []);
      }
      valueMap.get(val).push(row.harvey_internal_id);
    });

    const newDuplicateIds = new Set();
    let foundDuplicates = false;
    valueMap.forEach((ids, val) => {
      if (ids.length > 1) {
        ids.forEach((id) => newDuplicateIds.add(id));
        foundDuplicates = true;
      }
    });

    if (foundDuplicates && duplicateIds.size === 0) {
      import('@tauri-apps/plugin-dialog').then((d) => {
        d.message(
          `Duplicate values found in primary field "${currentPrimaryField}". Duplicates are highlighted in red.`,
          { title: 'Duplicate Primary Key', type: 'warning' }
        );
      });
    }

    duplicateIds = newDuplicateIds;
    // Debounce reformat to prevent cascade hangs
    debounce(() => reformatAllRows(), 50)();
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
    const values = tabulatorInstance.getRows().map((row) => row.getData()[field]);
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
    const isEditMode = mediaEditorStore.isLexicalEditMode;
    if (currentActiveViewType === 'pivot' || !isEditMode) {
      return [
        {
          label: 'Sort Ascending',
          action: (e, column) => tabulatorInstance.setSort(column.getField(), 'asc')
        },
        {
          label: 'Sort Descending',
          action: (e, column) => tabulatorInstance.setSort(column.getField(), 'desc')
        },
        { label: 'Copy Field', action: (e, column) => copyColumn(column) }
      ];
    }

    const menu = [
      { label: 'Edit Field', action: (e, column) => openFieldEditor(column) },
      { separator: true },
      {
        label: 'Sort Ascending',
        action: (e, column) => tabulatorInstance.setSort(column.getField(), 'asc')
      },
      {
        label: 'Sort Descending',
        action: (e, column) => tabulatorInstance.setSort(column.getField(), 'desc')
      },
      { separator: true },
      { label: 'Cut Field', action: (e, column) => cutColumn(column) },
      { label: 'Copy Field', action: (e, column) => copyColumn(column) }
    ];
    if (tableClipboard && tableClipboard.type === 'column') {
      menu.push({
        label: 'Paste Field Before',
        action: (e, column) => pasteColumn(column, 'before')
      });
      menu.push({
        label: 'Paste Field After',
        action: (e, column) => pasteColumn(column, 'after')
      });
    }
    menu.push({ separator: true });
    menu.push({
      label: 'Insert Field Before',
      action: (e, column) => insertColumn(column, 'before')
    });
    menu.push({
      label: 'Insert Field After',
      action: (e, column) => insertColumn(column, 'after')
    });
    menu.push({ separator: true });
    menu.push({ label: 'Delete Field', action: (e, column) => deleteColumn(column) });
    return menu;
  }

  let showEditFieldModal = $state(false);
  let showChartModal = $state(false);
  let showViewModal = $state(false);
  let initialViewToLoad = $state(null);
  let editingFieldData = $state({ name: '', schema: {} });
  let isAddingNewField = $state(false);
  let newFieldPosition = $state('after');
  let newFieldTargetColumn = $state(null);

  function openFieldEditor(column) {
    const field = column.getField() || '';
    editingFieldData = {
      name: field,
      schema: tableSchema[field] || { type: 'Text', subType: 'Small Text' }
    };
    showEditFieldModal = true;
  }

  function parseDate(str, schema) {
    if (!str || typeof str !== 'string') return null;
    const format = schema?.format || '';
    const subType = schema?.subType || 'Date';

    // Helper to normalize months
    const months = [
      'january',
      'february',
      'march',
      'april',
      'may',
      'june',
      'july',
      'august',
      'september',
      'october',
      'november',
      'december'
    ];
    const getMonthIndex = (m) => months.indexOf(m.toLowerCase());

    // Try standard ISO first
    let d = new Date(str);
    if (!isNaN(d.getTime())) return d;

    // Try format-specific parsing
    if (subType === 'Date') {
      if (format === 'DD/MM/YYYY') {
        const p = str.split('/');
        if (p.length === 3) return new Date(p[2], p[1] - 1, p[0]);
      } else if (format === 'MM/DD/YYYY') {
        const p = str.split('/');
        if (p.length === 3) return new Date(p[2], p[0] - 1, p[1]);
      } else if (format === 'YYYY') {
        if (/^\d{4}$/.test(str)) return new Date(str, 0, 1);
      } else if (format === 'MMMM') {
        const idx = getMonthIndex(str);
        if (idx !== -1) return new Date(new Date().getFullYear(), idx, 1);
      } else if (format === 'MMMM YYYY') {
        const p = str.split(' ');
        const idx = getMonthIndex(p[0]);
        if (p.length === 2 && idx !== -1) return new Date(p[1], idx, 1);
      }
    } else if (subType === 'Time') {
      const is12Hour = format.includes('A') || format.includes('a');
      const ampmMatch = str.match(/(AM|PM)/i);
      const ampm = ampmMatch ? ampmMatch[0].toUpperCase() : null;
      const timeParts = str
        .replace(/(AM|PM)/i, '')
        .trim()
        .split(':');

      if (timeParts.length >= 2) {
        let h = parseInt(timeParts[0]);
        const m = parseInt(timeParts[1]);
        const s = parseInt(timeParts[2] || 0);

        if (is12Hour && ampm) {
          if (ampm === 'PM' && h < 12) h += 12;
          if (ampm === 'AM' && h === 12) h = 0;
        }

        const d = new Date();
        d.setHours(h, m, s);
        return d;
      }
    } else if (subType === 'Date & Time') {
      // Improved split: find first T or space that separates date and time
      let dateStr, timeStr;
      if (str.includes('T')) {
        [dateStr, timeStr] = str.split('T');
      } else {
        // For space separator, we assume the date part is the first block
        // (which works for YYYY-MM-DD, DD/MM/YYYY, MM/DD/YYYY)
        const firstSpace = str.indexOf(' ');
        if (firstSpace !== -1) {
          dateStr = str.substring(0, firstSpace);
          timeStr = str.substring(firstSpace + 1);
        }
      }

      if (dateStr && timeStr) {
        const dateD = parseDate(dateStr, {
          type: 'DateTime',
          subType: 'Date',
          format: format.split(/[T ]/)[0]
        });
        const timeD = parseDate(timeStr, {
          type: 'DateTime',
          subType: 'Time',
          format: format.split(/[T ]/).slice(1).join(' ')
        });

        if (dateD && timeD) {
          dateD.setHours(timeD.getHours(), timeD.getMinutes(), timeD.getSeconds());
          return dateD;
        }
      }
    }

    return null;
  }

  function formatDate(d, schema) {
    if (!(d instanceof Date) || isNaN(d.getTime())) return '';
    const format = schema?.format || '';
    const subType = schema?.subType || 'Date';

    const pad = (n) => String(n).padStart(2, '0');
    const months = [
      'January',
      'February',
      'March',
      'April',
      'May',
      'June',
      'July',
      'August',
      'September',
      'October',
      'November',
      'December'
    ];

    if (subType === 'Date') {
      if (format === 'DD/MM/YYYY')
        return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
      if (format === 'MM/DD/YYYY')
        return `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${d.getFullYear()}`;
      if (format === 'YYYY') return `${d.getFullYear()}`;
      if (format === 'MMMM') return months[d.getMonth()];
      if (format === 'MMMM YYYY') return `${months[d.getMonth()]} ${d.getFullYear()}`;
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    } else if (subType === 'Time') {
      const h = d.getHours();
      const m = pad(d.getMinutes());
      const s = pad(d.getSeconds());
      if (format === 'HH:mm:ss') return `${pad(h)}:${m}:${s}`;
      if (format === 'hh:mm A') {
        const displayH = h % 12 || 12;
        const ampm = h >= 12 ? 'PM' : 'AM';
        return `${pad(displayH)}:${m} ${ampm}`;
      }
      return `${pad(h)}:${m}`;
    } else if (subType === 'Date & Time') {
      const formatParts = format.split(/[T ]/);
      const datePart = formatDate(d, { subType: 'Date', format: formatParts[0] });
      const timePart = formatDate(d, {
        subType: 'Time',
        format: formatParts.slice(1).join(' ') || ''
      });
      if (format.includes('T')) return `${datePart}T${timePart}`;
      return `${datePart} ${timePart}`;
    }
    return '';
  }

  async function handleSaveField(event) {
    const { oldName, newName, schema } = event.detail;
    if (!tablePath) return;

    try {
      if (isAddingNewField) {
        // ... (existing adding new field logic)
        tableSchema[newName] = { ...schema };
        const updatedData = tabulatorInstance.getData();
        updatedData.forEach((row) => {
          row[newName] = '';
        });
        const columns = tabulatorInstance.getColumns();
        let orderedHeaders = columns.filter((c) => c.getField()).map((c) => c.getField());
        if (newFieldTargetColumn) {
          const targetField = newFieldTargetColumn.getField();
          const index = orderedHeaders.indexOf(targetField);
          if (index !== -1) {
            if (newFieldPosition === 'before') orderedHeaders.splice(index, 0, newName);
            else orderedHeaders.splice(index + 1, 0, newName);
          } else orderedHeaders.push(newName);
        } else orderedHeaders.push(newName);
        await saveTableData(tablePath, updatedData, orderedHeaders);
        await saveTableSchema(tablePath, tableSchema);
        await initializeTable(tablePath, null, true);
      } else {
        const oldSchema = tableSchema[oldName];
        const isFormatChange =
          oldSchema &&
          oldSchema.type === 'DateTime' &&
          schema.type === 'DateTime' &&
          oldSchema.format !== schema.format;

        // Handle renaming/updating
        if (oldName !== newName) {
          await renameTableHeader(tablePath, oldName, newName);
          tableSchema[newName] = { ...schema };
          delete tableSchema[oldName];
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
          tableSchema[oldName] = { ...schema };
        }

        // If format changed, attempt to convert data
        if (isFormatChange && tabulatorInstance) {
          const data = tabulatorInstance.getData();
          data.forEach((row) => {
            const val = row[newName || oldName];
            if (val) {
              const dateObj = parseDate(val, oldSchema);
              if (dateObj) {
                row[newName || oldName] = formatDate(dateObj, schema);
              }
            }
          });
          const orderedHeaders = tabulatorInstance
            .getColumns()
            .filter((c) => c.getField())
            .map((c) => c.getField());
          await saveTableData(tablePath, data, orderedHeaders);
        }

        await saveTableSchema(tablePath, tableSchema);
        await initializeTable(tablePath, null, true);
      }
    } catch (error) {
      console.error('Failed to save field:', error);
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
      name: getUniqueColumnName('NewField'),
      schema: { type: 'Text', subType: 'Small Text' }
    };
    showEditFieldModal = true;
  }

  async function pasteColumn(column, position) {
    if (!tableClipboard || tableClipboard.type !== 'column') {
      alert('No field data on clipboard.');
      return;
    }
    const newFieldName = getUniqueColumnName(tableClipboard.header);
    const newColumnDef = {
      title: tableClipboard.header,
      field: newFieldName,
      editor: 'textarea',
      headerFilter: areFiltersVisible ? customHeaderFilterEditor : null
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
      const rowData = row.getData();
      const rowId = rowData.harvey_internal_id;
      await row.delete();
      pushToHistory({ type: 'rowDelete', rowData, rowId });
      await saveTableChanges();
    } catch (err) {
      console.error('Error deleting entry:', err);
    }
  }

  async function insertRow(row, position) {
    const newRowData = {};
    tabulatorInstance.getColumns().forEach((column) => {
      if (column.getField()) {
        newRowData[column.getField()] = '';
      }
    });

    // Calculate a new unique internal ID
    const allData = tabulatorInstance.getData();
    const maxId = allData.reduce((max, r) => Math.max(max, r.harvey_internal_id || 0), -1);
    newRowData.harvey_internal_id = maxId + 1;

    try {
      const addedRow = await tabulatorInstance.addRow(newRowData, position === 'before', row);
      pushToHistory({
        type: 'rowAdd',
        rowData: newRowData,
        rowId: newRowData.harvey_internal_id,
        position,
        relativeTo: row
      });

      // Workaround for suspected backend bug: "dirty" a cell to ensure the new entry is saved.
      const cells = addedRow.getCells();
      if (cells.length > 0 && cells[0].getField() !== 'harvey_internal_id') {
        cells[0].setValue(' ', true); // Set a single space, suppress cellEdited event
      }

      await saveTableChanges();
    } catch (err) {
      console.error('Error inserting entry:', err);
    }
  }

  async function pasteRow(row, position) {
    if (!tableClipboard || tableClipboard.type !== 'row') {
      alert('No entry data on clipboard.');
      return;
    }

    const newRowData = { ...tableClipboard.data };

    // Calculate a new unique internal ID
    const allData = tabulatorInstance.getData();
    const maxId = allData.reduce((max, r) => Math.max(max, r.harvey_internal_id || 0), -1);
    newRowData.harvey_internal_id = maxId + 1;

    try {
      await tabulatorInstance.addRow(newRowData, position === 'before', row);
      pushToHistory({
        type: 'rowAdd',
        rowData: newRowData,
        rowId: newRowData.harvey_internal_id,
        position,
        relativeTo: row
      });
      await saveTableChanges();
    } catch (err) {
      console.error('Error pasting entry:', err);
    }
  }

  function openEditEntryModal(row) {
    editingEntryData = { ...row.getData() };
    editingEntryIndex = row.getData().harvey_internal_id;
    tableColumnsForModal = tabulatorInstance
      .getColumnDefinitions()
      .filter((c) => c.field && c.field !== 'harvey_internal_id');
    showEditEntryModal = true;
  }

  async function handleSaveEntry(event) {
    const { rowData, rowIndex } = event.detail;
    if (tabulatorInstance) {
      const row = tabulatorInstance.getRow(rowIndex);
      if (row) {
        const oldData = { ...row.getData() };
        await row.update(rowData);

        // Track modal edits in history by comparing fields
        Object.keys(rowData).forEach((field) => {
          if (field !== 'harvey_internal_id' && rowData[field] !== oldData[field]) {
            pushToHistory({
              type: 'cellEdit',
              rowId: rowIndex,
              field: field,
              oldValue: oldData[field],
              newValue: rowData[field]
            });
          }
        });

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
      if (definition.field && definition.field !== 'harvey_pseudo_add_col') {
        newSnapshotColumns[definition.field] = {
          order: index,
          visible: column.isVisible(),
          width: column.getWidth()
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
      if (
        relativePath.startsWith(sep) ||
        relativePath.startsWith('/') ||
        relativePath.startsWith('\\')
      ) {
        relativePath = relativePath.substring(1);
      }
    }
    return relativePath.replaceAll('\\', '/');
  }

  function debounce(func, delay) {
    let timeout;
    return function (...args) {
      clearTimeout(timeout);
      timeout = setTimeout(() => func.apply(this, args), delay);
    };
  }

  async function applyHighlightToCells(color, cellsToModify) {
    if (!tabulatorInstance || !cellsToModify || cellsToModify.length === 0) return;

    let currentHighlights = get(project).currentTableHighlights || [];
    const orderedColumns = tabulatorInstance.getColumns().filter((c) => c.getField());

    cellsToModify.forEach((cell) => {
      const row = cell.getRow();
      const rowData = row.getData();
      const rowIndex = rowData.harvey_internal_id;
      const colField = cell.getField();
      const cellKey = `cell-${rowIndex}-${colField}`;

      // Find existing highlight to preserve metadata
      const existingHighlight = currentHighlights.find((h) => h.id === cellKey);

      // Remove existing highlight for this cell
      currentHighlights = currentHighlights.filter((h) => h.id !== cellKey);

      if (color) {
        const cellValue = rowData[colField];
        const text = `Cell [Entry ${rowIndex + 1}, ${colField}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ''}`;

        currentHighlights.push({
          id: cellKey,
          color: color,
          text: text,
          tags: existingHighlight ? existingHighlight.tags : [],
          comments: existingHighlight ? existingHighlight.comments : []
        });
      }
    });

    setTableHighlights(currentHighlights);
    await saveTableHighlights();
  }

  async function applyHighlightToRows(rows, color, preserveMetadata = true) {
    if (!tabulatorInstance || !rows || rows.length === 0) return;

    let currentHighlights = get(project).currentTableHighlights || [];
    const orderedColumns = tabulatorInstance.getColumns();

    // 1. Identify rows to highlight
    const newIndices = rows.map((r) => r.getData().harvey_internal_id);
    const newIndicesSet = new Set(newIndices);

    // 2. Intersection Logic (Precedence)
    // Separate row highlights from cell highlights
    let cellHighlights = currentHighlights.filter((h) => h.id?.startsWith('cell-'));
    let rowHighlights = currentHighlights.filter((h) => h.id?.startsWith('row-') || h.rowIndices);

    let updatedRowHighlights = [];

    rowHighlights.forEach((h) => {
      const hIndices = h.rowIndices || [parseInt(h.id.substring(4), 10)];
      // Remove any rows that are in the new selection
      const remainingIndices = hIndices
        .filter((idx) => !newIndicesSet.has(idx))
        .sort((a, b) => a - b);

      if (remainingIndices.length > 0) {
        // Check for continuity and split if necessary
        let currentGroup = [remainingIndices[0]];
        for (let i = 1; i < remainingIndices.length; i++) {
          if (remainingIndices[i] === remainingIndices[i - 1] + 1) {
            currentGroup.push(remainingIndices[i]);
          } else {
            // Split point: finish current group and start a new one
            updatedRowHighlights.push(
              createGroupedHighlight(currentGroup, h.color, h.tags, h.comments)
            );
            currentGroup = [remainingIndices[i]];
          }
        }
        updatedRowHighlights.push(
          createGroupedHighlight(currentGroup, h.color, h.tags, h.comments)
        );
      }
    });

    // 3. Add the new grouped highlight if color is provided
    if (color) {
      // Find existing metadata if requested
      let existingTags = [];
      let existingComments = [];
      if (preserveMetadata) {
        // If it was a group click, we look for the highlight that contained these rows.
        // We'll use the metadata if ANY of the new indices were part of an existing highlight.
        const sampleIdx = newIndices[0];
        const existing = rowHighlights.find((h) => {
          const idxs = h.rowIndices || [parseInt(h.id.substring(4), 10)];
          return idxs.includes(sampleIdx);
        });
        if (existing) {
          existingTags = existing.tags || [];
          existingComments = existing.comments || [];
        }
      }

      // Generate grouped text
      const textParts = [];
      newIndices
        .sort((a, b) => a - b)
        .forEach((idx) => {
          const rowData = tableData[idx];
          if (rowData) {
            const rowNum = idx + 1;
            const rowTextParts = [rowNum.toString()];
            orderedColumns.forEach((column) => {
              const field = column.getField();
              if (field) {
                const value = rowData[field];
                rowTextParts.push(value !== null && value !== undefined ? value : '');
              }
            });
            textParts.push(rowTextParts.join(' | '));
          }
        });
      const groupedText = textParts.join('\n');

      updatedRowHighlights.push({
        id: `rows-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
        color: color,
        text: groupedText,
        rowIndices: newIndices,
        tags: existingTags,
        comments: existingComments
      });
    }

    // Helper to create grouped highlight object
    function createGroupedHighlight(indices, hColor, hTags = [], hComments = []) {
      const textParts = [];
      indices
        .sort((a, b) => a - b)
        .forEach((idx) => {
          const rowData = tableData[idx];
          if (rowData) {
            const rowNum = idx + 1;
            const rowTextParts = [rowNum.toString()];
            orderedColumns.forEach((column) => {
              const field = column.getField();
              if (field) {
                const value = rowData[field];
                rowTextParts.push(value !== null && value !== undefined ? value : '');
              }
            });
            textParts.push(rowTextParts.join(' | '));
          }
        });
      return {
        id: `rows-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
        color: hColor,
        text: textParts.join('\n'),
        rowIndices: indices,
        tags: hTags,
        comments: hComments
      };
    }

    currentHighlights = [...cellHighlights, ...updatedRowHighlights];

    setTableHighlights(currentHighlights);
    await saveTableHighlights();

    const ranges = tabulatorInstance.getRanges();
    if (ranges) {
      ranges.forEach((range) => range.remove());
    }
  }

  // Custom header filter editor to prevent Enter key propagation
  function customHeaderFilterEditor(cell, onRendered, success, cancel, editorParams) {
    var editor = document.createElement('input');
    editor.setAttribute('type', 'text');
    editor.setAttribute('placeholder', 'Filter...');
    editor.style.width = '100%';
    editor.style.boxSizing = 'border-box';
    editor.style.padding = '4px';
    editor.style.border = '1px solid #ccc';
    editor.style.borderRadius = '3px';

    editor.value = cell.getValue();

    onRendered(function () {
      editor.focus();
      editor.style.css = '100%';
    });

    function successFunc() {
      success(editor.value);
    }

    editor.addEventListener('change', successFunc);
    editor.addEventListener('blur', successFunc);

    // Prevent Enter key from propagating
    editor.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
        successFunc(); // Apply filter on Enter
      }
      if (e.key === 'Escape') {
        cancel();
      }
    });

    return editor;
  }

  async function handleTableHighlightColorChange(color) {
    if (selectedRows && selectedRows.length > 0) {
      // Widget action for many rows (range or group): preserve metadata if it was an existing group click
      await applyHighlightToRows(selectedRows, color, true);
      showTableModifyToolbar = false;
      selectedRows = [];
    } else if (clickedRow) {
      // Widget action for single row: preserve metadata
      await applyHighlightToRows([clickedRow], color, true);
      showTableModifyToolbar = false;
      clickedRow = null;
    }
  }

  async function handleTableTagToggle(tagName) {
    let highlightId = activeHighlightIdForToolbar;
    const rowsToTag =
      selectedRows && selectedRows.length > 0 ? selectedRows : clickedRow ? [clickedRow] : [];

    if (!highlightId && rowsToTag.length > 0) {
      // No existing highlight, create one for the selection (default Yellow for tags)
      await applyHighlightToRows(rowsToTag, '#FFF275', true);

      // After applying, find the new highlight ID from the store
      const firstIdx = rowsToTag[0].getData().harvey_internal_id;
      const highlights = get(project).currentTableHighlights || [];
      const newHighlight = highlights.find((h) => {
        const indices = h.rowIndices || [parseInt(h.id?.substring(4), 10)];
        return !h.id?.startsWith('cell-') && indices.includes(firstIdx);
      });
      if (newHighlight) {
        highlightId = newHighlight.id;
        activeHighlightIdForToolbar = highlightId;
      }
    }

    if (!highlightId) return;

    import('$lib/stores/projectStore.js').then(({ toggleTagInHighlightLocal }) => {
      toggleTagInHighlightLocal(highlightId, tagName, 'table', tablePath);
    });
  }

  async function handleTableHighlightDelete() {
    if (activeHighlightIdForToolbar?.startsWith('cell-')) {
      let currentHighlights = get(project).currentTableHighlights || [];
      currentHighlights = currentHighlights.filter((h) => h.id !== activeHighlightIdForToolbar);
      setTableHighlights(currentHighlights);
      await saveTableHighlights();
      showTableModifyToolbar = false;
      activeHighlightIdForToolbar = null;
      return;
    }

    if (selectedRows && selectedRows.length > 0) {
      // Widget action: delete the whole group/range
      await applyHighlightToRows(selectedRows, null, false);
      showTableModifyToolbar = false;
      selectedRows = [];
      activeHighlightIdForToolbar = null;
    } else if (clickedRow) {
      // Widget action: delete single row
      await applyHighlightToRows([clickedRow], null, false);
      showTableModifyToolbar = false;
      clickedRow = null;
      activeHighlightIdForToolbar = null;
    }
  }

  function checkValidationErrors() {
    if (!tabulatorInstance) return;

    const rows = tabulatorInstance.getRows();
    let foundError = false;
    const newInvalidCells = new Map();

    rows.forEach((row) => {
      const rowIndex = row.getData().harvey_internal_id;
      row.getCells().forEach((cell) => {
        const colField = cell.getField();
        const value = cell.getValue();
        const schema = tableSchema[colField];
        if (schema) {
          const validation = performSoftValidation(value, schema);
          if (!validation.valid) {
            foundError = true;
            newInvalidCells.set(`${rowIndex}-${colField}`, validation.message);
          }
        }
      });
    });

    invalidCells = newInvalidCells;
    tableHasValidationErrors = foundError;

    // Extract filename from path (handle both / and \ separators)
    const filename = tablePath.split(/[\\/]/).pop() || 'Table';

    if (tableHasValidationErrors) {
      project.update((p) => ({ ...p, statusMessage: `${filename} contains validation errors.` }));
    } else if (foundError === false && tabulatorInstance) {
      // Restore default message if errors cleared
      project.update((p) => ({ ...p, statusMessage: `Ready: ${filename}` }));
    }

    // Debounce reformat to prevent cascade hangs
    debounce(() => reformatAllRows(), 50)();
  }

  function performSoftValidation(value, schema) {
    if (!schema) return { valid: true };
    const type = schema.type;
    const subType = schema.subType;

    const isBlank =
      value === null ||
      value === undefined ||
      (typeof value === 'string' && value.trim() === '') ||
      (Array.isArray(value) && value.length === 0);

    if (schema.required && isBlank) {
      return { valid: false, message: 'Field is required' };
    }

    if (!isBlank) {
      if (type === 'Numeric') {
        const num = parseFloat(value);
        if (isNaN(num) || !isFinite(value))
          return { valid: false, message: 'Must be a valid number' };
        if (schema.min !== null && schema.min !== undefined && num < schema.min)
          return { valid: false, message: `Value must be at least ${schema.min}` };
        if (schema.max !== null && schema.max !== undefined && num > schema.max)
          return { valid: false, message: `Value must be at most ${schema.max}` };
      } else if (type === 'Contact' && subType === 'Email') {
        if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value))
          return { valid: false, message: 'Invalid email format' };
      } else if (type === 'Contact' && subType === 'Phone') {
        if (!/^\+?[\d\s-]{7,20}$/.test(value))
          return { valid: false, message: 'Invalid phone format' };
      } else if (type === 'Contact' && subType === 'Hyperlink') {
        const urlRegex = /^(https?:\/\/)?([\da-z\.-]+)\.([a-z\.]{2,6})([\/\w \.-]*)*\/?$/i;
        if (!urlRegex.test(value)) return { valid: false, message: 'Invalid hyperlink format' };
      } else if (type === 'DateTime') {
        if (subType === 'Time') {
          if (schema.format === 'HH:mm' && !/^([01]\d|2[0-3]):([0-5]\d)$/.test(value))
            return { valid: false, message: 'Invalid time format (HH:mm)' };
          if (schema.format === 'HH:mm:ss' && !/^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/.test(value))
            return { valid: false, message: 'Invalid time format (HH:mm:ss)' };
          if (schema.format === 'hh:mm A' && !/^(0[1-9]|1[0-2]):([0-5]\d)\s?(AM|PM)$/i.test(value))
            return { valid: false, message: 'Invalid time format (hh:mm AM/PM)' };
          if (!/^([01]\d|2[0-3]):?([0-5]\d)/.test(value))
            return { valid: false, message: 'Invalid time format' };
        } else if (subType === 'Date') {
          if (
            schema.format === 'YYYY-MM-DD' &&
            !/^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(value)
          )
            return { valid: false, message: 'Invalid date format (YYYY-MM-DD)' };
          if (
            schema.format === 'DD/MM/YYYY' &&
            !/^(0[1-9]|[12]\d|3[01])\/(0[1-9]|1[0-2])\/\d{4}$/.test(value)
          )
            return { valid: false, message: 'Invalid date format (DD/MM/YYYY)' };
          if (
            schema.format === 'MM/DD/YYYY' &&
            !/^(0[1-9]|1[0-2])\/(0[1-9]|[12]\d|3[01])\/\d{4}$/.test(value)
          )
            return { valid: false, message: 'Invalid date format (MM/DD/YYYY)' };
          if (schema.format === 'YYYY' && !/^\d{4}$/.test(value))
            return { valid: false, message: 'Invalid year format (YYYY)' };
          if (
            schema.format === 'MMMM' &&
            ![
              'january',
              'february',
              'march',
              'april',
              'may',
              'june',
              'july',
              'august',
              'september',
              'october',
              'november',
              'december'
            ].includes(value.toLowerCase())
          )
            return { valid: false, message: 'Invalid month name' };
          if (schema.format === 'MMMM YYYY') {
            const parts = value.split(' ');
            const valid =
              parts.length === 2 &&
              [
                'january',
                'february',
                'march',
                'april',
                'may',
                'june',
                'july',
                'august',
                'september',
                'october',
                'november',
                'december'
              ].includes(parts[0].toLowerCase()) &&
              /^\d{4}$/.test(parts[1]);
            if (!valid) return { valid: false, message: 'Invalid Month YYYY format' };
          }

          if (!parseDate(value, schema)) return { valid: false, message: 'Invalid date' };
        } else {
          if (!parseDate(value, schema)) return { valid: false, message: 'Invalid date/time' };
        }
      } else if (type === 'Misc') {
        if (subType === 'Selectbox' && Array.isArray(schema.options)) {
          if (!schema.options.includes(value))
            return { valid: false, message: `Value must be one of: ${schema.options.join(', ')}` };
        } else if (subType === 'Multiselect' && Array.isArray(schema.options)) {
          const vals = Array.isArray(value)
            ? value
            : String(value)
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean);
          const invalidVals = vals.filter((v) => !schema.options.includes(v));
          if (invalidVals.length > 0)
            return { valid: false, message: `Invalid options selected: ${invalidVals.join(', ')}` };
        }
      }
    }
    return { valid: true };
  }

  // Custom soft validator wrapper for Tabulator
  function softValidator(cell, value, parameters) {
    // We always allow editing.
    // Validation highlighting is triggered globally on cellEdited to prevent
    // row reformatting from interrupting Tabulator's active edit/history cycle.
    return true;
  }

  async function getAllProjectAssets() {
    const currentProject = get(project);
    if (!currentProject?.id) return [];
    const { getProjectAssetsForLink } = await import('$lib/services/projectService.js');
    return await getProjectAssetsForLink(currentProject.id);
  }

  // Custom editors for Progress and Rating
  function progressEditor(cell, onRendered, success, cancel, editorParams) {
    const container = document.createElement('div');
    container.className = 'flex items-center w-full h-full px-2 relative group';
    container.style.minHeight = '24px';

    const min = editorParams.min ?? 0;
    const max = editorParams.max ?? 100;
    const initialVal = cell.getValue() ?? min;

    const input = document.createElement('input');
    input.type = 'range';
    input.min = min;
    input.max = max;
    input.step = '1';
    input.value = initialVal;

    const updateUI = (v) => {
      const percentage = ((v - min) / (max - min)) * 100;
      input.style.background = `linear-gradient(to right, #3b82f6 ${percentage}%, #e5e7eb ${percentage}%)`;
    };

    input.className =
      'progress-range w-full h-2 rounded-lg appearance-none cursor-pointer dark:bg-gray-700 flex-grow';
    input.style.width = '100%';
    updateUI(initialVal);

    const textLabel = document.createElement('span');
    textLabel.className =
      'text-xs font-medium text-gray-700 dark:text-gray-300 ml-2 min-w-[2.5rem] text-right shrink-0 pointer-events-none';
    textLabel.textContent = `${initialVal}/${max}`;

    container.appendChild(input);
    container.appendChild(textLabel);

    onRendered(() => {
      input.focus();
    });

    const saveVal = () => {
      success(parseFloat(input.value));
    };

    input.addEventListener('input', () => {
      updateUI(input.value);
      textLabel.textContent = `${input.value}/${max}`;
    });

    input.addEventListener('change', saveVal);
    // Delay the blur save slightly so clicking on the track doesn't immediately close if dragging
    input.addEventListener('blur', () => {
      // Check if we are still interacting with the input (mousedown)
      if (!input.matches(':active')) saveVal();
    });

    // Stop Tabulator from intercepting drag events
    input.addEventListener('mousedown', (e) => e.stopPropagation());
    input.addEventListener('touchstart', (e) => e.stopPropagation());

    return container;
  }

  function ratingEditor(cell, onRendered, success, cancel, editorParams) {
    const container = document.createElement('div');
    container.className = 'flex items-center justify-center w-full h-full gap-0.5 cursor-pointer';

    const maxStars = editorParams.stars || 5;
    let currentValue = cell.getValue() || 0;

    const stars = [];

    const renderStars = (hoverValue) => {
      const val = hoverValue !== null ? hoverValue : currentValue;
      stars.forEach((svg, i) => {
        const filled = i < val;
        svg.innerHTML = `<path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" fill="${filled ? 'currentColor' : 'none'}" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>`;
        if (filled) {
          svg.classList.add('text-yellow-400', 'dark:text-yellow-300');
          svg.classList.remove('text-gray-300', 'dark:text-gray-600');
        } else {
          svg.classList.remove('text-yellow-400', 'dark:text-yellow-300');
          svg.classList.add('text-gray-300', 'dark:text-gray-600');
        }
      });
    };

    for (let i = 0; i < maxStars; i++) {
      const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      svg.setAttribute('viewBox', '0 0 24 24');
      svg.setAttribute('width', '16');
      svg.setAttribute('height', '16');
      svg.className = 'transition-colors duration-150';

      svg.addEventListener('mouseenter', () => renderStars(i + 1));
      svg.addEventListener('click', (e) => {
        e.stopPropagation();
        currentValue = i + 1;
        renderStars(null);
        success(currentValue);
      });

      stars.push(svg);
      container.appendChild(svg);
    }

    container.addEventListener('mouseleave', () => renderStars(null));

    // Initial render
    renderStars(null);

    onRendered(() => {
      // Focus container to allow blur detection
      container.tabIndex = 0;
      container.focus();
    });

    const saveVal = () => {
      success(currentValue);
    };

    container.addEventListener('blur', saveVal);

    // Stop Tabulator from intercepting drag/click events
    container.addEventListener('mousedown', (e) => e.stopPropagation());
    container.addEventListener('touchstart', (e) => e.stopPropagation());

    return container;
  }

  // Custom editors for Date, Time, and DateTime
  function dateEditor(cell, onRendered, success, cancel, editorParams) {
    const container = document.createElement('div');
    container.style.position = 'relative';
    container.style.width = '100%';
    container.style.height = '100%';

    const field = cell.getField();
    const schema = tableSchema[field] || {};
    const displayFormat = schema.format || 'YYYY-MM-DD';
    // Datepicker uses lowercase for format
    const pickerFormat = displayFormat
      .toLowerCase()
      .replace('yyyy', 'yyyy')
      .replace('mm', 'mm')
      .replace('dd', 'dd');

    const editor = document.createElement('input');
    editor.setAttribute('type', 'text');
    editor.setAttribute('autocomplete', 'off');
    editor.setAttribute('autocorrect', 'off');
    editor.setAttribute('autocapitalize', 'off');
    editor.setAttribute('spellcheck', 'false');
    editor.style.padding = '4px';
    editor.style.width = '100%';
    editor.style.height = '100%';
    editor.style.boxSizing = 'border-box';
    editor.style.border = 'none';
    editor.value = cell.getValue() || '';

    container.appendChild(editor);

    let picker;

    onRendered(function () {
      editor.focus();
      picker = new Datepicker(editor, {
        format: pickerFormat,
        autohide: true,
        orientation: 'auto',
        todayBtn: true,
        clearBtn: true,
        container: 'body'
      });
      picker.show(); // Ensure picker appears immediately

      const finish = () => {
        if (picker) {
          const d = picker.getDate();
          let dateStr = editor.value;
          if (d instanceof Date && !isNaN(d)) {
            dateStr = formatDate(d, schema);
          }
          success(dateStr);
          cleanup();
        } else {
          cancel();
        }
      };

      editor.addEventListener('changeDate', (e) => {
        finish(); // Close immediately on date pick
      });

      // Handle outside click specifically for Tabulator inline
      const handleOutside = (e) => {
        const isClickInsideContainer = container.contains(e.target) || container === e.target;

        let isClickInsidePicker = false;
        if (e.target instanceof Element) {
          isClickInsidePicker =
            e.target.closest('.datepicker-dropdown') || e.target.closest('.datepicker');
        }

        if (!isClickInsideContainer && !isClickInsidePicker) {
          finish();
        }
      };

      document.addEventListener('mousedown', handleOutside, true);

      function cleanup() {
        document.removeEventListener('mousedown', handleOutside, true);
        if (picker) {
          picker.hide();
          picker.destroy();
          picker = null;
        }
      }

      editor.onremove = cleanup;
    });

    return container;
  }

  function timeEditor(cell, onRendered, success, cancel, editorParams) {
    const container = document.createElement('div');
    container.style.position = 'relative';
    container.style.width = '100%';
    container.style.height = '100%';

    const field = cell.getField();
    const schema = tableSchema[field] || {};
    const format = schema.format || '';
    const hasSeconds = format.includes(':ss');

    const input = document.createElement('input');
    input.type = 'text';
    input.setAttribute('autocomplete', 'off');
    input.setAttribute('autocorrect', 'off');
    input.setAttribute('autocapitalize', 'off');
    input.setAttribute('spellcheck', 'false');
    input.value = cell.getValue() || '';
    input.style.width = '100%';
    input.style.height = '100%';
    input.style.padding = '4px';
    input.readOnly = true;
    container.appendChild(input);

    onRendered(() => {
      const dropdownEl = document.createElement('div');
      dropdownEl.className = `z-[10000] ${hasSeconds ? 'w-36' : 'w-24'} bg-white dark:bg-gray-800 shadow-xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden`;
      document.body.appendChild(dropdownEl);

      const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
      const minutes = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));
      const seconds = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

      const content = document.createElement('div');
      content.className = 'flex h-48';

      const hCol = document.createElement('div');
      hCol.className = 'flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800';
      hours.forEach((h) => {
        const btn = document.createElement('button');
        btn.className = 'w-full py-1 text-xs hover:bg-blue-100 dark:hover:bg-blue-900/30';
        btn.textContent = h;
        btn.onclick = (e) => {
          e.stopPropagation();
          const d = parseDate(input.value, schema) || new Date();
          d.setHours(parseInt(h));
          input.value = formatDate(d, schema);
          updateSelected();
        };
        hCol.appendChild(btn);
      });

      const mCol = document.createElement('div');
      mCol.className =
        'flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700';
      minutes.forEach((m) => {
        const btn = document.createElement('button');
        btn.className = 'w-full py-1 text-xs hover:bg-blue-100 dark:hover:bg-blue-900/30';
        btn.textContent = m;
        btn.onclick = (e) => {
          e.stopPropagation();
          const d = parseDate(input.value, schema) || new Date();
          d.setMinutes(parseInt(m));
          input.value = formatDate(d, schema);
          if (!hasSeconds) {
            success(input.value);
            cleanup();
          } else {
            updateSelected();
          }
        };
        mCol.appendChild(btn);
      });

      content.appendChild(hCol);
      content.appendChild(mCol);

      if (hasSeconds) {
        const sCol = document.createElement('div');
        sCol.className =
          'flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700';
        seconds.forEach((s) => {
          const btn = document.createElement('button');
          btn.className = 'w-full py-1 text-xs hover:bg-blue-100 dark:hover:bg-blue-900/30';
          btn.textContent = s;
          btn.onclick = (e) => {
            e.stopPropagation();
            const d = parseDate(input.value, schema) || new Date();
            d.setSeconds(parseInt(s));
            input.value = formatDate(d, schema);
            success(input.value);
            cleanup();
          };
          sCol.appendChild(btn);
        });
        content.appendChild(sCol);
      }

      function updateSelected() {
        const d = parseDate(input.value, schema);
        if (!d) return;
        const h = d.getHours().toString().padStart(2, '0');
        const m = d.getMinutes().toString().padStart(2, '0');
        const s = d.getSeconds().toString().padStart(2, '0');

        Array.from(hCol.children).forEach((b) =>
          b.classList.toggle('bg-blue-500', b.textContent === h)
        );
        Array.from(mCol.children).forEach((b) =>
          b.classList.toggle('bg-blue-500', b.textContent === m)
        );
        if (hasSeconds) {
          const sCol = content.children[2];
          Array.from(sCol.children).forEach((b) =>
            b.classList.toggle('bg-blue-500', b.textContent === s)
          );
        }
      }

      dropdownEl.appendChild(content);
      updateSelected();

      const rect = input.getBoundingClientRect();
      dropdownEl.style.position = 'fixed';
      dropdownEl.style.top = `${rect.bottom}px`;
      dropdownEl.style.left = `${rect.left}px`;

      function cleanup() {
        document.removeEventListener('mousedown', handleOutside, true);
        if (dropdownEl.parentNode) dropdownEl.parentNode.removeChild(dropdownEl);
      }

      function handleOutside(e) {
        if (!dropdownEl.contains(e.target) && e.target !== input) {
          success(input.value);
          cleanup();
        }
      }

      document.addEventListener('mousedown', handleOutside, true);
      dropdownEl.addEventListener('mousedown', (e) => e.preventDefault());
    });

    return container;
  }

  function datetimeEditor(cell, onRendered, success, cancel, editorParams) {
    const container = document.createElement('div');
    container.className = 'flex items-center gap-1 w-full h-full p-1';

    const field = cell.getField();
    const schema = tableSchema[field] || {};
    const format = schema.format || '';
    const hasSeconds = format.includes(':ss');

    const val = cell.getValue() || '';
    const dateObj = parseDate(val, schema) || new Date();
    const datePart = formatDate(dateObj, {
      ...schema,
      subType: 'Date',
      format: format.split(/[T ]/)[0]
    });
    const timePart = formatDate(dateObj, {
      ...schema,
      subType: 'Time',
      format: format.split(/[T ]/).slice(1).join(' ') || 'HH:mm'
    });

    const dateInput = document.createElement('input');
    dateInput.type = 'text';
    dateInput.setAttribute('autocomplete', 'off');
    dateInput.setAttribute('autocorrect', 'off');
    dateInput.setAttribute('autocapitalize', 'off');
    dateInput.setAttribute('spellcheck', 'false');
    dateInput.value = datePart;
    dateInput.className = 'flex-1 min-w-0 h-full border-none p-0 text-xs';

    const timeInput = document.createElement('input');
    timeInput.type = 'text';
    timeInput.setAttribute('autocomplete', 'off');
    timeInput.setAttribute('autocorrect', 'off');
    timeInput.setAttribute('autocapitalize', 'off');
    timeInput.setAttribute('spellcheck', 'false');
    timeInput.value = timePart;
    timeInput.className = `${hasSeconds ? 'w-20' : 'w-16'} h-full border-none p-0 text-xs`;
    timeInput.readOnly = true;

    container.appendChild(dateInput);
    container.appendChild(timeInput);

    let datePicker;

    onRendered(() => {
      dateInput.focus();
      datePicker = new Datepicker(dateInput, {
        format: (format.split(/[T ]/)[0] || 'YYYY-MM-DD').toLowerCase(),
        autohide: true,
        container: 'body'
      });
      datePicker.show();

      const finish = () => {
        let dStr = dateInput.value;
        if (datePicker) {
          const d = datePicker.getDate();
          if (d instanceof Date && !isNaN(d)) {
            dStr = formatDate(d, { ...schema, subType: 'Date', format: format.split(/[T ]/)[0] });
          }
        }
        const finalDateObj = parseDate(`${dStr} ${timeInput.value}`, schema);
        success(formatDate(finalDateObj || new Date(), schema));
        cleanup();
      };

      dateInput.addEventListener('changeDate', () => {});

      let timeDropdownEl = null;

      timeInput.onclick = (e) => {
        e.stopPropagation();
        if (timeDropdownEl) {
          cleanupTimeDropdown();
          return;
        }

        timeDropdownEl = document.createElement('div');
        timeDropdownEl.className = `time-dropdown-container z-[10000] ${hasSeconds ? 'w-36' : 'w-24'} bg-white dark:bg-gray-800 shadow-xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden`;
        document.body.appendChild(timeDropdownEl);

        const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
        const minutes = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));
        const seconds = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

        const content = document.createElement('div');
        content.className = 'flex h-48';

        const hCol = document.createElement('div');
        hCol.className = 'flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800';
        hours.forEach((h) => {
          const btn = document.createElement('button');
          btn.className = `w-full py-1 text-sm hover:bg-blue-100 dark:hover:bg-blue-900/30`;
          btn.textContent = h;
          btn.onclick = (ev) => {
            ev.stopPropagation();
            const timeSubFormat = format.split(/[T ]/).slice(1).join(' ') || 'HH:mm';
            const currentT =
              parseDate(timeInput.value, { subType: 'Time', format: timeSubFormat }) || new Date();
            currentT.setHours(parseInt(h));
            timeInput.value = formatDate(currentT, { subType: 'Time', format: timeSubFormat });
            updateSelected();
          };
          hCol.appendChild(btn);
        });

        const mCol = document.createElement('div');
        mCol.className =
          'flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700';
        minutes.forEach((m) => {
          const btn = document.createElement('button');
          btn.className = `w-full py-1 text-sm hover:bg-blue-100 dark:hover:bg-blue-900/30`;
          btn.textContent = m;
          btn.onclick = (ev) => {
            ev.stopPropagation();
            const timeSubFormat = format.split(/[T ]/).slice(1).join(' ') || 'HH:mm';
            const currentT =
              parseDate(timeInput.value, { subType: 'Time', format: timeSubFormat }) || new Date();
            currentT.setMinutes(parseInt(m));
            timeInput.value = formatDate(currentT, { subType: 'Time', format: timeSubFormat });
            if (!hasSeconds) {
              cleanupTimeDropdown();
            } else {
              updateSelected();
            }
          };
          mCol.appendChild(btn);
        });

        content.appendChild(hCol);
        content.appendChild(mCol);

        if (hasSeconds) {
          const sCol = document.createElement('div');
          sCol.className =
            'flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700';
          seconds.forEach((s) => {
            const btn = document.createElement('button');
            btn.className = `w-full py-1 text-sm hover:bg-blue-100 dark:hover:bg-blue-900/30`;
            btn.textContent = s;
            btn.onclick = (ev) => {
              ev.stopPropagation();
              const timeSubFormat = format.split(/[T ]/).slice(1).join(' ') || 'HH:mm';
              const currentT =
                parseDate(timeInput.value, { subType: 'Time', format: timeSubFormat }) ||
                new Date();
              currentT.setSeconds(parseInt(s));
              timeInput.value = formatDate(currentT, { subType: 'Time', format: timeSubFormat });
              cleanupTimeDropdown();
            };
            sCol.appendChild(btn);
          });
          content.appendChild(sCol);
        }

        function updateSelected() {
          const timeSubFormat = format.split(/[T ]/).slice(1).join(' ') || 'HH:mm';
          const d = parseDate(timeInput.value, { subType: 'Time', format: timeSubFormat });
          if (!d) return;
          const h = d.getHours().toString().padStart(2, '0');
          const m = d.getMinutes().toString().padStart(2, '0');
          const s = d.getSeconds().toString().padStart(2, '0');

          Array.from(hCol.children).forEach((b) =>
            b.classList.toggle('bg-blue-500', b.textContent === h)
          );
          Array.from(mCol.children).forEach((b) =>
            b.classList.toggle('bg-blue-500', b.textContent === m)
          );
          if (hasSeconds) {
            const sCol = content.children[2];
            Array.from(sCol.children).forEach((b) =>
              b.classList.toggle('bg-blue-500', b.textContent === s)
            );
          }
        }

        timeDropdownEl.appendChild(content);
        updateSelected();

        const rect = timeInput.getBoundingClientRect();
        timeDropdownEl.style.position = 'fixed';
        timeDropdownEl.style.top = `${rect.bottom}px`;
        timeDropdownEl.style.left = `${rect.left}px`;

        timeDropdownEl.addEventListener('mousedown', (e) => e.preventDefault());
      };

      function cleanupTimeDropdown() {
        if (timeDropdownEl && timeDropdownEl.parentNode) {
          timeDropdownEl.parentNode.removeChild(timeDropdownEl);
        }
        timeDropdownEl = null;
      }

      const handleOutside = (e) => {
        const isClickInsideContainer = container.contains(e.target) || container === e.target;
        let isClickInsidePicker = false;
        let isClickInsideTimeDropdown = false;

        if (e.target instanceof Element) {
          isClickInsidePicker =
            e.target.closest('.datepicker-dropdown') || e.target.closest('.datepicker');
          isClickInsideTimeDropdown = e.target.closest('.time-dropdown-container');
        }

        if (!isClickInsideContainer && !isClickInsidePicker && !isClickInsideTimeDropdown) {
          finish();
        }
      };

      document.addEventListener('mousedown', handleOutside, true);

      function cleanup() {
        document.removeEventListener('mousedown', handleOutside, true);
        if (datePicker) datePicker.destroy();
        cleanupTimeDropdown();
      }
    });

    return container;
  }

  async function generateColumns(data, headers, savedLayoutObj, schema) {
    if (!headers || headers.length === 0) return [{ title: 'No Data', field: 'placeholder' }];

    currentPrimaryField = Object.keys(schema).find((key) => schema[key].primary) || null;

    let dataColumnDefs = headers.map((header) => {
      const colSchema = schema[header] || { type: 'Text', subType: 'Small Text' };
      const isPrimary = colSchema.primary === true;

      const colDef = {
        title: (() => {
          const container = document.createElement('div');
          container.style.width = '100%';
          container.style.height = '100%';
          container.style.display = 'flex';
          container.style.alignItems = 'center';

          mount(TableHeaderIcon, {
            target: container,
            props: {
              colSchema,
              header,
              onResizeStart: (offsetEvent) => {
                handleManualResizeStart(offsetEvent, header);
              }
            }
          });
          return container;
        })(),
        field: header,
        headerFilter: areFiltersVisible ? customHeaderFilterEditor : null,
        headerFilterPlaceholder: 'Filter...',
        headerFilterFunc: function (headerValue, rowValue, rowData, filterParams) {
          if (
            headerValue === null ||
            headerValue === undefined ||
            String(headerValue).trim() === ''
          )
            return true;
          if (rowValue === null || rowValue === undefined) return false;
          return String(rowValue).toLowerCase().includes(String(headerValue).toLowerCase());
        },
        sorter:
          colSchema.type === 'Numeric'
            ? 'number'
            : colSchema.type === 'DateTime'
              ? 'datetime'
              : 'string',
        validator: softValidator,
        headerContextMenu: getColumnContextMenu,
        headerTooltip: colSchema.description ? `${header}: ${colSchema.description}` : header,
        frozen: isPrimary
      };

      // Set editor based on schema
      if (colSchema.type === 'Misc') {
        if (colSchema.subType === 'Checkbox') {
          colDef.editor = false; // Disable editor to prevent tickCross "cross" icons
          colDef.formatter = (cell) => {
            const val = cell.getValue();
            const isChecked = val === true || val === 'true' || val === 1 || val === '1';
            return `<div class="flex items-center justify-center h-full">
                            <input type="checkbox" ${isChecked ? 'checked' : ''} class="h-4 w-4 text-blue-600 border-gray-300 rounded cursor-pointer" onclick="event.preventDefault()" />
                        </div>`;
          };
          colDef.cellClick = function (e, cell) {
            if (!mediaEditorStore.isLexicalEditMode) return;
            // Immediate toggle on single click
            const currentVal = cell.getValue();
            const isCurrentlyChecked =
              currentVal === true ||
              currentVal === 'true' ||
              currentVal === 1 ||
              currentVal === '1';
            const newVal = !isCurrentlyChecked;
            cell.setValue(newVal);

            // Manually trigger history and save
            pushToHistory({
              type: 'cellEdit',
              rowId: cell.getRow().getData().harvey_internal_id,
              field: cell.getField(),
              oldValue: currentVal,
              newValue: newVal
            });
            debouncedSave();
          };
          colDef.hozAlign = 'center';
          colDef.headerHozAlign = 'center';
          colDef.width = 50;
          colDef.resizable = false;
        } else if (colSchema.subType === 'Selectbox' || colSchema.subType === 'Multiselect') {
          colDef.editor = 'list';

          let values = colSchema.options || [];
          if (colSchema.subType === 'Selectbox') {
            // Use an explicit None option instead of the clearable cross, which reverts in Tabulator
            values = [
              { label: '-- None --', value: '' },
              ...values.map((o) => ({ label: o, value: o }))
            ];
          }

          colDef.editorParams = {
            values: values,
            multiselect: colSchema.subType === 'Multiselect'
          };
          colDef.formatter = (cell) => {
            const val = cell.getValue();
            if (colSchema.subType === 'Multiselect' && Array.isArray(val)) {
              return `<div class="flex flex-wrap gap-1">
                                ${val.map((v) => `<span class="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-[10px] font-medium rounded-full border border-blue-200 dark:border-blue-800/50 whitespace-nowrap">${v}</span>`).join('')}
                            </div>`;
            }
            return val || '';
          };
        } else if (colSchema.subType === 'Project Link') {
          colDef.editor = 'list';
          colDef.editorParams = {
            values: [{ label: '-- None --', value: '' }, ...projectAssetOptions]
          };
          colDef.accessorDownload = (value, data, type, params, column) => {
            return value; // Explicitly return the raw file path for export
          };
        }
      } else if (colSchema.type === 'Numeric') {
        colDef.editor = 'number';
        if (colSchema.subType === 'Progress') {
          colDef.editor = progressEditor;
          colDef.formatter = 'progress';
          const min = typeof colSchema.min === 'number' ? colSchema.min : 0;
          const max = typeof colSchema.max === 'number' ? colSchema.max : 100;
          colDef.formatterParams = { min, max };
          colDef.editorParams = { min, max };
          colDef.tooltip = (e, cell) => {
            try {
              const comp =
                cell && typeof cell.getValue === 'function'
                  ? cell
                  : e && typeof e.getValue === 'function'
                    ? e
                    : null;
              if (comp) {
                return `${comp.getValue() || 0} / ${max}`;
              }
            } catch (err) {
              console.error('[TableViewerPanel] Progress tooltip error:', err);
            }
            return '';
          };
        } else if (colSchema.subType === 'Rating') {
          colDef.editor = ratingEditor;
          colDef.formatter = 'star';
          const stars = typeof colSchema.max === 'number' ? colSchema.max : 5;
          colDef.formatterParams = { stars };
          colDef.editorParams = { stars };
          colDef.tooltip = (e, cell) => {
            try {
              const comp =
                cell && typeof cell.getValue === 'function'
                  ? cell
                  : e && typeof e.getValue === 'function'
                    ? e
                    : null;
              if (comp) {
                return `${comp.getValue() || 0} / ${stars} Stars`;
              }
            } catch (err) {
              console.error('[TableViewerPanel] Rating tooltip error:', err);
            }
            return '';
          };
          // Force edit on single click rather than waiting for double-click
          colDef.cellClick = function (e, cell) {
            if (!mediaEditorStore.isLexicalEditMode) return;

            if (!e || !e.target) {
              cell.edit(true);
              return;
            }
            // Optimization: if they click a star in the formatter directly, set it immediately
            const star = e.target.closest('svg');
            if (star) {
              const allStars = Array.from(cell.getElement().querySelectorAll('svg'));
              const index = allStars.indexOf(star);
              if (index !== -1) {
                const newVal = index + 1;
                const oldVal = cell.getValue();
                if (newVal !== oldVal) {
                  cell.setValue(newVal);
                  pushToHistory({
                    type: 'cellEdit',
                    rowId: cell.getRow().getData().harvey_internal_id,
                    field: cell.getField(),
                    oldValue: oldVal,
                    newValue: newVal
                  });
                  debouncedSave();
                }
                return;
              }
            }
            cell.edit(true);
          };
        } else if (colSchema.subType === 'Currency') {
          colDef.formatter = (cell) => {
            const val = cell.getValue();
            if (val === null || val === undefined || val === '') return '';
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
            return val !== null && val !== undefined && val !== '' ? val + '%' : '';
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

        // Add formatter to ensure UI display matches the desired format
        colDef.formatter = (cell) => {
          const val = cell.getValue();
          if (!val) return '';
          const dateObj = parseDate(val, colSchema);
          if (dateObj) {
            return formatDate(dateObj, colSchema);
          }
          return val;
        };
      } else if (colSchema.type === 'Text') {
        if (colSchema.subType === 'Small Text') {
          colDef.editor = 'input';
          colDef.editorParams = {
            elementAttributes: {
              autocomplete: 'off',
              autocorrect: 'off',
              autocapitalize: 'off',
              spellcheck: 'false'
            }
          };
        } else {
          colDef.editor = 'textarea';
          colDef.editorParams = {
            verticalNavigation: 'editor',
            shiftEnterSubmit: false,
            elementAttributes: {
              autocomplete: 'off',
              autocorrect: 'off',
              autocapitalize: 'off',
              spellcheck: 'false'
            }
          };
        }
      } else {
        colDef.editor = 'textarea';
        colDef.editorParams = {
          verticalNavigation: 'editor',
          shiftEnterSubmit: false,
          elementAttributes: {
            autocomplete: 'off',
            autocorrect: 'off',
            autocapitalize: 'off',
            spellcheck: 'false'
          }
        };
      }

      // Apply custom styling/highlighting formatter logic
      const baseFormatter = colDef.formatter;
      colDef.formatter = function (cell, formatterParams, onRendered) {
        const rowData = cell.getRow().getData();
        const rowIndex = rowData.harvey_internal_id;
        const colField = cell.getField();
        const cellKey = `cell-${rowIndex}-${colField}`;
        const cellElement = cell.getElement();
        if (!cellElement) return cell.getValue();

        const cellStyle = tableStyles.cellStyles[cellKey] || {};

        cellElement.style.backgroundColor = cellStyle.color || '';
        if (cellStyle.color) {
          cellElement.classList.add('highlighted-cell');
        } else {
          cellElement.classList.remove('highlighted-cell');
        }

        // Apply text styling if applicable
        if (
          !(
            colSchema.type === 'Misc' &&
            (colSchema.subType === 'Checkbox' ||
              colSchema.subType === 'Rating' ||
              colSchema.subType === 'Progress' ||
              colSchema.subType === 'Selectbox' ||
              colSchema.subType === 'Multiselect')
          )
        ) {
          cellElement.style.fontWeight = cellStyle.bold ? 'bold' : 'normal';
          cellElement.style.fontStyle = cellStyle.italic ? 'italic' : 'normal';
          cellElement.style.textDecoration = cellStyle.underline ? 'underline' : 'none';
          if (cellStyle.textColor) {
            cellElement.style.color = cellStyle.textColor;
          } else {
            cellElement.style.color = '';
          }
        } else {
          cellElement.style.fontWeight = 'normal';
          cellElement.style.fontStyle = 'normal';
          cellElement.style.textDecoration = 'none';
          cellElement.style.color = '';
        }

        // Validation border
        const validationError = invalidCells.get(`${rowIndex}-${colField}`);
        if (validationError) {
          cellElement.classList.add('invalid-cell');
          cellElement.title = validationError;
        } else {
          cellElement.classList.remove('invalid-cell');
          // If this cell previously had an error (we can check if title matches any known error or just clear if no error now)
          // In Tabulator, cell elements are often reused, so clearing title when no error is safer.
          if (!duplicateIds.has(rowIndex) || colField !== currentPrimaryField) {
            cellElement.title = '';
          }
        }

        // Primary duplicate highlighting
        if (colField === currentPrimaryField && duplicateIds.has(rowIndex)) {
          cellElement.classList.add('duplicate-primary-cell');
          cellElement.title = 'Duplicate value in primary field';
        } else if (colField === currentPrimaryField) {
          cellElement.classList.remove('duplicate-primary-cell');
          if (cellElement.title === 'Duplicate value in primary field') {
            cellElement.title = '';
          }
        }

        if (colSchema.type === 'Text' || colSchema.type === 'Misc' || !colSchema.type) {
          // Pre-wrap allows multi-line text, but for specific Misc subtypes like Project Link or Email,
          // we want to ensure they stay on a single line to prevent unnatural row stretching.
          if (
            colSchema.subType === 'Project Link' ||
            colSchema.subType === 'Hyperlink' ||
            colSchema.subType === 'Email'
          ) {
            cellElement.style.whiteSpace = 'nowrap';
          } else {
            cellElement.style.whiteSpace = 'pre-wrap';
          }
        }

        // Call base formatter if it exists
        let value = cell.getValue();
        let isHtmlElement = false;
        if (typeof baseFormatter === 'function') {
          value = baseFormatter.call(this, cell, formatterParams, onRendered);
          isHtmlElement = value instanceof HTMLElement;
        } else if (typeof baseFormatter === 'string') {
          if (baseFormatter === 'tickCross') {
            const icon = value === true || value === 'true' || value === 1 ? '✔' : '✖';
            value = `<div style="text-align:center">${icon}</div>`;
          } else if (baseFormatter === 'money') {
            if (value !== null && value !== undefined && value !== '') {
              value = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(
                value
              );
            }
          } else if (baseFormatter === 'progress' || baseFormatter === 'star') {
            // Use Tabulator's built-in formatters explicitly for these advanced types
            // Tabulator 6 exposes formatters via moduleBindings or getFormatter()
            const formatModule = cell.getTable().modules.format;
            const builtInFormatter =
              Tabulator.moduleBindings?.format?.formatters?.[baseFormatter] ||
              (formatModule && typeof formatModule.getFormatter === 'function'
                ? formatModule.getFormatter(baseFormatter)
                : null);

            if (builtInFormatter) {
              value = builtInFormatter.call(
                formatModule || this,
                cell,
                formatterParams,
                onRendered
              );
              isHtmlElement = value instanceof HTMLElement;
            } else {
              // Fallback if Tabulator's internal API is obfuscated in this version
              return cell.getValue();
            }
          }
        }

        if (isHtmlElement) {
          return value; // Do not attempt to string-replace on DOM nodes
        }

        const term = searchTerm.trim();
        let outputValue = value;
        if (term && value !== null && value !== undefined && typeof value === 'string') {
          const escapedTerm = term.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
          const regex = new RegExp(`(${escapedTerm})`, 'gi');
          outputValue = String(value).replace(
            regex,
            '<span class="search-match-highlight">$1</span>'
          );
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

          let displayLabel = outputValue;
          if (value && typeof value === 'string') {
            const normalizedSearchPath = value.replace(/\\/g, '/');
            const matchedAsset = projectAssetOptions.find((a) => {
              const aNormalized = a.value.replace(/\\/g, '/');
              if (aNormalized === normalizedSearchPath) return true;

              const proj = get(project);
              if (proj?.baseDirectory) {
                const baseDir = proj.baseDirectory.replace(/\\/g, '/');
                const aAbsolute = `${baseDir}/${aNormalized.replace(/^\/+/, '')}`;
                const searchAbsolute =
                  normalizedSearchPath.startsWith('/') || normalizedSearchPath.includes(':')
                    ? normalizedSearchPath
                    : `${baseDir}/${normalizedSearchPath.replace(/^\/+/, '')}`;

                return aAbsolute === searchAbsolute;
              }
              return false;
            });

            if (matchedAsset && matchedAsset.label) {
              displayLabel = matchedAsset.label;
              // Re-apply search highlighting to the label if there's a search term
              if (term) {
                const escapedTerm = term.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
                const regex = new RegExp(`(${escapedTerm})`, 'gi');
                displayLabel = String(displayLabel).replace(
                  regex,
                  '<span class="search-match-highlight">$1</span>'
                );
              }
            }
          }

          return `
                        <div class="flex items-center justify-between w-full h-full">
                            <span class="truncate mr-2">${displayLabel}</span>
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
      const primaryIdx = dataColumnDefs.findIndex((c) => c.field === currentPrimaryField);
      if (primaryIdx > 0) {
        const [primaryCol] = dataColumnDefs.splice(primaryIdx, 1);
        dataColumnDefs.unshift(primaryCol);
      }
    }

    if (savedLayoutObj?.columns) {
      dataColumnDefs.sort((a, b) => {
        if (a.frozen) return -1;
        if (b.frozen) return 1;
        if (a.field === 'harvey_pseudo_add_col') return 1;
        if (b.field === 'harvey_pseudo_add_col') return -1;
        return (
          (savedLayoutObj.columns[a.field]?.order ?? Infinity) -
          (savedLayoutObj.columns[b.field]?.order ?? Infinity)
        );
      });
    }

    // Append the pseudo-add-field column at the very end
    if (mediaEditorStore.isLexicalEditMode && !isViewingDocument) {
      dataColumnDefs.push({
        title: `
          <div class="flex items-center justify-center w-full h-[52px] group cursor-pointer" title="Add New Field">
            <div class="flex items-center justify-center w-[32px] h-[32px] rounded-md bg-blue-50/30 dark:bg-blue-900/20 group-hover:bg-blue-100/50 dark:group-hover:bg-blue-900/40 transition-all text-blue-500">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
            </div>
          </div>
        `,
        field: 'harvey_pseudo_add_col',
        width: 55,
        minWidth: 55,
        maxWidth: 55,
        resizable: false,
        editable: false,
        selectable: false,
        headerSort: false,
        tooltip: false,
        cssClass: 'harvey-pseudo-col px-0!',
        cellClick: (e, cell) => {
          e.preventDefault();
          e.stopPropagation();
          const cols = tabulatorInstance.getColumns();
          const lastRealCol = cols.filter(c => c.getField() !== 'harvey_pseudo_add_col').pop();
          insertColumn(lastRealCol, 'after');
        },
        headerClick: (e) => {
          e.preventDefault();
          const cols = tabulatorInstance.getColumns();
          // Insert relative to the last real data column
          const lastRealCol = cols.filter(c => c.getField() !== 'harvey_pseudo_add_col').pop();
          insertColumn(lastRealCol, 'after');
        },
        cellMouseEnter: (e, cell) => {
          tabulatorInstance.element.classList.add('harvey-add-field-hovering');
        },
        cellMouseLeave: (e, cell) => {
          tabulatorInstance.element.classList.remove('harvey-add-field-hovering');
        },
        headerMouseEnter: (e, col) => {
          tabulatorInstance.element.classList.add('harvey-add-field-hovering');
        },
        headerMouseLeave: (e, col) => {
          tabulatorInstance.element.classList.remove('harvey-add-field-hovering');
        }
      });
    }

    return dataColumnDefs;
  }



  export function openChart(chart) {
    if (!tabulatorInstance) return;
    tableColumnsForModal = tabulatorInstance
      .getColumnDefinitions()
      .filter((c) => c.field && c.field !== 'harvey_internal_id');
    initialChartToLoad = null;
    showChartModal = true;
    initialChartToLoad = chart;
    dispatch('requestviewchange', { type: 'chart_opened', item: chart });
  }

  let currentActiveView = $state(null);
  let baseTableColumns = $state([]); // Store base columns when applying a view
  let pivotDerivedSchema = $state({});
  let generatedPivotResult = $state({
    colHeaders: [],
    rows: [],
    rowFieldsCount: 0,
    colLeavesCount: 0,
    rowFields: [],
    colLeaves: []
  });
  let computedSchema = $derived({ ...tableSchema, ...pivotDerivedSchema });

  const debouncedLexicalSave = debounce(async (docPath, jsonString) => {
    if (!docPath || !jsonString) return;
    try {
      const projectStoreState = get(project);
      let absolutePath = docPath;
      if (
        !absolutePath.startsWith('/') &&
        !absolutePath.startsWith('\\') &&
        !absolutePath.includes(':') &&
        projectStoreState?.baseDirectory
      ) {
        absolutePath = `${projectStoreState.baseDirectory}/${docPath.replace(/^\/+/, '')}`;
      }
      await invoke('save_note_json', { targetPath: absolutePath, jsonContent: jsonString });
    } catch (e) {
      console.error('Failed to autosave lexical document:', e);
    }
  }, 750);

  const debouncedLexicalHighlightsSave = debounce(async (docPath, highlights) => {
    if (!docPath || !highlights || !Array.isArray(highlights)) return;
    try {
      const projectStoreState = get(project);
      let absolutePath = docPath;
      if (
        !absolutePath.startsWith('/') &&
        !absolutePath.startsWith('\\') &&
        !absolutePath.includes(':') &&
        projectStoreState?.baseDirectory
      ) {
        absolutePath = `${projectStoreState.baseDirectory}/${docPath.replace(/^\/+/, '')}`;
      }

      await invoke('save_lexical_highlights', {
        args: {
          projectId: projectStoreState.id,
          documentPath: absolutePath,
          highlightsJson: JSON.stringify(highlights)
        }
      });
    } catch (e) {
      console.error('Failed to autosave lexical highlights batch:', e);
    }
  }, 750);

  function handleLexicalDocumentChange(event) {
    const { jsonString } = event.detail;
    if (currentActiveDocumentPath && jsonString) {
      debouncedLexicalSave(currentActiveDocumentPath, jsonString);
    }
  }

  function handleLexicalHighlightsChange(event) {
    const { highlights } = event.detail;
    if (currentActiveDocumentPath && highlights) {
      setDocumentHighlights(highlights, false); // Update global store immediately so HighlightsPanel sees it, but don't mark global file dirty
      debouncedLexicalHighlightsSave(currentActiveDocumentPath, highlights);
    }
  }

  // Reactive watcher to capture tags/comments added via the HighlightsPanel sidebar
  // when a survey document is actively being viewed in the table viewer.
  $effect(() => {
    if (isViewingDocument && currentActiveDocumentPath && $project.currentDocumentHighlights) {
      // Debounce to prevent duplicate writes alongside handleLexicalHighlightsChange
      debouncedLexicalHighlightsSave(currentActiveDocumentPath, $project.currentDocumentHighlights);
    }
  });

  // Reactive watcher to capture tags/comments added to base tables
  // via floating toolbar or HighlightsPanel sidebar
  $effect(() => {
    if (!isViewingDocument && tableReady && $project.isTableHighlightsDirty) {
      saveTableHighlights();
    }
  });

  export async function openLexicalDocument(docPath) {
    if (!docPath) return;
    try {
      const projectStoreState = get(project);
      let absolutePath = docPath;
      if (
        !absolutePath.startsWith('/') &&
        !absolutePath.startsWith('\\') &&
        !absolutePath.includes(':') &&
        projectStoreState?.baseDirectory
      ) {
        absolutePath = `${projectStoreState.baseDirectory}/${docPath.replace(/^\/+/, '')}`;
      }

      const content = await invoke('load_note_json', {
        filePath: absolutePath
      });

      // Load highlights
      let loadedHighlights = [];
      try {
        const hData = await invoke('load_lexical_highlights', {
          args: { projectId: projectStoreState.id, documentPath: absolutePath }
        });
        if (hData) {
          loadedHighlights = JSON.parse(hData);
        }
      } catch (hErr) {
        console.error('Failed to load highlights for lexical document:', hErr);
      }

      if (content) {
        currentActiveDocumentJson = content;
        currentActiveDocumentHighlights = loadedHighlights;
        setDocumentHighlights(loadedHighlights, false); // Immediately push to store for HighlightsPanel without marking base file dirty
        isViewingDocument = true;
        currentActiveDocumentPath = docPath;
        activeSubItemPath = docPath;
        activeSubItemType = 'doc';
        dispatch('requestviewchange', { type: 'chart_opened', item: docPath });
      } else {
        console.error('Document content was empty.');
      }
    } catch (e) {
      console.error('Failed to open document:', e);
    }
  }

  export async function openView(view) {
    if (!view) return;
    try {
      const config = JSON.parse(view.config_json);
      if (currentActiveView || isViewingDocument) {
        // Must ensure we start from a clean slate so views don't stack their transformations
        await returnToBaseTable();
      }
      // Wait for Tabulator to be fully initialized and ready
      if (tabulatorInstance && !tableReady) {
        await new Promise((resolve) => {
          tabulatorInstance.on('tableBuilt', resolve);
        });
      }
      applyViewToTable(view.view_name, view.view_type, config);
      activeSubItemPath = view;
      activeSubItemType = 'view';
      dispatch('requestviewchange', { type: 'view_changed', item: view });
    } catch (e) {
      console.error('Failed to parse view config on open:', e);
      notificationStore.add('Failed to open view', 'error');
    }
  }

  export function configureView(view) {
    if (!tabulatorInstance) return;
    tableColumnsForModal = tabulatorInstance
      .getColumnDefinitions()
      .filter((c) => c.field && c.field !== 'harvey_internal_id');
    initialViewToLoad = null;
    showViewModal = true;
    initialViewToLoad = view;
  }

  export function handleDeletedView(deletedViewName) {
    if (currentActiveView && currentActiveView === deletedViewName) {
      returnToBaseTable();
    }
  }

  function applyViewToTable(viewName, viewType, config) {
    if (!tabulatorInstance) return;

    // Store base columns if not already stored
    if (!currentActiveView) {
      baseTableColumns = tabulatorInstance.getColumnDefinitions();
    }

    currentActiveView = viewName;
    currentActiveViewType = viewType;

    if (viewType === 'partial') {
      tabulatorInstance.clearFilter();
      if (config.selectedColumns && config.selectedColumns.length > 0) {
        const allCols = tabulatorInstance.getColumns();
        allCols.forEach((col) => {
          const field = col.getField();
          if (field && field !== 'harvey_internal_id') {
            if (config.selectedColumns.includes(field)) {
              col.show();
            } else {
              col.hide();
            }
          }
        });
      }
      if (config.filterField && config.filterValue) {
        tabulatorInstance.setFilter(
          config.filterField,
          config.filterOperator || 'like',
          config.filterValue
        );
      }
    } else if (viewType === 'pivot') {
      const { rowField, colField, rowFields, colFields, valueField, aggregation, valueFields } =
        config;

      let actualRowFields = rowFields || (rowField ? [rowField] : []);
      let actualColFields = colFields || (colField ? [colField] : []);
      let actualValueFields = valueFields || [];
      if (actualValueFields.length === 0 && valueField) {
        actualValueFields.push({ field: valueField, aggregation: aggregation || 'Sum' });
      }

      if (actualRowFields.length === 0 || actualValueFields.length === 0) return;

      let rowTree = {};
      let allColLeaves = new Set();

      tableData.forEach((row) => {
        let currentLevel = rowTree;
        for (let i = 0; i < actualRowFields.length; i++) {
          const field = actualRowFields[i];
          const val = String(row[field] || '(Blank)');
          if (!currentLevel[val]) {
            currentLevel[val] = {
              _val: val,
              _field: field,
              _children: i === actualRowFields.length - 1 ? null : {},
              _data: []
            };
          }
          currentLevel = currentLevel[val]._children || currentLevel[val];
          if (i === actualRowFields.length - 1) {
            currentLevel._data.push(row);
          }
        }

        let cVals = actualColFields ? actualColFields.map((f) => String(row[f] || '(Blank)')) : [];
        actualValueFields.forEach((vf) => {
          const keyParts = [...cVals, `${vf.field} (${vf.aggregation})`];
          allColLeaves.add(JSON.stringify(keyParts));
        });
      });

      const colLeaves = Array.from(allColLeaves)
        .map((c) => JSON.parse(c))
        .sort();

      function aggregateRows(rows, vfParts, colFieldsArray) {
        const matchColParts = vfParts.slice(0, -1);
        const vfPart = vfParts[vfParts.length - 1];
        const match = vfPart.match(/(.+) \((Sum|Count|Average|Min|Max)\)$/);
        if (!match) return null;
        const vField = match[1];
        const aggType = match[2];

        let filteredRows = rows;
        if (colFieldsArray && colFieldsArray.length > 0) {
          filteredRows = rows.filter((r) => {
            return colFieldsArray.every((cf, i) => String(r[cf] || '(Blank)') === matchColParts[i]);
          });
        }

        if (filteredRows.length === 0) return null;

        let vals = filteredRows.map((r) => parseFloat(r[vField]) || 0);
        if (aggType === 'Sum') return vals.reduce((a, b) => a + b, 0);
        if (aggType === 'Count') return vals.length;
        if (aggType === 'Average') return vals.reduce((a, b) => a + b, 0) / vals.length;
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
              const aggVal = aggregateRows(node._data, colLeafParts, actualColFields);
              rowData[`val_${i}`] =
                aggVal !== null
                  ? Number.isInteger(aggVal)
                    ? aggVal
                    : parseFloat(aggVal.toFixed(2))
                  : '';
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

      const colDepth = (actualColFields ? actualColFields.length : 0) + 1;
      let colHeaders = Array.from({ length: colDepth }, () => []);

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

      // Pivot views are natively rendered, so store them in reactive vars rather than Tabulator
      generatedPivotResult = {
        colHeaders,
        rows: childRows,
        rowFieldsCount: actualRowFields.length,
        colLeavesCount: colLeaves.length,
        colLeaves: colLeaves, // We need this for export mapping
        rowFields: actualRowFields // Keep track of the resolved row fields array
      };

      // Generate basic schema for export
      let newPivotSchema = {};
      colLeaves.forEach((leafParts, idx) => {
        newPivotSchema[`val_${idx}`] = { type: 'Numeric', subType: 'Decimal' };
      });
      pivotDerivedSchema = newPivotSchema;
    }
  }

  async function returnToBaseTable() {
    if (!tabulatorInstance && !isViewingDocument) return;
    currentActiveView = null;
    currentActiveViewType = null;
    pivotDerivedSchema = {};
    isViewingDocument = false;
    currentActiveDocumentPath = null;
    activeSubItemPath = null;
    activeSubItemType = null;

    dispatch('requestviewchange', { type: 'reset_base' });

    // The safest and most robust way to return to the base table and avoid Tabulator
    // duplicating rowHeader columns (or other formatter issues) is to re-initialize it.
    if (tabulatorInstance) {
      tabulatorInstance.destroy();
      tabulatorInstance = null;
    }
    tableReady = false;

    await initializeTable(tablePath, hasHeaders, true);
  }

  async function handleViewSaved(event) {
    dispatch('requestviewchange', { type: 'refresh_metadata' });

    await loadTableViews(tablePath); // Refresh the list of available views for modals

    // We only dynamically update the table if the view being autosaved is currently the active view.
    const { viewName, viewType, config, isAutoSave } = event.detail;
    if (!tabulatorInstance || !isAutoSave || currentActiveView !== viewName) return;

    // Perform in-place update if possible
    applyViewToTable(viewName, viewType, config);
    activeSubItemPath = { view_name: viewName, view_type: viewType };
    activeSubItemType = 'view';
    dispatch('requestviewchange', {
      type: 'view_changed',
      item: { view_name: viewName, view_type: viewType }
    });
  }

  async function handleViewApplied(event) {
    dispatch('requestviewchange', { type: 'refresh_metadata' });

    await loadTableViews(tablePath); // Ensure available views are up-to-date

    const { viewName, viewType, config } = event.detail;
    if (!tabulatorInstance) return;

    // Explicitly switching to this view. Start from clean slate if another view was active.
    if ((currentActiveView && currentActiveView !== viewName) || isViewingDocument) {
      await returnToBaseTable();
    }

    applyViewToTable(viewName, viewType, config);
    activeSubItemPath = { view_name: viewName, view_type: viewType };
    activeSubItemType = 'view';
    dispatch('requestviewchange', {
      type: 'view_changed',
      item: { view_name: viewName, view_type: viewType }
    });
  }

  export async function getExportData() {
    if (currentActiveViewType === 'pivot') {
      // For export, we flatten the pivot data into a 2D array of rows
      let rowFields = generatedPivotResult.rowFields || [];

      const headers = [
        ...rowFields,
        ...generatedPivotResult.colLeaves.map((parts) => parts.join(' '))
      ];
      const data = generatedPivotResult.rows.map((row) => {
        let out = {};
        // We map header names directly for the exporter
        row.headers.forEach((h, i) => {
          if (rowFields[i]) out[rowFields[i]] = h.val;
        });
        for (let i = 0; i < generatedPivotResult.colLeavesCount; i++) {
          out[generatedPivotResult.colLeaves[i].join(' ')] =
            row.data[`val_${i}`] !== undefined ? row.data[`val_${i}`] : '';
        }
        return out;
      });

      return {
        data,
        headers,
        styles: {}
      };
    }

    if (!tabulatorInstance) return null;

    const data = tabulatorInstance.getData();
    const headers = tabulatorInstance
      .getColumns()
      .filter((c) => c.getField())
      .map((c) => c.getField());

    // Deep copy data to avoid mutating the original
    const formattedData = JSON.parse(JSON.stringify(data));

    formattedData.forEach((row) => {
      for (const field in tableSchema) {
        const schema = tableSchema[field];
        if (schema.type === 'DateTime') {
          const val = row[field];
          if (val) {
            const dateObj = parseDate(val, schema);
            if (dateObj) {
              row[field] = formatDate(dateObj, schema);
            }
          }
        }
      }
    });

    // Extract style information for export
    const stylesMap = {};
    if (tableStyles) {
      // Re-map internal id -> string index based on current order if needed,
      // but our backend export uses the raw data array which might not match `harvey_internal_id` 1:1 if sorted.
      // Let's attach styles directly to the formatted data to ensure it matches.

      formattedData.forEach((row, rowIndex) => {
        const internalId = row.harvey_internal_id;
        headers.forEach((field, colIndex) => {
          const cellKey = `cell-${internalId}-${field}`;
          let color = null;
          let textColor = null;
          let bold = false;
          let italic = false;
          let underline = false;

          // Row highlights
          if (tableStyles.rowStyles && tableStyles.rowStyles[internalId]) {
            color = tableStyles.rowStyles[internalId];
          }

          // Cell highlights (override row color)
          if (tableStyles.cellStyles && tableStyles.cellStyles[cellKey]) {
            const s = tableStyles.cellStyles[cellKey];
            if (s.color) color = s.color;
            if (s.textColor) textColor = s.textColor;
            if (s.bold) bold = !!s.bold;
            if (s.italic) italic = !!s.italic;
            if (s.underline) underline = !!s.underline;
          }

          if (color || textColor || bold || italic || underline) {
            const coordKey = `${rowIndex},${colIndex}`;
            stylesMap[coordKey] = {
              color,
              textColor,
              bold,
              italic,
              underline
            };
          }
        });
      });
    }

    console.log(
      '[TableViewerPanel] Exporting data. stylesMap keys:',
      Object.keys(stylesMap).length
    );

    console.log('[TableViewerPanel] Export styles map:', stylesMap);
    return { data: formattedData, headers, styles: stylesMap };
  }

  async function loadTableViews(pathForTable) {
    try {
      const projectStoreState = get(project);
      if (!projectStoreState.id) return;
      const baseDir = projectStoreState.baseDirectory;
      let relative = pathForTable.startsWith(baseDir)
        ? pathForTable.substring(baseDir.length)
        : pathForTable;
      const normalizedTablePath = relative.replace(/\\/g, '/').replace(/^\//, '');

      availableViews = await invoke('load_table_views_command', {
        projectId: projectStoreState.id,
        tablePath: normalizedTablePath
      });
    } catch (error) {
      console.error('Failed to load table views:', error);
    }
  }

  async function initializeTable(pathForTable, newHasHeaders = null, force = false) {
    if (newHasHeaders !== null) hasHeaders = newHasHeaders;
    if (!pathForTable || !tableContainer) return;
    
    // Safety check: Don't reload if already loading this path, OR if already loaded (unless forced)
    if (isLoading) return;
    if (!force && tabulatorInstance && currentLoadedPath === pathForTable) return;

    currentLoadedPath = pathForTable;
    isLoading = true;
    error = null;
    tableData = [];

    if (tabulatorInstance) {
      tabulatorInstance.destroy();
      tabulatorInstance = null;
    }

    try {
      await loadTableViews(pathForTable);
      // 1. Load Table Data
      const response = await loadTableData(pathForTable, hasHeaders);
      tableData = response.data;
      tableData.forEach((d, i) => (d.harvey_internal_id = i));
      const tableHeaders = response.headers;

      // 2. Load Schema
      tableSchema = (await loadTableSchema(pathForTable)) || {};

      // 3. Transform Multiselect to arrays for UI consistency
      tableData.forEach((row) => {
        for (const field in tableSchema) {
          if (tableSchema[field].type === 'Misc' && tableSchema[field].subType === 'Multiselect') {
            if (typeof row[field] === 'string') {
              row[field] = row[field]
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean);
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
        } else if (
          typeof loadedHighlightsOrStyles === 'object' &&
          loadedHighlightsOrStyles.rowStyles
        ) {
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
                Object.keys(rowData).forEach((key) => {
                  if (key !== 'harvey_internal_id') {
                    const value = rowData[key];
                    textParts.push(value !== null && value !== undefined ? value : '');
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
                  const text = `Cell [Entry ${rowIndex + 1}, ${colField}]: ${cellValue !== null && cellValue !== undefined ? cellValue : ''}`;
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
      project.update((p) => ({ ...p, statusMessage: `Ready: ${filename}` }));

      await tick();
      if (!tableContainer) {
        error = 'Failed to initialize table viewer: container lost.';
        isLoading = false;
        return;
      }
      const projectBaseDir = get(project)?.baseDirectory;
      if (!projectBaseDir) {
        error = 'Project configuration error: base directory missing.';
        isLoading = false;
        return;
      }
      const relativeTablePath = getRelativePath(pathForTable, projectBaseDir);
      if (!relativeTablePath) {
        error = 'Error determining asset relative path.';
        isLoading = false;
        return;
      }
      let savedLayout = await loadTableLayoutPrefs(relativeTablePath).catch((e) =>
        console.error(`Error loading layout for ${relativeTablePath}:`, e)
      );

      // Reset duplicates state for new table
      duplicateIds = new Set();
      projectAssetOptions = await getAllProjectAssets();
      const generatedColumns = await generateColumns(
        tableData,
        tableHeaders,
            savedLayout,
            tableSchema
      );

      tabulatorInstance = new Tabulator(tableContainer, {
        data: [...JSON.parse(JSON.stringify(tableData)), { harvey_internal_id: 'harvey_pseudo_add_row' }],
        reactiveData: false,
        index: 'harvey_internal_id',
        clipboard: 'copy',
        rowFormatter: (row) => {
          const data = row.getData();
          const rowElement = row.getElement();

          // 1. Handle Pseudo-Add-Entry Row
          if (data.harvey_internal_id === 'harvey_pseudo_add_row') {
            rowElement.classList.add('harvey-pseudo-row');
            rowElement.innerHTML = `
              <div class="w-full flex items-center group cursor-pointer hover:bg-blue-50/50 dark:hover:bg-blue-900/20 transition-all border-t border-gray-200 dark:border-gray-700" style="height: 36px;">
                <div class="w-[55px] min-w-[55px] h-full flex items-center justify-center bg-blue-50/30 dark:bg-blue-900/20 border-r border-gray-200 dark:border-gray-700">
                  <div class="flex items-center justify-center w-[24px] h-[24px] rounded-md bg-blue-50/50 dark:bg-blue-900/40 text-blue-500">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                  </div>
                </div>
                <div class="flex-1 h-full px-4 flex items-center">
                  <div class="w-full border border-blue-400/30 dark:border-blue-600/30 rounded-md py-0.5 flex items-center justify-center text-blue-500 font-medium bg-blue-50/20 dark:bg-blue-900/5 group-hover:bg-blue-100/30 dark:group-hover:bg-blue-900/10 transition-all text-xs">
                    Add New Entry
                  </div>
                </div>
              </div>
            `;
            rowElement.onclick = (e) => {
              e.preventDefault();
              const rows = tabulatorInstance.getRows();
              const lastRealRow = rows.filter(r => r.getData().harvey_internal_id !== 'harvey_pseudo_add_row').pop();
              insertRow(lastRealRow, 'after');
            };
            return; // Don't apply regular row styles to the pseudo-row
          }

          // 2. Handle Regular Row Styling (Highlights)
          const rowIndex = data.harvey_internal_id;
          const rowColor = tableStyles.rowStyles[rowIndex];
          rowElement.style.backgroundColor = rowColor || '';
          if (rowColor) {
            rowElement.classList.add('highlighted-row');
          } else {
            rowElement.classList.remove('highlighted-row');
          }
        },
        renderComplete: () => {
          setTimeout(updateTableDimensions, 100);
        },
        tableBuilt: () => {
          setTimeout(updateTableDimensions, 100);
        },
        langs: {
          "default": {
            "columns": generatedColumns
          }
        },
        layout: 'fitColumns',
        columns: generatedColumns,
        nestedFieldSeparator: false,
        height: '100%',
        placeholder: 'No Data Available',
        selectableRange: true,
        selectableRangeColumns: true,
        selectableRangeRows: true,

        history: true,
        editTriggerEvent: 'dblclick',
        rangeSelected: (range) => {
          const rows = range.getRows();
          if (rows && rows.length > 1) {
            selectedRows = rows;
            clickedRow = null;
            lastRangeSelectedTime = Date.now();

            const lastRow = rows[rows.length - 1];
            const lastRowEl = lastRow.getElement();
            if (lastRowEl) {
              const rect = lastRowEl.getBoundingClientRect();
              const showBelow = rect.top < 150;

              tableModifyToolbarPosition = {
                top: showBelow ? rect.bottom + 5 : rect.top - 45,
                left: Math.min(
                  window.innerWidth - 130,
                  Math.max(10, rect.left + rect.width / 2 - 60)
                )
              };
              showTableModifyToolbar = true;
            }
          } else if (rows && rows.length === 1) {
            selectedRows = rows;
          } else {
            selectedRows = [];
          }
        },

        movableColumns: true,
        resizableColumnFit: true,
        rowContextMenu: (e, row) => {
          const isEditMode = mediaEditorStore.isLexicalEditMode;

          const ranges = tabulatorInstance.getRanges();
          let selectedRowsForMenu = [row];

          if (ranges.length > 0) {
            const activeRange = ranges[0];
            const rangeRows = activeRange.getRows();
            if (rangeRows.some((r) => r.getIndex() === row.getIndex())) {
              selectedRowsForMenu = rangeRows;
            }
          }

          const highlightAction = (color) => {
            // Action from right-click: overwrite metadata (Intersection logic)
            applyHighlightToRows(selectedRowsForMenu, color, false);
          };

          const highlightColorOptions = highlightOptions.map((option) => ({
            label: `<span style='display:inline-block; width:15px; height:15px; background-color:${option.value}; margin-right: 8px; vertical-align: middle;'></span>${option.label}`,
            action: () => highlightAction(option.value)
          }));

          if (currentActiveViewType === 'pivot' || !isEditMode) {
            return [
              { label: 'Copy Entry', action: (e, row) => copyRow(row) },
              { separator: true },
              { label: 'Highlight Entry', menu: highlightColorOptions },
              { label: 'Clear Entry Highlight', action: () => highlightAction(null) }
            ];
          }

          const menu = [
            { label: 'Edit Entry', action: (e, row) => openEditEntryModal(row) },
            { separator: true },
            { label: 'Cut Entry', action: (e, row) => cutRow(row) },
            { label: 'Copy Entry', action: (e, row) => copyRow(row) }
          ];

          if (tableClipboard && tableClipboard.type === 'row') {
            menu.push({ label: 'Paste Entry Above', action: (e, row) => pasteRow(row, 'before') });
            menu.push({ label: 'Paste Entry Below', action: (e, row) => pasteRow(row, 'after') });
          }

          menu.push({ separator: true });
          menu.push({ label: 'Insert Entry Above', action: (e, row) => insertRow(row, 'before') });
          menu.push({ label: 'Insert Entry Below', action: (e, row) => insertRow(row, 'after') });
          menu.push({ separator: true });
          menu.push({ label: 'Delete Entry', action: (e, row) => deleteRow(row) });
          menu.push({ separator: true });
          menu.push({ label: 'Highlight Entry', menu: highlightColorOptions });
          menu.push({ label: 'Clear Entry Highlight', action: () => highlightAction(null) });

          return menu;
        },
        columnDefaults: {
          headerSort: false,
          headerHozAlign: 'center',
          headerVAlign: 'middle',
          editor: 'textarea',
          editable: function (cell) {
            return currentActiveViewType !== 'pivot' && mediaEditorStore.isLexicalEditMode;
          },
          editorParams: { verticalNavigation: 'editor', shiftEnterSubmit: false },
          resizable: true,
          width: 200,
          minWidth: 100
        },
        rowHeader: {
          resizable: false,
          frozen: true,
          headerSort: false,
          hozAlign: 'center',
          formatter: function (cell) {
            const rowNum = cell.getRow().getPosition(true);
            const container = document.createElement('div');
            container.className =
              'row-number-container group relative flex items-center justify-center h-full w-full';

            const span = document.createElement('span');
            span.className = 'row-number-text group-hover:hidden';
            span.textContent = rowNum;

            if (currentActiveViewType !== 'pivot' && mediaEditorStore.isLexicalEditMode) {
              const button = document.createElement('button');
              button.className =
                'edit-icon-placeholder hidden group-hover:flex items-center justify-center h-full w-full text-blue-500 hover:text-blue-600 transition-colors';
              button.title = 'Edit Entry';

              mount(TableIcon, {
                target: button,
                props: { icon: Pencil, size: 14 }
              });
              container.appendChild(button);
            }

            container.appendChild(span);
            return container;
          },
          cellClick: (e, cell) => {
            if (currentActiveViewType === 'pivot' || !mediaEditorStore.isLexicalEditMode) return;
            if (e.target.closest('.edit-icon-placeholder')) {
              e.preventDefault();
              e.stopPropagation();
              openEditEntryModal(cell.getRow());
            }
          },
          width: 50,
          minWidth: 40,
          cssClass: 'range-header-col tabulator-row-number-column'
        },
        clipboard: true,
        clipboardCopyStyled: false,
        clipboardCopyConfig: { rowHeaders: false, columnHeaders: false },
        clipboardCopyRowRange: 'range',
        clipboardPasteParser: 'range',
        clipboardPasteAction: 'range'
      });
      tabulatorInstance.on('tableBuilt', () => {
        tableReady = true;
        detectDuplicates();
        checkValidationErrors();

        if (
          activeSubItemType === 'doc' &&
          activeSubItemPath &&
          typeof activeSubItemPath === 'string'
        ) {
          openLexicalDocument(activeSubItemPath);
        } else if (activeSubItemType === 'view' && activeSubItemPath?.view_name) {
          const viewToRestore = availableViews.find(
            (v) => v.view_name === activeSubItemPath.view_name
          );
          if (viewToRestore) {
            try {
              applyViewToTable(
                viewToRestore.view_name,
                viewToRestore.view_type,
                JSON.parse(viewToRestore.view_config_json)
              );
            } catch (e) {
              console.error('Failed to restore view:', e);
            }
          }
        }
      });
      const saveCurrentTableLayout = debounce(async () => {
        if (!tabulatorInstance || !currentLoadedPath) return;
        const baseDirForSave = get(project)?.baseDirectory;
        const relativePathForSave = getRelativePath(currentLoadedPath, baseDirForSave);
        if (!baseDirForSave || !relativePathForSave) return;
        updateTableLayoutSnapshot();
        await saveTableLayoutPrefs(relativePathForSave, tableLayoutSnapshot).catch((err) =>
          console.error(`Failed to save layout:`, err)
        );
      }, 750);
      tabulatorInstance.on('columnResized', (column) => {
        saveCurrentTableLayout();
        if (tabulatorInstance) {
          tabulatorInstance.redraw();
        }
      });

      // Event-driven layout saving for structural changes
      tabulatorInstance.on('columnMoved', saveCurrentTableLayoutImmediately);
      tabulatorInstance.on('columnAdded', saveCurrentTableLayoutImmediately);
      tabulatorInstance.on('columnDeleted', saveCurrentTableLayoutImmediately);

      tabulatorInstance.on('cellEdited', (cell) => {
        if (!isUndoRedoActive) {
          const oldValue = cell.getOldValue();
          const newValue = cell.getValue();
          if (oldValue !== newValue) {
            pushToHistory({
              type: 'cellEdit',
              rowId: cell.getRow().getData().harvey_internal_id,
              field: cell.getField(),
              oldValue: oldValue,
              newValue: newValue
            });
          }
        }
        debouncedSave();
        if (cell.getField() === currentPrimaryField) {
          detectDuplicates();
        }
        checkValidationErrors(); // Check and update error outlines safely after edit is finished

        const row = cell.getRow();
        const rowData = row.getData();
        const rowIndex = rowData.harvey_internal_id;
        const highlightId = `row-${rowIndex}`;

        let currentHighlights = get(project).currentTableHighlights || [];
        const highlightIndex = currentHighlights.findIndex((h) => h.id === highlightId);

        if (highlightIndex !== -1) {
          const newText = Object.values(rowData)
            .filter((val) => val !== null && val !== undefined)
            .join(' | ');
          const updatedHighlights = [
            ...currentHighlights.slice(0, highlightIndex),
            { ...currentHighlights[highlightIndex], text: newText },
            ...currentHighlights.slice(highlightIndex + 1)
          ];
          setTableHighlights(updatedHighlights);
          saveTableHighlights();
        }
      });
      columnFields = tabulatorInstance
        .getColumnDefinitions()
        .map((c) => c.field)
        .filter(Boolean);
    } catch (err) {
      error = `Failed to load table: ${err.message || err}`;
    } finally {
      isLoading = false;
    }
  }

  function handleSearch() {
    if (!tabulatorInstance) return;

    // Clear existing highlights and reset matches
    cellMatches.forEach((cell) => {
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
    activeRows.forEach((row) => {
      row.getCells().forEach((cell) => {
        const cellValue = cell.getValue();
        if (
          cellValue !== null &&
          cellValue !== undefined &&
          String(cellValue).toLowerCase().includes(term)
        ) {
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
      ranges.forEach((range) => range.remove());
    }

    currentMatchIndex = index;
    const currentCell = cellMatches[currentMatchIndex];

    // Scroll to the entry of the current cell first to ensure it is visible
    await currentCell
      .getRow()
      .scrollTo()
      .catch((err) => console.error('Scroll to entry failed', err));

    // Use Tabulator's built-in range selection to highlight the active cell
    tabulatorInstance.addRange(currentCell, currentCell);
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
    // initializeTable is now strictly handled by the $effect monitoring tablePath
    // if (tablePath) initializeTable(tablePath);

    const handleKeyDown = (e) => {
      // Ignore custom shortcuts if user is typing in an input/textarea so native text undo/redo works
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
        return;
      }

      if (e.metaKey && e.key === 'c') {
        e.preventDefault();
        tabulatorInstance?.copyToClipboard('range');
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        e.stopPropagation();
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        e.stopPropagation();
        redo();
      }
    };
    tableContainer?.addEventListener('keydown', handleKeyDown);

    const handleHeaderFilterKeydown = (e) => {
      if (
        e.target.tagName === 'INPUT' &&
        e.target.closest('.tabulator-header-filter') &&
        e.key === 'Enter'
      ) {
        e.preventDefault();
        e.stopPropagation();
      }
    };
    tableContainer?.addEventListener('keydown', handleHeaderFilterKeydown);

    // Prevent Tabulator from stealing arrow keys and Shift+Enter when editing text inputs/textareas
    const handleEditorArrowKeys = (e) => {
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
          e.stopPropagation();
        }
      } else if (e.key === 'Enter' && e.shiftKey && e.target.tagName === 'TEXTAREA') {
        // Manually insert newline for Shift+Enter to bypass Tabulator's native interception
        e.preventDefault();
        e.stopPropagation();
        const start = e.target.selectionStart;
        const end = e.target.selectionEnd;
        const val = e.target.value;
        e.target.value = val.substring(0, start) + '\n' + val.substring(end);
        e.target.selectionStart = e.target.selectionEnd = start + 1;
      }
    };
    tableContainer?.addEventListener('keydown', handleEditorArrowKeys, true); // use capture

    document.addEventListener('click', handleTableClick);

    // Workaround to make Tabulator 6 submenus expand on hover like standard desktop applications
    const handleMenuHover = (e) => {
      if (
        e.target &&
        e.target.classList &&
        e.target.classList.contains('tabulator-menu-item-submenu')
      ) {
        // To avoid rapid click loops, we check if the submenu is already open
        // Tabulator attaches a `.tabulator-menu` popup to the body when clicked
        // It is difficult to query directly, so we just trigger a click if we haven't recently
        if (!e.target.dataset.hoverOpened) {
          e.target.dataset.hoverOpened = 'true';
          e.target.click();

          // Reset the hover flag when mouse leaves the menu item
          e.target.addEventListener(
            'mouseleave',
            function resetHover() {
              e.target.dataset.hoverOpened = '';
              e.target.removeEventListener('mouseleave', resetHover);
            },
            { once: true }
          );
        }
      }
    };
    document.addEventListener('mouseover', handleMenuHover, true);

    return () => {
      tabulatorInstance?.destroy();
      tableContainer?.removeEventListener('keydown', handleKeyDown);
      tableContainer?.removeEventListener('keydown', handleHeaderFilterKeydown);
      tableContainer?.removeEventListener('keydown', handleEditorArrowKeys, true);
      document.removeEventListener('click', handleTableClick);
      document.removeEventListener('mouseover', handleMenuHover, true);
    };
  });

  function pushToHistory(action) {
    if (isUndoRedoActive) return;
    svelteUndoStack.push(action);
    if (svelteUndoStack.length > 50) svelteUndoStack.shift(); // Keep last 50 actions
    svelteRedoStack = []; // Clear redo stack on new action
  }

  $effect(() => {
    if (mediaEditorStore.isLexicalEditMode !== undefined && tabulatorInstance && tableReady) {
      // Small timeout to ensure DOM is settled after a state change or initial build
      setTimeout(updateTableDimensions, 100);
    }
  });

  async function applyHistoryAction(action, isUndo) {
    if (!tabulatorInstance) return;
    isUndoRedoActive = true;
    try {
      if (action.type === 'cellEdit') {
        const row = tabulatorInstance.getRow(action.rowId);
        if (row) {
          const cell = row.getCell(action.field);
          if (cell) {
            cell.setValue(isUndo ? action.oldValue : action.newValue);
          }
        }
      } else if (action.type === 'rowAdd') {
        if (isUndo) {
          const row = tabulatorInstance.getRow(action.rowId);
          if (row) await row.delete();
        } else {
          await tabulatorInstance.addRow(
            action.rowData,
            action.position === 'before',
            action.relativeTo
          );
        }
      } else if (action.type === 'rowDelete') {
        if (isUndo) {
          await tabulatorInstance.addRow(action.rowData); // Best effort restore
        } else {
          const row = tabulatorInstance.getRow(action.rowId);
          if (row) await row.delete();
        }
      }
      debouncedSave();
      checkValidationErrors();
      detectDuplicates();
      reformatAllRows();
    } catch (e) {
      console.error('History action failed', e);
    } finally {
      isUndoRedoActive = false;
    }
  }

  function undo() {
    if (svelteUndoStack.length === 0) return;
    const action = svelteUndoStack.pop();
    svelteRedoStack.push(action);
    applyHistoryAction(action, true);
  }

  function redo() {
    if (svelteRedoStack.length === 0) return;
    const action = svelteRedoStack.pop();
    svelteUndoStack.push(action);
    applyHistoryAction(action, false);
  }

  $effect(() => {
    if (tablePath && tablePath !== untrack(() => currentLoadedPath)) {
      initializeTable(tablePath);
    }
  });

  $effect(() => {
    if (panelState.tagsLeftPanelCollapsed !== undefined && tabulatorInstance) {
      // Debounce this to avoid excessive redraws during rapid toggling
      debounce(() => {
        reformatAllRows();
      }, 100)();
    }
  });
</script>

<svelte:window on:mousedown={handleOutsideClick} />

{#if showEditFieldModal}
  <EditFieldModal
    fieldName={editingFieldData.name}
    colSchema={editingFieldData.schema}
    {currentPrimaryField}
    on:save={handleSaveField}
    on:cancel={() => {
      showEditFieldModal = false;
      isAddingNewField = false;
      newFieldTargetColumn = null;
    }}
  />
{/if}

{#if showViewModal}
  <ViewModal
    bind:open={showViewModal}
    {tablePath}
    columns={tableColumnsForModal}
    {tableData}
    schema={computedSchema}
    initialView={initialViewToLoad}
    views={availableViews}
    activeViewName={currentActiveView}
    on:viewSaved={handleViewSaved}
    on:viewApplied={handleViewApplied}
    on:viewDeleted={(event) => {
      const deletedViewName = event.detail?.viewName;
      loadTableViews(tablePath);
      dispatch('requestviewchange', { type: 'refresh_metadata' });
      if (currentActiveView && currentActiveView === deletedViewName) {
        returnToBaseTable();
      }
    }}
  />
{/if}

{#if showChartModal}
  <ChartModal
    bind:open={showChartModal}
    {tablePath}
    columns={tableColumnsForModal}
    {tableData}
    schema={computedSchema}
    initialChart={initialChartToLoad}
    views={availableViews}
    activeViewName={currentActiveView}
    on:chartSaved={() => {
      dispatch('requestviewchange', { type: 'refresh_metadata' });
    }}
    on:chartSavedToImages={() => {
      dispatch('requestviewchange', { type: 'refresh_metadata' });
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
    on:cancel={() => (showEditEntryModal = false)}
  />
{/if}

<div
  bind:this={mainPanelContainer}
  class="flex flex-col h-full w-full bg-white dark:bg-gray-900 shadow overflow-hidden relative"
>
  {#if !isViewingDocument}
    <div
      class="toolbar relative flex items-center flex-nowrap gap-x-1 border-b border-gray-300 dark:border-gray-700 h-9 px-2 flex-shrink-0 bg-white dark:bg-gray-950 shadow-md z-10 justify-between overflow-x-auto"
    >
      <div class="flex items-center gap-1">
        {#if currentActiveView}
          <button
            on:click={returnToBaseTable}
            class="flex items-center gap-1 bg-blue-600 hover:bg-blue-700 text-white border border-blue-600 rounded focus:outline-none focus:ring-2 focus:ring-blue-300 font-medium px-2.5 py-1 transition duration-150 ease-in-out text-xs mr-2 shadow-sm"
            title="Return to Base Table"
          >
            <Undo2 size={14} />
            <span>Return to Base Table</span>
          </button>
          <div class="separator mx-0.5 mr-2"></div>
        {/if}

        <button
          id="history-undo"
          on:click={undo}
          class="mini-toolbar-button"
          title="Undo"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Undo2 size={14} />
        </button>

        <button
          id="history-redo"
          on:click={redo}
          class="mini-toolbar-button"
          title="Redo"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Redo2 size={14} />
        </button>

        <div class="separator mx-0.5"></div>

        <button
          id="style-bold"
          on:click={() => toggleStyle('bold')}
          class="mini-toolbar-button"
          title="Bold"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Bold size={14} />
        </button>

        <button
          id="style-italic"
          on:click={() => toggleStyle('italic')}
          class="mini-toolbar-button"
          title="Italic"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Italic size={14} />
        </button>

        <button
          id="style-underline"
          on:click={() => toggleStyle('underline')}
          class="mini-toolbar-button"
          title="Underline"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Underline size={14} />
        </button>

        <div class="separator mx-0.5"></div>

        <div class="relative" bind:this={colorDropdownRef}>
          <button
            id="style-color"
            on:click={toggleColorDropdown}
            class="mini-toolbar-button flex items-center"
            title="Text Color"
            disabled={!mediaEditorStore.isLexicalEditMode}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="h-3.5 w-3.5"
              fill="currentColor"
              viewBox="0 0 16 16"
            >
              <path
                d="m13.498.795.149-.149a1.207 1.207 0 1 1 1.707 1.708l-.149.148a1.5 1.5 0 0 1-.059 2.059L4.854 14.854a.5.5 0 0 1-.233.131l-4 1a.5.5 0 0 1-.606-.606l1-4a.5.5 0 0 1 .131-.232l9.642-9.642a.5.5 0 0 0-.642.056L6.854 4.854a.5.5 0 1 1-.708-.708L9.44.854A1.5 1.5 0 0 1 11.5.796a1.5 1.5 0 0 1 1.998-.001m-.644.766a.5.5 0 0 0-.707 0L1.95 11.756l-.764 3.057 3.057-.764L14.44 3.854a.5.5 0 0 0 0-.708z"
              />
            </svg>
            <svg
              class="ml-0.5 h-3 w-3"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 20 20"
              fill="currentColor"
              ><path
                fill-rule="evenodd"
                d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z"
                clip-rule="evenodd"
              /></svg
            >
          </button>
          {#if isColorDropdownOpen}
            <div
              class="absolute top-full left-0 mt-1 z-20 w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 shadow-lg rounded-md"
            >
              {#each colorOptions as option (option.value)}
                <button
                  class="w-full text-left px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => applyTextColor(option.value)}
                >
                  <span
                    class="w-4 h-4 border border-gray-400 dark:border-gray-500 rounded-full shrink-0"
                    style="background-color: {option.value === 'transparent'
                      ? '#fff'
                      : option.value};"
                  ></span>
                  <span class="truncate">{option.label}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="separator mx-0.5"></div>

        <button
          id="style-clear"
          on:click={clearFormatting}
          class="mini-toolbar-button"
          title="Clear Formatting"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <Eraser size={14} />
        </button>
        <div class="separator mx-0.5"></div>

        <button
          id="insert-charts"
          on:click={() => {
            tableColumnsForModal = tabulatorInstance
              .getColumnDefinitions()
              .filter((c) => c.field && c.field !== 'harvey_internal_id');
            initialChartToLoad = null;
            showChartModal = true;
          }}
          class="mini-toolbar-button flex items-center gap-1"
          title="Insert Charts"
          disabled={!mediaEditorStore.isLexicalEditMode}
        >
          <ChartBar size={14} />
          <span>Insert Charts</span>
        </button>

        <div class="separator mx-0.5"></div>
        <button
          id="create-views"
          on:click={() => {
            tableColumnsForModal = tabulatorInstance
              .getColumnDefinitions()
              .filter((c) => c.field && c.field !== 'harvey_internal_id');
            initialViewToLoad = null;
            showViewModal = true;
          }}
          class="mini-toolbar-button flex items-center gap-1 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-800 hover:bg-blue-50 dark:hover:bg-blue-900/30"
          title="Create Views"
          disabled={currentActiveViewType === 'pivot' || !mediaEditorStore.isLexicalEditMode}
        >
          <Table2 size={14} />
          <span>Create Views</span>
        </button>
      </div>

      {#if !isLoading && !error}
        <div class="flex items-center gap-2">
          <div class="flex items-center gap-1 relative">
            <!-- Using native input to seamlessly match toolbar height styles -->
            <div class="relative w-48">
              <div class="absolute inset-y-0 left-0 flex items-center pl-2 pointer-events-none">
                <svg
                  class="w-3 h-3 text-gray-500 dark:text-gray-400"
                  aria-hidden="true"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 20 20"
                >
                  <path
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="m19 19-4-4m0-7A7 7 0 1 1 1 8a7 7 0 0 1 14 0Z"
                  />
                </svg>
              </div>
              <input
                type="text"
                bind:this={searchInputRef}
                bind:value={searchTerm}
                on:input={handleSearch}
                on:keydown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    e.stopPropagation();
                    goToNextMatch();
                  }
                }}
                placeholder="Search table..."
                class="w-full text-xs border border-gray-300 dark:border-gray-600 pl-7 pr-6 py-1 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500 rounded outline-none"
              />
              {#if searchTerm}
                <button
                  class="absolute inset-y-0 right-0 flex items-center pr-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none"
                  on:click={() => {
                    searchTerm = '';
                    handleSearch();
                    searchInputRef?.focus();
                  }}
                  title="Clear search"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="h-3 w-3"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                  >
                    <path
                      fill-rule="evenodd"
                      d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                      clip-rule="evenodd"
                    />
                  </svg>
                </button>
              {/if}
            </div>

            {#if searchTerm}
              <div class="flex items-center gap-[1px]">
                <button
                  on:click={goToPreviousMatch}
                  disabled={cellMatches.length === 0}
                  class="mini-toolbar-button"
                  title="Previous Match"
                >
                  <ChevronLeft size={14} />
                </button>
                <button
                  on:click={goToNextMatch}
                  disabled={cellMatches.length === 0}
                  class="mini-toolbar-button"
                  title="Next Match"
                >
                  <ChevronRight size={14} />
                </button>
              </div>
            {/if}
          </div>

          <div class="separator mx-0.5"></div>

          <div class="relative">
            <button class="mini-toolbar-button" title="Options">
              <MoreVertical size={14} />
            </button>
            <Dropdown placement="bottom-end">
              <DropdownItem on:click={toggleFilters} class="text-xs py-1.5 px-3">
                {areFiltersVisible ? 'Hide' : 'Show'} Column Filters
              </DropdownItem>
          </Dropdown>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <div class="flex-grow overflow-auto min-h-0 relative flex flex-col bg-white dark:bg-gray-900">
    <!-- Popovers (URL, Project Link, Email) -->
    {#if showUrlPopover}
      <div
        class="url-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1"
        style="left: {popoverX}px; top: {popoverY}px; transform: translateX(-100%); min-width: 140px;"
      >
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleOpenUrl}
        >
          <ExternalLink size={14} /> Open in browser
        </button>
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleCopyUrl}
        >
          {#if isUrlCopied}
            <Check size={14} class="text-green-500" /> Copied
          {:else}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="lucide lucide-copy"
              ><rect width="14" height="14" x="8" y="8" rx="2" ry="2" /><path
                d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"
              /></svg
            > Copy
          {/if}
        </button>
      </div>
    {/if}

    {#if showProjectLinkPopover}
      <div
        class="project-link-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1"
        style="left: {popoverProjectLinkX}px; top: {popoverProjectLinkY}px; transform: translateX(-100%); min-width: 140px;"
      >
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleOpenProjectLink}
        >
          <FolderOpen size={14} /> Open File
        </button>
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleRevealProjectLink}
        >
          <FolderSearch size={14} />
          {revealButtonLabel}
        </button>
      </div>
    {/if}

    {#if showEmailPopover}
      <div
        class="email-popover-container fixed z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl py-1"
        style="left: {popoverEmailX}px; top: {popoverEmailY}px; transform: translateX(-100%); min-width: 140px;"
      >
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleOpenEmail}
        >
          <Mail size={14} /> Send Email
        </button>
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2"
          on:click={handleCopyEmail}
        >
          {#if isEmailCopied}
            <Check size={14} class="text-green-500" /> Copied
          {:else}
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              class="lucide lucide-copy"
              ><rect width="14" height="14" x="8" y="8" rx="2" ry="2" /><path
                d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"
              /></svg
            > Copy Email
          {/if}
        </button>
      </div>
    {/if}

    <!-- Loading / Error States -->
    {#if isLoading}
      <div
        class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400 z-10"
      >
        Loading table data...
      </div>
    {:else if error}
      <div
        class="absolute inset-0 flex items-center justify-center text-red-600 dark:text-red-400 p-4 text-center z-10"
      >
        Error: {error}
      </div>
    {/if}

    <!-- Main Content Selection (Document, Pivot, or Table) -->
    <div class="relative flex-grow min-h-0 flex flex-col">
      {#if isViewingDocument}
        <div
          class="w-full h-full bg-white dark:bg-gray-800 overflow-hidden relative z-20 flex flex-col"
        >
          {#key currentActiveDocumentPath}
            <div class="flex-grow min-h-0">
              <LexicalEditor
                initialJson={currentActiveDocumentJson}
                editable={mediaEditorStore.isLexicalEditMode}
                allowReadModeHighlights={true}
                placeholder="Start typing your document..."
                enableTableCellMenu={true}
                enableTableCellResize={true}
                enableSearch={true}
                documentPath={currentActiveDocumentPath}
                initialHighlights={currentActiveDocumentHighlights}
                documentHighlights={$project.currentDocumentHighlights}
                on:change={handleLexicalDocumentChange}
                on:highlightschange={handleLexicalHighlightsChange}
                toolbarConfig={{
                  undo: true,
                  redo: true,
                  blockType: false,
                  bold: true,
                  italic: true,
                  underline: true,
                  strikethrough: true,
                  align: false,
                  insertMenu: false,
                  link: false,
                  outdent: false,
                  indent: false,
                  textColor: true,
                  highlight: true,
                  clearFormatting: false,
                  search: true,
                  fontFamily: false
                }}
              >
                <svelte:fragment slot="toolbar_prepend">
                  <button
                    on:click={returnToBaseTable}
                    class="flex items-center gap-1 bg-blue-600 hover:bg-blue-700 text-white border border-blue-600 rounded focus:outline-none focus:ring-2 focus:ring-blue-300 font-medium px-2.5 py-1 transition duration-150 ease-in-out text-xs mr-2 shadow-sm"
                    title="Return to Base Table"
                  >
                    <Undo2 size={14} />
                    <span>Return to Base Table</span>
                  </button>
                  <div class="separator mx-0.5 mr-2"></div>
                </svelte:fragment>
              </LexicalEditor>
            </div>
          {/key}
        </div>
      {:else if currentActiveViewType === 'pivot'}
        <div
          class="w-full h-full bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 overflow-auto relative z-20"
        >
          <table class="w-full text-sm text-left text-gray-500 dark:text-gray-400 border-collapse">
            <thead
              class="text-xs text-gray-700 uppercase bg-gray-100 dark:bg-gray-700 dark:text-gray-400 sticky top-0 z-10 shadow-sm"
            >
              {#if generatedPivotResult && generatedPivotResult.colHeaders.length > 0}
                {#each generatedPivotResult.colHeaders as headerRow, levelIndex (levelIndex)}
                  <tr>
                    {#if levelIndex === generatedPivotResult.colHeaders.length - 1}
                      {#each generatedPivotResult.rowFields as rowField (rowField)}
                        <th
                          scope="col"
                          class="px-6 py-3 whitespace-nowrap font-bold border border-gray-200 dark:border-gray-600 bg-gray-200 dark:bg-gray-600 align-bottom"
                        >
                          {rowField}
                        </th>
                      {/each}
                    {:else if generatedPivotResult.rowFieldsCount > 0}
                      <th
                        colspan={generatedPivotResult.rowFieldsCount}
                        class="border border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-700"
                      ></th>
                    {/if}

                    {#each headerRow as h (h.val)}
                      <th
                        scope="col"
                        colspan={h.colspan}
                        class="px-6 py-3 whitespace-nowrap text-center border border-gray-200 dark:border-gray-600 {levelIndex ===
                        generatedPivotResult.colHeaders.length - 1
                          ? 'bg-gray-100 dark:bg-gray-700'
                          : 'bg-gray-200 dark:bg-gray-600'}"
                      >
                        {h.val}
                      </th>
                    {/each}
                  </tr>
                {/each}
              {/if}
            </thead>
            <tbody>
              {#if generatedPivotResult && generatedPivotResult.rows.length > 0}
                {#each generatedPivotResult.rows as row, i (i)}
                  <tr
                    class="bg-white border-b dark:bg-gray-800 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors"
                  >
                    {#each row.headers as header (header.val)}
                      {#if header.rowspan > 0}
                        <td
                          rowspan={header.rowspan}
                          class="px-6 py-4 whitespace-nowrap font-bold text-gray-900 dark:text-white border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 align-top"
                        >
                          {header.val}
                        </td>
                      {/if}
                    {/each}

                    {#each Array(generatedPivotResult.colLeavesCount) as _, colIndex (colIndex)}
                      <td
                        class="px-6 py-4 whitespace-nowrap text-right border border-gray-200 dark:border-gray-700"
                      >
                        {row.data[`val_${colIndex}`] !== undefined
                          ? row.data[`val_${colIndex}`]
                          : ''}
                      </td>
                    {/each}
                  </tr>
                {/each}
              {:else}
                <tr>
                  <td colspan="100%" class="px-6 py-8 text-center text-gray-500">
                    No data available.
                  </td>
                </tr>
              {/if}
            </tbody>
          </table>
        </div>
      {:else}
        <!-- Standard Tabulator Table with Standalone Buttons -->
        <div id="table-view-container" class="relative flex-grow min-h-0 flex flex-col">
          <div
            bind:this={tableContainer}
            on:click={handleTableContainerClick}
            on:mouseup={handleTableMouseUp}
            class="w-full h-full"
          >
            {#if !isLoading && !error && tableData.length === 0 && tablePath && !isViewingDocument}
              <div class="p-4 text-center text-gray-500 dark:text-gray-400">
                Table is empty or data could not be loaded.
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <FloatingTableHighlightToolbar
    showToolbar={showTableModifyToolbar}
    toolbarPosition={tableModifyToolbarPosition}
    onChangeColor={handleTableHighlightColorChange}
    onDelete={handleTableHighlightDelete}
    onClose={() => {
      showTableModifyToolbar = false;
      clickedRow = null;
      selectedRows = [];
      activeHighlightIdForToolbar = null;
    }}
    highlightId={activeHighlightIdForToolbar}
    docType="table"
    filePath={tablePath}
    onTagToggle={handleTableTagToggle}
  />
</div>

<style lang="postcss">
  .min-h-0 {
    min-height: 0;
  }
  :global(.tabulator) {
    height: 100% !important;
    border: none;
  }
  :global(
    .tabulator
      .tabulator-header
      .tabulator-col
      .tabulator-col-content
      .tabulator-col-title-holder
      .tabulator-col-title
  ) {
    white-space: normal !important;
    @apply text-gray-900 dark:text-gray-200 font-semibold;
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
    border-bottom: 1px solid #ddd; /* Move horizontal border from row to cell */
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
    border-bottom: 1px solid #374151;
  }
  :global(html.dark .tabulator-row:hover .tabulator-row-number-column) {
    background-color: #374151 !important;
  }

  /* Legacy add-column-header styles removed as they are consolidated into harvey-pseudo-col */


  .toolbar button.mini-toolbar-button {
    @apply p-1 rounded inline-flex items-center justify-center
             focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-500
             dark:focus:ring-offset-[var(--app-bg)] transition duration-150 ease-in-out
             text-xs disabled:opacity-50 disabled:cursor-not-allowed;
    color: var(--ui-icon-color);
    border: 1px solid var(--ui-select-border);
    background-color: transparent;
    margin-right: 2px;
    line-height: 1.2;
    min-height: 24px;
    height: 24px;
  }

  .toolbar button.mini-toolbar-button:hover:not(:disabled) {
    background-color: var(--ui-icon-hover-bg);
    border-color: var(--ui-select-border);
  }

  html.dark .toolbar button.mini-toolbar-button {
    color: #e5e5e5;
    border: 1px solid #404040;
    background-color: transparent;
  }

  html.dark .toolbar button.mini-toolbar-button:hover:not(:disabled) {
    background-color: #404040;
    border-color: #404040;
  }

  .separator {
    width: 1px;
    height: 1.25rem;
    background-color: var(--ui-select-border);
    margin: 0 0.25rem;
  }

  html.dark .separator {
    background-color: #404040;
  }

  .flex-grow {
    position: relative;
  }
  .flex-grow > div[bind\:this='{tableContainer}'] {
    position: absolute;
    inset: 0;
  }

  :global(.tabulator .tabulator-header .tabulator-col) {
    padding-left: 0px !important;
    @apply bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-200 border-gray-300 dark:border-gray-700;
  }

  :global(.tabulator .tabulator-header) {
    @apply bg-gray-100 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-700;
  }

  :global(.tabulator-cell) {
    overflow: hidden;
    word-break: break-word;
    border-right: 1px solid #ddd;
    border-bottom: 1px solid #ddd; /* Moved from row to cell */
    min-height: 38px; /* Ensures blank inserted rows exactly match text-filled rows (padding + line height) */
  }
  /* Suppress row borders at the row level to control them at the cell level */
  :global(.tabulator-row) {
    border-bottom: none !important;
  }
  :global(html.dark .tabulator-cell) {
    border-right: 1px solid #374151;
    border-bottom: 1px solid #374151;
  }


  /* Fix Tabulator Star Formatter SVG stacking */
  :global(.tabulator-cell svg) {
    display: inline-block;
    vertical-align: middle;
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

  /* Redundant add-column-header cell styles removed */


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

  :global(
    .tabulator-cell.interactive-contact-cell:hover .hyperlink-icon-container,
    .tabulator-cell.interactive-contact-cell:hover .email-icon-container,
    .tabulator-cell.interactive-contact-cell:hover .project-link-icon-container
  ) {
    display: flex !important;
  }
  /* Progress Editor Styling */
  :global(.progress-range) {
    -webkit-appearance: none;
    background: #e5e7eb;
    height: 6px !important;
    border-radius: 3px;
    outline: none;
    margin: 0;
    padding: 0;
  }
  :global(.progress-range::-webkit-slider-thumb) {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
    border: 2px solid white;
    box-shadow: 0 0 2px rgba(0, 0, 0, 0.3);
    margin-top: -4px; /* Center thumb on track */
  }
  :global(.progress-range::-moz-range-thumb) {
    width: 14px;
    height: 14px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
    border: 2px solid white;
    box-shadow: 0 0 2px rgba(0, 0, 0, 0.3);
  }

  /* Tabulator Menu / Context Menu Dark Mode Styling */
  :global(html.dark .tabulator-menu) {
    background-color: #1f2937 !important; /* gray-800 */
    border: 1px solid #374151 !important; /* gray-700 */
    color: #f3f4f6 !important; /* gray-100 */
  }
  :global(html.dark .tabulator-menu .tabulator-menu-item) {
    background-color: #1f2937 !important;
    color: #f3f4f6 !important;
  }
  :global(html.dark .tabulator-menu .tabulator-menu-item:hover) {
    background-color: #374151 !important; /* gray-700 */
  }
  :global(html.dark .tabulator-menu .tabulator-menu-separator) {
    border-top: 1px solid #374151 !important; /* gray-700 */
  }

  /* Ensure manual resize handle reaches the true column border */
  :global(.tabulator .tabulator-header .tabulator-col .tabulator-col-content) {
    padding: 0 !important;
    height: 100% !important;
    display: flex !important;
    align-items: center !important;
  }
  :global(.tabulator .tabulator-header .tabulator-col .tabulator-col-content .tabulator-col-title-holder) {
    padding: 0 !important;
    margin: 0 !important;
    width: 100% !important;
    height: 100% !important;
    display: flex !important;
    align-items: center !important;
  }
  :global(.tabulator .tabulator-header .tabulator-col .tabulator-col-content .tabulator-col-title-holder .tabulator-col-title) {
    padding: 0 !important;
    margin: 0 !important;
    width: 100% !important;
    height: 100% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    flex-grow: 1 !important;
  }

  /* Virtual Pseudo-Column/Row Styling */
  :global(.harvey-pseudo-col) {
    /* Use an opaque background to mask row borders for a "merged" look */
    background-color: #f8fbff !important; 
    border-left: 1px dotted rgba(59, 130, 246, 0.5) !important;
    border-right: 1px dotted rgba(59, 130, 246, 0.5) !important;
    transition: background-color 0.15s ease-in-out;
  }
  :global(html.dark .harvey-pseudo-col) {
    background-color: #0d1222 !important; 
    border-left: 1px dotted rgba(59, 130, 246, 0.6) !important;
    border-right: 1px dotted rgba(59, 130, 246, 0.6) !important;
  }
  /* Hide cells under the Add Field column and style as one continuous vertical bar */
  :global(.tabulator-row .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]) {
    border-top: none !important;
    border-bottom: none !important;
    border-left: 1px dotted rgba(59, 130, 246, 0.5) !important;
    border-right: 1px dotted rgba(59, 130, 246, 0.5) !important;
    background-color: #f8fbff !important; 
    cursor: pointer !important;
    color: transparent !important;
    position: relative;
    z-index: 10;
    outline: none !important;
    box-shadow: none !important;
  }
  :global(.tabulator-row .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]:focus),
  :global(.tabulator-row .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]:active) {
    outline: none !important;
    box-shadow: none !important;
    border-top: none !important;
    border-bottom: none !important;
  }
  :global(html.dark .tabulator-row .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]) {
    background-color: #0d1222 !important; 
    border-left: 1px dotted rgba(59, 130, 246, 0.6) !important;
    border-right: 1px dotted rgba(59, 130, 246, 0.6) !important;
    border-top: none !important;
    border-bottom: none !important;
  }
  :global(.harvey-add-field-hovering .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]) {
    background-color: #f0f7ff !important;
  }
  :global(html.dark .harvey-add-field-hovering .tabulator-cell[tabulator-field="harvey_pseudo_add_col"]) {
    background-color: #111a33 !important;
  }
  :global(.harvey-add-field-hovering .harvey-pseudo-col) {
    background-color: #f0f7ff !important;
  }
  :global(html.dark .harvey-add-field-hovering .harvey-pseudo-col) {
    background-color: #111a33 !important;
  }


  :global(.harvey-pseudo-row) {
    background-color: transparent !important;
    transition: background-color 0.15s ease-in-out;
  }
  :global(.harvey-pseudo-row:hover) {
    background-color: transparent !important;
  }

  :global(.harvey-pseudo-row .tabulator-cell) {
    padding: 0 !important;
    border-right: none !important;
    width: 100% !important;
    display: block !important;
    background: transparent !important;
  }
</style>
