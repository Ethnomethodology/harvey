<!-- src/lib/components/projectview/lexical/LexicalEditor.svelte -->
<script>
  import { onMount, onDestroy, tick } from 'svelte';
  import {
    createEditor, $getRoot as _getRoot, $getSelection as _getSelection, $setSelection as _setSelection,
    $isRangeSelection as _isRangeSelection, $isElementNode as _isElementNode,
    $isTextNode as _isTextNode, $getNodeByKey as _getNodeByKey,
    $createParagraphNode as _createParagraphNode, $isParagraphNode as _isParagraphNode,
    FORMAT_TEXT_COMMAND, FORMAT_ELEMENT_COMMAND, INDENT_CONTENT_COMMAND,
    OUTDENT_CONTENT_COMMAND, SELECTION_CHANGE_COMMAND, CLICK_COMMAND,
    UNDO_COMMAND, REDO_COMMAND, KEY_MODIFIER_COMMAND,
    BLUR_COMMAND, FOCUS_COMMAND,
    COMMAND_PRIORITY_LOW, COMMAND_PRIORITY_NORMAL, COMMAND_PRIORITY_CRITICAL, COMMAND_PRIORITY_HIGH, COMMAND_PRIORITY_EDITOR,
    KEY_ENTER_COMMAND,
    RootNode, ParagraphNode, TextNode, LineBreakNode,
    $getNearestNodeFromDOMNode as _getNearestNodeFromDOMNode,
    $insertNodes as _insertNodes,
    KEY_TAB_COMMAND,
    $normalizeSelection__EXPERIMENTAL as _normalizeSelection
  } from 'lexical';

  import {
      mergeRegister,
      $findMatchingParent as _findMatchingParent,
      $getNearestNodeOfType as _getNearestNodeOfType,
      calculateZoomLevel
  } from '@lexical/utils';

  import { $wrapNodes as _internalWrapNodes } from '@lexical/selection';
  const _wrapNodes = _internalWrapNodes;

  import * as lexicalCore from 'lexical';
  const _createTextNode = lexicalCore.$createTextNode;

  import {
    HeadingNode, QuoteNode, $isHeadingNode as _isHeadingNode, $createHeadingNode as _createHeadingNode,
    $createQuoteNode as _createQuoteNode, $isQuoteNode as _isQuoteNode, registerRichText
  } from '@lexical/rich-text';
  import {
    CodeNode, $createCodeNode as _createCodeNode, $isCodeNode as _isCodeNode
  } from '@lexical/code';
  import {
    ListNode, ListItemNode, $isListNode as _isListNode, $isListItemNode as _isListItemNode,
    INSERT_ORDERED_LIST_COMMAND, INSERT_UNORDERED_LIST_COMMAND,
    REMOVE_LIST_COMMAND, registerList
  } from '@lexical/list';
  import {
      TableNode, TableRowNode, TableCellNode,
      $isTableNode as _isTableNode,
      $isTableRowNode as _isTableRowNode,
      $isTableCellNode as _isTableCellNode,
      $createTableNode as _createTableNode,
      $createTableRowNode as _createTableRowNode,
      $createTableCellNode as _createTableCellNode,
      $isTableSelection as _isTableSelection,
      INSERT_TABLE_COMMAND,
      TableCellHeaderStates,
      $computeTableMapSkipCellCheck as _computeTableMapSkipCellCheck,
      $getTableNodeFromLexicalNodeOrThrow as _getTableNodeFromLexicalNodeOrThrow,
      $getTableRowIndexFromTableCellNode as _getTableRowIndexFromTableCellNode,
      $getTableColumnIndexFromTableCellNode as _getTableColumnIndexFromTableCellNode,
  } from '@lexical/table';

  import {
    LinkNode, $isLinkNode as _isLinkNode, TOGGLE_LINK_COMMAND, $createLinkNode as _createLinkNode
  } from '@lexical/link';
  import {
    $setBlocksType as _setBlocksType, $patchStyleText as _patchStyleText,
    $getSelectionStyleValueForProperty as _getSelectionStyleValueForProperty
  } from '@lexical/selection';
  import { createEmptyHistoryState, registerHistory } from '@lexical/history';
  import { createEventDispatcher } from 'svelte';
  import { v4 as uuidv4 } from 'uuid';

  import {
    ExtendedTextNode,
    $createExtendedTextNode as _createExtendedTextNode,
    $isExtendedTextNode as _isExtendedTextNode
  } from '$lib/nodes/ExtendedTextNode.js';

  import { DOCX_LAYOUT_COLUMN_CONFIGS } from '$lib/constants/exportLayouts.js';

  import LinkModal from '../modals/LinkModal.svelte';
  import InsertTableModal from '../modals/InsertTableModal.svelte';
  import FindReplaceModal from '../modals/FindReplaceModal.svelte';
  import TableCellActionMenu from './TableCellActionMenu.svelte';
  import FloatingModifyHighlightToolbar from './FloatingModifyHighlightToolbar.svelte';
  import notificationStore from '$lib/stores/notificationStore.js';

  export let initialJson = null;
  export let editable = true;
  export let placeholder = 'Enter text...';
  export let activeLayout = 'Layout1'; // New prop
  export let toolbarConfig = {
    undo: true,
    redo: true,
    blockType: true,
    bold: true,
    italic: true,
    underline: true,
    strikethrough: true,
    link: true,
    insertMenu: true,
    indent: true,
    outdent: true,
    align: true,
    textColor: true,
    highlight: true,
    clearFormatting: true,
    search: true
  };
  export let enableTableCellMenu = false;
  export let enableTableCellResize = false;
  export let enableSearch = false;
  export let enableFloatingToolbar = true;
  export let enableSegmentPlayback = false; // New prop
  export let backgroundClass = 'bg-white dark:bg-surface-2';
  export let documentPath = null;
  export let initialHighlights = [];
  export let documentHighlights = [];
  export let externalHighlightedRowIndex = -1; // Prop to allow external highlighting

  let editorWrapper;
  let editorContainer;
  let editor = null;
  let isReady = false;
  let isFocused = false; // Track focus state
  let internalCursorRowIndex = -1; // Track local cursor position
  let unregisterListeners = () => {};
  let historyState = createEmptyHistoryState();
  let savedSelection = null;
  let canUndo = false;
  let canRedo = false;

  let isBold = false; let isItalic = false; let isUnderline = false; let isStrikethrough = false;
  let isLink = false;
  let showLinkModal = false;
  let currentModalUrl = '';
  let isEditingLink = false;

  let showSearchBox = false;
  let searchTerm = '';
  let searchResults = [];
  let currentSearchResultIndex = -1;
  let showFindReplaceModal = false;
  let showSearchOptionsDropdown = false;
  let searchOptionsDropdownRef;

  const SEARCH_MATCH_BACKGROUND_LIGHT = 'rgba(255, 215, 0, 0.4)';
  const SEARCH_MATCH_BACKGROUND_DARK = 'rgba(75, 125, 175, 0.4)';

  let blockType = 'paragraph';
  let isBlockDropdownOpen = false;
  let blockDropdownRef;

  let isInsertDropdownOpen = false;
  let insertDropdownRef;

  let showInsertTableModal = false;

  let showTableCellMenu = false;
  let tableCellMenuPosition = { top: 0, left: 0 };
  let activeTableCellKey = null;

  let searchUiContainerElement;
  let searchToggleButtonElement;
  let searchInputRef;

  let latestScrollTargetKey = null; // New component-level variable
  let areHighlightsReady = false; // Track if highlights have been loaded from backend
  let areNodesReady = false; // Track if initial nodes have been loaded into Lexical

  let isResizing = false;
  let resizeDirection = null;
  let resizeTargetCellKey = null;
  let resizeStartPos = { x: 0, y: 0 };
  let resizerLineStyle = 'display: none;';

  let showModifyToolbar = false;
  let modifyToolbarPosition = { top: 0, left: 0 };
  let clickedNodeKey = null;

  let hoveredRowKey = null;
  let playButtonPosition = { top: 0, left: 0 };
  let showPlayButton = false;

  const MIN_COLUMN_WIDTH = 50;


  export const editorNodes = [
    ExtendedTextNode,
    { replace: TextNode, with: (node) => _createExtendedTextNode(node.getTextContent()) },
    RootNode, ParagraphNode, LineBreakNode,
    HeadingNode, QuoteNode, CodeNode,
    ListNode, ListItemNode,
    LinkNode,
    TableNode, TableRowNode, TableCellNode
  ];

  function handleShortcut(event) {
      if (!editable) return;
      const mod = event.metaKey || event.ctrlKey;

      if (mod && event.altKey) {
            const map = { '0': 'paragraph', '1': 'h1', '2': 'h2', '3': 'h3', '4': 'ul', '5': 'ol', 'q': 'quote', 'c': 'code' };
            const key = event.code && event.code.startsWith('Digit') ? event.code.slice(5) : event.key.toLowerCase();
            const type = map[key];
            if (type) { selectBlockType(type); event.preventDefault(); return; }
        }
        if (mod && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'k') {
            event.preventDefault();
            toggleLink();
            return;
        }
  }

  function toggleBlockDropdown() {
    if (!editable) return;
    isBlockDropdownOpen = !isBlockDropdownOpen;
  }

  function selectBlockType(type) {
    handleBlockTypeChange({ target: { value: type } });
    isBlockDropdownOpen = false;
  }

  function toggleInsertDropdown() {
      if (!editable) return;
      isInsertDropdownOpen = !isInsertDropdownOpen;
  }

  function openInsertTableDialog() {
      if (!editable) return;
      closeTableCellMenu(false);
      showInsertTableModal = true;
      isInsertDropdownOpen = false;
  }

  function handleInsertTableConfirm(event) {
      if (!editor || !isReady || !editor.isEditable()) return;
      const { rows, columns } = event.detail;
      editor.dispatchCommand(INSERT_TABLE_COMMAND, { rows: String(rows), columns: String(columns) });
      showInsertTableModal = false;
  }

  async function loadHighlights() {
    if (!documentPath) {
        areHighlightsReady = true;
        return;
    }
    try {
      const highlightsJson = await invoke('load_lexical_highlights', {
        args: {
          projectId: get(project).id,
          documentPath,
        }
      });
      if (highlightsJson && editor) {
        const highlights = JSON.parse(highlightsJson);
        editor.update(() => {
          for (const highlight of highlights) {
            const node = _getNodeByKey(highlight.nodeKey);
            if (_isExtendedTextNode(node)) {
              node.setStyle(`background-color: ${highlight.color}`);
              node.setHighlightId(highlight.id);
            }
          }
        });
      }
    } catch (error) {
      console.error('Error loading highlights:', error);
    } finally {
        areHighlightsReady = true;
    }
  }


  let selectedAlignment = 'left';
  let selectedTextColor = '#000000';
  let selectedHighlightColor = 'transparent';

  const isMac = typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC');
  const optLabel = isMac ? 'Opt' : 'Alt';
  const modLabel = isMac ? '⌘' : 'Ctrl';

  const blockTypeOptions = [
    { value: 'paragraph', label: 'Normal',        shortcut: `${modLabel}+${optLabel}+0` },
    { value: 'h1',        label: 'Heading 1',     shortcut: `${modLabel}+${optLabel}+1` },
    { value: 'h2',        label: 'Heading 2',     shortcut: `${modLabel}+${optLabel}+2` },
    { value: 'h3',        label: 'Heading 3',     shortcut: `${modLabel}+${optLabel}+3` },
    { value: 'ul',        label: 'Bullet List',   shortcut: `${modLabel}+${optLabel}+4` },
    { value: 'ol',        label: 'Numbered List', shortcut: `${modLabel}+${optLabel}+5` },
    { value: 'quote',     label: 'Quote',         shortcut: `${modLabel}+${optLabel}+Q` },
    { value: 'code',      label: 'Code Block',    shortcut: `${modLabel}+${optLabel}+C` }
  ];

  const insertOptions = [
      { value: 'table', label: 'Table', action: openInsertTableDialog, icon: `
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 bi bi-table" fill="currentColor" viewBox="0 0 16 16">
        <path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm15 2h-4v3h4zm0 4h-4v3h4zm0 4h-4v3h3a1 1 0 0 0 1-1zm-5 3v-3H6v3zm-5 0v-3H1v2a1 1 0 0 0 1 1zm-4-4h4V8H1zm0-4h4V4H1zm5-3v3h4V4zm4 4H6v3h4z"/>
      </svg>
    ` },
  ];

  const blockTypeIcons = {
    paragraph: `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path d="M4 4h12v2H4V4zM4 8h8v2H4V8zM4 12h12v2H4v-2zM4 16h8v2H4v-2z"/></svg>`,
    h1:        `<span class="inline-block w-4 text-xs font-semibold">H1</span>`,
    h2:        `<span class="inline-block w-4 text-xs font-semibold">H2</span>`,
    h3:        `<span class="inline-block w-4 text-xs font-semibold">H3</span>`,
    ul:        `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path d="M4 5h2v2H4V5zM8 5h8v2H8V5zM4 9h2v2H4V9zM8 9h8v2H8V9zM4 13h2v2H4V13zM8 13h8v2H8V13z"/></svg>`,
    ol:        `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path d="M3 5h2v2H3V5zM8 5h8v2H8V5zM3 9h2v2H3V9zM8 9h8v2H8V9zM3 13h2v2H3V13zM8 13h8v2H8V13z"/></svg>`,
    check:     `<span class="inline-block w-4 text-xs">☑</span>`,
    quote:     `<span class="inline-block w-4 text-xs">❝</span>`,
    code:      `<span class="inline-block w-4 text-xs"></></span>`
  };
  const alignmentOptions = [ { value: 'left', label: 'Left' }, { value: 'center', label: 'Center' }, { value: 'right', label: 'Right' }, { value: 'justify', label: 'Justify' } ];
  const alignmentIcons = {
    left:    `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M3 4h14v2H3V4zM3 8h10v2H3V8zM3 12h14v2H3v-2zM3 16h10v2H3v-2z" clip-rule="evenodd"/></svg>`,
    center:  `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M5 4h10v2H5V4zM3 8h14v2H3V8zM5 12h10v2H5v-2zM3 16h14v2H3v-2z" clip-rule="evenodd"/></svg>`,
    right:   `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M3 4h14v2H3V4zM7 8h10v2H7V8zM3 12h14v2H3v-2zM7 16h10v2H7v-2z" clip-rule="evenodd"/></svg>`,
    justify: `<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M3 4h14v2H3V4zM3 8h14v2H3V8zM3 12h14v2H3v-2zM3 16h14v2H3v-2z" clip-rule="evenodd"/></svg>`
  };

  let isAlignDropdownOpen = false;
  let alignmentDropdownRef;
  function toggleAlignDropdown() {
    if (!editable) return;
    isAlignDropdownOpen = !isAlignDropdownOpen;
  }

  const colorOptions = [
    { value: '#000000', label: 'Black' }, { value: '#FF0000', label: 'Red' }, { value: '#000080', label: 'Navy' },
    { value: '#228B22', label: 'Forest Green' }, { value: '#FF8C00', label: 'Dark Orange' }, { value: '#800080', label: 'Purple' },
    { value: '#008B8B', label: 'Teal' }, { value: 'transparent', label: 'Default'}
  ];
  import { HIGHLIGHT_OPTIONS_WITH_NONE } from '$lib/constants/highlightOptions.js';
  const highlightOptions = HIGHLIGHT_OPTIONS_WITH_NONE;
  let isHighlightDropdownOpen = false;
  let highlightDropdownRef;
  function toggleHighlightDropdown() {
    if (!editable) return;
    isHighlightDropdownOpen = !isHighlightDropdownOpen;
  }

  let isColorDropdownOpen = false;
  let colorDropdownRef;
  function toggleColorDropdown() {
    if (!editable) return;
    isColorDropdownOpen = !isColorDropdownOpen;
  }

  import { get } from 'svelte/store';
  import { project } from '$lib/stores/projectStore.js';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  function createInitialEditorState(jsonProp) {
      if (jsonProp && typeof jsonProp === 'string' && jsonProp.trim() !== '' && jsonProp !== 'null' && jsonProp !== 'undefined') {
          try {
              if (jsonProp.startsWith('{') && jsonProp.endsWith('}')) {
                  const parsedForValidation = JSON.parse(jsonProp);
                  if (parsedForValidation && parsedForValidation.root && Array.isArray(parsedForValidation.root.children)) {
                    return jsonProp;
                  } else {
                    console.warn(`[LexicalEditor] initialJson prop looks like JSON but lacks root.children. Using default empty state.`);
                  }
              } else {
                  console.warn(`[LexicalEditor] initialJson prop doesn't look like a JSON object string. Using default empty state.`);
              }
          } catch(e) {
              console.error(`[LexicalEditor] Error during basic validation of initialJson prop. Using default empty state.`, e);
          }
        }
      return JSON.stringify({
          root: { children: [{ type: 'paragraph', version: 1, children: [] }], direction: null, format: '', indent: 0, type: 'root', version: 1 }
      });
  }


  onMount(() => {
    console.log('[LexicalEditor] onMount. enableSegmentPlayback:', enableSegmentPlayback);
    const instanceId = Math.random().toString(36).substring(7);

    if (!editorContainer) {
        console.error(`[LexicalEditor ${instanceId}] Critical: editorContainer element not found on mount!`);
        return;
    }

    editor = createEditor({
      namespace: `SvelteLexicalEditor-${instanceId}`,
      nodes: editorNodes,
      theme: {
        paragraph: 'speech-plain-text',
        'live-transcription': 'text-gray-500 italic',
        text: { bold: 'font-bold', italic: 'italic', underline: 'underline', strikethrough: 'line-through' },
        heading: { h1: 'text-2xl font-bold mb-1 mt-2', h2: 'text-xl font-semibold mb-1 mt-1', h3: 'text-lg font-semibold mb-1' },
        list: {
          ul: 'list-disc list-inside mb-1 pl-4',
          ol: 'list-decimal list-inside mb-1 pl-4',
          checklist: 'list-none mb-1 pl-0',
          listitem: 'mb-0.5 pl-1 relative list-item-checkbox',
        },
        quote: 'border-l-4 border-gray-300 dark:border-border pl-2 italic my-1',
        code: 'bg-gray-100 dark:bg-gray-700 dark:text-gray-200 font-mono p-0.5 my-0.5 text-sm block whitespace-pre-wrap',
        link: 'text-blue-600 dark:text-blue-400 underline cursor-pointer hover:text-blue-800 dark:hover:text-blue-300',
        table: 'editor-table w-full border-collapse border dark:border-border my-2 table-fixed',
        tableCell: 'editor-table-cell border dark:border-border px-2 py-1 align-top min-w-[50px] relative',
        tableCellHeader: 'editor-table-cell-header font-semibold bg-gray-100 dark:bg-gray-700 dark:text-gray-200 text-center',
        tableRow: 'editor-table-row',
        tableCellResizer: 'editor-table-cell-resizer',
        placeholder: 'lexical-placeholder-theme-class absolute top-0 left-0 text-gray-400 dark:text-gray-500 text-sm select-none pointer-events-none opacity-50 p-2',
        align_left: 'text-left', align_center: 'text-center', align_right: 'text-right', align_justify: 'text-justify',
      },
      onError: (error, editorInstance) => {
            console.error(`[LexicalEditor ${instanceId}] Editor Error:`, error);
      },
      editable: editable,
      historyState: historyState,
    });


    let initialStateString = createInitialEditorState(initialJson);
    try {
        const parsedState = editor.parseEditorState(initialStateString);
        editor.setEditorState(parsedState);
        areNodesReady = true;
    } catch (e) {
        console.error(`[LexicalEditor ${instanceId}] Failed to parse and set initial editor state:`, e);
        editor.update(() => {
            const root = _getRoot();
            root.clear();
            root.append(_createParagraphNode());
            areNodesReady = true;
        });
    }


    editor.setRootElement(editorContainer);

    editorContainer.addEventListener('click', (e) => {
        const anchor = e.target.closest('a');
        if (anchor) { e.preventDefault(); }
    }, true);

    editorContainer.addEventListener('pointerdown', handlePointerDownOnContainer);
    editorWrapper.addEventListener('pointermove', handlePointerHover);
    editorContainer.addEventListener('contextmenu', handleContextMenu, true);

    unregisterListeners = mergeRegister(
        editor.registerUpdateListener(({ editorState }) => {
            if (isReady) {
                try {
                    editorState.read(updateToolbarState);
                } catch (readError) {
                    console.error("Error reading editor state in update listener:", readError);
                }
                const jsonString = JSON.stringify(editorState.toJSON());
                dispatch('change', { jsonString: jsonString });
            }
            if (showTableCellMenu) {
                try {
                    editorState.read(() => {
                        const selection = _getSelection();
                        let show = false;
                        if (_isRangeSelection(selection) || _isTableSelection(selection)) {
                            const anchorNode = selection.anchor.getNode();
                            const cellNode = _findMatchingParent(anchorNode, _isTableCellNode);
                            show = cellNode?.getKey() === activeTableCellKey;
                        }
                        if (!show) {
                            closeTableCellMenu(false);
                        }
                    });
                } catch (readError) {
                    console.error("Error reading editor state in update listener (menu check):", readError);
                }
            }
        }),
        editor.registerCommand(SELECTION_CHANGE_COMMAND, () => {
            if (isReady && editor) {
                try {
                    editor.getEditorState().read(updateToolbarState);
                    if (enableFloatingToolbar) {
                        showModifyToolbar = false;
                        clickedNodeKey = null;
                    }
                } catch(readError) {
                    console.error("Error reading state on selection change:", readError);
                }
            }
            return false;
        }, COMMAND_PRIORITY_LOW),
        editor.registerCommand(
          CLICK_COMMAND,
          (payload) => {
            const event = payload;
            if (event.button !== 0 || !editor || !editor.isEditable()) return false;
            let linkNode = null;
            let clickedCell = null;
            try {
                editor.read(() => {
                  const domNode = event.target;
                  const targetNode = _getNearestNodeFromDOMNode(domNode);
                  if (targetNode) {
                      linkNode = _getNearestNodeOfType(targetNode, LinkNode);
                      clickedCell = _findMatchingParent(targetNode, _isTableCellNode);
                  }
                });
            } catch (readError) {
                console.error("Error reading editor state during CLICK command:", readError);
                return false;
            }
            if (linkNode) {
              console.log("Clicked on link node:", linkNode.getURL());
              currentModalUrl = linkNode.getURL();
              isEditingLink = true;
              showLinkModal = true;
              closeTableCellMenu(false);
              return true;
            }
            if (!clickedCell && showTableCellMenu) {
                closeTableCellMenu(false);
            }
            return false;
          },
          COMMAND_PRIORITY_LOW
        ),
        editor.registerCommand(
          CLICK_COMMAND,
          (payload) => {
            const event = payload;
            if (event.button !== 0 || !editor || !editor.isEditable()) return false;

            editor.update(() => {
                const selection = _getSelection();
                if (selection.isCollapsed()) {
                    const node = selection.anchor.getNode();
                    const parent = node.getParent();
                    if (_isExtendedTextNode(node) && node.getHighlightId()) {
                        const domElement = editor.getElementByKey(node.getKey());
                        if (domElement) {
                            const rect = domElement.getBoundingClientRect();
                            modifyToolbarPosition = {
                                top: rect.top - 40,
                                left: rect.left,
                            };
                            showModifyToolbar = true;
                            clickedNodeKey = node.getKey();
                        }
                    } else if (_isExtendedTextNode(parent) && parent.getHighlightId()) {
                        const domElement = editor.getElementByKey(parent.getKey());
                        if (domElement) {
                            const rect = domElement.getBoundingClientRect();
                            modifyToolbarPosition = {
                                top: rect.top - 40,
                                left: rect.left,
                            };
                            showModifyToolbar = true;
                            clickedNodeKey = parent.getKey();
                        }
                    }
                }
            });

            return false;
          },
          COMMAND_PRIORITY_LOW
        ),
        registerRichText(editor),
        registerHistory(editor, historyState, 300),
        registerList(editor),
        editor.registerCommand(
          KEY_ENTER_COMMAND,
          (event) => {
            if (!editor || !editor.isEditable()) {
              return false;
            }
            let shouldIntercept = false;
            try {
              editor.getEditorState().read(() => {
                const selection = _getSelection();
                if (_isRangeSelection(selection)) {
                  const anchorNode = selection.anchor.getNode();
                  const cellNode = _findMatchingParent(anchorNode, _isTableCellNode);
                  if (cellNode) {
                    shouldIntercept = true;
                  }
                }
              });
            } catch (readError) {
              console.error("Error reading state during Enter key check (High Priority):", readError);
              return false;
            }
            if (shouldIntercept) {
              event.preventDefault();
              editor.update(() => {
                const selection = _getSelection();
                if (_isRangeSelection(selection)) {
                  selection.insertNodes([_createParagraphNode()]);
                }
              });
              return true;
            }
            return false;
          },
          COMMAND_PRIORITY_HIGH
        ),
        editor.registerCommand(
            KEY_ENTER_COMMAND,
            (event) => {
                if (!editor || !editor.isEditable()) {
                    return false;
                }
                const selection = _getSelection();
                if (_isRangeSelection(selection)) {
                    event.preventDefault();
                    editor.update(() => {
                        const currentSelection = _getSelection();
                        if (_isRangeSelection(currentSelection)) {
                            currentSelection.insertParagraph();
                        }
                    });
                    return true;
                }
                return false;
            },
            COMMAND_PRIORITY_LOW
        ),
        editor.registerCommand(
            INSERT_TABLE_COMMAND,
            (payload) => {
                if (!editor.isEditable()) return false;
                closeTableCellMenu(false);
                const { rows, columns } = payload;
                const numRows = parseInt(rows, 10);
                const numCols = parseInt(columns, 10);
                if (isNaN(numRows) || isNaN(numCols) || numRows <= 0 || numCols <= 0) return true;
                editor.update(() => {
                    const selection = _getSelection();
                    if (!_isRangeSelection(selection)) return;

                    const newColWidths = Array(numCols).fill(MIN_COLUMN_WIDTH);
                    const tableNode = _createTableNode();
                    tableNode.setColWidths(newColWidths);

                    for (let i = 0; i < numRows; i++) {
                        const rowNode = _createTableRowNode();
                        for (let j = 0; j < numCols; j++) {
                            const cellNode = _createTableCellNode({ headerState: TableCellHeaderStates.NO_STATUS });
                            cellNode.append(_createParagraphNode());
                            rowNode.append(cellNode);
                        }
                        tableNode.append(rowNode);
                    }

                    const focusNode = selection.focus.getNode();
                    let parentBlock = _findMatchingParent(focusNode, (node) => _isElementNode(node) && !node.isInline());
                    if (!parentBlock) parentBlock = typeof focusNode.getTopLevelElement === 'function' ? focusNode.getTopLevelElement() : null;

                    if (parentBlock && parentBlock.isEmpty() && _isParagraphNode(parentBlock)) {
                        parentBlock.replace(tableNode);
                    } else {
                        _insertNodes([tableNode]);
                    }
                    const newParagraph = _createParagraphNode();
                    tableNode.insertAfter(newParagraph);
                    newParagraph.selectStart();
                });
                return true;
            },
            COMMAND_PRIORITY_EDITOR
        ),
        editor.registerCommand(
            KEY_TAB_COMMAND,
            (event) => {
                if (!editor || !editor.isEditable()) return false;
                const selection = _getSelection();
                if (!(_isRangeSelection(selection) || _isTableSelection(selection))) {
                  return false;
                }
                if (_isRangeSelection(selection)) {
                    const anchorNode = selection.anchor.getNode();
                    const cellNode = _findMatchingParent(anchorNode, _isTableCellNode);
                    if (cellNode) {
                        event.preventDefault();
                        editor.update(() => {
                            const tableNode = _findMatchingParent(cellNode, _isTableNode);
                            if (!_isTableNode(tableNode)) return;
                            const tableElement = editor.getElementByKey(tableNode.getKey());
                            if (!tableElement) return;

                            const cells = Array.from(tableElement.querySelectorAll('.editor-table-cell'));
                            const currentCellElement = editor.getElementByKey(cellNode.getKey());
                            const currentIndex = cells.findIndex(c => c === currentCellElement);

                            if (currentIndex !== -1) {
                                const nextIndex = event.shiftKey ? currentIndex - 1 : currentIndex + 1;
                                if (nextIndex >= 0 && nextIndex < cells.length) {
                                    const nextCellElement = cells[nextIndex];
                                    const nextCellNode = _getNearestNodeFromDOMNode(nextCellElement);
                                    if (_isTableCellNode(nextCellNode)) {
                                        nextCellNode.selectStart();
                                    }
                                } else {
                                    if (!event.shiftKey) tableNode.selectNext();
                                    else tableNode.selectPrevious();
                                }
                            }
                        });
                        return true;
                    }
                }
                return false;
            },
            COMMAND_PRIORITY_HIGH,
          ),
        editor.registerCommand(
            TOGGLE_LINK_COMMAND,
            (payload) => {
                if (!editor) return false;
                editor.update(() => {
                  const selection = _getSelection();
                  if (_isRangeSelection(selection)) {
                    if (payload === null) {
                      const linkNodes = new Set();
                      selection.getNodes().forEach(node => {
                        let linkParent = _findMatchingParent(node, _isLinkNode);
                        if (linkParent) linkNodes.add(linkParent);
                        if (_isLinkNode(node)) linkNodes.add(node);
                      });
                      linkNodes.forEach(linkNode => {
                        const children = linkNode.getChildren();
                        children.forEach(child => child.selectNext());
                        linkNode.replace(...children);
                      });
                    } else {
                        _wrapNodes(selection, () => _createLinkNode(payload));
                    }
                  }
                });
                return true;
            },
            COMMAND_PRIORITY_HIGH
        ),
        editor.registerCommand(FOCUS_COMMAND, () => {
            isFocused = true;
            updateToolbarState();
            return false;
        }, COMMAND_PRIORITY_LOW),
        editor.registerCommand(BLUR_COMMAND, () => {
            isFocused = false;
            updateToolbarState();
            return false;
        }, COMMAND_PRIORITY_LOW)
    );

    tick().then(() => {
      if (!editor) return;
      isReady = true;
      loadHighlights();
      if (editor.isEditable()) {
        setTimeout(() => { if(editor) editor.focus(); }, 0);
        try {
            editor.getEditorState().read(updateToolbarState);
        } catch(readError){
            console.error("Error reading state during initial toolbar update:", readError);
        }
      }
    });

    return () => {
      unregisterListeners();
      if (editorWrapper) {
          editorWrapper.removeEventListener('pointermove', handlePointerHover);
      }
      if (editorContainer) {
          editorContainer.removeEventListener('pointerdown', handlePointerDownOnContainer);
          editorContainer.removeEventListener('contextmenu', handleContextMenu, true);
          editorContainer.removeEventListener('click', (e) => {
              const anchor = e.target.closest('a');
              if (anchor) { e.preventDefault(); }
          }, true);
      }
      editor = null;
      isReady = false;
    };
  });

    export function updateLiveTranscriptionText(text, isFinal, startTime, endTime) {
    if (!editor || !isReady || !editable) return;

    editor.update(() => {
        const root = _getRoot();
        let lastParagraph = root.getLastChild();
        let livePara = null;

        // Check if the last paragraph is our dedicated live paragraph
        if (lastParagraph && _isParagraphNode(lastParagraph) && typeof lastParagraph.hasStyle === 'function' && lastParagraph.hasStyle('live-transcription')) {
            livePara = lastParagraph;
        } else {
            livePara = _createParagraphNode().setStyle('live-transcription');
            root.append(livePara);
        }

        if (isFinal) {
            // On final result, clear the live paragraph and append the final text.
            livePara.clear();
            const timestamp = `[${new Date(startTime * 1000).toISOString().substr(11, 12)} - ${new Date(endTime * 1000).toISOString().substr(11, 12)}]`;
            livePara.append(_createTextNode(timestamp + ' ' + text + ' '));
            // Then, remove the style so it becomes a normal paragraph.
            livePara.setStyle('');
            // And create a new, empty live paragraph for the next utterance.
            const newLivePara = _createParagraphNode().setStyle('live-transcription');
            root.append(newLivePara);
            newLivePara.selectEnd();
        } else {
            // For interim results, replace the content of the live paragraph.
            livePara.clear();
            livePara.append(_createTextNode(text));
            livePara.selectEnd();
        }
    });
  }

  export function resetEditorState(jsonString = null) {
    if (!editor) { console.warn("[LexicalEditor] resetEditorState called before editor initialized."); return; }
    console.log("[LexicalEditor] resetEditorState called.");
    closeTableCellMenu(false);
    areNodesReady = false;
    editor.update(() => {
      try {
        let newState;
        let stateToParse = jsonString;
        if (!stateToParse || typeof stateToParse !== 'string' || stateToParse.trim() === '' || stateToParse === 'null' || stateToParse === 'undefined') {
          stateToParse = JSON.stringify({
              root: { children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }], direction: null, format: '', indent: 0, type: 'root', version: 1 }
          });
        } else if (!stateToParse.startsWith('{') || !stateToParse.endsWith('}')) {
            console.warn("[LexicalEditor] resetEditorState received non-JSON object string, wrapping in paragraph.");
            const pNode = _createParagraphNode();
            pNode.append(_createTextNode(stateToParse));
            stateToParse = JSON.stringify({
                root: { children: [pNode.exportJSON()], direction: null, format: '', indent: 0, type: 'root', version: 1 }
            });
        }
        newState = editor.parseEditorState(stateToParse);
        editor.setEditorState(newState);

        historyState.undoStack = [];
        historyState.redoStack = [];
        areNodesReady = true;
      } catch (e) {
        console.error('[LexicalEditor] Error parsing JSON during resetEditorState:', e, "Attempted JSON:", jsonString?.substring(0, 100));
        try {
            editor.setEditorState(editor.parseEditorState(JSON.stringify({
                root: { children: [{ children: [], direction: null, format: '', indent: 0, type: 'paragraph', version: 1 }], direction: null, format: '', indent: 0, type: 'root', version: 1 }
            })));
            historyState.undoStack = [];
            historyState.redoStack = [];
            areNodesReady = true;
        } catch (fallbackError) {
            console.error('[LexicalEditor] CRITICAL: Failed to set even fallback state during resetEditorState:', fallbackError);
        }
      }
    });
  }


  $: if (editor && typeof editor.setEditable === 'function') {
    editor.setEditable(editable);
    if (!editable) {
        closeTableCellMenu(false);
    }
  }

  export function updateContent(newJsonString) {
    if (!editor) { console.warn('[LexicalEditor] updateContent called but editor not initialized.'); return; }
    if (!isReady) { console.warn('[LexicalEditor] updateContent called before editor is ready.'); return; }
    closeTableCellMenu(false);
    editor.update(() => {
      try {
          let parsedState;
          if (newJsonString && typeof newJsonString === 'string' && newJsonString.startsWith('{') && newJsonString.endsWith('}')) {
              parsedState = editor.parseEditorState(newJsonString);
              let isValid = false;
              try {
                  parsedState.read(() => {
                      const root = _getRoot();
                      isValid = !!root && root.getType() === 'root';
                  });
              } catch (readErr) {
                  console.error("Error validating parsed state in updateContent:", readErr);
                  isValid = false;
              }
              if (!isValid) {
                  console.error('[LexicalEditor] Invalid state structure after parsing in updateContent. Aborting.');
                  return;
              }
          } else {
              console.error('[LexicalEditor] Invalid JSON string format provided to updateContent:', newJsonString ? newJsonString.substring(0, 200) + '...' : 'null');
              return;
          }
        editor.setEditorState(parsedState, { tag: 'history-merge' });

      } catch (e) {
        console.error('[LexicalEditor] Failed to parse JSON in updateContent:', e);
        console.error('[LexicalEditor] Faulty JSON for updateContent:', newJsonString ? newJsonString.substring(0, 200) + '...' : 'null');
      }
    }, { tag: 'external' });
  }

  export function getScrollElement() {
      return editorWrapper;
  }

  export function getTopVisibleRowInfo() {
    if (!editorWrapper || !editor) return { index: -1, offset: 0 };
    
    const wrapperRect = editorWrapper.getBoundingClientRect();
    
    // Attempt fast path using elementFromPoint
    // We check a point slightly inside the wrapper to find the row at the top
    const centerX = wrapperRect.left + (wrapperRect.width / 2);
    const topY = wrapperRect.top + 5; // 5px down to avoid borders
    
    const elAtTop = document.elementFromPoint(centerX, topY);
    const rowAtTop = elAtTop?.closest('.editor-table-row');
    
    const rows = Array.from(editorWrapper.querySelectorAll('.editor-table-row'));
    
    if (rowAtTop) {
        const index = rows.indexOf(rowAtTop);
        if (index !== -1) {
            const rowRect = rowAtTop.getBoundingClientRect();
            return { index, offset: Math.round(rowRect.top - wrapperRect.top) };
        }
    }
    
    // Fallback to iteration with a stable threshold
    for (let i = 0; i < rows.length; i++) {
        const rowRect = rows[i].getBoundingClientRect();
        // Use a 2px threshold to ignore tiny slivers that might cause jitter
        if (rowRect.bottom > wrapperRect.top + 2) { 
            return { index: i, offset: Math.round(rowRect.top - wrapperRect.top) };
        }
    }
    
    return { index: -1, offset: 0 };
  }

  export function getCursorRowInfo() {
    if (!editorWrapper || !editor) return { index: -1, offset: 0, visible: false };
    
    let info = { index: -1, offset: 0, visible: false };
    
    editor.getEditorState().read(() => {
        const selection = _getSelection();
        if (_isRangeSelection(selection)) {
            const anchorNode = selection.anchor.getNode();
            const element = editor.getElementByKey(anchorNode.getKey());
            const row = element?.closest('.editor-table-row');
            
            if (row) {
                const wrapperRect = editorWrapper.getBoundingClientRect();
                const rowRect = row.getBoundingClientRect();
                const rows = Array.from(editorWrapper.querySelectorAll('.editor-table-row'));
                
                info.index = rows.indexOf(row);
                info.offset = Math.round(rowRect.top - wrapperRect.top);
                // Visible if the row is within the viewport
                info.visible = (rowRect.bottom > wrapperRect.top && rowRect.top < wrapperRect.bottom);
            }
        }
    });
    
    return info;
  }

  export function scrollToRow(index, offset) {
    if (!editorWrapper || !editor || index < 0) return;
    
    const rows = Array.from(editorWrapper.querySelectorAll('.editor-table-row'));
    if (index >= rows.length) return;
    
    const targetRow = rows[index];
    if (targetRow) {
        const wrapperRect = editorWrapper.getBoundingClientRect();
        const rowRect = targetRow.getBoundingClientRect();
        
        const currentOffset = rowRect.top - wrapperRect.top;
        const targetOffset = offset;
        
        const diff = currentOffset - targetOffset;
        
        // Only scroll if the difference is significant (more than 1 pixel) to avoid jitter
        if (Math.abs(diff) >= 1) {
            editorWrapper.scrollTop = Math.round(editorWrapper.scrollTop + diff);
        }
    }
  }

  function updateToolbarState() {
    if (!editor || !isReady) { return; }
    const selection = _getSelection();
    isBold = false; isItalic = false; isUnderline = false; isStrikethrough = false;
    isLink = false;
    blockType = 'paragraph';
    selectedAlignment = 'left';
    selectedTextColor = '#000000';
    selectedHighlightColor = 'transparent';

    if (_isRangeSelection(selection)) {
      isBold = selection.hasFormat('bold');
      isItalic = selection.hasFormat('italic');
      isUnderline = selection.hasFormat('underline');
      isStrikethrough = selection.hasFormat('strikethrough');
      selectedTextColor = _getSelectionStyleValueForProperty(selection, 'color', '#000000') || '#000000';
      selectedHighlightColor = _getSelectionStyleValueForProperty(selection, 'background-color', 'transparent') || 'transparent';

      const anchorNode = selection.anchor.getNode();
      if (anchorNode) {
          let element = _findMatchingParent(anchorNode, (node) => _isElementNode(node) && !node.isInline());
          if (!element) {
              let maybeTopLevel = anchorNode;
              while(maybeTopLevel && maybeTopLevel.getParent() && !_getRoot().is(maybeTopLevel.getParent())) {
                  maybeTopLevel = maybeTopLevel.getParent();
              }
              if (_isElementNode(maybeTopLevel) && !maybeTopLevel.isInline()) {
                  element = maybeTopLevel;
              }
              if (!element) {
                  element = _findMatchingParent(anchorNode, _isParagraphNode) || anchorNode.getTopLevelElement();
              }
          }

          if (element && typeof element.getType === 'function') {
            const type = element.getType();
            if (_isHeadingNode(element)) { blockType = element.getTag(); }
            else if (_isListItemNode(element)) {
                const parentList = _findMatchingParent(element, _isListNode);
                blockType = parentList ? parentList.getListType() : 'paragraph';
            }
            else if (_isTableCellNode(element)) {
                const firstChild = element.getFirstChild();
                if (_isHeadingNode(firstChild)) { blockType = firstChild.getTag(); }
                else if (_isListNode(firstChild)) { blockType = firstChild.getListType(); }
                else if (_isQuoteNode(firstChild)) { blockType = 'quote'; }
                else if (_isCodeNode(firstChild)) { blockType = 'code'; }
                else { blockType = 'paragraph'; }
            }
            else if (type === 'paragraph' || type === 'quote' || type === 'code') { blockType = type; }
            else { blockType = 'paragraph'; }

            let formatElement = element;
            if (_isTableCellNode(element)) {
                formatElement = element.getFirstChild();
            } else if (element.isInline?.()) {
                formatElement = element.getParent();
            }

            if (_isElementNode(formatElement) && typeof formatElement.getFormatType === 'function') {
                selectedAlignment = formatElement.getFormatType() || 'left';
            } else { selectedAlignment = 'left'; }
          } else { blockType = 'paragraph'; selectedAlignment = 'left'; isLink = false; }

          const nodeForLinkCheck = selection.isCollapsed() ? anchorNode : selection.anchor.getNode();
          const parentForLinkCheck = nodeForLinkCheck ? nodeForLinkCheck.getParent() : null;
          isLink = _isLinkNode(nodeForLinkCheck) || _isLinkNode(parentForLinkCheck);

          // Track current row index for glowing highlight
          if (editorWrapper) {
              const domNode = editor.getElementByKey(anchorNode.getKey());
              const row = domNode?.closest('.editor-table-row');
              if (row) {
                  const rows = Array.from(editorWrapper.querySelectorAll('.editor-table-row'));
                  const newIndex = rows.indexOf(row);
                  if (newIndex !== internalCursorRowIndex) {
                      internalCursorRowIndex = newIndex;
                      dispatch('cursorrowchange', { index: internalCursorRowIndex });
                  }
              } else {
                  internalCursorRowIndex = -1;
              }
          }

      } else { blockType = 'paragraph'; selectedAlignment = 'left'; isLink = false; internalCursorRowIndex = -1;}

    } else if (_isTableSelection(selection)) {
        blockType = 'paragraph'; selectedAlignment = 'left'; isLink = false;
        isBold = false; isItalic = false; isUnderline = false; isStrikethrough = false;
        selectedTextColor = '#000000'; selectedHighlightColor = 'transparent';
        internalCursorRowIndex = -1;
    } else {
        isBold = false; isItalic = false; isUnderline = false; isStrikethrough = false;
        isLink = false; blockType = 'paragraph'; selectedAlignment = 'left';
        selectedTextColor = '#000000'; selectedHighlightColor = 'transparent';
        internalCursorRowIndex = -1;
    }

    isBold = isBold; isItalic = isItalic; isUnderline = isUnderline; isStrikethrough = isStrikethrough;
    isLink = isLink; blockType = blockType; selectedAlignment = selectedAlignment;
    selectedTextColor = selectedTextColor; selectedHighlightColor = selectedHighlightColor;
    canUndo = historyState.undoStack.length > 0;
    canRedo = historyState.redoStack.length > 0;
  }

  // Reactive row highlighting logic
  $: if (editorWrapper && (internalCursorRowIndex !== undefined || externalHighlightedRowIndex !== undefined)) {
      const rows = editorWrapper.querySelectorAll('.editor-table-row');
      rows.forEach((row, i) => {
          const shouldGlow = (i === externalHighlightedRowIndex) || (i === internalCursorRowIndex && isFocused);
          if (shouldGlow) {
              row.classList.add('cursor-row-glow');
          } else {
              row.classList.remove('cursor-row-glow');
          }
      });
  }


  function formatText(formatType) { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(FORMAT_TEXT_COMMAND, formatType); }
  function alignElement(alignType) { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(FORMAT_ELEMENT_COMMAND, alignType); isAlignDropdownOpen = false;}
  function indentContent() { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(INDENT_CONTENT_COMMAND, undefined); }
  function outdentContent() { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(OUTDENT_CONTENT_COMMAND, undefined); }

  function applyStyle(styleName, value) {
      if (!editor || !isReady || !editor.isEditable()) return;
      editor.update(() => {
          const selection = _getSelection();
          if (_isTableSelection(selection)) {
              _normalizeSelection(selection);
          }
          const normalizedSelection = _getSelection();
          if (_isRangeSelection(normalizedSelection)) {
              _patchStyleText(normalizedSelection, { [styleName]: value || null });
          }
      });
  }

  function applyTextColor(color) {
      if (!editor || !editable) return;
      applyStyle('color', color === 'transparent' ? null : color);
      isColorDropdownOpen = false;
  }

  function applyHighlightColor(colorToApply) {
    if (!editor || !editable) return;
    editor.update(() => {
        const selection = _getSelection();
        if (_isTableSelection(selection)) {
            _normalizeSelection(selection);
        }
        const normalizedSelection = _getSelection();
        if (_isRangeSelection(normalizedSelection)) {
            const isDarkMode = document.documentElement.classList.contains('dark');
            const styles = {};

            if (colorToApply !== 'transparent') {
                styles['background-color'] = colorToApply;
                styles['color'] = isDarkMode ? '#111827' : '#000000';
            } else {
                styles['background-color'] = null;
                styles['color'] = null;
            }

            _patchStyleText(normalizedSelection, styles);

            const selectedNodes = normalizedSelection.getNodes();
            const newId = uuidv4();
            for (const node of selectedNodes) {
                let targetNode = node;
                if (targetNode.getParent() && _isExtendedTextNode(targetNode.getParent())) {
                     // Check if it's a segmented node within an ExtendedTextNode
                     targetNode = targetNode.getParent();
                }

                if (_isExtendedTextNode(targetNode)) {
                    const extendedNode = targetNode;
                    const currentHighlightId = extendedNode.getHighlightId();

                    if (colorToApply !== 'transparent') {
                        // Always assign a new ID for the new highlight range
                        extendedNode.setHighlightId(newId);
                    } else {
                        if (currentHighlightId !== null) {
                            extendedNode.setHighlightId(null);
                        }
                    }
                }
            }

            const allHighlights = gatherAllHighlights();
            updateAndSaveHighlights(allHighlights);
        }
    });
    isHighlightDropdownOpen = false;
}

function gatherAllHighlights() {
    const root = _getRoot();
    const allTextNodes = [];
    
    // 1. Collect ALL text nodes in document order to identify gaps correctly
    const visit = (node) => {
        if (_isExtendedTextNode(node)) {
            allTextNodes.push(node);
        } else if (_isElementNode(node)) {
            node.getChildren().forEach(visit);
        }
    };
    visit(root);

    if (allTextNodes.length === 0) return [];

    // Use latest highlights from store for metadata merging
    const currentHighlights = get(project).currentDocumentHighlights || [];
    const existingHighlightsMap = new Map(currentHighlights.map(h => [h.id, h]));

    // 2. Group into blocks that are contiguous in the document flow
    const blocks = [];
    let currentBlock = [];

    for (const node of allTextNodes) {
        const highlightId = node.getHighlightId();
        if (highlightId) {
            if (currentBlock.length > 0) {
                const lastNode = currentBlock[currentBlock.length - 1];
                // Group if they share the same ID. 
                // This allows highlights to span multiple paragraphs/nodes while remaining one annotation.
                if (lastNode.getHighlightId() === highlightId) {
                    currentBlock.push(node);
                } else {
                    blocks.push(currentBlock);
                    currentBlock = [node];
                }
            } else {
                currentBlock = [node];
            }
        } else {
            // Unhighlighted text node! This is a gap that forces a split.
            if (currentBlock.length > 0) {
                blocks.push(currentBlock);
                currentBlock = [];
            }
        }
    }
    if (currentBlock.length > 0) blocks.push(currentBlock);

    // 3. Normalize IDs and merge metadata
    const finalHighlights = [];
    const seenIds = new Set();

    for (let i = 0; i < blocks.length; i++) {
        const block = blocks[i];
        if (block.length === 0) continue;
        
        const firstNode = block[0];
        let highlightId = firstNode.getHighlightId();
        const style = firstNode.getStyle();
        // Robust regex to capture color regardless of semicolons
        const colorMatch = style.match(/background-color:\s*([^;]+)/);
        const color = colorMatch ? colorMatch[1].trim() : 'transparent';

        const originalId = highlightId;
        if (seenIds.has(highlightId)) {
            // This is a disjoint part of an original highlight.
            // Assign a new ID to ensure it's a separate annotation entry.
            highlightId = uuidv4();
            block.forEach(n => n.setHighlightId(highlightId));
        } else {
            seenIds.add(highlightId);
        }

        const metadata = existingHighlightsMap.get(originalId);
        
        finalHighlights.push({
            id: highlightId,
            text: block.map(n => n.getTextContent()).join(''),
            nodeKey: firstNode.getKey(), 
            color: color,
            tags: metadata ? [...(metadata.tags || [])] : [],
            comments: metadata ? [...(metadata.comments || [])] : [],
            documentOrder: i // Assign order based on current sequence in document
        });
    }

    return finalHighlights;
}

function updateAndSaveHighlights(highlights) {
    if (!editor || !documentPath) return;

    dispatch('highlightschange', { highlights });
}

function scrollToHighlight(id) {
    if (!id || !editor) return;
    
    let attempts = 0;
    const maxAttempts = 15; // Increased attempts
    
    const tryScroll = () => {
        // Recursive function to find node by highlight ID - MUST be called inside tryScroll to retry search
        const findNodeKey = () => {
            let foundKey = null;
            editor.getEditorState().read(() => {
                const root = _getRoot();
                const nodesToVisit = [root];
                while(nodesToVisit.length > 0) {
                    const node = nodesToVisit.pop();
                    if (_isExtendedTextNode(node) && node.getHighlightId() === id) {
                        foundKey = node.getKey();
                        break;
                    }
                    if (node.getChildren) {
                        const children = node.getChildren();
                        for (let i = children.length - 1; i >= 0; i--) {
                            nodesToVisit.push(children[i]);
                        }
                    }
                }
            });
            return foundKey;
        };

        const targetNodeKey = findNodeKey();

        if (targetNodeKey) {
            const domElement = editor.getElementByKey(targetNodeKey);
            if (domElement) {
                console.log(`[LexicalEditor] Scrolling to highlight ${id} (Node ${targetNodeKey}) after ${attempts} attempts`);
                domElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
                // Pulse effect
                domElement.style.transition = 'outline 0.3s ease';
                domElement.style.outline = '4px solid #3b82f6';
                domElement.style.outlineOffset = '2px';
                setTimeout(() => {
                    if (domElement) domElement.style.outline = 'none';
                }, 2000);
                
                // Success - clear the request
                project.update(p => ({ ...p, requestedHighlightId: null }));
                return;
            }
        }

        // If either node not found OR DOM element not found, retry
        if (attempts < maxAttempts) {
            attempts++;
            setTimeout(tryScroll, 150); // Slightly longer delay between retries
        } else {
            console.warn(`[LexicalEditor] Failed to scroll to highlight ${id} after ${maxAttempts} attempts. Node found: ${!!targetNodeKey}`);
            project.update(p => ({ ...p, requestedHighlightId: null }));
        }
    };
    
    // Give Lexical a moment to finish its current update cycle if any
    setTimeout(tryScroll, 50);
}

// Trigger scroll when editor is ready AND highlights are loaded AND nodes are loaded AND there is a requested ID
$: if ($project.requestedHighlightId && isReady && areHighlightsReady && areNodesReady) {
    scrollToHighlight($project.requestedHighlightId);
}

  function handleBlockTypeChange(event) {
    const type = event.target.value; if (!editor || !isReady || !editor.isEditable()) return;
    if (type === 'paragraph' || type === 'h1' || type === 'h2' || type === 'h3' || type === 'quote' || type === 'code') {
      editor.update(() => {
          const selection = _getSelection();
          if (_isTableSelection(selection)) {
              _normalizeSelection(selection);
          }
          const normalizedSelection = _getSelection();
          if (_isRangeSelection(normalizedSelection)) {
            const createNodeFn = type === 'paragraph' ? _createParagraphNode
              : type === 'h1' ? () => _createHeadingNode('h1')
              : type === 'h2' ? () => _createHeadingNode('h2')
              : type === 'h3' ? () => _createHeadingNode('h3')
              : type === 'quote' ? _createQuoteNode
              : type === 'code' ? _createCodeNode
              : null;
            if (createNodeFn) { _setBlocksType(normalizedSelection, createNodeFn); }
          }
      });
    } else if (type === 'ul') { editor.dispatchCommand(INSERT_UNORDERED_LIST_COMMAND, undefined); }
      else if (type === 'ol') { editor.dispatchCommand(INSERT_ORDERED_LIST_COMMAND, undefined); }
  }

  function clearFormatting() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.update(() => {
        const selection = _getSelection();
        if (_isRangeSelection(selection)) {
            try {
                selection.getNodes().forEach(node => {
                    let targetNode = node;
                    if (selection.isCollapsed() && _isLinkNode(node.getParent())) {
                        targetNode = node.getParent();
                    }

                    if (_isExtendedTextNode(targetNode)) {
                        const highlightId = targetNode.getHighlightId();
                        if (highlightId) {
                             dispatch('highlightevent', {
                                type: 'remove',
                                id: highlightId,
                                nodeKey: targetNode.getKey(),
                                color: 'transparent' // Part of highlight data
                            });
                            targetNode.setHighlightId(null);
                        }
                    }

                    if (_isTextNode(targetNode)) {
                        targetNode.setFormat(0).setStyle('');
                    } else if (_isLinkNode(targetNode)) {
                        const children = targetNode.getChildren();
                        children.forEach(child => {
                            if(_isTextNode(child)) {
                                if (_isExtendedTextNode(child)) {
                                    const childHighlightId = child.getHighlightId();
                                    if (childHighlightId) {
                                        dispatch('highlightevent', { type: 'remove', id: childHighlightId, nodeKey: child.getKey(), color: 'transparent' });
                                        child.setHighlightId(null);
                                    }
                                }
                                child.setFormat(0).setStyle('');
                            }
                        });
                        targetNode.replace(...children);
                    }
                });
                _setBlocksType(selection, () => _createParagraphNode());
            } catch (error) {
                console.error("[LexicalEditor] Error during clearFormatting (Range):", error);
            }
        } else if (_isTableSelection(selection)) {
            try {
                _normalizeSelection(selection);
                const rangeSelection = _getSelection();
                if (_isRangeSelection(rangeSelection)) {
                    rangeSelection.getNodes().forEach(node => {
                         if (_isExtendedTextNode(node)) {
                            const highlightId = node.getHighlightId();
                            if (highlightId) {
                                 dispatch('highlightevent', { type: 'remove', id: highlightId, nodeKey: node.getKey(), color: 'transparent' });
                                node.setHighlightId(null);
                            }
                        }
                        if (_isTextNode(node)) {
                            node.setFormat(0).setStyle('');
                        } else if (_isElementNode(node)) {
                        }
                    });
                }
            } catch (error) {
                console.error("[LexicalEditor] Error during clearFormatting (Table):", error);
            }
        }
    });
  }

  function undo() { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(UNDO_COMMAND, undefined); }
  function redo() { if (!editor || !isReady || !editor.isEditable()) return; editor.dispatchCommand(REDO_COMMAND, undefined); }

  async function toggleLink() {
      if (!editor || !editable) return;
      closeTableCellMenu(false);
      currentModalUrl = ''; isEditingLink = false;
      editor.focus(); await tick();
      try {
          editor.getEditorState().read(() => {
            const selection = _getSelection();
            if (_isRangeSelection(selection)) {
              savedSelection = selection.clone();
              const node = selection.anchor.getNode();
              const parent = node.getParent();
              if (_isLinkNode(parent)) { currentModalUrl = parent.getURL(); isEditingLink = true; }
              else if (_isLinkNode(node)) { currentModalUrl = node.getURL(); isEditingLink = true; }
              else { currentModalUrl = ''; isEditingLink = false; }
            } else {
                savedSelection = null;
            }
          });
      } catch(readError) {
          console.error("Error reading state for toggleLink:", readError);
          return;
      }
      if (!isEditingLink && (!savedSelection || savedSelection.isCollapsed())) {
          console.warn("Cannot toggle link without a text selection or editing an existing link.");
          savedSelection = null;
          return;
      }
      showLinkModal = true;
  }

  function handleLinkConfirm(event) {
      const { url } = event.detail;
      if (!editor) return;
      editor.focus();
      editor.update(() => {
          if (savedSelection) { _setSelection(savedSelection.clone()); }
          if (url && url.trim() !== '') {
              console.log("Dispatching TOGGLE_LINK_COMMAND with URL:", url);
              editor.dispatchCommand(TOGGLE_LINK_COMMAND, url.trim());
          } else {
              console.log("Dispatching TOGGLE_LINK_COMMAND with null (empty URL received).");
              editor.dispatchCommand(TOGGLE_LINK_COMMAND, null);
          }
      });
      savedSelection = null;
      showLinkModal = false;
  }

  function handleLinkDelete() {
      if (!editor) return;
      editor.focus();
      editor.update(() => {
          if (savedSelection) { _setSelection(savedSelection.clone()); }
          console.log("Dispatching TOGGLE_LINK_COMMAND with null to remove link.");
          editor.dispatchCommand(TOGGLE_LINK_COMMAND, null);
      });
      savedSelection = null;
      showLinkModal = false;
  }


  function handleContextMenu(event) {
      if (!editor || !editor.isEditable()) return;
      // Prevent table cell menu if disabled
      if (!enableTableCellMenu) {
          closeTableCellMenu(false);
          return;
      }

      let tableCellNode = null;
      let domNode = null;
      try {
          editor.read(() => {
              const targetNode = _getNearestNodeFromDOMNode(event.target);
              if (targetNode) {
                  tableCellNode = _findMatchingParent(targetNode, _isTableCellNode);
                  if (tableCellNode) {
                      domNode = editor.getElementByKey(tableCellNode.getKey());
                  }
              }
          });
      } catch (readError) {
          console.error("Error reading editor state during context menu:", readError);
          closeTableCellMenu(false);
          return;
      }
      if (tableCellNode && domNode) {
          if (isResizing) {
              event.preventDefault();
              return;
          }
          event.preventDefault();
          openTableCellMenu(tableCellNode.getKey(), event, domNode);
      } else {
          closeTableCellMenu(false);
      }
  }

  function openTableCellMenu(nodeKey, event, domNode) {
    if (!domNode) return;
    const menuWidth = 220;
    const menuHeight = 300;
    let left = event.clientX;
    let top = event.clientY;
    if (left + menuWidth > window.innerWidth) {
      left = window.innerWidth - menuWidth - 10;
    }
    left = Math.max(5, left);
    if (top + menuHeight > window.innerHeight) {
      top = window.innerHeight - menuHeight - 10;
    }
    top = Math.max(5, top);

    activeTableCellKey = nodeKey;
    tableCellMenuPosition = { top, left };
    showTableCellMenu = true;
  }

  function closeTableCellMenu(clearSelection = true) {
      if (showTableCellMenu) {
          showTableCellMenu = false;
          activeTableCellKey = null;
      }
  }

  function handleTableCellMenuClose(event) {
      showTableCellMenu = false;
      activeTableCellKey = null;
  }


  const MIN_WIDTH = 50;
  const MIN_HEIGHT = 30;
  const RESIZE_BORDER_WIDTH = 10;

  function detectResizeTarget(event) {
      const target = event.target;
      if (!(target instanceof HTMLElement)) return null;
      const cellElement = target.closest('.editor-table-cell');
      if (!cellElement) return null;

      const rect = cellElement.getBoundingClientRect();
      const zoom = calculateZoomLevel(editorContainer) || 1;
      const x = (event.clientX - rect.left);
      const y = (event.clientY - rect.top);
      const w = rect.width;
      const h = rect.height;
      const bw = RESIZE_BORDER_WIDTH;

      // Check Right Edge (standard)
      if (Math.abs(x - w) <= bw) return { element: cellElement, direction: 'col' };

      // Check Left Edge (resize previous column)
      if (Math.abs(x) <= bw) {
          const prev = cellElement.previousElementSibling;
          if (prev && prev.classList.contains('editor-table-cell')) {
              return { element: prev, direction: 'col' };
          }
      }

      // Check Bottom Edge (standard)
      if (Math.abs(y - h) <= bw) return { element: cellElement, direction: 'row' };

      // Check Top Edge (resize previous row)
      if (Math.abs(y) <= bw) {
          const row = cellElement.parentElement;
          const prevRow = row?.previousElementSibling;
          if (prevRow) {
              // Any cell in previous row works to identify the row index
              const prevCell = Array.from(prevRow.children).find(c => c.classList.contains('editor-table-cell'));
              if (prevCell) return { element: prevCell, direction: 'row' };
          }
      }

      return null;
  }

  function handlePointerHover(event) {
      if (!editorContainer || !editorWrapper || isResizing || !enableSegmentPlayback || !editor) return;
      
      const x = event.clientX;
      const y = event.clientY;
      const wrapperRect = editorWrapper.getBoundingClientRect();
      
      // Check if we are in the gutter (first 60px of the wrapper)
      const isWithinGutterX = x >= wrapperRect.left && x <= wrapperRect.left + 60;
      
      // If we are in the gutter, scan slightly to the right to find the row at this Y level
      const scanX = isWithinGutterX ? wrapperRect.left + 80 : x;
      
      // Use elementsFromPoint to find the row
      const elements = document.elementsFromPoint(scanX, y);
      const rowElement = elements.find(el => 
          el.classList?.contains('editor-table-row') || 
          el.closest?.('.editor-table-row')
      );
      const actualRow = rowElement?.classList?.contains('editor-table-row') ? rowElement : rowElement?.closest?.('.editor-table-row');
      
      if (actualRow) {
          // Skip if this is a header row
          const isHeaderRow = actualRow.querySelector('th') || actualRow.querySelector('.editor-table-cell-header');
          if (isHeaderRow) {
              if (showPlayButton) {
                  showPlayButton = false;
                  hoveredRowKey = null;
              }
              return;
          }

          let rowKey = actualRow.getAttribute('data-lexical-key');
          
          // Fallback if data-lexical-key is missing from DOM
          if (!rowKey) {
              editor.read(() => {
                  const node = _getNearestNodeFromDOMNode(actualRow);
                  if (node) rowKey = node.getKey();
              });
          }

          if (rowKey && rowKey !== hoveredRowKey) {
              // console.log('[LexicalEditor] Row detected. Key:', rowKey);
              hoveredRowKey = rowKey;
              const rect = actualRow.getBoundingClientRect();
              
              // Position button in the gutter (left: 20px relative to wrapper)
              playButtonPosition = {
                  top: rect.top - wrapperRect.top + editorWrapper.scrollTop + (rect.height / 2),
                  left: 20,
              };
              showPlayButton = true;
          }
      } else {
          // If NOT over a row, we hide if we are also NOT over the play button itself
          // and NOT in the gutter (to prevent flickering)
          const currentElements = document.elementsFromPoint(x, y);
          const isOverPlayButton = currentElements.some(el => el.classList?.contains('play-segment-hover-btn'));
          
          if (!isOverPlayButton && !isWithinGutterX) {
              if (showPlayButton) {
                  showPlayButton = false;
                  hoveredRowKey = null;
              }
          }
      }

      // If table cell resizing is disabled, do nothing
      if (!enableTableCellResize) return;

      const targetInfo = detectResizeTarget(event);
      if (targetInfo) {
          editorContainer.style.cursor = targetInfo.direction === 'col' ? 'col-resize' : 'row-resize';
      } else {
          editorContainer.style.cursor = '';
      }
  }

  function parseTimestamp(text) {
      // Flexible format: (HH:)?MM:SS.mmm - (HH:)?MM:SS.mmm
      const timePattern = /(?:(\d{1,2}):)?(\d{1,2}):(\d{1,2}(?:\.\d{1,3})?)/;
      const regex = new RegExp(`^${timePattern.source}\\s*-\\s*${timePattern.source}$`);
      const match = text.match(regex);
      if (!match) return null;
      
      const startTime = timeStringToSeconds(match[1], match[2], match[3]);
      const endTime = timeStringToSeconds(match[4], match[5], match[6]);
      return { startTime, endTime };
  }
  
  function timeStringToSeconds(h, m, s) {
      const hours = parseInt(h || '0', 10);
      const minutes = parseInt(m || '0', 10);
      const seconds = parseFloat(s || '0');
      return hours * 3600 + minutes * 60 + seconds;
  }

  function handlePlaySegmentClick() {
      if (!hoveredRowKey || !editor) return;
      
      editor.getEditorState().read(() => {
          const rowNode = _getNodeByKey(hoveredRowKey);
          if (_isTableRowNode(rowNode)) {
              const cells = rowNode.getChildren();
              let timestampText = '';
              for (const cell of cells) {
                  if (_isTableCellNode(cell)) {
                      const text = cell.getTextContent().trim();
                      // Flexible check for timestamp pattern
                      if (/(?:\d{1,2}:)?\d{1,2}:\d{1,2}(?:\.\d{1,3})?\s*-\s*(?:\d{1,2}:)?\d{1,2}:\d{1,2}(?:\.\d{1,3})?/.test(text)) {
                          timestampText = text;
                          break;
                      }
                  }
              }

              if (timestampText) {
                  const parsed = parseTimestamp(timestampText);
                  if (parsed) {
                      dispatch('playsegment', parsed);
                  } else {
                      notificationStore.add('Invalid timestamp values. Expected format: MM:SS.mmm or HH:MM:SS.mmm', 'error');
                  }
              } else {
                  notificationStore.add('Could not find a valid timestamp in this row.', 'error');
              }
          }
      });
  }


  function handlePointerDownOnContainer(event) {
      if (!editable || !editor || !editorContainer) return;

      // Prevent table cell resizing if disabled
      if (!enableTableCellResize) return;

      const targetInfo = detectResizeTarget(event);
      if (targetInfo) {
          const { element, direction } = targetInfo;
          let cellNodeKeyToResize = null;
          try {
              editor.read(() => {
                  const cellNode = _getNearestNodeFromDOMNode(element);
                  if (_isTableCellNode(cellNode)) {
                      cellNodeKeyToResize = cellNode.getKey();
                  }
              });
          } catch (readError) {
              console.error("Error reading editor state during resize check:", readError);
              return;
          }

          if (!cellNodeKeyToResize) return;

          event.preventDefault();
          event.stopPropagation();
          isResizing = true;
          resizeDirection = direction;
          resizeTargetCellKey = cellNodeKeyToResize;
          resizeStartPos = { x: event.clientX, y: event.clientY };
          updateResizerLine(event.clientX, event.clientY);
          document.body.style.cursor = direction === 'col' ? 'col-resize' : 'row-resize';
      }
  }

  function handlePointerMove(event) {
      if (!isResizing || !editable || !editorContainer) return;
      event.preventDefault();
      event.stopPropagation();
      updateResizerLine(event.clientX, event.clientY);
  }

  function updateResizerLine(clientX, clientY) {
      if (!editorContainer || !resizeTargetCellKey) {
          resizerLineStyle = 'display: none;';
          return;
      }
      const containerRect = editorContainer.getBoundingClientRect();
      const zoom = calculateZoomLevel(editorContainer);
      const currentX = clientX / zoom;
      const currentY = clientY / zoom;
      const targetCellElement = editorContainer.querySelector(`[data-lexical-key="${resizeTargetCellKey}"]`);
      const tableElement = targetCellElement?.closest('.editor-table');
      if (!tableElement) {
          resizerLineStyle = 'display: none;';
          return;
      };
      const tableRect = tableElement.getBoundingClientRect();

      const tableRelativeLeft = (tableRect.left - containerRect.left) / zoom;
      const tableRelativeTop = (tableRect.top - containerRect.top) / zoom;
      const tableRelativeWidth = tableRect.width / zoom;
      const tableRelativeHeight = tableRect.height / zoom;
      const relativeX = (clientX - containerRect.left) / zoom;
      const relativeY = (clientY - containerRect.top) / zoom;

      if (resizeDirection === 'col') {
          const left = Math.max(tableRelativeLeft, relativeX);
          resizerLineStyle = `
              display: block; position: absolute;
              top: ${tableRelativeTop}px;
              left: ${left}px;
              height: ${tableRelativeHeight}px; width: 2px;
              background-color: #4A90E2; opacity: 0.7;
              pointer-events: none; z-index: 50;
          `;
      } else {
          const top = Math.max(tableRelativeTop, relativeY);
          resizerLineStyle = `
              display: block; position: absolute;
              top: ${top}px;
              left: ${tableRelativeLeft}px;
              width: ${tableRelativeWidth}px; height: 2px;
              background-color: #4A90E2; opacity: 0.7;
              pointer-events: none; z-index: 50;
          `;
      }
  }

  function handlePointerUp(event) {
      if (!isResizing || !editable || !editor) return;
      event.preventDefault();
      event.stopPropagation();
      const zoom = calculateZoomLevel(editorContainer);
      const diffX = (event.clientX - resizeStartPos.x) / zoom;
      const diffY = (event.clientY - resizeStartPos.y) / zoom;
      editor.update(() => {
          const cellNode = _getNodeByKey(resizeTargetCellKey);
          if (!_isTableCellNode(cellNode)) return;
          try {
              const tableNode = _getTableNodeFromLexicalNodeOrThrow(cellNode);
              const tableElement = editor.getElementByKey(tableNode.getKey());
              const [tableMap] = _computeTableMapSkipCellCheck(tableNode, null, null);
              if (resizeDirection === 'col') {
                  const colIndex = _getTableColumnIndexFromTableCellNode(cellNode, tableMap);
                  if (colIndex !== undefined) {
                      const colSpan = cellNode.getColSpan();
                      const targetColIndex = colIndex + colSpan - 1;

                      const currentWidths = tableNode.getColWidths()?.slice() || [];
                      let currentWidthVal = currentWidths[targetColIndex];

                      // Handle string/percentage widths
                      if (typeof currentWidthVal === 'string') {
                          if (currentWidthVal.endsWith('%')) {
                              const pct = parseFloat(currentWidthVal);
                              const tableWidth = tableElement.getBoundingClientRect().width / zoom;
                              currentWidthVal = (tableWidth * pct) / 100;
                          } else {
                              currentWidthVal = parseFloat(currentWidthVal);
                          }
                      }

                      // Initialize if undefined or NaN
                      if (currentWidthVal === undefined || isNaN(currentWidthVal)) {
                          if (cellNode.getColSpan() === 1) {
                              const domElement = editor.getElementByKey(cellNode.getKey());
                              if (domElement) {
                                  const rect = domElement.getBoundingClientRect();
                                  currentWidthVal = rect.width / zoom;
                              }
                          }
                          if (currentWidthVal === undefined || isNaN(currentWidthVal)) {
                              currentWidthVal = MIN_WIDTH;
                          }
                      }
                      
                      // Ensure previous columns have valid widths too (convert % to px if needed) to avoid mixing types causing layout issues?
                      // Ideally yes, but let's just ensure they exist.
                      for (let i = 0; i < targetColIndex; i++) {
                          if (currentWidths[i] === undefined) currentWidths[i] = MIN_WIDTH;
                      }

                      const newWidth = Math.max(MIN_WIDTH, currentWidthVal + diffX);
                      currentWidths[targetColIndex] = newWidth;
                      tableNode.setColWidths(currentWidths);
                  }
              } else {
                  const rowIndex = _getTableRowIndexFromTableCellNode(cellNode, tableMap);
                  if (rowIndex !== undefined) {
                      const actualRowIndex = rowIndex + (cellNode.getRowSpan() - 1);
                      const tableRows = tableNode.getChildren();
                      if (actualRowIndex < tableRows.length) {
                          const rowNode = tableRows[actualRowIndex];
                          if (_isTableRowNode(rowNode)) {
                              let currentHeight = rowNode.getHeight();
                              if (currentHeight === undefined) {
                                  const rowElement = editor.getElementByKey(rowNode.getKey());
                                  currentHeight = rowElement ? rowElement.getBoundingClientRect().height : MIN_HEIGHT;
                              }
                              const newHeight = Math.max(MIN_HEIGHT, currentHeight + diffY);
                              rowNode.setHeight(newHeight);
                          }
                      }
                  }
              }
          } catch(e) {
              console.error("Error during table resize update:", e);
          }
      }, { tag: 'skip-scroll' });
      isResizing = false;
      resizeDirection = null;
      resizeTargetCellKey = null;
      resizerLineStyle = 'display: none;';
      document.body.style.cursor = 'auto';
      if (editorContainer) editorContainer.style.cursor = 'auto';
  }

function updateSearchHighlights() {
  if (typeof CSS === 'undefined' || !CSS.highlights) {
    return;
  }

  // Fast path: if no results or search inactive (and replace modal closed), clear everything immediately
  if ((!showSearchBox && !showFindReplaceModal) || !searchTerm.trim() || searchResults.length === 0) {
    const prevMatch = CSS.highlights.get('search-match');
    if (prevMatch) {
      prevMatch.clear();
      CSS.highlights.delete('search-match');
    }
    
    const prevActive = CSS.highlights.get('search-match-active');
    if (prevActive) {
      prevActive.clear();
      CSS.highlights.delete('search-match-active');
    }
    return;
  }

  const matchRanges = [];
  const activeRanges = [];

  searchResults.forEach((result, index) => {
    // result.nodes is an array of { nodeKey, startOffset, endOffset }
    result.nodes.forEach(nodeMatch => {
        const domNode = editor.getElementByKey(nodeMatch.nodeKey);
        if (!domNode) return;

        let textNode = null;
        if (domNode.nodeType === Node.TEXT_NODE) {
          textNode = domNode;
        } else {
          const walker = document.createTreeWalker(domNode, NodeFilter.SHOW_TEXT, null);
          textNode = walker.nextNode();
        }

        if (textNode) {
          try {
            const range = new Range();
            range.setStart(textNode, nodeMatch.startOffset);
            range.setEnd(textNode, nodeMatch.endOffset);
            
            if (index === currentSearchResultIndex) {
              activeRanges.push(range);
            } else {
              matchRanges.push(range);
            }
          } catch (e) {}
        }
    });
  });

  CSS.highlights.set('search-match', new Highlight(...matchRanges));
  CSS.highlights.set('search-match-active', new Highlight(...activeRanges));
}

function handleSearchInputKeydown(event) {
  if (event.key === 'Enter') {
    event.preventDefault();
    if (event.shiftKey) {
        navigateToPreviousResult();
    } else {
        navigateToNextResult();
    }
  }
}

let latestSearchTerm = '';
function executeSearch(termToSearch, options = {}) {
  if (!editor) return;
  const { isCaseSensitive = false, isRegex = false, isWholeWord = false } = options;
  searchTerm = termToSearch; 
  const term = termToSearch; 
  latestSearchTerm = term;

  searchResults = [];
  currentSearchResultIndex = -1;
  updateSearchHighlights();

  if (term === '') {
    dispatch('searchresultsupdated', { results: [], term: '' });
    dispatch('searchindexchanged', { currentIndex: -1, currentResult: null });
    return;
  }

  editor.getEditorState().read(() => {
    if (term !== latestSearchTerm) return;

    const root = _getRoot();
    
    // 1. Flatten document text and track node offsets
    let fullText = '';
    const textNodeOffsets = []; // { nodeKey, start, end }

    const visit = (node) => {
        if (_isTextNode(node)) {
            const nodeText = node.getTextContent();
            textNodeOffsets.push({
                nodeKey: node.getKey(),
                start: fullText.length,
                end: fullText.length + nodeText.length
            });
            fullText += nodeText;
        } else if (_isElementNode(node)) {
            node.getChildren().forEach(visit);
            // Add newline for block elements to separate text flow? 
            // Lexical plain text search usually ignores blocks unless they are paragraphs.
            // For now, let's keep it simple as continuous flow for finding "partially highlighted words".
        }
    };
    visit(root);

    const newResults = [];
    let regex;
    try {
      let pattern = isRegex ? term : term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      if (isWholeWord) pattern = `\\b${pattern}\\b`;
      regex = new RegExp(pattern, isCaseSensitive ? 'g' : 'gi');
    } catch (e) {
      console.warn("Invalid search pattern:", term);
      return;
    }

    // 2. Search in flattened text
    let match;
    while ((match = regex.exec(fullText)) !== null) {
        const matchStart = match.index;
        const matchEnd = match.index + match[0].length;
        
        // 3. Map match range back to nodes
        const nodesInMatch = [];
        for (const tno of textNodeOffsets) {
            if (tno.end > matchStart && tno.start < matchEnd) {
                // This node overlaps with match
                nodesInMatch.push({
                    nodeKey: tno.nodeKey,
                    startOffset: Math.max(0, matchStart - tno.start),
                    endOffset: Math.min(tno.end - tno.start, matchEnd - tno.start)
                });
            }
            if (tno.start >= matchEnd) break;
        }

        if (nodesInMatch.length > 0) {
            newResults.push({
                nodes: nodesInMatch,
                text: match[0]
            });
        }
        
        if (match.index === regex.lastIndex) regex.lastIndex++;
    }
    
    if (term !== latestSearchTerm) return;

    searchResults = newResults;
    if (searchResults.length > 0) {
      currentSearchResultIndex = 0;
    } else {
      currentSearchResultIndex = -1;
      dispatch('searchindexchanged', { currentIndex: -1, currentResult: null });
    }
    
    updateSearchHighlights();
  });

  dispatch('searchresultsupdated', { results: searchResults, term: term });
}

function toggleSearchOptionsDropdown() {
  showSearchOptionsDropdown = !showSearchOptionsDropdown;
}

function openFindReplaceModal() {
  showSearchOptionsDropdown = false;
  showFindReplaceModal = true;
}

function handleReplace(event) {
  const { find, replace } = event.detail;
  if (currentSearchResultIndex >= 0 && searchResults.length > 0) {
      const result = searchResults[currentSearchResultIndex];
      
      editor.update(() => {
          if (result.nodes.length > 0) {
              const first = result.nodes[0];
              const last = result.nodes[result.nodes.length - 1];
              
              const firstNode = _getNodeByKey(first.nodeKey);
              const lastNode = _getNodeByKey(last.nodeKey);
              
              if (_isTextNode(firstNode) && _isTextNode(lastNode)) {
                  try {
                      const selection = _getSelection();
                      if (_isRangeSelection(selection)) {
                          selection.anchor.set(first.nodeKey, first.startOffset, 'text');
                          selection.focus.set(last.nodeKey, last.endOffset, 'text');
                          selection.insertText(replace);
                      }
                  } catch (e) {
                      console.error("Replace failed:", e);
                  }
              }
          }
      }, { tag: 'replace-one' });
      
      executeSearch(find);
  }
}

function handleReplaceAll(event) {
  const { find, replace } = event.detail;
  if (searchResults.length === 0) return;

  editor.update(() => {
      // Replacement must be done in reverse order to keep offsets valid for preceding matches
      for (let i = searchResults.length - 1; i >= 0; i--) {
          const result = searchResults[i];
          if (result.nodes.length > 0) {
              const first = result.nodes[0];
              const last = result.nodes[result.nodes.length - 1];
              const firstNode = _getNodeByKey(first.nodeKey);
              const lastNode = _getNodeByKey(last.nodeKey);
              
              if (_isTextNode(firstNode) && _isTextNode(lastNode)) {
                  const selection = _getSelection();
                  if (_isRangeSelection(selection)) {
                      selection.anchor.set(first.nodeKey, first.startOffset, 'text');
                      selection.focus.set(last.nodeKey, last.endOffset, 'text');
                      selection.insertText(replace);
                  }
              }
          }
      }
  }, { tag: 'replace-all' });
  
  executeSearch(find);
}

function clearSearchTermInput() {
  console.log('[clearSearchTermInput] Called.');
  searchTerm = '';
  searchResults = [];
  currentSearchResultIndex = -1;
  updateSearchHighlights();

  const updateData = { results: searchResults, term: searchTerm };
  const indexChangeData = { currentIndex: currentSearchResultIndex, currentResult: null };

  console.log('[clearSearchTermInput] Dispatching searchresultsupdated with:', updateData);
  dispatch('searchresultsupdated', updateData);
  console.log('[clearSearchTermInput] Dispatching searchindexchanged with:', indexChangeData);
  dispatch('searchindexchanged', indexChangeData);

  if (showSearchBox) {
    searchInputRef?.focus();
  }
}

function navigateToResult(index, shouldFocus = true) {
  if (!editor) return;
  console.log('[navigateToResult] Called with index:', index, 'Total results:', searchResults.length);

  if (index < 0 || index >= searchResults.length) {
    currentSearchResultIndex = -1;
    updateSearchHighlights();
    dispatch('searchindexchanged', { currentIndex: -1, currentResult: null });
    return;
  }

  const result = searchResults[index];
  currentSearchResultIndex = index;

  if (shouldFocus) {
      editor.focus(); 
  }
  latestScrollTargetKey = null;

  editor.update(() => {
    if (result.nodes.length > 0) {
        const first = result.nodes[0];
        const last = result.nodes[result.nodes.length - 1];
        const firstNode = _getNodeByKey(first.nodeKey);
        const lastNode = _getNodeByKey(last.nodeKey);

        if (_isTextNode(firstNode) && _isTextNode(lastNode)) {
            const selection = _getSelection();
            if (_isRangeSelection(selection)) {
                selection.anchor.set(first.nodeKey, first.startOffset, 'text');
                selection.focus.set(last.nodeKey, last.endOffset, 'text');
                latestScrollTargetKey = first.nodeKey;
            }
        }
    }
  }, { tag: 'search-navigate' });

  tick().then(updateSearchHighlights);

  if (latestScrollTargetKey) {
    const keyToScroll = latestScrollTargetKey;
    tick().then(() => {
      try {
        const domElement = editor.getElementByKey(keyToScroll);
        if (domElement) {
          domElement.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        }
      } catch (e) {}
    });
  }
  latestScrollTargetKey = null;

  const dispatchData = { currentIndex: currentSearchResultIndex, currentResult: result };
  dispatch('searchindexchanged', dispatchData);
}

function navigateToPreviousResult() {
  console.log('[navigateToPreviousResult] Called. currentSearchResultIndex:', currentSearchResultIndex, 'Total results:', searchResults.length);
  if (searchResults.length === 0) return;
  
  let newIndex = currentSearchResultIndex - 1;
  if (newIndex < 0) newIndex = searchResults.length - 1;
  
  navigateToResult(newIndex, false);
}

function navigateToNextResult() {
  console.log('[navigateToNextResult] Called. currentSearchResultIndex:', currentSearchResultIndex, 'Total results:', searchResults.length);
  if (searchResults.length === 0) return;
  
  let newIndex = currentSearchResultIndex + 1;
  if (newIndex >= searchResults.length) newIndex = 0;

  navigateToResult(newIndex, false);
}

let previousLayout = null;
let initialSyncDone = false;

function syncLayout() {
    if (!editor) return;
    const layoutConfig = DOCX_LAYOUT_COLUMN_CONFIGS[activeLayout];
    if (!layoutConfig) return;

    // Detect if layout actively changed by user interaction
    const layoutChanged = activeLayout !== previousLayout;
    
    // Capture initial status BEFORE updating it, for use in the closure
    const isInitialRun = !initialSyncDone;

    // Update tracking variables
    if (isInitialRun) {
        previousLayout = activeLayout;
        initialSyncDone = true;
    } else if (layoutChanged) {
        previousLayout = activeLayout;
    }

    editor.update(() => {
        const root = _getRoot();
        const findTableNodes = (node, foundTables) => {
            if (_isTableNode(node)) {
                foundTables.push(node);
            }
            if (typeof node.getChildren === 'function') {
                for (const child of node.getChildren()) {
                    findTableNodes(child, foundTables);
                }
            }
        };

        const allTableNodes = [];
        findTableNodes(root, allTableNodes);

        allTableNodes.forEach(tableNode => {
            // Set column widths
            if (layoutConfig.colgroup) {
                const currentWidths = tableNode.getColWidths();
                const hasWidths = currentWidths && currentWidths.length > 0 && !currentWidths.every(w => w === undefined);

                // Apply defaults ONLY if:
                // 1. Table has no widths set (new/raw table)
                // 2. OR Layout was explicitly changed by user (runtime switch), NOT just initial load
                // We use isInitialRun (captured const) to safely check this inside the callback
                if (!hasWidths || (layoutChanged && !isInitialRun)) {
                    const newColWidths = layoutConfig.colgroup;
                    tableNode.setColWidths(newColWidths);
                }
            }

            // Hide columns
            const rows = tableNode.getChildren();
            rows.forEach(row => {
                if (_isTableRowNode(row)) {
                    const cells = row.getChildren();
                    cells.forEach((cell, i) => {
                        if (_isTableCellNode(cell)) {
                            const shouldHide = layoutConfig.hiddenColumns?.includes(i);
                            const cellStyle = cell.getStyle() || '';
                            let newStyle = cellStyle.replace(/display:\s*none\s*;?/, '').trim();

                            if (shouldHide) {
                                if (!newStyle.endsWith(';')) newStyle += ';';
                                newStyle += ' display: none;';
                            }

                            if (newStyle.trim() !== cellStyle.trim()) {
                                cell.setStyle(newStyle.trim());
                            }
                        }
                    });
                }
            });
        });
    });
}


$: if (editor && activeLayout) {
    syncLayout();
}
</script>

<div class="lexical-editor-root h-full flex flex-col {backgroundClass} overflow-visible shadow-sm">
  {#if editable}
    <div class="toolbar relative flex items-center flex-wrap gap-x-1 border-b border-gray-300 dark:border-border p-1 flex-shrink-0 bg-gray-50 dark:bg-surface-3 shadow-md z-10">
      {#if toolbarConfig.undo}
        <button class="mini-toolbar-button" on:click={undo} title="Undo ({modLabel}+Z)" disabled={!editable || !canUndo}>↺</button>
      {/if}
      {#if toolbarConfig.redo}
        <button class="mini-toolbar-button" on:click={redo} title="Redo ({modLabel}+{isMac ? 'Shift+Z' : 'Y'})" disabled={!editable || !canRedo}>↻</button>
      {/if}
      {#if (toolbarConfig.undo || toolbarConfig.redo) && (toolbarConfig.blockType || toolbarConfig.bold || toolbarConfig.italic || toolbarConfig.underline || toolbarConfig.strikethrough)}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.blockType}
        <div class="relative" bind:this={blockDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center gap-1"
            on:click={toggleBlockDropdown}
            title="Block Type"
            disabled={!editable}
          >
            {@html blockTypeIcons[blockType] ?? blockTypeIcons.paragraph}
            <svg class="ml-0.5 h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
            </svg>
          </button>
          {#if isBlockDropdownOpen}
            <div class="absolute mt-1 z-20 w-64 bg-white dark:bg-gray-700 border border-gray-300 dark:border-border shadow-lg overflow-hidden">
              {#each blockTypeOptions as option}
                <div
                  class="px-3 py-1 flex justify-between items-center cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => selectBlockType(option.value)}
                  role="menuitem"
                  tabindex="-1"
                >
                  <span class="flex items-center gap-3 mr-3">
                    {@html blockTypeIcons[option.value]}
                    <span>{option.label}</span>
                  </span>
                  <span class="text-xs text-gray-500">{option.shortcut}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if toolbarConfig.blockType && (toolbarConfig.bold || toolbarConfig.italic || toolbarConfig.underline || toolbarConfig.strikethrough)}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.bold}
        <button class="mini-toolbar-button font-bold" on:click={() => formatText('bold')} class:active={isBold} title="Bold ({modLabel}+B)" disabled={!editable}>B</button>
      {/if}
      {#if toolbarConfig.italic}
        <button class="mini-toolbar-button italic" on:click={() => formatText('italic')} class:active={isItalic} title="Italic ({modLabel}+I)" disabled={!editable}>I</button>
      {/if}
      {#if toolbarConfig.underline}
        <button class="mini-toolbar-button underline" on:click={() => formatText('underline')} class:active={isUnderline} title="Underline ({modLabel}+U)" disabled={!editable}>U</button>
      {/if}
      {#if toolbarConfig.strikethrough}
        <button class="mini-toolbar-button line-through" on:click={() => formatText('strikethrough')} class:active={isStrikethrough} title="Strikethrough" disabled={!editable}>S</button>
      {/if}
      {#if toolbarConfig.link}
          <button
            class="mini-toolbar-button"
            on:click={toggleLink}
            class:active={isLink}
            title="Insert/Edit Link ({modLabel}+K)"
            disabled={!editable}
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-link w-4 h-4" viewBox="0 0 16 16">
                <path d="M6.354 5.5H4a3 3 0 0 0 0 6h3a3 3 0 0 0 2.83-4H9q-.13 0-.25.031A2 2 0 0 1 7 10.5H4a2 2 0 1 1 0-4h1.535c.218-.376.495-.714.82-1z"/>
                <path d="M9 5.5a3 3 0 0 0-2.83 4h1.098A2 2 0 0 1 9 6.5h3a2 2 0 1 1 0 4h-1.535a4 4 0 0 1-.82 1H12a3 3 0 1 0 0-6z"/>
            </svg>
          </button>
      {/if}
      {#if toolbarConfig.insertMenu}
        <div class="separator"></div>
          <div class="relative" bind:this={insertDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center"
            on:click={toggleInsertDropdown}
            title="Insert"
            disabled={!editable}
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M10 5a1 1 0 011 1v3h3a1 1 0 110 2h-3v3a1 1 0 11-2 0v-3H6a1 1 0 110-2h3V6a1 1 0 011-1z" clip-rule="evenodd" />
            </svg>
            <span class="ml-1 hidden sm:inline">Insert</span>
            <svg class="ml-1 h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
            </svg>
          </button>
          {#if isInsertDropdownOpen}
            <div class="absolute mt-1 z-20 w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-border shadow-lg overflow-hidden">
              {#each insertOptions as option}
              <div
                class="px-3 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                on:click={option.action}
                role="menuitem"
                tabindex="-1"
              >
                {@html option.icon}
                <span>{option.label}</span>
              </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if (toolbarConfig.insertMenu || toolbarConfig.link || toolbarConfig.bold) && (toolbarConfig.align)}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.align}
        <div class="relative" bind:this={alignmentDropdownRef}>
          <button class="mini-toolbar-button flex items-center" on:click={toggleAlignDropdown} title="Alignment" disabled={!editable}>
            {@html alignmentIcons[selectedAlignment]}
            <svg class="ml-1 h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
            </svg>
          </button>
          {#if isAlignDropdownOpen}
            <div class="absolute mt-1 z-20 w-40 bg-white dark:bg-gray-700 border border-gray-300 dark:border-border shadow-lg overflow-hidden">
              {#each alignmentOptions as option}
                <div
                  class="px-3 py-1 flex items-center cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => alignElement(option.value)}
                  role="menuitem"
                  tabindex="-1"
                >
                  <span class="flex items-center gap-3">
                    {@html alignmentIcons[option.value]}
                    <span>{option.label}</span>
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if toolbarConfig.align && (toolbarConfig.outdent || toolbarConfig.indent)}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.outdent}
        <button class="mini-toolbar-button" on:click={outdentContent} title="Outdent" disabled={!editable}>
            <svg class="w-[1rem] h-[1rem] indent-outdent-icon" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" viewBox="0 0 24 24">
                <path fill-rule="evenodd" d="M5 6a1 1 0 0 1 1-1h12a1 1 0 1 1 0 2H6a1 1 0 0 1-1-1Zm0 12a1 1 0 0 1 1-1h12a1 1 0 1 1 0 2H6a1 1 0 0 1-1-1Zm3.85-9.76A1 1 0 0 1 10.5 9v6a1 1 0 0 1-1.65.76l-3.5-3a1 1 0 0 1 0-1.52l3.5-3ZM12 10a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2h-5a1 1 0 0 1-1-1Zm0 4a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2h-5a1 1 0 0 1-1-1Z" clip-rule="evenodd"/>
            </svg>
          </button>
      {/if}
      {#if toolbarConfig.indent}
        <button class="mini-toolbar-button" on:click={indentContent} title="Indent" disabled={!editable}>
            <svg class="w-[1rem] h-[1rem] indent-outdent-icon" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" viewBox="0 0 24 24">
                <path fill-rule="evenodd" d="M5 6a1 1 0 0 1 1-1h12a1 1 0 1 1 0 2H6a1 1 0 0 1-1-1Zm0 12a1 1 0 0 1 1-1h12a1 1 0 1 1 0 2H6a1 1 0 0 1-1-1Zm1.65-9.76A1 1 0 0 0 5 9v6a1 1 0 0 0 1.65.76l3.5-3a1 1 0 0 0 0-1.52l-3.5-3ZM12 10a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2h-5a1 1 0 0 1-1-1Zm0 4a1 1 0 0 1 1-1h5a1 1 0 1 1 0 2h-5a1 1 0 0 1-1-1Z" clip-rule="evenodd"/>
            </svg>
          </button>
      {/if}
      {#if (toolbarConfig.outdent || toolbarConfig.indent) && toolbarConfig.textColor}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.textColor}
        <div class="relative" bind:this={colorDropdownRef}>
          <button class="mini-toolbar-button flex items-center" on:click={toggleColorDropdown} title="Text Color" disabled={!editable} style="color: {selectedTextColor === 'transparent' ? 'currentColor': selectedTextColor}">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 16 16">
                <path d="m13.498.795.149-.149a1.207 1.207 0 1 1 1.707 1.708l-.149.148a1.5 1.5 0 0 1-.059 2.059L4.854 14.854a.5.5 0 0 1-.233.131l-4 1a.5.5 0 0 1-.606-.606l1-4a.5.5 0 0 1 .131-.232l9.642-9.642a.5.5 0 0 0-.642.056L6.854 4.854a.5.5 0 1 1-.708-.708L9.44.854A1.5 1.5 0 0 1 11.5.796a1.5 1.5 0 0 1 1.998-.001m-.644.766a.5.5 0 0 0-.707 0L1.95 11.756l-.764 3.057 3.057-.764L14.44 3.854a.5.5 0 0 0 0-.708z"/>
            </svg>
            <svg class="ml-1 h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd" /></svg>
          </button>
          {#if isColorDropdownOpen}
            <div class="absolute mt-1 z-20 w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 shadow-lg">
              {#each colorOptions as option}
                <div
                  class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => applyTextColor(option.value)}
                  role="menuitemradio"
                  aria-checked={selectedTextColor === option.value}
                  tabindex="-1"
                >
                  <span class="w-4 h-4 border border-gray-400 dark:border-gray-500 rounded-full" style="background-color: {option.value === 'transparent' ? '#fff' : option.value};"></span>
                  <span>{option.label}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if toolbarConfig.textColor && (toolbarConfig.highlight || toolbarConfig.clearFormatting)}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.highlight}
        <div class="relative" bind:this={highlightDropdownRef}>
          <button class="mini-toolbar-button flex items-center" on:click={toggleHighlightDropdown} title="Highlight Color" disabled={!editable} style="background-color: {selectedHighlightColor === 'transparent' ? 'transparent': selectedHighlightColor}; color: {selectedHighlightColor !== 'transparent' && selectedHighlightColor !== null ? '#000' : 'currentColor'}">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 16 16">
                <path fill-rule="evenodd" d="M11.096.644a2 2 0 0 1 2.791.036l1.433 1.433a2 2 0 0 1 .035 2.791l-.413.435-8.07 8.995a.5.5 0 0 1-.372.166h-3a.5.5 0 0 1-.234-.058l-.412.412A.5.5 0 0 1 2.5 15h-2a.5.5 0 0 1-.354-.854l1.412-1.412A.5.5 0 0 1 1.5 12.5v-3a.5.5 0 0 1 .166-.372l8.995-8.07zm-.115 1.47L2.727 9.52l3.753 3.753 7.406-8.254zm3.585 2.17.064-.068a1 1 0 0 0-.017-1.396L13.18 1.387a1 1 0 0 0-1.396-.018l-.068.065zM5.293 13.5 2.5 10.707v1.586L3.707 13.5z"/>
            </svg>
            <svg class="ml-1 h-3 w-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 011.08 1.04l-4.25 4.25a.75.75 0 01-1.08 0L5.21 8.27a.75.75 0 01.02-1.06z" clip-rule="evenodd"/>
            </svg>
          </button>
          {#if isHighlightDropdownOpen}
            <div class="absolute mt-1 z-20 w-32 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 shadow-lg">
              {#each highlightOptions as option}
                <div
                  class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => applyHighlightColor(option.value)}
                  role="menuitemradio"
                  aria-checked={selectedHighlightColor === option.value}
                  tabindex="-1"
                >
                  <span class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500" style="background-color: {option.value};"></span>
                  <span>{option.label}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if toolbarConfig.highlight && toolbarConfig.clearFormatting}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.clearFormatting}
        <button class="mini-toolbar-button" on:click={clearFormatting} title="Clear Formatting" disabled={!editable}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-eraser" viewBox="0 0 16 16">
            <path d="M8.086 2.207a2 2 0 0 1 2.828 0l3.879 3.879a2 2 0 0 1 0 2.828l-5.5 5.5A2 2 0 0 1 7.879 15H5.12a2 2 0 0 1-1.414-.586l-2.5-2.5a2 2 0 0 1 0-2.828zm2.121.707a1 1 0 0 0-1.414 0L4.16 7.547l5.293 5.293 4.633-4.633a1 1 0 0 0 0-1.414zM8.746 13.547 3.453 8.254 1.914 9.793a1 1 0 0 0 0 1.414l2.5 2.5a1 1 0 0 0 .707.293H7.88a1 1 0 0 0 .707-.293z"/>
          </svg>
        </button>
      {/if}
      {#if toolbarConfig.clearFormatting && toolbarConfig.search}
        <div class="separator"></div>
      {/if}

      {#if toolbarConfig.search}
        <div class="ml-auto relative flex items-center" bind:this={searchToggleButtonElement}>
          <button
            class="mini-toolbar-button"
            class:active={showSearchBox}
            on:click={() => {
                showSearchBox = !showSearchBox;
                if (showSearchBox) {
                    tick().then(() => {
                        const input = searchUiContainerElement?.querySelector('input');
                        if (input) input.focus();
                    });
                } else {
                    updateSearchHighlights();
                }
            }}
            title="Search"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16">
              <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001q.044.06.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1 1 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0"/>
            </svg>
          </button>

          {#if showSearchBox}
            <div
              class="absolute right-0 top-full mt-1 z-20 bg-white dark:bg-gray-800 border border-gray-300 dark:border-border shadow-lg p-2 flex items-center gap-2 min-w-[320px] rounded"
              bind:this={searchUiContainerElement}
            >
              <div class="relative flex-grow flex items-center">
                <input
                  type="text"
                  placeholder="Search..."
                  class="w-full text-xs border border-gray-300 dark:border-border pl-2 pr-16 py-1 bg-white dark:bg-dark-bg-form-field text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500 rounded outline-none search-input-with-count"
                  bind:value={searchTerm}
                  bind:this={searchInputRef}
                  on:input={(e) => executeSearch(e.currentTarget.value)}
                  on:keydown={handleSearchInputKeydown}
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                  spellcheck="false"
                />
                <div class="absolute right-1 flex items-center gap-1 pointer-events-none">
                  {#if searchTerm}
                    <span class="text-[10px] text-gray-500 dark:text-gray-400 whitespace-nowrap">
                      {#if searchResults.length > 0}
                        {currentSearchResultIndex + 1}/{searchResults.length}
                      {:else}
                        0/0
                      {/if}
                    </span>
                    <button
                      class="p-0.5 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-full pointer-events-auto transition-colors"
                      on:click|stopPropagation={clearSearchTermInput}
                      title="Clear Search"
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor" class="bi bi-x-lg" viewBox="0 0 16 16">
                        <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z"/>
                      </svg>
                    </button>
                  {/if}
                </div>
              </div>
              
              <div class="flex items-center gap-0.5">
                <button
                  class="mini-toolbar-button !p-1"
                  on:click={navigateToPreviousResult}
                  disabled={searchResults.length === 0}
                  title="Previous Match"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-chevron-left" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M11.354 1.646a.5.5 0 0 1 0 .708L5.707 8l5.647 5.646a.5.5 0 0 1-.708.708l-6-6a.5.5 0 0 1 0-.708l6-6a.5.5 0 0 1 .708 0"/>
                  </svg>
                </button>
                <button
                  class="mini-toolbar-button !p-1"
                  on:click={navigateToNextResult}
                  disabled={searchResults.length === 0}
                  title="Next Match"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-chevron-right" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/>
                  </svg>
                </button>

                <div class="relative" bind:this={searchOptionsDropdownRef}>
                  <button
                    class="mini-toolbar-button !p-1"
                    on:click={toggleSearchOptionsDropdown}
                    title="Search Options"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-three-dots-vertical" viewBox="0 0 16 16">
                      <path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/>
                    </svg>
                  </button>
                  {#if showSearchOptionsDropdown}
                    <div class="absolute right-0 top-full mt-1 z-30 bg-white dark:bg-gray-700 border border-gray-300 dark:border-border shadow-lg rounded overflow-hidden min-w-[120px]">
                      <button
                        class="w-full text-left px-3 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 whitespace-nowrap"
                        on:click={openFindReplaceModal}
                      >
                        Find & Replace
                      </button>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <div
    class="lexical-wrapper flex-grow min-h-0 relative"
    style="{enableSegmentPlayback ? 'padding-left: 2.5rem !important;' : ''}"
    bind:this={editorWrapper}
  >
    <div
        bind:this={editorContainer}
        class="lexical-content focus:outline-none min-h-full h-auto relative"
        contenteditable={editable ? 'true' : 'false'}
        role="textbox"
        aria-multiline="true"
        spellcheck="true"
        data-placeholder={placeholder}
    ></div>

    <div class="resizer-line" style={resizerLineStyle}></div>

    {#if showPlayButton}
      <button
        class="play-segment-hover-btn absolute z-30 w-6 h-6 flex items-center justify-center bg-blue-600 hover:bg-blue-700 text-white rounded-full shadow-md transition-all duration-200 border-2 border-white dark:border-gray-800"
        style="top: {playButtonPosition.top}px; left: {playButtonPosition.left}px; transform: translateY(-50%);"
        on:click|stopPropagation={handlePlaySegmentClick}
        title="Play this segment"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 ml-0.5">
          <path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" />
        </svg>
      </button>
    {/if}
  </div>

  {#if enableTableCellMenu}
    <TableCellActionMenu
      editor={editor}
      anchorElement={editorWrapper}
      bind:isOpen={showTableCellMenu}
      bind:cellNodeKey={activeTableCellKey}
      bind:position={tableCellMenuPosition}
      on:close={handleTableCellMenuClose}
    />
  {/if}
</div>

<LinkModal
  bind:showModal={showLinkModal}
  initialUrl={currentModalUrl}
  isEditing={isEditingLink}
  on:confirm={handleLinkConfirm}
  on:delete={handleLinkDelete}
  on:close={() => showLinkModal = false}
/>

<FindReplaceModal
  bind:showModal={showFindReplaceModal}
  bind:initialSearchTerm={searchTerm}
  currentMatchIndex={currentSearchResultIndex}
  totalMatches={searchResults.length}
  on:replace={handleReplace}
  on:replaceall={handleReplaceAll}
  on:findnext={navigateToNextResult}
  on:findchange={(e) => executeSearch(e.detail.term, { isCaseSensitive: e.detail.isCaseSensitive, isRegex: e.detail.isRegex, isWholeWord: e.detail.isWholeWord })}
  on:close={() => showFindReplaceModal = false}
/>

<InsertTableModal
  bind:showModal={showInsertTableModal}
  on:confirm={handleInsertTableConfirm}
  on:close={() => showInsertTableModal = false}
/>

{#if enableFloatingToolbar}
<FloatingModifyHighlightToolbar
  editor={editor}
  showToolbar={showModifyToolbar}
  toolbarPosition={modifyToolbarPosition}
  onChangeColor={(color) => {
    if (clickedNodeKey) {
      editor.update(() => {
        const clickedNode = _getNodeByKey(clickedNodeKey);
        if (!_isExtendedTextNode(clickedNode)) return;
        const highlightId = clickedNode.getHighlightId();
        if (!highlightId) return;

        const root = _getRoot();
        const nodesToVisit = [root];
        while(nodesToVisit.length > 0) {
            const currentNode = nodesToVisit.pop();
            if (_isExtendedTextNode(currentNode) && currentNode.getHighlightId() === highlightId) {
                currentNode.setStyle(`background-color: ${color};`);
            }
            if (currentNode.getChildren) {
                const children = currentNode.getChildren();
                for (let i = children.length - 1; i >= 0; i--) {
                    nodesToVisit.push(children[i]);
                }
            }
        }

        const allHighlights = gatherAllHighlights();
        updateAndSaveHighlights(allHighlights);
      });
    }
    showModifyToolbar = false;
    clickedNodeKey = null;
  }}
  onDelete={() => {
    if (clickedNodeKey) {
      editor.update(() => {
        const clickedNode = _getNodeByKey(clickedNodeKey);
        if (!_isExtendedTextNode(clickedNode)) return;
        const highlightId = clickedNode.getHighlightId();
        if (!highlightId) return;

        const root = _getRoot();
        const nodesToVisit = [root];
        while(nodesToVisit.length > 0) {
            const currentNode = nodesToVisit.pop();
            if (_isExtendedTextNode(currentNode) && currentNode.getHighlightId() === highlightId) {
                currentNode.setStyle('background-color: transparent;');
                currentNode.setHighlightId(null);
            }
            if (currentNode.getChildren) {
                const children = currentNode.getChildren();
                for (let i = children.length - 1; i >= 0; i--) {
                    nodesToVisit.push(children[i]);
                }
            }
        }
        const allHighlights = gatherAllHighlights();
        updateAndSaveHighlights(allHighlights);
      });
    }
    showModifyToolbar = false;
    clickedNodeKey = null;
  }}
/>
{/if}


<style lang="postcss">
  .toolbar button.mini-toolbar-button, .toolbar select.mini-toolbar-select {
      @apply p-1.5 rounded inline-flex items-center justify-center
             focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-500
             dark:focus:ring-offset-[var(--app-bg)] transition duration-150 ease-in-out
             text-xs disabled:opacity-50 disabled:cursor-not-allowed;
      color: var(--ui-icon-color);
      border: 1px solid var(--ui-select-border);
      background-color: transparent; /* Default for light mode, will be overridden by dark mode or specific hover */
      margin-right: 2px;
      line-height: 1.2;
      min-height: 24px;
  }

  .toolbar button.mini-toolbar-button:hover:not(:disabled),
  .toolbar select.mini-toolbar-select:hover:not(:disabled) {
      background-color: var(--ui-icon-hover-bg);
      border-color: var(--ui-select-border);
  }

  html.dark .toolbar button.mini-toolbar-button,
  html.dark .toolbar select.mini-toolbar-select {
      color: var(--color-text-primary);
      border: 1px solid var(--color-border);
      background-color: transparent;
  }

  html.dark .toolbar button.mini-toolbar-button:hover:not(:disabled),
  html.dark .toolbar select.mini-toolbar-select:hover:not(:disabled) {
      background-color: var(--color-border);
      border-color: var(--color-border);
  }

  html.dark .toolbar button.mini-toolbar-button.active {
    @apply bg-blue-500 text-white;
  }

  .lexical-content {
      min-width: 150px; /* Prevent it from being too tiny when empty */
  }

  .editor-table {
      border-collapse: collapse;
      width: 100%;
  }

  .editor-table-cell {
      border: 1px solid #ccc;
      padding: 8px;
      min-width: 50px; /* Ensure cells have a minimum width */
      position: relative; /* Needed for resizer positioning */
  }

  .editor-table-cell-header {
      background-color: #f2f2f2;
      font-weight: bold;
      text-align: center;
  }

  .resizer-line {
      /* The style is dynamically applied in the script */
  }

  .indent-outdent-icon {
      transform: scaleX(-1); /* Flips the icon horizontally */
  }

  button.active {
    @apply bg-gray-300 dark:bg-gray-500;
  }

  /* Search match highlights using CSS Custom Highlight API */
  :global(::highlight(search-match)) {
    background-color: rgba(255, 215, 0, 0.4);
    color: black;
  }

  :global(::highlight(search-match-active)) {
    background-color: rgba(255, 165, 0, 0.7);
    color: black;
  }

  :global(html.dark ::highlight(search-match)) {
    background-color: rgba(255, 215, 0, 0.3);
    color: white;
  }

  :global(html.dark ::highlight(search-match-active)) {
    background-color: rgba(255, 165, 0, 0.6);
    color: white;
  }

  /* Glowing row highlight */
  :global(.editor-table-row.cursor-row-glow) {
    box-shadow: inset 0 0 4px 1px rgba(59, 130, 246, 0.5), 0 0 4px 1px rgba(59, 130, 246, 0.5);
    background-color: rgba(59, 130, 246, 0.05);
    transition: box-shadow 0.2s ease, background-color 0.2s ease;
    z-index: 5;
    position: relative;
  }

  :global(html.dark .editor-table-row.cursor-row-glow) {
    box-shadow: inset 0 0 6px 1px rgba(96, 165, 250, 0.4), 0 0 6px 1px rgba(96, 165, 250, 0.4);
    background-color: rgba(96, 165, 250, 0.1);
  }

  .play-segment-hover-btn {
      pointer-events: auto;
      cursor: pointer;
      opacity: 0.9;
      box-shadow: 0 2px 4px rgba(0,0,0,0.2);
  }

  .play-segment-hover-btn:hover {
      opacity: 1;
      transform: translateY(-50%) scale(1.1);
  }

</style>
