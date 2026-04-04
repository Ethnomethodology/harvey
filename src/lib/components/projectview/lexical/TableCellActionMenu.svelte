<script>
  import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
  import {
    $getSelection as _getSelection,
    $isRangeSelection as _isRangeSelection,
    $setSelection as _setSelection,
    $getNodeByKey as _getNodeByKey
  } from 'lexical';
  import {
    $isTableCellNode as _isTableCellNode,
    $isTableSelection as _isTableSelection,
    TableCellNode,
    TableRowNode,
    $getTableNodeFromLexicalNodeOrThrow as _getTableNodeFromLexicalNodeOrThrow,
    $insertTableRowAtSelection as _insertTableRowAtSelection,
    $insertTableColumnAtSelection as _insertTableColumnAtSelection,
    $deleteTableRowAtSelection as _deleteTableRowAtSelection,
    $deleteTableColumnAtSelection as _deleteTableColumnAtSelection,
    $getNodeTriplet as _getNodeTriplet
  } from '@lexical/table';

  export let editor; // LexicalEditor instance
  export let anchorElement; // The element to position relative to (usually editor container)
  export let cellNodeKey = null; // Key of the target TableCellNode
  export let isOpen = false;
  export let position = { top: 0, left: 0 }; // Calculated position { top, left }

  const dispatch = createEventDispatcher();

  let menuElement;
  let selectionCounts = { rows: 1, columns: 1 }; // Still needed for insert count logic

  const colors = ['#FFFFFF', '#FFCDD2', '#C8E6C9', '#BBDEFB', '#FFF9C4'];

  let openSubmenuLabel = null;
  let submenuPosition = { top: 0, left: 0 };
  let submenuElement = null;

  async function openSubmenu(item, event) {
    const buttonRect = event.currentTarget.getBoundingClientRect();
    const estimateWidth = 200;
    let left = buttonRect.right;
    let top = buttonRect.top;
    // Flip to left if not enough space
    if (buttonRect.right + estimateWidth > window.innerWidth) {
      left = buttonRect.left - estimateWidth;
    }
    openSubmenuLabel = item.label;
    submenuPosition = { top, left };
    await tick();
    if (submenuElement) {
      const submenuRect = submenuElement.getBoundingClientRect();
      const actualWidth = submenuRect.width;
      if (buttonRect.right + estimateWidth > window.innerWidth) {
        left = buttonRect.left - actualWidth;
      } else {
        left = buttonRect.right;
        if (left + actualWidth > window.innerWidth) {
          left = window.innerWidth - actualWidth - 10;
        }
      }
      submenuPosition = { top, left };
    }
  }

  function closeSubmenu() {
    openSubmenuLabel = null;
  }

  $: if (isOpen && cellNodeKey && editor) {
    // Check editor exists
    try {
      editor.getEditorState().read(() => {
        const node = _getNodeByKey(cellNodeKey);
        if (!_isTableCellNode(node)) {
          closeMenu(); // Close if node is invalid
          return;
        }
        // Update selection counts for insert actions
        const selection = _getSelection();
        if (_isTableSelection(selection)) {
          const shape = selection.getShape();
          selectionCounts = {
            rows: shape.toY - shape.fromY + 1,
            columns: shape.toX - shape.fromX + 1
          };
        } else {
          selectionCounts = { rows: 1, columns: 1 };
        }
      });
    } catch (readError) {
      console.error('Error reading state in menu reactive block:', readError);
      closeMenu();
    }
  }

  function handleClickOutside(event) {
    if (menuElement && !menuElement.contains(event.target)) {
      closeMenu(false); // Don't clear selection if clicking outside
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside, true); // Use capture phase
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside, true);
  });

  function closeMenu(clearSelection = true) {
    if (isOpen) {
      isOpen = false; // Trigger reactivity
      dispatch('close', { clearSelection });
    }
  }

  // Helper to ensure the correct cell (or cells if TableSelection) is selected before an action
  function selectCell() {
    if (!editor || !cellNodeKey) return false; // Guard against missing editor/key
    let success = false;
    try {
      // No need for editor.update() here, as selectStart() only affects selection state,
      // which is handled within the action's update block anyway.
      const editorState = editor.getEditorState(); // Get current state
      const node = editorState.read(() => _getNodeByKey(cellNodeKey)); // Read node from current state

      if (_isTableCellNode(node) && node.isAttached()) {
        // We don't actually select here, just confirm the node is valid.
        // The action's update block will handle the selection if needed.
        success = true;
      } else {
        console.warn(
          'TableCellActionMenu: Target cell node is not attached or invalid, cannot select.'
        );
      }
    } catch (readError) {
      console.error('Error during selectCell read:', readError);
    }
    return success;
  }

  function handleInsertRow(above) {
    if (!editor) return;
    editor.update(
      () => {
        if (!selectCell()) {
          // Call selectCell inside update if it modifies selection
          console.warn('Cannot insert row: Target cell invalid or detached.');
          return;
        }
        for (let i = 0; i < selectionCounts.rows; i++) {
          _insertTableRowAtSelection(!above);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleInsertColumn(left) {
    if (!editor) return;
    editor.update(
      () => {
        if (!selectCell()) {
          console.warn('Cannot insert column: Target cell invalid or detached.');
          return;
        }
        for (let i = 0; i < selectionCounts.columns; i++) {
          _insertTableColumnAtSelection(!left);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleDeleteRow() {
    if (!editor) return;
    editor.update(
      () => {
        if (!selectCell()) {
          console.warn('Cannot delete row: Target cell invalid or detached.');
          return;
        }
        _deleteTableRowAtSelection();
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleDeleteColumn() {
    if (!editor) return;
    editor.update(
      () => {
        if (!selectCell()) {
          console.warn('Cannot delete column: Target cell invalid or detached.');
          return;
        }
        _deleteTableColumnAtSelection();
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleDeleteTable() {
    if (!editor || !cellNodeKey) return;
    editor.update(
      () => {
        const node = _getNodeByKey(cellNodeKey);
        if (!_isTableCellNode(node)) {
          console.warn('Cannot delete table: Invalid starting node key.');
          return;
        }
        try {
          const tableNode = _getTableNodeFromLexicalNodeOrThrow(node);
          tableNode.remove();
          _setSelection(null); // Clear selection after deletion
        } catch (e) {
          console.error('Failed to delete table:', e);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu(false); // Selection is gone anyway
  }

  function handleColorSelect(color) {
    if (!editor || !cellNodeKey) return;
    editor.update(
      () => {
        try {
          const cell = _getNodeByKey(cellNodeKey);
          if (!_isTableCellNode(cell)) return;
          const rowNode = cell.getParent();
          if (!(rowNode instanceof TableRowNode)) return;
          const cells = rowNode.getChildren().filter((child) => _isTableCellNode(child));
          cells.forEach((c) => {
            c.setBackgroundColor(color === '#FFFFFF' ? null : color);
          });
        } catch (e) {
          console.error('Error applying row background color:', e);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleColumnColorSelect(color) {
    if (!editor || !cellNodeKey) return;
    editor.update(
      () => {
        try {
          const cell = _getNodeByKey(cellNodeKey);
          if (!_isTableCellNode(cell)) return;
          const rowNode = cell.getParent();
          const tableNode = rowNode.getParent();
          if (!tableNode || typeof tableNode.getChildren !== 'function') return;
          // Determine column index
          const headerCells = rowNode.getChildren().filter((child) => _isTableCellNode(child));
          const colIndex = headerCells.indexOf(cell);
          if (colIndex < 0) return;
          // Apply to all rows
          tableNode
            .getChildren()
            .filter((r) => r instanceof TableRowNode)
            .forEach((r) => {
              const cells = r.getChildren().filter((child) => _isTableCellNode(child));
              const target = cells[colIndex];
              if (target) {
                target.setBackgroundColor(color === '#FFFFFF' ? null : color);
              }
            });
        } catch (e) {
          console.error('Error applying column background color:', e);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  function handleCellColorSelect(color) {
    if (!editor || !cellNodeKey) return;
    editor.update(
      () => {
        try {
          const cell = _getNodeByKey(cellNodeKey);
          if (!_isTableCellNode(cell)) return;
          cell.setBackgroundColor(color === '#FFFFFF' ? null : color);
        } catch (e) {
          console.error('Error applying cell background color:', e);
        }
      },
      { tag: 'history-merge' }
    );
    closeMenu();
  }

  // --- Simplified Menu Items ---
  const menuStructure = [
    {
      type: 'submenu',
      label: 'Styles',
      items: [{ type: 'cellColors' }, { type: 'rowColors' }, { type: 'columnColors' }]
    },
    { type: 'separator' },
    { type: 'item', label: 'Insert row above', action: () => handleInsertRow(true) },
    { type: 'item', label: 'Insert row below', action: () => handleInsertRow(false) },
    { type: 'separator' },
    { type: 'item', label: 'Insert column left', action: () => handleInsertColumn(true) },
    { type: 'item', label: 'Insert column right', action: () => handleInsertColumn(false) },
    { type: 'separator' },
    { type: 'item', label: 'Delete column', action: handleDeleteColumn },
    { type: 'item', label: 'Delete row', action: handleDeleteRow },
    { type: 'item', label: 'Delete table', action: handleDeleteTable }
  ];
</script>

{#if isOpen && cellNodeKey}
  <div
    class="absolute z-40 min-w-[180px] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md shadow-lg py-1 text-sm"
    bind:this={menuElement}
    style:top="{position.top}px"
    style:left="{position.left}px"
    role="menu"
    aria-orientation="vertical"
    aria-labelledby="table-cell-options-menu"
  >
    {#each menuStructure as item, index (index)}
      {#if item.type === 'separator'}
        <hr class="my-1 border-gray-200 dark:border-gray-700" />
      {:else if item.type === 'submenu'}
        <div class="relative" on:mouseleave={closeSubmenu}>
          <button
            type="button"
            class="w-full flex items-center px-3 py-1.5 text-left text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700"
            role="menuitem"
            tabindex="-1"
            on:mouseenter={(e) => openSubmenu(item, e)}
          >
            <span>{item.label}</span>
            <span class="ml-auto">▶</span>
          </button>
          {#if openSubmenuLabel === item.label}
            <div
              class="fixed z-50 min-w-[180px] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md shadow-lg py-1 text-sm"
              style="top: {submenuPosition.top}px; left: {submenuPosition.left}px;"
              role="menu"
              aria-orientation="vertical"
              bind:this={submenuElement}
            >
              {#each item.items as subitem, idx (idx)}
                {#if subitem.type === 'separator'}
                  <hr class="my-1 border-gray-200 dark:border-gray-700" />
                {:else if subitem.type === 'cellColors'}
                  <div class="px-3 py-1">
                    <div class="text-sm text-gray-700 dark:text-gray-200 mb-1">
                      Cell Background Color
                    </div>
                    <div class="flex items-center space-x-2">
                      {#each colors as color (color)}
                        <button
                          type="button"
                          role="menuitem"
                          tabindex="-1"
                          class="w-5 h-5 rounded-full border"
                          style="background-color: {color};"
                          on:click|stopPropagation={() => handleCellColorSelect(color)}
                        ></button>
                      {/each}
                    </div>
                  </div>
                {:else if subitem.type === 'rowColors'}
                  <div class="px-3 py-1">
                    <div class="text-sm text-gray-700 dark:text-gray-200 mb-1">
                      Row Background Color
                    </div>
                    <div class="flex items-center space-x-2">
                      {#each colors as color (color)}
                        <button
                          type="button"
                          role="menuitem"
                          tabindex="-1"
                          class="w-5 h-5 rounded-full border"
                          style="background-color: {color};"
                          on:click|stopPropagation={() => handleColorSelect(color)}
                        ></button>
                      {/each}
                    </div>
                  </div>
                {:else if subitem.type === 'columnColors'}
                  <div class="px-3 py-1">
                    <div class="text-sm text-gray-700 dark:text-gray-200 mb-1">
                      Column Background Color
                    </div>
                    <div class="flex items-center space-x-2">
                      {#each colors as color (color)}
                        <button
                          type="button"
                          role="menuitem"
                          tabindex="-1"
                          class="w-5 h-5 rounded-full border"
                          style="background-color: {color};"
                          on:click|stopPropagation={() => handleColumnColorSelect(color)}
                        ></button>
                      {/each}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {:else if item.type === 'item'}
        <button
          type="button"
          class="w-full flex items-center px-3 py-1.5 text-left text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 focus:outline-none focus:bg-gray-100 dark:focus:bg-gray-700"
          role="menuitem"
          tabindex="-1"
          on:click|stopPropagation={() => {
            item.action();
          }}
        >
          <span>{item.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  /* Minimal styles needed */
</style>
