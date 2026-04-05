<!-- src/lib/components/projectview/lexical/LexicalEditor.svelte -->
<script>
  import { onMount, onDestroy, tick } from 'svelte';
  import {
    createEditor,
    $getRoot as _getRoot,
    $getSelection as _getSelection,
    $setSelection as _setSelection,
    $isRangeSelection as _isRangeSelection,
    $isElementNode as _isElementNode,
    $isTextNode as _isTextNode,
    $getNodeByKey as _getNodeByKey,
    $createParagraphNode as _createParagraphNode,
    $isParagraphNode as _isParagraphNode,
    FORMAT_TEXT_COMMAND,
    FORMAT_ELEMENT_COMMAND,
    INDENT_CONTENT_COMMAND,
    OUTDENT_CONTENT_COMMAND,
    SELECTION_CHANGE_COMMAND,
    CLICK_COMMAND,
    UNDO_COMMAND,
    REDO_COMMAND,
    KEY_MODIFIER_COMMAND,
    BLUR_COMMAND,
    FOCUS_COMMAND,
    COMMAND_PRIORITY_LOW,
    COMMAND_PRIORITY_NORMAL,
    COMMAND_PRIORITY_CRITICAL,
    COMMAND_PRIORITY_HIGH,
    COMMAND_PRIORITY_EDITOR,
    KEY_ENTER_COMMAND,
    RootNode,
    ParagraphNode,
    TextNode,
    LineBreakNode,
    $getNearestNodeFromDOMNode as _getNearestNodeFromDOMNode,
    KEY_TAB_COMMAND,
    $normalizeSelection__EXPERIMENTAL as _normalizeSelection,
    createCommand,
    $isNodeSelection as _isNodeSelection,
    $createNodeSelection as _createNodeSelection,
    $insertNodes as _insertNodes
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
    HeadingNode,
    QuoteNode,
    $isHeadingNode as _isHeadingNode,
    $createHeadingNode as _createHeadingNode,
    $createQuoteNode as _createQuoteNode,
    $isQuoteNode as _isQuoteNode,
    registerRichText
  } from '@lexical/rich-text';
  import {
    CodeNode,
    $createCodeNode as _createCodeNode,
    $isCodeNode as _isCodeNode
  } from '@lexical/code';
  import {
    ListNode,
    ListItemNode,
    $isListNode as _isListNode,
    $isListItemNode as _isListItemNode,
    INSERT_ORDERED_LIST_COMMAND,
    INSERT_UNORDERED_LIST_COMMAND,
    INSERT_CHECK_LIST_COMMAND,
    REMOVE_LIST_COMMAND,
    registerList,
    registerCheckList,
    $createListNode as _createListNode
  } from '@lexical/list';
  import {
    TableNode,
    TableRowNode,
    TableCellNode,
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
    $getTableColumnIndexFromTableCellNode as _getTableColumnIndexFromTableCellNode
  } from '@lexical/table';

  import {
    LinkNode,
    $isLinkNode as _isLinkNode,
    TOGGLE_LINK_COMMAND,
    $createLinkNode as _createLinkNode,
    toggleLink as _toggleLink
  } from '@lexical/link';
  import {
    $setBlocksType as _setBlocksType,
    $patchStyleText as _patchStyleText,
    $getSelectionStyleValueForProperty as _getSelectionStyleValueForProperty
  } from '@lexical/selection';
  import { $generateHtmlFromNodes as _generateHtmlFromNodes } from '@lexical/html';
  import { createEmptyHistoryState, registerHistory } from '@lexical/history';
  import { createEventDispatcher } from 'svelte';
  
  /**
   * Action to portal an element to the body
   */
  function portal(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node);
      }
    };
  }

  let dropdownStyle = '';
  
  function updateDropdownPosition(ref) {
    if (!ref) return;
    const rect = ref.getBoundingClientRect();
    // Use fixed positioning to escape overflow containers
    dropdownStyle = `position: fixed; top: ${rect.bottom + 4}px; left: ${rect.left}px; z-index: 10000;`;
  }
  import { v4 as uuidv4 } from 'uuid';

  import {
    ExtendedTextNode,
    $createExtendedTextNode as _createExtendedTextNode,
    $isExtendedTextNode as _isExtendedTextNode
  } from '$lib/nodes/ExtendedTextNode.js';

  import {
    HorizontalRuleNode,
    $createHorizontalRuleNode as _createHorizontalRuleNode
  } from '$lib/nodes/HorizontalRuleNode.js';
  import {
    ImageNode,
    $createImageNode as _createImageNode,
    $isImageNode as _isImageNode
  } from '$lib/nodes/ImageNode.js';
  import {
    DateNode,
    $createDateNode as _createDateNode,
    $isDateNode as _isDateNode
  } from '$lib/nodes/DateNode.js';
  import {
    EquationNode,
    $createEquationNode as _createEquationNode,
    $isEquationNode as _isEquationNode
  } from '$lib/nodes/EquationNode.js';
  import { DOCX_LAYOUT_COLUMN_CONFIGS } from '$lib/constants/exportLayouts.js';
  import { SHARED_NODES } from '$lib/nodes/LexicalConfig.js';

  import LinkModal from '../modals/LinkModal.svelte';
  import InsertTableModal from '../modals/InsertTableModal.svelte';
  import InsertImageModal from './InsertImageModal.svelte';
  import InsertEquationModal from './InsertEquationModal.svelte';
  import DatePromptModal from '../modals/DatePromptModal.svelte';
  import FindReplaceModal from '../modals/FindReplaceModal.svelte';
  import TableCellActionMenu from './TableCellActionMenu.svelte';
  import FloatingHighlightToolbar from './FloatingHighlightToolbar.svelte';
  import FloatingModifyHighlightToolbar from './FloatingModifyHighlightToolbar.svelte';
  import notificationStore from '$lib/stores/notificationStore.js';
  import {
    Undo2,
    Redo2,
    Bold as BoldIcon,
    Italic as ItalicIcon,
    Underline as UnderlineIcon,
    Strikethrough as StrikethroughIcon,
    Plus,
    ChevronDown,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    Outdent,
    Indent,
    PaintBucket,
    Eraser,
    Search,
    List,
    ListOrdered,
    ListChecks,
    Quote as QuoteIcon,
    Code as CodeIcon,
    Heading1,
    Heading2,
    Heading3,
    Type,
    Highlighter,
    Baseline,
    X,
    ChevronUp,
    CheckSquare,
    Table as TableIcon,
    Minus,
    Link as LinkIcon,
    ChevronLeft,
    ChevronRight,
    MoreVertical,
    Play,
    CaseSensitive,
    Subscript as SubscriptIcon,
    Superscript as SuperscriptIcon,
    CalendarDays
  } from '@lucide/svelte';
  export let initialJson = null;
  export let editable = true;
  export let allowReadModeHighlights = false;
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
    fontFamily: true,
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
  export let backgroundClass = 'bg-white dark:bg-gray-900';
  export let documentPath = null;
  export let initialHighlights = [];
  export let documentHighlights = [];
  export let externalHighlightedRowIndex = -1; // Prop to allow external highlighting

  let editorRoot;
  let editorWrapper;
  let editorContainer;
  let editor = null;
  let isReady = false;
  let isFocused = false; // Track focus state
  let internalCursorRowIndex = -1; // Track local cursor position
  let unregisterListeners = () => {};
  let historyState = createEmptyHistoryState();
  let savedSelection = null; // Used for Link toggling
  let canUndo = false;
  let canRedo = false;

  let isBold = false;
  let isItalic = false;
  let isUnderline = false;
  let isStrikethrough = false;
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

  let isTextFormatDropdownOpen = false;
  let textFormatDropdownRef;

  let showInsertTableModal = false;

  let showTableCellMenu = false;
  let tableCellMenuPosition = { top: 0, left: 0 };
  let activeTableCellKey = null;

  const INSERT_HORIZONTAL_RULE_COMMAND = createCommand('INSERT_HORIZONTAL_RULE_COMMAND');
  const INSERT_DATE_COMMAND = createCommand('INSERT_DATE_COMMAND');

  let searchUiContainerElement;
  let searchToggleButtonElement;
  let searchInputRef;

  let latestScrollTargetKey = null; // New component-level variable
  let areHighlightsReady = false; // Track if highlights have been loaded from backend
  let areNodesReady = false; // Track if initial nodes have been loaded into Lexical
  let showCreateToolbar = false;
  let createToolbarPosition = { top: 0, left: 0 };

  let previousDocumentHighlightsIds = new Set();

  let selectedImageKey = null;
  let imageResizerRect = null;
  let isResizingImage = false;
  let imageResizeDirection = null; // 'nw', 'ne', 'sw', 'se'
  let imageResizeStartParams = null; // { width, height, x, y }

  let showInsertImageModal = false;
  let showDateModal = false;
  let dateNodeToEditKey = null;
  let lastUsedDateConfig = {
    format: 'YYYY-MM-DD',
    showTime: false,
    timeFormat: 'HH:mm'
  };
  let dateInitialData = {
    date: new Date().toISOString(),
    ...lastUsedDateConfig
  };

  let showInsertEquationModal = false;
  let equationNodeToEditKey = null;
  let equationInitialData = { equation: '', inline: true };

  let savedImageSelection = null;

  let isResizing = false;
  let resizeDirection = null;
  let resizeTargetCellKey = null;
  let resizeStartPos = { x: 0, y: 0 };
  let resizerLineStyle = 'display: none;';

  let showModifyToolbar = false;
  let modifyToolbarPosition = { top: 0, left: 0 };
  let clickedNodeKey = null;
  $: clickedHighlightId = (() => {
    if (!clickedNodeKey || !editor) return null;
    let hid = null;
    editor.getEditorState().read(() => {
      const node = _getNodeByKey(clickedNodeKey);
      if (_isExtendedTextNode(node)) {
        hid = node.getHighlightId();
      }
    });
    return hid;
  })();

  let editorUpdateTracker = 0; // Added reactive statement to track editor updates and re-trigger image resolution

  let hoveredRowKey = null;
  let playButtonPosition = { top: 0, left: 0 };
  let showPlayButton = false;

  const MIN_COLUMN_WIDTH = 20;

  export const editorNodes = SHARED_NODES;

  function handleShortcut(event) {
    if (event.key === 'Escape') {
      showCreateToolbar = false;
      showModifyToolbar = false;
      clickedNodeKey = null;
      return;
    }
    if (!editable || !editor) return;
    const mod = event.metaKey || event.ctrlKey;

    if (mod && event.altKey) {
      const map = {
        '0': 'paragraph',
        '1': 'h1',
        '2': 'h2',
        '3': 'h3',
        '4': 'ul',
        '5': 'ol',
        q: 'quote',
        c: 'code'
      };
      const key =
        event.code && event.code.startsWith('Digit')
          ? event.code.slice(5)
          : event.key.toLowerCase();
      const type = map[key];
      if (type) {
        selectBlockType(type);
        event.preventDefault();
        return;
      }
    }
    if (mod && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      toggleLink();
      return;
    }
    if (mod && event.key === ']') {
      event.preventDefault();
      indentContent();
      return;
    }
    if (mod && event.key === '[') {
      event.preventDefault();
      outdentContent();
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      if (event.shiftKey) {
        outdentContent();
      } else {
        indentContent();
      }
      return;
    }
  }

  function handleClickOutside(event) {
    // Check if click was inside any of the dropdown buttons or their menus
    const refs = [
      blockDropdownRef,
      insertDropdownRef,
      alignmentDropdownRef,
      colorDropdownRef,
      highlightDropdownRef,
      searchOptionsDropdownRef,
      fontDropdownRef,
      fontSizeDropdownRef,
      textFormatDropdownRef
    ];

    let clickedInsideDropdown = false;
    for (const ref of refs) {
      if (ref && ref.contains(event.target)) {
        clickedInsideDropdown = true;
        break;
      }
    }

    if (!clickedInsideDropdown) {
      closeAllDropdowns();
    }

    // Dismiss floating toolbars if click is outside the entire editor root
    if (editorRoot && !editorRoot.contains(event.target)) {
      showCreateToolbar = false;
      showModifyToolbar = false;
      clickedNodeKey = null;
    }
  }

  function closeAllDropdowns() {
    isBlockDropdownOpen = false;
    isInsertDropdownOpen = false;
    isAlignDropdownOpen = false;
    isColorDropdownOpen = false;
    isHighlightDropdownOpen = false;
    showSearchOptionsDropdown = false;
    isFontDropdownOpen = false;
    isFontSizeDropdownOpen = false;
    isTextFormatDropdownOpen = false;
  }

  function updateImageResizer() {
    if (!selectedImageKey || !editor || !editorWrapper) {
      imageResizerRect = null;
      return;
    }
    const domElement = editor.getElementByKey(selectedImageKey);
    if (domElement) {
      const img = domElement.querySelector('img');
      if (img) {
        const rect = img.getBoundingClientRect();
        const wrapperRect = editorWrapper.getBoundingClientRect();
        imageResizerRect = {
          top: rect.top - wrapperRect.top + editorWrapper.scrollTop,
          left: rect.left - wrapperRect.left + editorWrapper.scrollLeft,
          width: rect.width,
          height: rect.height
        };
        return;
      }
    }
    imageResizerRect = null;
  }

  function handleImageResizeStart(event, direction) {
    if (!editable) return;
    event.preventDefault();
    event.stopPropagation();
    isResizingImage = true;
    imageResizeDirection = direction;
    imageResizeStartParams = {
      width: imageResizerRect.width,
      height: imageResizerRect.height,
      x: event.clientX,
      y: event.clientY,
      ratio: imageResizerRect.width / imageResizerRect.height
    };
    window.addEventListener('pointermove', handleImageResizeMove);
    window.addEventListener('pointerup', handleImageResizeEnd);
  }

  function handleImageResizeMove(event) {
    if (!isResizingImage || !imageResizeStartParams) return;
    event.preventDefault();

    const deltaX = event.clientX - imageResizeStartParams.x;
    let newWidth = imageResizeStartParams.width;

    if (imageResizeDirection === 'se' || imageResizeDirection === 'ne') {
      newWidth += deltaX;
    } else if (imageResizeDirection === 'sw' || imageResizeDirection === 'nw') {
      newWidth -= deltaX;
    }

    newWidth = Math.max(50, newWidth);
    const newHeight = newWidth / imageResizeStartParams.ratio;

    imageResizerRect = {
      ...imageResizerRect,
      width: newWidth,
      height: newHeight
    };
  }

  function handleImageResizeEnd(event) {
    if (!isResizingImage) return;
    isResizingImage = false;
    window.removeEventListener('pointermove', handleImageResizeMove);
    window.removeEventListener('pointerup', handleImageResizeEnd);

    if (editor && selectedImageKey && imageResizerRect) {
      editor.update(() => {
        const node = _getNodeByKey(selectedImageKey);
        if (_isImageNode(node)) {
          node.setWidthAndHeight(
            Math.round(imageResizerRect.width),
            Math.round(imageResizerRect.height)
          );
        }
      });
    }
  }

  function toggleBlockDropdown(event) {
    if (!editable) return;
    const nextState = !isBlockDropdownOpen;
    closeAllDropdowns();
    isBlockDropdownOpen = nextState;
    if (isBlockDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  function selectBlockType(type) {
    handleBlockTypeChange({ target: { value: type } });
    isBlockDropdownOpen = false;
  }

  function toggleInsertDropdown(event) {
    if (!editable) return;
    const nextState = !isInsertDropdownOpen;
    closeAllDropdowns();
    isInsertDropdownOpen = nextState;
    if (isInsertDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  function toggleTextFormatDropdown(event) {
    if (!editable) return;
    const nextState = !isTextFormatDropdownOpen;
    closeAllDropdowns();
    isTextFormatDropdownOpen = nextState;
    if (isTextFormatDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  function applyTextFormat(formatType) {
    if (!editor || !isReady || !editor.isEditable()) return;

    if (['strikethrough', 'subscript', 'superscript'].includes(formatType)) {
      editor.dispatchCommand(FORMAT_TEXT_COMMAND, formatType);
    } else {
      editor.update(() => {
        const selection = _getSelection();
        if (_isRangeSelection(selection)) {
          const nodes = selection.extract();
          nodes.forEach((node) => {
            if (_isTextNode(node)) {
              let text = node.getTextContent();
              if (formatType === 'uppercase') text = text.toUpperCase();
              else if (formatType === 'lowercase') text = text.toLowerCase();
              else if (formatType === 'capitalize') {
                text = text.replace(/\b\w/g, (c) => c.toUpperCase());
              } else if (formatType === 'sentencecase') {
                text = text.toLowerCase().replace(/(^\s*\w|[.!?]\s+\w)/g, (c) => c.toUpperCase());
              }
              node.setTextContent(text);
            }
          });
        }
      });
    }
    isTextFormatDropdownOpen = false;
  }

  function openInsertTableDialog() {
    if (!editable) return;
    closeTableCellMenu(false);
    showInsertTableModal = true;
    isInsertDropdownOpen = false;
  }

  function insertHorizontalRule() {
    if (!editor || !editable) return;
    editor.dispatchCommand(INSERT_HORIZONTAL_RULE_COMMAND, undefined);
    isInsertDropdownOpen = false;
  }

  function handleInsertTableConfirm(event) {
    if (!editor || !isReady || !editor.isEditable()) return;
    const { rows, columns } = event.detail;
    editor.dispatchCommand(INSERT_TABLE_COMMAND, {
      rows: String(rows),
      columns: String(columns)
    });
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
          documentPath
        }
      });
      if (highlightsJson && editor) {
        const highlights = JSON.parse(highlightsJson);
        previousDocumentHighlightsIds = new Set(highlights.map((h) => h.id));
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

  const isMac =
    typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC');
  const optLabel = isMac ? 'Opt' : 'Alt';
  const modLabel = isMac ? '⌘' : 'Ctrl';

  const blockTypeOptions = [
    {
      value: 'paragraph',
      label: 'Normal',
      shortcut: `${modLabel}+${optLabel}+0`
    },
    { value: 'h1', label: 'Heading 1', shortcut: `${modLabel}+${optLabel}+1` },
    { value: 'h2', label: 'Heading 2', shortcut: `${modLabel}+${optLabel}+2` },
    { value: 'h3', label: 'Heading 3', shortcut: `${modLabel}+${optLabel}+3` },
    {
      value: 'ul',
      label: 'Bullet List',
      shortcut: `${modLabel}+${optLabel}+4`
    },
    {
      value: 'ol',
      label: 'Numbered List',
      shortcut: `${modLabel}+${optLabel}+5`
    },
    { value: 'check', label: 'Check List', shortcut: '' },
    { value: 'quote', label: 'Quote', shortcut: `${modLabel}+${optLabel}+Q` },
    {
      value: 'code',
      label: 'Code Block',
      shortcut: `${modLabel}+${optLabel}+C`
    }
  ];

  import { Image as ImageIcon, Sigma } from '@lucide/svelte';

  const insertOptions = [
    {
      value: 'table',
      label: 'Table',
      action: openInsertTableDialog,
      iconComponent: TableIcon
    },
    {
      value: 'hr',
      label: 'Horizontal Rule',
      action: insertHorizontalRule,
      iconComponent: Minus
    },
    {
      value: 'date',
      label: 'Date',
      action: openInsertDateDialog,
      iconComponent: CalendarDays
    },
    {
      value: 'link',
      label: 'Link',
      action: toggleLink,
      iconComponent: LinkIcon
    },
    {
      value: 'image',
      label: 'Image',
      action: insertImage,
      iconComponent: ImageIcon
    },
    {
      value: 'equation',
      label: 'Equation',
      action: openInsertEquationDialog,
      iconComponent: Sigma
    }
  ];

  async function insertImage() {
    if (!editor || !editable || !documentPath) return;

    editor.getEditorState().read(() => {
      const selection = _getSelection();
      if (selection) {
        savedImageSelection = selection.clone();
      } else {
        savedImageSelection = null;
      }
    });

    isInsertDropdownOpen = false;
    showInsertImageModal = true;
  }

  function openInsertDateDialog() {
    dateNodeToEditKey = null;
    dateInitialData = { date: new Date().toISOString(), ...lastUsedDateConfig };
    showDateModal = true;
    isInsertDropdownOpen = false;
  }

  function openInsertEquationDialog() {
    equationNodeToEditKey = null;
    equationInitialData = { equation: '', inline: true };
    showInsertEquationModal = true;
    isInsertDropdownOpen = false;
  }

  function handleEquationConfirm(event) {
    const { equation, inline } = event.detail;
    if (editor) {
      editor.update(() => {
        if (equationNodeToEditKey) {
          const existingNode = _getNodeByKey(equationNodeToEditKey);
          if (existingNode && _isEquationNode(existingNode)) {
            const newNode = _createEquationNode(equation, inline);
            existingNode.replace(newNode);
          }
        } else {
          const selection = _getSelection();
          if (_isRangeSelection(selection)) {
            const node = _createEquationNode(equation, inline);
            _insertNodes([node]);
          }
        }
      });
      editor.focus();
    }
    showInsertEquationModal = false;
    equationNodeToEditKey = null;
  }
  function handleDateConfirm(event) {
    const { date, format, showTime, timeFormat, displayValue, insertAsText } = event.detail;

    // Remember last used config
    lastUsedDateConfig = { format, showTime, timeFormat };

    if (dateNodeToEditKey) {
      // Update existing node
      editor.update(() => {
        const node = _getNodeByKey(dateNodeToEditKey);
        if (_isDateNode(node)) {
          if (insertAsText) {
            const textNode = _createTextNode(displayValue);
            node.replace(textNode);
          } else {
            const writable = node.getWritable();
            writable.__date = date;
            writable.__format = format;
            writable.__showTime = showTime;
            writable.__timeFormat = timeFormat;
            writable.__displayValue = displayValue;
          }
        }
      });
    } else {
      // Insert new node or text
      editor.update(() => {
        if (insertAsText) {
          const textNode = _createTextNode(displayValue);
          _insertNodes([textNode]);
        } else {
          const dateNode = _createDateNode(date, format, showTime, timeFormat, displayValue);
          _insertNodes([dateNode]);
        }
      });
    }
    showDateModal = false;
    dateNodeToEditKey = null;
  }

  function handleDateDelete() {
    if (dateNodeToEditKey) {
      editor.update(() => {
        const node = _getNodeByKey(dateNodeToEditKey);
        if (node) {
          node.remove();
        }
      });
    }
    showDateModal = false;
    dateNodeToEditKey = null;
  }

  function handleInsertImageAttached(event) {
    const { path } = event.detail;
    if (!path) return;
    const filename = path.split(/[\\/]/).pop();

    editor.update(() => {
      if (savedImageSelection) {
        _setSelection(savedImageSelection.clone());
      }
      const imageNode = _createImageNode(filename, filename);
      const selection = _getSelection();
      if (_isRangeSelection(selection)) {
        selection.insertNodes([imageNode]);
      } else {
        const root = _getRoot();
        const p = _createParagraphNode();
        p.append(imageNode);
        root.append(p);
      }
    });
  }

  async function handleInsertImageExternal(event) {
    const { path } = event.detail;
    if (!path) return;

    try {
      const projectStoreState = get(project);
      let relPath = documentPath;
      if (documentPath.startsWith(projectStoreState.baseDirectory)) {
        relPath = documentPath.substring(projectStoreState.baseDirectory.length);
        relPath = relPath.replace(/\\/g, '/').replace(/^\//, '');
      }

      const uploadedPath = await invoke('upload_attachment', {
        projectXmlPathStr: projectStoreState.xmlPath,
        assetRelativePath: relPath,
        sourceFilePathStr: path
      });

      if (uploadedPath) {
        const filename = path.split(/[\\/]/).pop();

        editor.update(() => {
          if (savedImageSelection) {
            _setSelection(savedImageSelection.clone());
          }
          const imageNode = _createImageNode(filename, filename);
          const selection = _getSelection();
          if (_isRangeSelection(selection)) {
            selection.insertNodes([imageNode]);
          } else {
            const root = _getRoot();
            const p = _createParagraphNode();
            p.append(imageNode);
            root.append(p);
          }
        });

        dispatch('attachmentadded');
        triggerRefresh();
      }
    } catch (error) {
      console.error('Error inserting external image:', error);
      notificationStore.add(`Failed to insert global/local image: ${error}`, 'error');
    }
  }

  const blockTypeIcons = {
    paragraph: Type,
    h1: Heading1,
    h2: Heading2,
    h3: Heading3,
    ul: List,
    ol: ListOrdered,
    check: ListChecks,
    quote: QuoteIcon,
    code: CodeIcon
  };

  const fontOptions = [
    { label: 'Inter', value: 'Inter' },
    { label: 'Anton', value: 'Anton' },
    { label: 'Arial', value: 'Arial, Helvetica, sans-serif' },
    { label: 'Bangers', value: 'Bangers' },
    {
      label: 'Calibri',
      value: 'Calibri, Candara, Segoe, "Segoe UI", Optima, Arial, sans-serif'
    },
    { label: 'Comic Neue', value: "'Comic Neue'" },
    { label: 'Comic Sans', value: '"Comic Sans MS", "Comic Sans", cursive' },
    {
      label: 'Console',
      value: 'Monaco, Consolas, "Lucida Console", monospace'
    },
    { label: 'Courier Prime', value: "'Courier Prime'" },
    { label: 'Dancing Script', value: "'Dancing Script'" },
    { label: 'Indie Flower', value: "'Indie Flower'" },
    { label: 'JetBrains Mono', value: "'JetBrains Mono'" },
    { label: 'Merriweather', value: "'Merriweather'" },
    { label: 'Montserrat', value: 'Montserrat' },
    {
      label: 'Palatino Linotype',
      value: '"Palatino Linotype", "Book Antiqua", Palatino, serif'
    },
    { label: 'Playfair Display', value: "'Playfair Display'" },
    { label: 'Roboto', value: 'Roboto' },
    { label: 'Roboto Slab', value: "'Roboto Slab'" },
    { label: 'Times New Roman', value: '"Times New Roman", Times, serif' }
  ];

  const fontSizeOptions = [
    '10',
    '11',
    '12',
    '13',
    '14',
    '15',
    '16',
    '17',
    '18',
    '19',
    '20',
    '22',
    '24',
    '26',
    '28',
    '30',
    '32',
    '36',
    '40',
    '48',
    '64',
    '72',
    '96'
  ];

  const alignmentOptions = [
    { value: 'left', label: 'Left' },
    { value: 'center', label: 'Center' },
    { value: 'right', label: 'Right' },
    { value: 'justify', label: 'Justify' }
  ];
  const alignmentIcons = {
    left: 'left',
    center: 'center',
    right: 'right',
    justify: 'justify'
  };

  let isAlignDropdownOpen = false;
  let alignmentDropdownRef;
  function toggleAlignDropdown(event) {
    if (!editable) return;
    const nextState = !isAlignDropdownOpen;
    closeAllDropdowns();
    isAlignDropdownOpen = nextState;
    if (isAlignDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  let selectedFontFamily = 'Inter';
  let isFontDropdownOpen = false;
  let fontDropdownRef;

  function toggleFontDropdown(event) {
    if (!editable) return;
    const nextState = !isFontDropdownOpen;
    closeAllDropdowns();
    isFontDropdownOpen = nextState;
    if (isFontDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  function applyFontFamily(fontFamily) {
    if (!editor || !isReady || !editor.isEditable()) return;
    applyStyle('font-family', fontFamily);
    isFontDropdownOpen = false;
  }

  let selectedFontSize = '15';
  let isFontSizeDropdownOpen = false;
  let fontSizeDropdownRef;

  function toggleFontSizeDropdown(event) {
    if (!editable) return;
    const nextState = !isFontSizeDropdownOpen;
    closeAllDropdowns();
    isFontSizeDropdownOpen = nextState;
    if (isFontSizeDropdownOpen && event) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  function applyFontSize(fontSize) {
    if (!editor || !isReady || !editor.isEditable()) return;

    editor.update(() => {
      const selection = _getSelection();
      if (_isTableSelection(selection)) {
        _normalizeSelection(selection);
      }
      const normalizedSelection = _getSelection();
      if (_isRangeSelection(normalizedSelection)) {
        _patchStyleText(normalizedSelection, { 'font-size': fontSize + 'px' });

        // Also apply the font size to parent ListItemNodes so bullets/numbers scale
        const nodes = normalizedSelection.getNodes();
        const listItems = new Set();
        for (const node of nodes) {
          let current = node;
          while (current !== null) {
            if (_isListItemNode(current)) {
              listItems.add(current.getKey());
              break;
            }
            current = current.getParent();
          }
        }

        for (const key of listItems) {
          const liNode = _getNodeByKey(key);
          if (liNode && typeof liNode.getStyle === 'function') {
            const currentStyle = liNode.getStyle() || '';
            const newStyle = currentStyle.replace(/font-size:\s*[^;]+;?/g, '').trim();
            liNode.setStyle(`${newStyle} font-size: ${fontSize}px;`.trim());
          }
        }
      }
    });

    isFontSizeDropdownOpen = false;
  }

  function updateFontSize(delta) {
    if (!editor || !isReady || !editor.isEditable()) return;
    const currentSize = parseInt(selectedFontSize, 10) || 15;
    let newSize = currentSize + delta;

    // Clamp between 10 and 96
    newSize = Math.max(10, Math.min(96, newSize));

    applyFontSize(String(newSize));
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
  import { HIGHLIGHT_OPTIONS_WITH_NONE } from '$lib/constants/highlightOptions.js';
  const highlightOptions = HIGHLIGHT_OPTIONS_WITH_NONE;
  let isHighlightDropdownOpen = false;
  let highlightDropdownRef;
  function toggleHighlightDropdown(event) {
    if (!editable && !allowReadModeHighlights) return;
    const nextState = !isHighlightDropdownOpen;
    closeAllDropdowns();
    isHighlightDropdownOpen = nextState;
    if (isHighlightDropdownOpen) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  let isColorDropdownOpen = false;
  let colorDropdownRef;
  function toggleColorDropdown(event) {
    if (!editable) return;
    const nextState = !isColorDropdownOpen;
    closeAllDropdowns();
    isColorDropdownOpen = nextState;
    if (isColorDropdownOpen) {
      updateDropdownPosition(event.currentTarget);
    }
  }

  import { get } from 'svelte/store';
  import { project, toggleTagInHighlightLocal } from '$lib/stores/projectStore.js';
  import { triggerRefresh } from '$lib/stores/refresherStore.js';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  function createInitialEditorState(jsonProp) {
    if (
      jsonProp &&
      typeof jsonProp === 'string' &&
      jsonProp.trim() !== '' &&
      jsonProp !== 'null' &&
      jsonProp !== 'undefined'
    ) {
      try {
        if (jsonProp.startsWith('{') && jsonProp.endsWith('}')) {
          const parsedForValidation = JSON.parse(jsonProp);
          if (
            parsedForValidation &&
            parsedForValidation.root &&
            Array.isArray(parsedForValidation.root.children)
          ) {
            return jsonProp;
          } else {
            console.warn(
              `[LexicalEditor] initialJson prop looks like JSON but lacks root.children. Using default empty state.`
            );
          }
        } else {
          // Support plain text by wrapping it
          return JSON.stringify({
            root: {
              children: [
                {
                  type: 'paragraph',
                  version: 1,
                  children: [{ type: 'text', text: jsonProp, version: 1 }]
                }
              ],
              direction: null,
              format: '',
              indent: 0,
              type: 'root',
              version: 1
            }
          });
        }
      } catch (e) {
        console.error(
          `[LexicalEditor] Error during basic validation of initialJson prop. Using default empty state.`,
          e
        );
      }
    }
    return JSON.stringify({
      root: {
        children: [{ type: 'paragraph', version: 1, children: [] }],
        direction: null,
        format: '',
        indent: 0,
        type: 'root',
        version: 1
      }
    });
  }

  onMount(() => {
    console.log('[LexicalEditor] onMount. enableSegmentPlayback:', enableSegmentPlayback);
    const instanceId = Math.random().toString(36).substring(7);

    if (!editorContainer) {
      console.error(
        `[LexicalEditor ${instanceId}] Critical: editorContainer element not found on mount!`
      );
      return;
    }

    const isDocument =
      enableTableCellResize ||
      documentPath?.toLowerCase().includes('/documents/') ||
      documentPath?.toLowerCase().includes('\\documents\\');
    const EFFECTIVE_MIN_WIDTH = isDocument ? 10 : 50;
    console.log(
      `[LexicalEditor ${instanceId}] isDocument:`,
      isDocument,
      'EFFECTIVE_MIN_WIDTH:',
      EFFECTIVE_MIN_WIDTH,
      'path:',
      documentPath
    );

    editor = createEditor({
      namespace: `SvelteLexicalEditor-${instanceId}`,
      nodes: editorNodes,
      theme: {
        paragraph: 'speech-plain-text',
        'live-transcription': 'text-gray-500 italic',
        text: {
          bold: 'font-bold',
          italic: 'italic',
          underline: 'underline',
          strikethrough: 'line-through'
        },
        heading: {
          h1: 'text-2xl font-bold mb-1 mt-2',
          h2: 'text-xl font-semibold mb-1 mt-1',
          h3: 'text-lg font-semibold mb-1'
        },
        list: {
          ul: 'list-disc list-outside mb-1 lexical-ul',
          ol: 'list-decimal list-outside mb-1 lexical-ol',
          checklist: 'list-none mb-1 pl-0',
          listitem: 'mb-0.5 relative lexical-li list-item-checkbox',
          nested: {
            listitem: 'lexical-nested-listitem'
          }
        },
        quote: 'border-l-4 border-gray-300 dark:border-gray-700 pl-4 ml-4 italic my-1',
        code: 'editor-code-block bg-gray-100 dark:bg-gray-700 dark:text-gray-200 p-4 my-2 block whitespace-pre-wrap overflow-x-auto',
        link: 'text-blue-600 dark:text-blue-400 underline cursor-pointer hover:text-blue-800 dark:hover:text-blue-300 link-text',
        table: `editor-table border-collapse border border-gray-300 dark:border-gray-700 my-2 table-fixed`,
        tableCell: `editor-table-cell border border-gray-300 dark:border-gray-700 px-2 py-1 align-top min-w-[20px] relative overflow-hidden`,
        tableCellHeader:
          'editor-table-cell-header text-gray-900 dark:text-gray-100 px-2 py-1 align-top min-w-[20px] font-normal border border-gray-300 dark:border-gray-700 overflow-hidden relative',
        tableRow: 'editor-table-row',
        tableCellResizer: 'editor-table-cell-resizer',
        placeholder:
          'lexical-placeholder-theme-class absolute top-0 left-0 text-gray-400 dark:text-gray-500 text-sm select-none pointer-events-none opacity-50 p-2',
        align_left: 'text-left',
        align_center: 'text-center',
        align_right: 'text-right',
        align_justify: 'text-justify'
      },
      onError: (error, editorInstance) => {
        console.error(`[LexicalEditor ${instanceId}] Editor Error:`, error);
      },
      editable: editable,
      historyState: historyState
    });

    editor.setRootElement(editorContainer);

    editorContainer.addEventListener(
      'click',
      (e) => {
        const anchor = e.target.closest('a');
        if (anchor) {
          e.preventDefault();
          return;
        }

        // Handle custom checklist clicks natively before Lexical gets it
        if (!editable || !editor) return;
        const closestLi = e.target.closest('li.list-item-checkbox');
        if (closestLi) {
          const rect = closestLi.getBoundingClientRect();
          const fontSizePx = parseFloat(window.getComputedStyle(closestLi).fontSize) || 15;
          const clickX = e.clientX;
          const clickY = e.clientY;

          // Checkbox pseudo-element bounding box (with padding), approximate 1.5em box
          const checkboxSizePx = fontSizePx * 1.5;
          const checkboxLeft = rect.left;
          const checkboxRight = rect.left + checkboxSizePx;
          const checkboxTop = rect.top;
          const checkboxBottom = rect.top + checkboxSizePx;

          if (
            clickX >= checkboxLeft &&
            clickX <= checkboxRight &&
            clickY >= checkboxTop &&
            clickY <= checkboxBottom
          ) {
            // We are inside the checkbox pseudo-element!
            e.stopPropagation();
            e.preventDefault();

            // Now safely update the editor state
            editor.update(() => {
              const liNode = _getNearestNodeFromDOMNode(closestLi);
              const nodeToToggle = _isListItemNode(liNode)
                ? liNode
                : _findMatchingParent(liNode, _isListItemNode);

              if (nodeToToggle) {
                const parentList = nodeToToggle.getParent();
                if (_isListNode(parentList) && parentList.getListType() === 'check') {
                  nodeToToggle.toggleChecked();
                }
              }
            });
          }
        }
      },
      true
    ); // Use capture phase so we get it before bubbling

    editorContainer.addEventListener('pointerdown', handlePointerDownOnContainer);
    editorWrapper.addEventListener('pointermove', handlePointerHover);
    editorContainer.addEventListener('contextmenu', handleContextMenu, true);
    editorContainer.addEventListener('keydown', handleShortcut);
    window.addEventListener('mousedown', handleClickOutside, true);

    isReady = true; // Set to true before registering listener and setting state

    unregisterListeners = mergeRegister(
      editor.registerUpdateListener(({ editorState, tags }) => {
        if (isReady) {
          try {
            editorState.read(() => {
              updateToolbarState();
              const selection = _getSelection();
              let newSelectedImageKey = null;
              if (_isNodeSelection(selection)) {
                const nodes = selection.getNodes();
                if (nodes.length === 1 && _isImageNode(nodes[0])) {
                  newSelectedImageKey = nodes[0].getKey();
                }
              }
              if (newSelectedImageKey !== selectedImageKey) {
                selectedImageKey = newSelectedImageKey;
                updateImageResizer();
              } else if (selectedImageKey) {
                updateImageResizer();
              }
            });
          } catch (readError) {
            console.error('Error reading editor state in update listener:', readError);
          }
          const jsonString = JSON.stringify(editorState.toJSON());
          let htmlString = '';
          let textContent = '';
          try {
            editorState.read(() => {
              htmlString = _generateHtmlFromNodes(editor);
              textContent = _getRoot().getTextContent();
            });
          } catch (htmlError) {
            console.error('Error generating HTML or text in update listener:', htmlError);
          }
          const chars = textContent.length;
          const words = textContent.trim() ? textContent.trim().split(/\s+/).length : 0;
          dispatch('change', {
            jsonString,
            htmlString,
            textContent,
            chars,
            words,
            tags: Array.from(tags)
          });
          dispatch('textcountchange', { chars, words });
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
            console.error('Error reading editor state in update listener (menu check):', readError);
          }
        }
      }),
      editor.registerCommand(
        SELECTION_CHANGE_COMMAND,
        () => {
          if (isReady && editor) {
            try {
              editor.getEditorState().read(updateToolbarState);
              if (enableFloatingToolbar) {
                showModifyToolbar = false;
                showCreateToolbar = false;
                clickedNodeKey = null;
              }
            } catch (readError) {
              console.error('Error reading state on selection change:', readError);
            }
          }
          return false;
        },
        COMMAND_PRIORITY_LOW
      ),
      editor.registerCommand(
        CLICK_COMMAND,
        (payload) => {
          const event = payload;
          if (event.button !== 0 || !editor) return false;

          let linkNode = null;
          let clickedCell = null;
          let clickedImageKey = null;

          try {
            editor.read(() => {
              const domNode = event.target;
              const targetNode = _getNearestNodeFromDOMNode(domNode);
              if (targetNode) {
                linkNode = _getNearestNodeOfType(targetNode, LinkNode);
                clickedCell = _findMatchingParent(targetNode, _isTableCellNode);
                if (_isImageNode(targetNode)) {
                  clickedImageKey = targetNode.getKey();
                } else if (_isDateNode(targetNode)) {
                  // Only handle special nodes if editable or explicitly allowed
                  if (editor.isEditable()) {
                    dateNodeToEditKey = targetNode.getKey();
                    dateInitialData = {
                      date: targetNode.__date,
                      format: targetNode.__format,
                      showTime: targetNode.__showTime,
                      timeFormat: targetNode.__timeFormat
                    };
                    showDateModal = true;
                    return true;
                  }
                } else if (_isEquationNode(targetNode)) {
                  if (editor.isEditable()) {
                    equationNodeToEditKey = targetNode.getKey();
                    equationInitialData = {
                      equation: targetNode.__equation,
                      inline: targetNode.__inline
                    };
                    showInsertEquationModal = true;
                    return true;
                  }
                }
              }
            });
          } catch (readError) {
            console.error('Error reading editor state during CLICK command:', readError);
            return false;
          }

          if (linkNode) {
            // ALWAYS prevent default for links to avoid external navigation
            event.preventDefault();
            console.log('Clicked on link node:', linkNode.getURL());
            currentModalUrl = linkNode.getURL();
            isEditingLink = true;
            showLinkModal = true;
            closeTableCellMenu(false);
            return true;
          }

          if (!editor.isEditable()) return false;

          if (clickedImageKey) {
            editor.update(() => {
              const nodeSelection = _createNodeSelection();
              nodeSelection.add(clickedImageKey);
              _setSelection(nodeSelection);
            });
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
          if (event.button !== 0 || !editor) return false;
          if (!editor.isEditable() && !allowReadModeHighlights) return false;

          editor.update(() => {
            const selection = _getSelection();
            if (selection && _isRangeSelection(selection) && selection.isCollapsed()) {
              const node = selection.anchor.getNode();
              const parent = node.getParent();
              if (_isExtendedTextNode(node) && node.getHighlightId()) {
                const domElement = editor.getElementByKey(node.getKey());
                if (domElement) {
                  const rect = domElement.getBoundingClientRect();
                  modifyToolbarPosition = {
                    top: rect.top - 45,
                    left: Math.max(5, rect.left + rect.width / 2 - 100)
                  };
                  showModifyToolbar = true;
                  clickedNodeKey = node.getKey();
                }
              } else if (_isExtendedTextNode(parent) && parent.getHighlightId()) {
                const domElement = editor.getElementByKey(parent.getKey());
                if (domElement) {
                  const rect = domElement.getBoundingClientRect();
                  modifyToolbarPosition = {
                    top: rect.top - 45,
                    left: Math.max(5, rect.left + rect.width / 2 - 100)
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
      registerCheckList(editor),
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
            console.error('Error reading state during Enter key check (High Priority):', readError);
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
      // Exit list on double Enter (pressing Enter on an empty list item)
      editor.registerCommand(
        KEY_ENTER_COMMAND,
        (event) => {
          if (!editor || !editor.isEditable()) return false;
          let isEmptyListItem = false;
          let listItemNode = null;
          let listNode = null;
          try {
            editor.getEditorState().read(() => {
              const selection = _getSelection();
              if (!_isRangeSelection(selection) || !selection.isCollapsed()) return;
              const anchorNode = selection.anchor.getNode();
              const li = _findMatchingParent(anchorNode, _isListItemNode);
              if (!li) return;
              const parent = li.getParent();
              if (!_isListNode(parent)) return;
              // Empty if the list item's text content is blank
              const text = li.getTextContent();
              if (text === '') {
                isEmptyListItem = true;
                listItemNode = li;
                listNode = parent;
              }
            });
          } catch (e) {
            console.error('Error reading state during list Enter check:', e);
            return false;
          }
          if (isEmptyListItem) {
            event.preventDefault();
            editor.update(() => {
              const li = listItemNode;
              const list = listNode;
              const children = list.getChildren();
              const index = children.indexOf(li);
              const paragraph = _createParagraphNode();

              if (children.length === 1) {
                // Only item in the list: Replace the list with a paragraph
                list.replace(paragraph);
              } else if (index === 0) {
                // First item in the list: Remove it and put paragraph before list
                li.remove();
                list.insertBefore(paragraph);
              } else if (index === children.length - 1) {
                // Last item in the list: Remove it and put paragraph after list
                li.remove();
                list.insertAfter(paragraph);
              } else {
                // Middle item: Split the list
                const listType = list.getListType();
                // Extract remaining siblings into an array before we start moving them
                const siblingsToMove = children.slice(index + 1);

                li.remove();

                const newList = _createListNode(listType);
                if (listType === 'number') newList.setStart(1); // Reset numbering for the split list

                for (const sibling of siblingsToMove) {
                  newList.append(sibling);
                }

                list.insertAfter(paragraph);
                paragraph.insertAfter(newList);
              }

              paragraph.select();
            });
            return true;
          }
          return false;
        },
        COMMAND_PRIORITY_NORMAL
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

            let defaultWidth = MIN_COLUMN_WIDTH;
            if (editorContainer) {
              const containerWidth = editorContainer.getBoundingClientRect().width;
              const availableWidth = containerWidth - 40; // 40px margin/padding buffer
              if (availableWidth > 0) {
                defaultWidth = Math.max(MIN_COLUMN_WIDTH, Math.floor(availableWidth / numCols));
              }
            }
            const newColWidths = Array(numCols).fill(defaultWidth);
            const tableNode = _createTableNode();
            tableNode.setColWidths(newColWidths);

            for (let i = 0; i < numRows; i++) {
              const rowNode = _createTableRowNode();
              for (let j = 0; j < numCols; j++) {
                const cellNode = _createTableCellNode({
                  headerState: TableCellHeaderStates.NO_STATUS
                });
                cellNode.append(_createParagraphNode());
                rowNode.append(cellNode);
              }
              tableNode.append(rowNode);
            }

            const focusNode = selection.focus.getNode();
            let parentBlock = _findMatchingParent(
              focusNode,
              (node) => _isElementNode(node) && !node.isInline()
            );
            if (!parentBlock)
              parentBlock =
                typeof focusNode.getTopLevelElement === 'function'
                  ? focusNode.getTopLevelElement()
                  : null;

            if (parentBlock && parentBlock.isEmpty() && _isParagraphNode(parentBlock)) {
              parentBlock.replace(tableNode);
            } else {
              _insertNodes([tableNode]);
            }
            const pBefore = _createParagraphNode();
            pBefore.append(_createTextNode('\u00A0')); // Add a non-breaking space to ensure it's selectable? Or just leave empty?
            // Usually an empty paragraph is fine in Lexical as it often has a default <br> internally.
            // But let's check other usages. Line 810 uses empty paragraph.

            const pAfter = _createParagraphNode();
            tableNode.insertBefore(pBefore);
            tableNode.insertAfter(pAfter);
            pAfter.selectStart();
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
                const currentIndex = cells.findIndex((c) => c === currentCellElement);

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
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        TOGGLE_LINK_COMMAND,
        (payload) => {
          if (!editor) return false;
          editor.update(() => {
            const selection = _getSelection();
            if (_isRangeSelection(selection) || _isTableSelection(selection)) {
              _toggleLink(payload);
            }
          });
          return true;
        },
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        FOCUS_COMMAND,
        () => {
          isFocused = true;
          updateToolbarState();
          return false;
        },
        COMMAND_PRIORITY_LOW
      ),
      editor.registerCommand(
        BLUR_COMMAND,
        () => {
          isFocused = false;
          updateToolbarState();
          return false;
        },
        COMMAND_PRIORITY_LOW
      ),
      editor.registerCommand(
        INSERT_HORIZONTAL_RULE_COMMAND,
        () => {
          editor.update(() => {
            const selection = _getSelection();
            if (!_isRangeSelection(selection)) {
              return;
            }
            const focusNode = selection.focus.getNode();

            if (focusNode !== null) {
              const horizontalRuleNode = _createHorizontalRuleNode();
              _insertNodes([horizontalRuleNode]);
            }
          });
          return true;
        },
        COMMAND_PRIORITY_EDITOR
      ),
      editor.registerCommand(
        INSERT_DATE_COMMAND,
        () => {
          openInsertDateDialog();
          return true;
        },
        COMMAND_PRIORITY_EDITOR
      )
    );

    unregisterListeners = mergeRegister(
      unregisterListeners,
      editor.registerUpdateListener(() => {
        editorUpdateTracker++;
      })
    );

    // Now set the initial state, which will trigger the listener we just registered
    let initialStateString = createInitialEditorState(initialJson);
    try {
      const parsedState = editor.parseEditorState(initialStateString);
      editor.setEditorState(parsedState);
      areNodesReady = true;

      // Force immediate text count dispatch for initial state since listener might skip it if unchanged
      editor.getEditorState().read(() => {
        const textContent = _getRoot().getTextContent();
        const chars = textContent.length;
        const words = textContent.trim() ? textContent.trim().split(/\s+/).length : 0;
        dispatch('textcountchange', { chars, words });
      });
    } catch (e) {
      console.error(`[LexicalEditor] Failed to parse and set initial editor state:`, e);
      editor.update(() => {
        const root = _getRoot();
        root.clear();
        root.append(_createParagraphNode());
        areNodesReady = true;
      });
    }

    tick().then(() => {
      if (!editor) return;
      // loadHighlights is already called via setEditorState if highlights were in the state
      loadHighlights();
      if (editor.isEditable()) {
        setTimeout(() => {
          if (editor) editor.focus();
        }, 0);
        try {
          editor.getEditorState().read(updateToolbarState);
        } catch (readError) {
          console.error('Error reading state during initial toolbar update:', readError);
        }
      }
    });

    return () => {
      unregisterListeners();
      window.removeEventListener('mousedown', handleClickOutside, true);
      if (editorWrapper) {
        editorWrapper.removeEventListener('pointermove', handlePointerHover);
      }
      if (editorContainer) {
        editorContainer.removeEventListener('pointerdown', handlePointerDownOnContainer);
        editorContainer.removeEventListener('contextmenu', handleContextMenu, true);
        editorContainer.removeEventListener('keydown', handleShortcut);
        editorContainer.removeEventListener(
          'click',
          (e) => {
            const anchor = e.target.closest('a');
            if (anchor) {
              e.preventDefault();
            }
          },
          true
        );
      }
      editor = null;
      isReady = false;
    };
  });

  export function removeImageByPath(imagePath) {
    if (!editor || !isReady || !editable) return;
    const filename = imagePath.split(/[\\/]/).pop();
    editor.update(() => {
      const root = _getRoot();
      const nodesToVisit = [root];
      while (nodesToVisit.length > 0) {
        const currentNode = nodesToVisit.pop();
        // Since $isImageNode wasn't aliased to _isImageNode locally, we just use getType()
        if (currentNode.getType() === 'image' && currentNode.getFilename() === filename) {
          currentNode.remove();
        } else if (typeof currentNode.getChildren === 'function') {
          const children = currentNode.getChildren();
          for (let i = children.length - 1; i >= 0; i--) {
            nodesToVisit.push(children[i]);
          }
        }
      }
    });
  }

  export function updateLiveTranscriptionText(
    text,
    isFinal,
    startTime,
    endTime,
    addTimestamps = false
  ) {
    if (!editor || !isReady || !editable) return;

    editor.update(() => {
      const root = _getRoot();
      let lastParagraph = root.getLastChild();
      let livePara = null;

      let trimmedText = text.trim();
      if (!trimmedText) return;

      if (addTimestamps) {
        // Check if the last paragraph is our dedicated live paragraph
        if (
          lastParagraph &&
          _isParagraphNode(lastParagraph) &&
          typeof lastParagraph.hasStyle === 'function' &&
          lastParagraph.hasStyle('live-transcription')
        ) {
          livePara = lastParagraph;
        } else {
          livePara = _createParagraphNode().setStyle('live-transcription');
          root.append(livePara);
        }

        if (isFinal) {
          // On final result, clear the live paragraph and append the final text.
          livePara.clear();
          const timestamp = `[${new Date(startTime * 1000).toISOString().substr(11, 12)} - ${new Date(endTime * 1000).toISOString().substr(11, 12)}]`;
          const finalText = timestamp + ' ' + trimmedText;
          livePara.append(_createTextNode(finalText));
          // Then, remove the style so it becomes a normal paragraph.
          livePara.setStyle('');
          // And create a new, empty live paragraph for the next utterance.
          const newLivePara = _createParagraphNode().setStyle('live-transcription');
          root.append(newLivePara);
          newLivePara.selectEnd();
        } else {
          // For interim results, replace the content of the live paragraph.
          livePara.clear();
          livePara.append(_createTextNode(trimmedText));
          livePara.selectEnd();
        }
      } else {
        // When not adding timestamps, append to the same paragraph unless the user created a new one.
        if (!lastParagraph || !_isParagraphNode(lastParagraph)) {
          lastParagraph = _createParagraphNode();
          root.append(lastParagraph);
        }

        // Find if there is an existing live interim text node at the end of the paragraph
        let liveTextNode = null;
        const children = lastParagraph.getChildren();
        if (children.length > 0) {
          const lastChild = children[children.length - 1];
          if (
            _isTextNode(lastChild) &&
            typeof lastChild.hasStyle === 'function' &&
            lastChild.hasStyle('live-transcription')
          ) {
            liveTextNode = lastChild;
          }
        }

        if (isFinal) {
          // Remove interim node if it exists
          if (liveTextNode) {
            liveTextNode.remove();
          }

          // Determine if we need a leading space before appending
          let prefixSpace = '';
          const currentTextContent = lastParagraph.getTextContent();
          if (
            currentTextContent.length > 0 &&
            !currentTextContent.endsWith(' ') &&
            !/^[.,!?]/.test(trimmedText)
          ) {
            prefixSpace = ' ';
          }

          const finalNode = _createTextNode(prefixSpace + trimmedText + ' ');
          lastParagraph.append(finalNode);
          finalNode.selectEnd();
        } else {
          // Update interim node
          let prefixSpace = '';
          const currentTextContent = lastParagraph.getTextContent();
          // When evaluating prefix space for interim, ignore the live node text itself
          const textWithoutLive = liveTextNode
            ? currentTextContent.substring(
                0,
                currentTextContent.length - liveTextNode.getTextContent().length
              )
            : currentTextContent;
          if (
            textWithoutLive.length > 0 &&
            !textWithoutLive.endsWith(' ') &&
            !/^[.,!?]/.test(trimmedText)
          ) {
            prefixSpace = ' ';
          }

          if (!liveTextNode) {
            liveTextNode = _createTextNode(prefixSpace + trimmedText).setStyle(
              'live-transcription'
            );
            lastParagraph.append(liveTextNode);
          } else {
            liveTextNode.setTextContent(prefixSpace + trimmedText);
          }
          liveTextNode.selectEnd();
        }
      }
    });
  }

  export function resetEditorState(jsonString = null) {
    if (!editor) {
      console.warn('[LexicalEditor] resetEditorState called before editor initialized.');
      return;
    }
    console.log('[LexicalEditor] resetEditorState called.');
    closeTableCellMenu(false);
    areNodesReady = false;
    editor.update(() => {
      try {
        let newState;
        let stateToParse = jsonString;
        if (
          !stateToParse ||
          typeof stateToParse !== 'string' ||
          stateToParse.trim() === '' ||
          stateToParse === 'null' ||
          stateToParse === 'undefined'
        ) {
          stateToParse = JSON.stringify({
            root: {
              children: [
                {
                  children: [],
                  direction: null,
                  format: '',
                  indent: 0,
                  type: 'paragraph',
                  version: 1
                }
              ],
              direction: null,
              format: '',
              indent: 0,
              type: 'root',
              version: 1
            }
          });
        } else if (!stateToParse.startsWith('{') || !stateToParse.endsWith('}')) {
          console.warn(
            '[LexicalEditor] resetEditorState received non-JSON object string, wrapping in paragraph.'
          );
          const pNode = _createParagraphNode();
          pNode.append(_createTextNode(stateToParse));
          stateToParse = JSON.stringify({
            root: {
              children: [pNode.exportJSON()],
              direction: null,
              format: '',
              indent: 0,
              type: 'root',
              version: 1
            }
          });
        }
        newState = editor.parseEditorState(stateToParse);
        editor.setEditorState(newState);

        historyState.undoStack = [];
        historyState.redoStack = [];
        initialSyncDone = false; // Allow syncLayout() to treat this as a fresh run
        areNodesReady = true;
      } catch (e) {
        console.error(
          '[LexicalEditor] Error parsing JSON during resetEditorState:',
          e,
          'Attempted JSON:',
          jsonString?.substring(0, 100)
        );
        try {
          editor.setEditorState(
            editor.parseEditorState(
              JSON.stringify({
                root: {
                  children: [
                    {
                      children: [],
                      direction: null,
                      format: '',
                      indent: 0,
                      type: 'paragraph',
                      version: 1
                    }
                  ],
                  direction: null,
                  format: '',
                  indent: 0,
                  type: 'root',
                  version: 1
                }
              })
            )
          );
          historyState.undoStack = [];
          historyState.redoStack = [];
          initialSyncDone = false; // Also for fallback
          areNodesReady = true;
        } catch (fallbackError) {
          console.error(
            '[LexicalEditor] CRITICAL: Failed to set even fallback state during resetEditorState:',
            fallbackError
          );
        }
      }
    });

    // Explicitly trigger sync for the new content
    // Use a small delay to allow Lexical to process the update, then sync layout.
    // We also add a secondary check in case the first one misses the tables due to async rendering.
    setTimeout(() => {
      syncLayout(true); // First attempt
      setTimeout(() => {
        if (!initialSyncDone) {
          console.debug(
            `[LexicalEditor resetEditorState] Secondary syncLayout attempt for ${documentPath}`
          );
          syncLayout(true);
        }
      }, 300);
    }, 50);
  }

  $: if (editor && typeof editor.setEditable === 'function') {
    editor.setEditable(editable);
    if (!editable) {
      closeTableCellMenu(false);
    }
  }

  export function updateContent(newJsonString) {
    if (!editor) {
      console.warn('[LexicalEditor] updateContent called but editor not initialized.');
      return;
    }
    if (!isReady) {
      console.warn('[LexicalEditor] updateContent called before editor is ready.');
      return;
    }
    closeTableCellMenu(false);
    editor.update(
      () => {
        try {
          let parsedState;
          if (
            newJsonString &&
            typeof newJsonString === 'string' &&
            newJsonString.startsWith('{') &&
            newJsonString.endsWith('}')
          ) {
            parsedState = editor.parseEditorState(newJsonString);
            let isValid = false;
            try {
              parsedState.read(() => {
                const root = _getRoot();
                isValid = !!root && root.getType() === 'root';
              });
            } catch (readErr) {
              console.error('Error validating parsed state in updateContent:', readErr);
              isValid = false;
            }
            if (!isValid) {
              console.error(
                '[LexicalEditor] Invalid state structure after parsing in updateContent. Aborting.'
              );
              return;
            }
          } else {
            console.error(
              '[LexicalEditor] Invalid JSON string format provided to updateContent:',
              newJsonString ? newJsonString.substring(0, 200) + '...' : 'null'
            );
            return;
          }
          editor.setEditorState(parsedState, { tag: 'history-merge' });
        } catch (e) {
          console.error('[LexicalEditor] Failed to parse JSON in updateContent:', e);
          console.error(
            '[LexicalEditor] Faulty JSON for updateContent:',
            newJsonString ? newJsonString.substring(0, 200) + '...' : 'null'
          );
        }
      },
      { tag: 'external' }
    );
  }

  export function getScrollElement() {
    return editorWrapper;
  }

  export function getTopVisibleRowInfo() {
    if (!editorWrapper || !editor) return { index: -1, offset: 0 };

    const wrapperRect = editorWrapper.getBoundingClientRect();

    // Attempt fast path using elementFromPoint
    // We check a point slightly inside the wrapper to find the row at the top
    const centerX = wrapperRect.left + wrapperRect.width / 2;
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

  export function insertImageByPath(imagePath) {
    if (!imagePath) return;
    handleInsertImageAttached({ detail: { path: imagePath } });
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
          info.visible = rowRect.bottom > wrapperRect.top && rowRect.top < wrapperRect.bottom;
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
    if (!editor || !isReady) {
      return;
    }
    const selection = _getSelection();
    isBold = false;
    isItalic = false;
    isUnderline = false;
    isStrikethrough = false;
    isLink = false;
    blockType = 'paragraph';
    selectedAlignment = 'left';
    selectedTextColor = '#000000';
    selectedHighlightColor = 'transparent';
    selectedFontFamily = 'Inter';
    selectedFontSize = '15px';

    if (_isRangeSelection(selection)) {
      isBold = selection.hasFormat('bold');
      isItalic = selection.hasFormat('italic');
      isUnderline = selection.hasFormat('underline');
      isStrikethrough = selection.hasFormat('strikethrough');
      selectedTextColor =
        _getSelectionStyleValueForProperty(selection, 'color', '#000000') || '#000000';
      selectedHighlightColor =
        _getSelectionStyleValueForProperty(selection, 'background-color', 'transparent') ||
        'transparent';
      selectedFontFamily =
        _getSelectionStyleValueForProperty(selection, 'font-family', 'Inter') || 'Inter';

      const rawFontSize =
        _getSelectionStyleValueForProperty(selection, 'font-size', '15px') || '15px';
      selectedFontSize = rawFontSize.replace('px', '');

      const anchorNode = selection.anchor.getNode();
      if (anchorNode) {
        let element = _findMatchingParent(
          anchorNode,
          (node) => _isElementNode(node) && !node.isInline()
        );
        if (!element) {
          let maybeTopLevel = anchorNode;
          while (
            maybeTopLevel &&
            maybeTopLevel.getParent() &&
            !_getRoot().is(maybeTopLevel.getParent())
          ) {
            maybeTopLevel = maybeTopLevel.getParent();
          }
          if (_isElementNode(maybeTopLevel) && !maybeTopLevel.isInline()) {
            element = maybeTopLevel;
          }
          if (!element) {
            element =
              _findMatchingParent(anchorNode, _isParagraphNode) || anchorNode.getTopLevelElement();
          }
        }

        if (element && typeof element.getType === 'function') {
          const type = element.getType();
          if (_isHeadingNode(element)) {
            blockType = element.getTag();
          } else if (_isListItemNode(element)) {
            const parentList = _findMatchingParent(element, _isListNode);
            if (parentList) {
              const listType = parentList.getListType();
              blockType =
                listType === 'bullet'
                  ? 'ul'
                  : listType === 'number'
                    ? 'ol'
                    : listType === 'check'
                      ? 'check'
                      : 'paragraph';
            } else {
              blockType = 'paragraph';
            }
          } else if (_isTableCellNode(element)) {
            const firstChild = element.getFirstChild();
            if (_isHeadingNode(firstChild)) {
              blockType = firstChild.getTag();
            } else if (_isListNode(firstChild)) {
              const listType = firstChild.getListType();
              blockType =
                listType === 'bullet'
                  ? 'ul'
                  : listType === 'number'
                    ? 'ol'
                    : listType === 'check'
                      ? 'check'
                      : 'paragraph';
            } else if (_isQuoteNode(firstChild)) {
              blockType = 'quote';
            } else if (_isCodeNode(firstChild)) {
              blockType = 'code';
            } else {
              blockType = 'paragraph';
            }
          } else if (type === 'paragraph' || type === 'quote' || type === 'code') {
            blockType = type;
          } else {
            blockType = 'paragraph';
          }

          let formatElement = element;
          if (_isTableCellNode(element)) {
            formatElement = element.getFirstChild();
          } else if (element.isInline?.()) {
            formatElement = element.getParent();
          }

          if (_isElementNode(formatElement) && typeof formatElement.getFormatType === 'function') {
            selectedAlignment = formatElement.getFormatType() || 'left';
          } else {
            selectedAlignment = 'left';
          }
        } else {
          blockType = 'paragraph';
          selectedAlignment = 'left';
          isLink = false;
        }

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
      } else {
        blockType = 'paragraph';
        selectedAlignment = 'left';
        isLink = false;
        internalCursorRowIndex = -1;
      }
    } else if (_isTableSelection(selection)) {
      blockType = 'paragraph';
      selectedAlignment = 'left';
      isLink = false;
      isBold = false;
      isItalic = false;
      isUnderline = false;
      isStrikethrough = false;
      selectedTextColor = '#000000';
      selectedHighlightColor = 'transparent';
      internalCursorRowIndex = -1;
    } else {
      isBold = false;
      isItalic = false;
      isUnderline = false;
      isStrikethrough = false;
      isLink = false;
      blockType = 'paragraph';
      selectedAlignment = 'left';
      selectedTextColor = '#000000';
      selectedHighlightColor = 'transparent';
      internalCursorRowIndex = -1;
    }

    isBold = isBold;
    isItalic = isItalic;
    isUnderline = isUnderline;
    isStrikethrough = isStrikethrough;
    isLink = isLink;
    blockType = blockType;
    selectedAlignment = selectedAlignment;
    selectedTextColor = selectedTextColor;
    selectedHighlightColor = selectedHighlightColor;
    canUndo = historyState.undoStack.length > 0;
    canRedo = historyState.redoStack.length > 0;
  }

  // Reactive row highlighting logic
  $: if (
    editorWrapper &&
    (internalCursorRowIndex !== undefined || externalHighlightedRowIndex !== undefined)
  ) {
    const rows = editorWrapper.querySelectorAll('.editor-table-row');
    rows.forEach((row, i) => {
      const shouldGlow =
        i === externalHighlightedRowIndex || (i === internalCursorRowIndex && isFocused);
      if (shouldGlow) {
        row.classList.add('cursor-row-glow');
      } else {
        row.classList.remove('cursor-row-glow');
      }
    });
  }

  function formatText(formatType) {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(FORMAT_TEXT_COMMAND, formatType);
  }
  function alignElement(alignType) {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(FORMAT_ELEMENT_COMMAND, alignType);
    isAlignDropdownOpen = false;
  }
  function indentContent() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(INDENT_CONTENT_COMMAND, undefined);
  }
  function outdentContent() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(OUTDENT_CONTENT_COMMAND, undefined);
  }

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
    if (!editor || (!editable && !allowReadModeHighlights)) return null;
    let generatedHighlightId = null;
    editor.update(() => {
      const selection = _getSelection();
      if (_isTableSelection(selection)) {
        _normalizeSelection(selection);
      }
      const normalizedSelection = _getSelection();
      if (_isRangeSelection(normalizedSelection)) {
        const styles = {};

        if (colorToApply !== 'transparent') {
          styles['background-color'] = colorToApply;
        } else {
          styles['background-color'] = null;
        }

        _patchStyleText(normalizedSelection, styles);

        const selectedNodes = normalizedSelection.getNodes();
        generatedHighlightId = uuidv4();
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
              extendedNode.setHighlightId(generatedHighlightId);
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
    return generatedHighlightId;
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
    const currentHighlights = documentHighlights || [];
    const existingHighlightsMap = new Map(currentHighlights.map((h) => [h.id, h]));

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
        block.forEach((n) => n.setHighlightId(highlightId));
      } else {
        seenIds.add(highlightId);
      }

      const metadata = existingHighlightsMap.get(originalId);

      finalHighlights.push({
        id: highlightId,
        text: block.map((n) => n.getTextContent()).join(''),
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

    previousDocumentHighlightsIds = new Set(highlights.map((h) => h.id));
    dispatch('highlightschange', { highlights });
  }

  function scrollToHighlight(id, currentEditor) {
    if (!id || !currentEditor) return;

    let attempts = 0;
    const maxAttempts = 15; // Increased attempts

    const tryScroll = () => {
      if (!currentEditor) return;

      // Recursive function to find node by highlight ID - MUST be called inside tryScroll to retry search
      const findNodeKey = () => {
        let foundKey = null;
        try {
          if (currentEditor && typeof currentEditor.getEditorState === 'function') {
            currentEditor.getEditorState().read(() => {
              const root = _getRoot();
              const nodesToVisit = [root];
              while (nodesToVisit.length > 0) {
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
          }
        } catch (error) {
          console.error('[LexicalEditor] Error in scrollToHighlight:', error);
        }
        return foundKey;
      };

      const targetNodeKey = findNodeKey();

      if (targetNodeKey) {
        const domElement = currentEditor.getElementByKey(targetNodeKey);
        if (domElement) {
          console.log(
            `[LexicalEditor] Scrolling to highlight ${id} (Node ${targetNodeKey}) after ${attempts} attempts`
          );
          domElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
          // Pulse effect
          domElement.style.transition = 'outline 0.3s ease';
          domElement.style.outline = '4px solid #3b82f6';
          domElement.style.outlineOffset = '2px';
          setTimeout(() => {
            if (domElement) domElement.style.outline = 'none';
          }, 2000);

          // Success - clear the request
          project.update((p) => {
            if (p.requestedHighlightId === id) return { ...p, requestedHighlightId: null };
            return p;
          });
          return;
        }
      }

      // If either node not found OR DOM element not found, retry
      if (attempts < maxAttempts) {
        attempts++;
        setTimeout(tryScroll, 150); // Slightly longer delay between retries
      } else {
        console.warn(
          `[LexicalEditor] Failed to scroll to highlight ${id} after ${maxAttempts} attempts. Node found: ${!!targetNodeKey}`
        );
        project.update((p) => {
          if (p.requestedHighlightId === id) return { ...p, requestedHighlightId: null };
          return p;
        });
      }
    };

    // Give Lexical a moment to finish its current update cycle if any
    setTimeout(tryScroll, 50);
  }

  // Trigger scroll when editor is ready AND highlights are loaded AND nodes are loaded AND there is a requested ID
  $: if (
    $project.requestedHighlightId &&
    isReady &&
    areHighlightsReady &&
    areNodesReady &&
    editor
  ) {
    scrollToHighlight($project.requestedHighlightId, editor);
  }

  function handleBlockTypeChange(event) {
    const type = event.target.value;
    if (!editor || !isReady || !editor.isEditable()) return;
    if (
      type === 'paragraph' ||
      type === 'h1' ||
      type === 'h2' ||
      type === 'h3' ||
      type === 'quote' ||
      type === 'code'
    ) {
      editor.update(() => {
        const selection = _getSelection();
        if (_isTableSelection(selection)) {
          _normalizeSelection(selection);
        }
        const normalizedSelection = _getSelection();
        if (_isRangeSelection(normalizedSelection)) {
          const createNodeFn =
            type === 'paragraph'
              ? _createParagraphNode
              : type === 'h1'
                ? () => _createHeadingNode('h1')
                : type === 'h2'
                  ? () => _createHeadingNode('h2')
                  : type === 'h3'
                    ? () => _createHeadingNode('h3')
                    : type === 'quote'
                      ? _createQuoteNode
                      : type === 'code'
                        ? _createCodeNode
                        : null;
          if (createNodeFn) {
            _setBlocksType(normalizedSelection, createNodeFn);
          }
        }
      });
    } else if (type === 'ul') {
      editor.dispatchCommand(INSERT_UNORDERED_LIST_COMMAND, undefined);
    } else if (type === 'ol') {
      editor.dispatchCommand(INSERT_ORDERED_LIST_COMMAND, undefined);
    } else if (type === 'check') {
      editor.dispatchCommand(INSERT_CHECK_LIST_COMMAND, undefined);
    }
  }

  function clearFormatting() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.update(() => {
      const selection = _getSelection();
      if (_isRangeSelection(selection)) {
        try {
          selection.getNodes().forEach((node) => {
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
              children.forEach((child) => {
                if (_isTextNode(child)) {
                  if (_isExtendedTextNode(child)) {
                    const childHighlightId = child.getHighlightId();
                    if (childHighlightId) {
                      dispatch('highlightevent', {
                        type: 'remove',
                        id: childHighlightId,
                        nodeKey: child.getKey(),
                        color: 'transparent'
                      });
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
          console.error('[LexicalEditor] Error during clearFormatting (Range):', error);
        }
      } else if (_isTableSelection(selection)) {
        try {
          _normalizeSelection(selection);
          const rangeSelection = _getSelection();
          if (_isRangeSelection(rangeSelection)) {
            rangeSelection.getNodes().forEach((node) => {
              if (_isExtendedTextNode(node)) {
                const highlightId = node.getHighlightId();
                if (highlightId) {
                  dispatch('highlightevent', {
                    type: 'remove',
                    id: highlightId,
                    nodeKey: node.getKey(),
                    color: 'transparent'
                  });
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
          console.error('[LexicalEditor] Error during clearFormatting (Table):', error);
        }
      }
    });
  }

  function undo() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(UNDO_COMMAND, undefined);
  }
  function redo() {
    if (!editor || !isReady || !editor.isEditable()) return;
    editor.dispatchCommand(REDO_COMMAND, undefined);
  }

  async function toggleLink() {
    if (!editor || !editable) return;
    closeAllDropdowns();
    closeTableCellMenu(false);
    currentModalUrl = '';
    isEditingLink = false;
    editor.focus();
    await tick();
    try {
      editor.getEditorState().read(() => {
        const selection = _getSelection();
        if (_isRangeSelection(selection)) {
          savedSelection = selection.clone();
          const node = selection.anchor.getNode();
          const parent = node.getParent();
          if (_isLinkNode(parent)) {
            currentModalUrl = parent.getURL();
            isEditingLink = true;
          } else if (_isLinkNode(node)) {
            currentModalUrl = node.getURL();
            isEditingLink = true;
          } else {
            currentModalUrl = '';
            isEditingLink = false;
          }
        } else {
          savedSelection = null;
        }
      });
    } catch (readError) {
      console.error('Error reading state for toggleLink:', readError);
      return;
    }
    if (!isEditingLink && (!savedSelection || savedSelection.isCollapsed())) {
      console.warn('Cannot toggle link without a text selection or editing an existing link.');
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
      if (savedSelection) {
        _setSelection(savedSelection.clone());
      }
      if (url && url.trim() !== '') {
        console.log('Dispatching TOGGLE_LINK_COMMAND with URL:', url);
        editor.dispatchCommand(TOGGLE_LINK_COMMAND, url.trim());
      } else {
        console.log('Dispatching TOGGLE_LINK_COMMAND with null (empty URL received).');
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
      if (savedSelection) {
        _setSelection(savedSelection.clone());
      }
      console.log('Dispatching TOGGLE_LINK_COMMAND with null to remove link.');
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
      console.error('Error reading editor state during context menu:', readError);
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
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
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
        const prevCell = Array.from(prevRow.children).find((c) =>
          c.classList.contains('editor-table-cell')
        );
        if (prevCell) return { element: prevCell, direction: 'row' };
      }
    }

    return null;
  }

  function handlePointerHover(event) {
    if (!editorContainer || !editorWrapper || isResizing || !editor) return;

    if (enableSegmentPlayback) {
      const x = event.clientX;
      const y = event.clientY;
      const wrapperRect = editorWrapper.getBoundingClientRect();

      // Check if we are in the gutter (first 60px of the wrapper)
      const isWithinGutterX = x >= wrapperRect.left && x <= wrapperRect.left + 60;

      // If we are in the gutter, scan slightly to the right to find the row at this Y level
      const scanX = isWithinGutterX ? wrapperRect.left + 80 : x;

      // Use elementsFromPoint to find the row
      const elements = document.elementsFromPoint(scanX, y);
      const rowElement = elements.find(
        (el) => el.classList?.contains('editor-table-row') || el.closest?.('.editor-table-row')
      );
      const actualRow = rowElement?.classList?.contains('editor-table-row')
        ? rowElement
        : rowElement?.closest?.('.editor-table-row');

      if (actualRow) {
        // Skip if this is a header row
        const isHeaderRow =
          actualRow.querySelector('th') || actualRow.querySelector('.editor-table-cell-header');
        // Also skip if it's the very first row of the table (index 0), as this is invariably the header in our transcript structure
        // We use actualRow.rowIndex if available (standard HTMLTableRowElement), or fallback to checking siblings
        const isFirstRow = actualRow.rowIndex === 0 || !actualRow.previousElementSibling;

        if (isHeaderRow || isFirstRow) {
          if (showPlayButton) {
            showPlayButton = false;
            hoveredRowKey = null;
          }
          // We continue here to allow resize detection even if over header
        } else {
          let rowKey = actualRow.getAttribute('data-lexical-key');

          // Fallback if data-lexical-key is missing from DOM
          if (!rowKey) {
            editor.read(() => {
              const node = _getNearestNodeFromDOMNode(actualRow);
              if (node) rowKey = node.getKey();
            });
          }

          if (rowKey && rowKey !== hoveredRowKey) {
            hoveredRowKey = rowKey;
            const rect = actualRow.getBoundingClientRect();

            // Position button in the gutter (left: 20px relative to wrapper)
            playButtonPosition = {
              top: rect.top - wrapperRect.top + editorWrapper.scrollTop + rect.height / 2,
              left: 20
            };
            showPlayButton = true;
          }
        }
      } else {
        // If NOT over a row, we hide if we are also NOT over the play button itself
        // and NOT in the gutter (to prevent flickering)
        const currentElements = document.elementsFromPoint(x, y);
        const isOverPlayButton = currentElements.some((el) =>
          el.classList?.contains('play-segment-hover-btn')
        );

        if (!isOverPlayButton && !isWithinGutterX) {
          if (showPlayButton) {
            showPlayButton = false;
            hoveredRowKey = null;
          }
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
            if (
              /(?:\d{1,2}:)?\d{1,2}:\d{1,2}(?:\.\d{1,3})?\s*-\s*(?:\d{1,2}:)?\d{1,2}:\d{1,2}(?:\.\d{1,3})?/.test(
                text
              )
            ) {
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
            notificationStore.add(
              'Invalid timestamp values. Expected format: MM:SS.mmm or HH:MM:SS.mmm',
              'error'
            );
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
        console.error('Error reading editor state during resize check:', readError);
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

      window.addEventListener('pointermove', handlePointerMove);
      window.addEventListener('pointerup', handlePointerUp);
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
    const targetCellElement = editorContainer.querySelector(
      `[data-lexical-key="${resizeTargetCellKey}"]`
    );
    const tableElement = targetCellElement?.closest('.editor-table');
    if (!tableElement) {
      resizerLineStyle = 'display: none;';
      return;
    }
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

    const scrollPos = editorWrapper ? editorWrapper.scrollTop : 0;
    editor.update(
      () => {
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
              const currentWidths = (tableNode.getColWidths() || []).map((w) =>
                w === null ? undefined : w
              );
              let currentWidthVal = currentWidths[targetColIndex];

              const localMinWidth = 20;

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

              // Ensure ALL columns have valid widths from DOM if missing to avoid abrupt layout shifts
              const colCount = tableMap.columns;
              while (currentWidths.length < colCount) {
                currentWidths.push(undefined);
              }

              for (let i = 0; i < colCount; i++) {
                if (
                  currentWidths[i] === undefined ||
                  currentWidths[i] === null ||
                  isNaN(currentWidths[i]) ||
                  (typeof currentWidths[i] === 'string' && currentWidths[i].endsWith('%'))
                ) {
                  // Find any cell in this column to get its width
                  const cellInfo = tableMap.grid.find((row) => row[i] && row[i].node);
                  if (cellInfo) {
                    const rowWithCell = tableMap.grid.find((r) => r[i]);
                    const cellNode = rowWithCell[i].node;
                    const domElement = editor.getElementByKey(cellNode.getKey());
                    if (domElement) {
                      currentWidths[i] = domElement.getBoundingClientRect().width / zoom;
                    } else {
                      currentWidths[i] = localMinWidth;
                    }
                  } else {
                    currentWidths[i] = localMinWidth;
                  }
                } else if (typeof currentWidths[i] === 'string') {
                  currentWidths[i] = parseFloat(currentWidths[i]);
                }
              }

              // Re-fetch currentWidthVal after full initialization
              currentWidthVal = currentWidths[targetColIndex];

              const newWidth = Math.max(localMinWidth, currentWidthVal + diffX);
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
                    currentHeight = rowElement
                      ? rowElement.getBoundingClientRect().height
                      : MIN_HEIGHT;
                  }
                  const newHeight = Math.max(MIN_HEIGHT, currentHeight + diffY);
                  rowNode.setHeight(newHeight);
                }
              }
            }
          }
        } catch (e) {
          console.error('Error during table resize update:', e);
        }
      },
      { tag: 'skip-scroll' }
    );

    if (editorWrapper) {
      tick().then(() => {
        editorWrapper.scrollTop = scrollPos;
      });
    }

    isResizing = false;
    resizeDirection = null;
    resizeTargetCellKey = null;
    resizerLineStyle = 'display: none;';
    document.body.style.cursor = 'auto';
    if (editorContainer) editorContainer.style.cursor = 'auto';

    window.removeEventListener('pointermove', handlePointerMove);
    window.removeEventListener('pointerup', handlePointerUp);
  }

  function handleDocumentHighlightsChange(highlights) {
    if (!editor || !areHighlightsReady || !areNodesReady) return;

    const currentHighlights = highlights || [];
    const currentHighlightsIds = new Set(currentHighlights.map((h) => h.id));

    // 1. Handle Deletions: Find nodes with IDs not in currentHighlightsIds and clear them
    let deletedIds = new Set();
    for (const id of previousDocumentHighlightsIds) {
      if (!currentHighlightsIds.has(id)) {
        deletedIds.add(id);
      }
    }

    if (deletedIds.size > 0) {
      editor.update(() => {
        const root = _getRoot();
        const nodesToVisit = [root];
        while (nodesToVisit.length > 0) {
          const currentNode = nodesToVisit.pop();
          if (_isExtendedTextNode(currentNode)) {
            const id = currentNode.getHighlightId();
            if (id && deletedIds.has(id)) {
              currentNode.setStyle('');
              currentNode.setHighlightId(null);
            }
          }
          if (currentNode.getChildren) {
            const children = currentNode.getChildren();
            for (let i = children.length - 1; i >= 0; i--) {
              nodesToVisit.push(children[i]);
            }
          }
        }
      });
    }

    // 2. Handle Additions/Updates: Ensure all highlights in the array are applied
    editor.update(() => {
      const root = _getRoot();
      const existingNodeMap = new Map(); // id -> nodeKey

      // First pass: scan for existing highlight IDs in the editor
      const nodesToVisit = [root];
      while (nodesToVisit.length > 0) {
        const currentNode = nodesToVisit.pop();
        if (_isExtendedTextNode(currentNode)) {
          const id = currentNode.getHighlightId();
          if (id) {
            existingNodeMap.set(id, currentNode.getKey());
          }
        }
        if (currentNode.getChildren) {
          const children = currentNode.getChildren();
          for (let i = children.length - 1; i >= 0; i--) {
            nodesToVisit.push(children[i]);
          }
        }
      }

      // Second pass: apply missing or updated highlights
      for (const highlight of currentHighlights) {
        let node = null;
        if (existingNodeMap.has(highlight.id)) {
          node = _getNodeByKey(existingNodeMap.get(highlight.id));
        } else if (highlight.nodeKey) {
          // Try applying by nodeKey if not found by ID (for newly created highlights)
          node = _getNodeByKey(highlight.nodeKey);
        }

        if (_isExtendedTextNode(node)) {
          // Check if we need to update style or ID
          const currentStyle = node.getStyle() || '';
          const targetStyle = `background-color: ${highlight.color}`;
          if (node.getHighlightId() !== highlight.id || !currentStyle.includes(targetStyle)) {
            node.setStyle(targetStyle);
            node.setHighlightId(highlight.id);
          }
        }
      }
    });

    // Update our reference tracker
    previousDocumentHighlightsIds = currentHighlightsIds;
  }

  $: handleDocumentHighlightsChange(documentHighlights);

  // Effect to resolve and inject absolute src for ImageNodes based on their relative filenames
  // We place it down here where reactive statements are collected
  let resolveImagesTimeout;
  $: if (
    editor &&
    areNodesReady &&
    documentPath &&
    $project.baseDirectory &&
    editorUpdateTracker >= 0
  ) {
    // Trigger evaluation when these change, but also need to hook into editor updates
    clearTimeout(resolveImagesTimeout);
    resolveImagesTimeout = setTimeout(async () => {
      if (!editorContainer) return;
      try {
        const { convertFileSrc } = await import('@tauri-apps/api/core');
        const images = editorContainer.querySelectorAll('img[data-filename]:not([src])');

        if (images.length > 0) {
          const separator = documentPath.includes('\\') ? '\\' : '/';
          const parts = documentPath.split(separator);
          parts.pop(); // Remove the JSON filename
          let dirPath = parts.join(separator);
          if (dirPath && !dirPath.endsWith(separator)) dirPath += separator;

          let absDirPath;
          // If documentPath is already absolute, use it directly. Otherwise, prepend baseDir.
          if (documentPath.startsWith($project.baseDirectory)) {
            absDirPath = `${dirPath}attachments${separator}`;
          } else {
            let baseDir = $project.baseDirectory;
            if (baseDir && !baseDir.endsWith(separator)) baseDir += separator;
            absDirPath = `${baseDir}${dirPath}attachments${separator}`;
          }

          for (const img of images) {
            const filename = img.getAttribute('data-filename');
            if (filename) {
              const fullPath = `${absDirPath}${filename}`;
              img.src = convertFileSrc(fullPath);
            }
          }
        }
      } catch (e) {
        console.error('[LexicalEditor] Error resolving image sources', e);
      }
    }, 300);
  }

  function updateSearchHighlights() {
    if (typeof CSS === 'undefined' || !CSS.highlights) {
      return;
    }

    // Fast path: if no results or search inactive (and replace modal closed), clear everything immediately
    if (
      (!showSearchBox && !showFindReplaceModal) ||
      !searchTerm.trim() ||
      searchResults.length === 0
    ) {
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
      result.nodes.forEach((nodeMatch) => {
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
        console.warn('Invalid search pattern:', term);
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
        dispatch('searchindexchanged', {
          currentIndex: -1,
          currentResult: null
        });
      }

      updateSearchHighlights();
    });

    dispatch('searchresultsupdated', { results: searchResults, term: term });
  }

  function toggleSearchOptionsDropdown() {
    const nextState = !showSearchOptionsDropdown;
    closeAllDropdowns();
    showSearchOptionsDropdown = nextState;
  }

  function openFindReplaceModal() {
    showSearchOptionsDropdown = false;
    showFindReplaceModal = true;
  }

  function handleReplace(event) {
    const { find, replace } = event.detail;
    if (currentSearchResultIndex >= 0 && searchResults.length > 0) {
      const result = searchResults[currentSearchResultIndex];

      editor.update(
        () => {
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
                console.error('Replace failed:', e);
              }
            }
          }
        },
        { tag: 'replace-one' }
      );

      executeSearch(find);
    }
  }

  function handleReplaceAll(event) {
    const { find, replace } = event.detail;
    if (searchResults.length === 0) return;

    editor.update(
      () => {
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
      },
      { tag: 'replace-all' }
    );

    executeSearch(find);
  }

  function clearSearchTermInput() {
    console.log('[clearSearchTermInput] Called.');
    searchTerm = '';
    searchResults = [];
    currentSearchResultIndex = -1;
    updateSearchHighlights();

    const updateData = { results: searchResults, term: searchTerm };
    const indexChangeData = {
      currentIndex: currentSearchResultIndex,
      currentResult: null
    };

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
    console.log(
      '[navigateToResult] Called with index:',
      index,
      'Total results:',
      searchResults.length
    );

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

    editor.update(
      () => {
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
      },
      { tag: 'search-navigate' }
    );

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

    const dispatchData = {
      currentIndex: currentSearchResultIndex,
      currentResult: result
    };
    dispatch('searchindexchanged', dispatchData);
  }

  function navigateToPreviousResult() {
    console.log(
      '[navigateToPreviousResult] Called. currentSearchResultIndex:',
      currentSearchResultIndex,
      'Total results:',
      searchResults.length
    );
    if (searchResults.length === 0) return;

    let newIndex = currentSearchResultIndex - 1;
    if (newIndex < 0) newIndex = searchResults.length - 1;

    navigateToResult(newIndex, false);
  }

  function navigateToNextResult() {
    console.log(
      '[navigateToNextResult] Called. currentSearchResultIndex:',
      currentSearchResultIndex,
      'Total results:',
      searchResults.length
    );
    if (searchResults.length === 0) return;

    let newIndex = currentSearchResultIndex + 1;
    if (newIndex >= searchResults.length) newIndex = 0;

    navigateToResult(newIndex, false);
  }

  let previousLayout = null;
  let initialSyncDone = false;

  function syncLayout(isFromReset = false) {
    if (!editor) return;
    const layoutConfig = DOCX_LAYOUT_COLUMN_CONFIGS[activeLayout];
    if (!layoutConfig) return;

    // Detect if layout actively changed by user interaction
    const layoutChanged = activeLayout !== previousLayout;

    // Capture initial status BEFORE updating it, for use in the closure
    const isInitialRun = !initialSyncDone || isFromReset;

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

      allTableNodes.forEach((tableNode) => {
        // Set column widths
        if (layoutConfig.colgroup) {
          const currentWidths = tableNode.getColWidths();
          const hasWidths =
            currentWidths &&
            currentWidths.length > 0 &&
            !currentWidths.every((w) => w === undefined || w === null);

          // Apply defaults ONLY if:
          // 1. Table has no widths set (new/raw table)
          // 2. OR Layout was explicitly changed by user (runtime switch), NOT just initial load
          // 3. OR it's initial run AND we have a layout defined AND does NOT have percentages set yet (fixed pixels, null, or undefined)
          // We use isInitialRun (captured const) to safely check this inside the callback
          const numCols = currentWidths ? currentWidths.length : 0;

          // Determine if this is a correct column count for the active layout (usually 4 for transcripts)
          // If numCols is 0, it might be uninitialized, which we'll allow on initialRun to set defaults.
          const isCorrectColCount =
            layoutConfig.colgroup &&
            (numCols === layoutConfig.colgroup.length || (isInitialRun && numCols === 0));

          if (documentPath?.includes('transcripts') || !initialSyncDone) {
            console.debug(`[LexicalEditor syncLayout] Checking table widths for ${documentPath}:`, {
              numCols,
              hasWidths,
              currentWidths,
              isInitialRun,
              isCorrectColCount,
              activeLayout
            });
          }

          const hasPercentWidths = hasWidths && currentWidths.some((w) => typeof w === 'string');
          const hasPixelWidths = hasWidths && currentWidths.every((w) => typeof w === 'number');

          // Determine if we should strictly avoid touching this table's widths
          // Documents should prefer their stored pixel widths once set.
          const isDocument = documentPath?.includes('/Documents/');
          const shouldBypass = hasPixelWidths && (isDocument || !layoutChanged);

          if (
            (!hasWidths ||
              (layoutChanged && !isInitialRun && isCorrectColCount) ||
              (isInitialRun && isCorrectColCount && !hasPixelWidths)) &&
            !shouldBypass
          ) {
            if (documentPath?.includes('transcripts') || !initialSyncDone) {
              console.log(
                `[LexicalEditor syncLayout] APPLYING layout widths for ${documentPath}:`,
                layoutConfig.colgroup
              );
            }
            const newColWidths = layoutConfig.colgroup;
            tableNode.setColWidths(newColWidths);
          } else {
            if (documentPath?.includes('transcripts') || !initialSyncDone) {
              console.debug(
                `[LexicalEditor syncLayout] BYPASSING layout apply (already has pixels or explicit widths):`,
                {
                  hasWidths,
                  hasPixelWidths,
                  isDocument,
                  shouldBypass,
                  layoutChanged,
                  isInitialRun,
                  isCorrectColCount
                }
              );
            }
          }
        }

        // Hide columns
        const rows = tableNode.getChildren();
        rows.forEach((row) => {
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

  function handleMouseUp() {
    if (!editor || (!editable && !allowReadModeHighlights)) return;
    if (!toolbarConfig.highlight) return;

    // Small delay to ensure Lexical has updated its internal selection state
    setTimeout(() => {
      editor.getEditorState().read(() => {
        const selection = _getSelection();
        if (_isRangeSelection(selection) && !selection.isCollapsed()) {
          const domSelection = window.getSelection();
          if (domSelection.rangeCount > 0) {
            const range = domSelection.getRangeAt(0);
            const rect = range.getBoundingClientRect();
            const containerRect = editorWrapper.getBoundingClientRect();

            // Position toolbar above the selection, centered
            createToolbarPosition = {
              top: rect.top - 45,
              left: Math.max(5, rect.left + rect.width / 2 - 100)
            };

            // Boundary check for top
            if (createToolbarPosition.top < 5) createToolbarPosition.top = 5;

            showCreateToolbar = true;
            showModifyToolbar = false; // Ensure modify toolbar is hidden
          }
        } else {
          showCreateToolbar = false;
        }
      });
    }, 20);
  }

  function handleCreateHighlight(color) {
    const newHighlightId = applyHighlightColor(color);
    if (newHighlightId) {
      showCreateToolbar = false;
      // Seamless transition to Modify Flow (Flow B)
      setTimeout(() => {
        clickedHighlightId = newHighlightId;
        modifyToolbarPosition = { ...createToolbarPosition };

        // Find the node key to pass to the modify toolbar
        editor.getEditorState().read(() => {
          const root = _getRoot();
          const nodesToVisit = [root];
          while (nodesToVisit.length > 0) {
            const node = nodesToVisit.pop();
            if (_isExtendedTextNode(node) && node.getHighlightId() === newHighlightId) {
              clickedNodeKey = node.getKey();
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
        showModifyToolbar = true;
      }, 50);
    }
  }

  function handleCreateTagDirectly(tagName) {
    // Flow C: Assign tag directly to unhighlighted text (Defaults to Yellow)
    const newHighlightId = applyHighlightColor('#FFF275'); // Default yellow
    if (newHighlightId) {
      showCreateToolbar = false;

      // Immediately assign the tag to the new highlight in the store
      toggleTagInHighlightLocal(
        newHighlightId,
        tagName,
        $project.selectedDocumentPath === documentPath
          ? $project.selectedDocumentType
          : $project.currentStandaloneTranscriptPath === documentPath
            ? 'standalone_transcript'
            : $project.activeTranscriptPathInDataTab === documentPath
              ? $project.activeTranscriptTypeInDataTab || 'audio_transcript'
              : 'doc',
        documentPath
      );

      setTimeout(() => {
        clickedHighlightId = newHighlightId;
        modifyToolbarPosition = { ...createToolbarPosition };

        editor.getEditorState().read(() => {
          const root = _getRoot();
          const nodesToVisit = [root];
          while (nodesToVisit.length > 0) {
            const node = nodesToVisit.pop();
            if (_isExtendedTextNode(node) && node.getHighlightId() === newHighlightId) {
              clickedNodeKey = node.getKey();
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
        showModifyToolbar = true;
      }, 50);
    }
  }

  function handleRemoveHighlightFromToolbar() {
    applyHighlightColor('transparent');
    showCreateToolbar = false;
  }
</script>

<div
  bind:this={editorRoot}
  class="lexical-editor-root h-full flex flex-col {backgroundClass} shadow-sm layout-{activeLayout}"
  class:resizing-disabled={!enableTableCellResize}
  style="overflow: visible;"
>
  {#if editable || allowReadModeHighlights || toolbarConfig.search || $$slots.toolbar_prepend}
    <div
      class="toolbar relative flex items-center flex-nowrap gap-x-1 border-b border-gray-300 dark:border-gray-700 h-9 px-2 flex-shrink-0 bg-white dark:bg-gray-950 shadow-md z-10 overflow-x-auto"
    >
      <slot name="toolbar_prepend"></slot>

      {#if toolbarConfig.undo}
        <button
          class="mini-toolbar-button"
          on:click={undo}
          title="Undo ({modLabel}+Z)"
          disabled={!editable || !canUndo}><Undo2 size={14} /></button
        >
      {/if}
      {#if toolbarConfig.redo}
        <button
          class="mini-toolbar-button"
          on:click={redo}
          title="Redo ({modLabel}+{isMac ? 'Shift+Z' : 'Y'})"
          disabled={!editable || !canRedo}><Redo2 size={14} /></button
        >
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
            {#if blockType === 'h1'}<Heading1 size={14} />{:else if blockType === 'h2'}<Heading2
                size={14}
              />{:else if blockType === 'h3'}<Heading3
                size={14}
              />{:else if blockType === 'ul'}<List
                size={14}
              />{:else if blockType === 'ol'}<ListOrdered
                size={14}
              />{:else if blockType === 'check'}<ListChecks
                size={14}
              />{:else if blockType === 'quote'}<QuoteIcon
                size={14}
              />{:else if blockType === 'code'}<CodeIcon size={14} />{:else}<Type size={14} />{/if}
            <ChevronDown size={12} class="ml-0.5" />
          </button>
          {#if isBlockDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-64 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-hidden"
            >
              {#each blockTypeOptions as option (option.value)}
                <div
                  class="px-3 py-1 flex justify-between items-center cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => selectBlockType(option.value)}
                  role="menuitem"
                  tabindex="-1"
                >
                  <span class="flex items-center gap-3 mr-3">
                    <svelte:component this={blockTypeIcons[option.value]} size={16} />
                    <span>{option.label}</span>
                  </span>
                  <span class="text-xs text-gray-500">{option.shortcut}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if toolbarConfig.fontFamily}
        <div class="relative" bind:this={fontDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center gap-1 min-w-[100px] justify-between"
            on:click={toggleFontDropdown}
            title="Font Family"
            disabled={!editable}
          >
            <span class="truncate"
              >{fontOptions.find((f) => f.value === selectedFontFamily)?.label ??
                selectedFontFamily}</span
            >
            <ChevronDown class="ml-0.5 h-3 w-3 flex-shrink-0" />
          </button>
          {#if isFontDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-y-auto max-h-64"
            >
              {#each fontOptions as option (option.value)}
                <div
                  class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                  on:click={() => applyFontFamily(option.value)}
                  style="font-family: {option.value}"
                  role="menuitem"
                  tabindex="-1"
                >
                  {option.label}
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="relative flex items-center gap-0.5" bind:this={fontSizeDropdownRef}>
          <button
            class="mini-toolbar-button !px-1"
            on:click={() => updateFontSize(-1)}
            on:mousedown|preventDefault
            title="Decrease Font Size"
            disabled={!editable}
          >
            -
          </button>
          <button
            class="mini-toolbar-button flex items-center justify-center min-w-[32px] px-1"
            on:click={toggleFontSizeDropdown}
            on:mousedown|preventDefault
            title="Font Size"
            disabled={!editable}
          >
            <span class="truncate">{selectedFontSize}</span>
          </button>
          <button
            class="mini-toolbar-button !px-1"
            on:click={() => updateFontSize(1)}
            on:mousedown|preventDefault
            title="Increase Font Size"
            disabled={!editable}
          >
            +
          </button>
          {#if isFontSizeDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-24 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-y-auto max-h-64"
            >
              {#each fontSizeOptions as size (size)}
                <div
                  class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                  on:click={() => applyFontSize(size)}
                  role="menuitem"
                  tabindex="-1"
                >
                  {size}
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
        <button
          class="mini-toolbar-button font-bold"
          on:click={() => formatText('bold')}
          class:active={isBold}
          title="Bold ({modLabel}+B)"
          disabled={!editable}><BoldIcon size={14} /></button
        >
      {/if}
      {#if toolbarConfig.italic}
        <button
          class="mini-toolbar-button italic"
          on:click={() => formatText('italic')}
          class:active={isItalic}
          title="Italic ({modLabel}+I)"
          disabled={!editable}><ItalicIcon size={14} /></button
        >
      {/if}
      {#if toolbarConfig.underline}
        <button
          class="mini-toolbar-button underline"
          on:click={() => formatText('underline')}
          class:active={isUnderline}
          title="Underline ({modLabel}+U)"
          disabled={!editable}><UnderlineIcon size={14} /></button
        >
      {/if}
      {#if toolbarConfig.strikethrough}
        <div class="relative" bind:this={textFormatDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center"
            on:click={toggleTextFormatDropdown}
            title="Text Formatting"
            disabled={!editable}
            class:active={isStrikethrough || isTextFormatDropdownOpen}
          >
            <CaseSensitive size={15} strokeWidth={1.5} />
            <ChevronDown size={12} strokeWidth={1.5} class="ml-1" />
          </button>
          {#if isTextFormatDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-hidden flex flex-col"
            >
              <div
                class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('uppercase')}
                role="menuitem"
                tabindex="-1"
              >
                Uppercase
              </div>
              <div
                class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('lowercase')}
                role="menuitem"
                tabindex="-1"
              >
                Lowercase
              </div>
              <div
                class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('sentencecase')}
                role="menuitem"
                tabindex="-1"
              >
                Sentencecase
              </div>
              <div
                class="px-3 py-1.5 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('capitalize')}
                role="menuitem"
                tabindex="-1"
              >
                Capitalize
              </div>

              <div class="w-full h-px bg-gray-200 dark:bg-gray-600 my-1"></div>

              <div
                class="px-3 py-1.5 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('strikethrough')}
                role="menuitem"
                tabindex="-1"
              >
                <StrikethroughIcon size={14} />
                <span>Strikethrough</span>
              </div>
              <div
                class="px-3 py-1.5 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('subscript')}
                role="menuitem"
                tabindex="-1"
              >
                <SubscriptIcon size={14} />
                <span>Subscript</span>
              </div>
              <div
                class="px-3 py-1.5 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200 text-sm"
                on:click={() => applyTextFormat('superscript')}
                role="menuitem"
                tabindex="-1"
              >
                <SuperscriptIcon size={14} />
                <span>Superscript</span>
              </div>
            </div>
          {/if}
        </div>
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
            <Plus class="h-4 w-4" />
            <span class="ml-1 hidden sm:inline">Insert</span>
            <ChevronDown size={12} class="ml-1" />
          </button>
          {#if isInsertDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-hidden"
            >
              {#each insertOptions as option (option.label)}
                <div
                  class="px-3 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={option.action}
                  role="menuitem"
                  tabindex="-1"
                >
                  <svelte:component this={option.iconComponent} size={14} />
                  <span>{option.label}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if (toolbarConfig.insertMenu || toolbarConfig.link || toolbarConfig.bold) && toolbarConfig.align}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.align}
        <div class="relative" bind:this={alignmentDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center"
            on:click={toggleAlignDropdown}
            title="Alignment"
            disabled={!editable}
          >
            {#if selectedAlignment === 'left'}<AlignLeft
                size={14}
              />{:else if selectedAlignment === 'center'}<AlignCenter
                size={14}
              />{:else if selectedAlignment === 'right'}<AlignRight size={14} />{:else}<AlignJustify
                size={14}
              />{/if}
            <ChevronDown size={12} class="ml-1" />
          </button>
          {#if isAlignDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-40 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg overflow-hidden"
            >
              {#each alignmentOptions as option (option.value)}
                <div
                  class="px-3 py-1 flex items-center cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => alignElement(option.value)}
                  role="menuitem"
                  tabindex="-1"
                >
                  <span class="flex items-center gap-3">
                    {#if option.value === 'left'}<AlignLeft
                        size={14}
                      />{:else if option.value === 'center'}<AlignCenter
                        size={14}
                      />{:else if option.value === 'right'}<AlignRight
                        size={14}
                      />{:else}<AlignJustify size={14} />{/if}
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
        <button
          class="mini-toolbar-button"
          on:click={outdentContent}
          title="Outdent"
          disabled={!editable}
        >
          <Outdent size={14} />
        </button>
      {/if}
      {#if toolbarConfig.indent}
        <button
          class="mini-toolbar-button"
          on:click={indentContent}
          title="Indent"
          disabled={!editable}
        >
          <Indent size={14} />
        </button>
      {/if}
      {#if (toolbarConfig.outdent || toolbarConfig.indent) && toolbarConfig.textColor}
        <div class="separator"></div>
      {/if}
      {#if toolbarConfig.textColor}
        <div class="relative" bind:this={colorDropdownRef}>
          <button
            class="mini-toolbar-button flex items-center"
            on:click={toggleColorDropdown}
            title="Text Color"
            disabled={!editable}
            style="color: {selectedTextColor === 'transparent'
              ? 'currentColor'
              : selectedTextColor}"
          >
            <Baseline size={14} />
            <ChevronDown size={12} class="ml-1" />
          </button>
          {#if isColorDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-48 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 shadow-lg"
            >
              {#each colorOptions as option (option.value)}
                <div
                  class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => applyTextColor(option.value)}
                  role="menuitemradio"
                  aria-checked={selectedTextColor === option.value}
                  tabindex="-1"
                >
                  <span
                    class="w-4 h-4 border border-gray-400 dark:border-gray-500 rounded-full"
                    style="background-color: {option.value === 'transparent'
                      ? '#fff'
                      : option.value};"
                  ></span>
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
          <button
            class="mini-toolbar-button flex items-center"
            on:click={toggleHighlightDropdown}
            title="Highlight Color"
            disabled={!editable && !allowReadModeHighlights}
            style="background-color: {selectedHighlightColor === 'transparent'
              ? 'transparent'
              : selectedHighlightColor}; color: {selectedHighlightColor !== 'transparent' &&
            selectedHighlightColor !== null
              ? '#000'
              : 'currentColor'}"
          >
            <Highlighter size={14} />
            <ChevronDown size={12} class="ml-1" />
          </button>
          {#if isHighlightDropdownOpen}
            <div
              use:portal
              style={dropdownStyle}
              class="w-32 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 shadow-lg"
            >
              {#each highlightOptions as option (option.value)}
                <div
                  class="px-2 py-1 flex items-center gap-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-200"
                  on:click={() => applyHighlightColor(option.value)}
                  role="menuitemradio"
                  aria-checked={selectedHighlightColor === option.value}
                  tabindex="-1"
                >
                  <span
                    class="w-4 h-4 rounded-full border border-gray-400 dark:border-gray-500"
                    style="background-color: {option.value};"
                  ></span>
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
        <button
          class="mini-toolbar-button"
          on:click={clearFormatting}
          title="Clear Formatting"
          disabled={!editable}
        >
          <Eraser size={14} />
        </button>
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
            <Search class="w-4 h-4" />
          </button>

          {#if showSearchBox}
            <div
              class="absolute right-0 top-full mt-1 z-20 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 shadow-lg p-2 flex items-center gap-2 min-w-[320px] rounded"
              bind:this={searchUiContainerElement}
            >
              <div class="relative flex-grow flex items-center">
                <input
                  type="text"
                  placeholder="Search..."
                  class="w-full text-xs border border-gray-300 dark:border-gray-700 pl-2 pr-16 py-1 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:ring-blue-500 focus:border-blue-500 rounded outline-none search-input-with-count"
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
                      <X class="w-3 h-3" />
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
                  <ChevronLeft class="w-3.5 h-3.5" />
                </button>
                <button
                  class="mini-toolbar-button !p-1"
                  on:click={navigateToNextResult}
                  disabled={searchResults.length === 0}
                  title="Next Match"
                >
                  <ChevronRight class="w-3.5 h-3.5" />
                </button>

                <div class="relative" bind:this={searchOptionsDropdownRef}>
                  <button
                    class="mini-toolbar-button !p-1"
                    on:click={toggleSearchOptionsDropdown}
                    title="Search Options"
                  >
                    <MoreVertical class="w-3.5 h-3.5" />
                  </button>
                  {#if showSearchOptionsDropdown}
                    <div
                      class="absolute right-0 top-full mt-1 z-30 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-700 shadow-lg rounded overflow-hidden min-w-[120px]"
                    >
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
    class="lexical-wrapper flex-grow min-h-0 relative overflow-visible"
    style={enableSegmentPlayback ? 'padding-left: 2.5rem !important;' : ''}
    bind:this={editorWrapper}
  >
    <div
      bind:this={editorContainer}
      class="lexical-content focus:outline-none min-h-full h-auto relative"
      contenteditable={editable ? 'true' : 'false'}
      role="textbox"
      aria-multiline="true"
      spellcheck={['ul', 'ol', 'check'].includes(blockType) ? 'false' : 'true'}
      autocomplete={['ul', 'ol', 'check'].includes(blockType) ? 'off' : 'on'}
      autocorrect={['ul', 'ol', 'check'].includes(blockType) ? 'off' : 'on'}
      autocapitalize={['ul', 'ol', 'check'].includes(blockType) ? 'off' : 'on'}
      data-placeholder={placeholder}
      on:mouseup={handleMouseUp}
    ></div>

    <div class="resizer-line" style={resizerLineStyle}></div>

    {#if selectedImageKey && imageResizerRect}
      <div
        class="absolute border-2 border-blue-500 box-border pointer-events-none z-[100]"
        style="top: {imageResizerRect.top}px; left: {imageResizerRect.left}px; width: {imageResizerRect.width}px; height: {imageResizerRect.height}px;"
      >
        <div
          class="absolute w-3 h-3 bg-blue-500 border border-white cursor-nwse-resize pointer-events-auto shadow-sm"
          style="top: -6px; left: -6px;"
          on:pointerdown={(e) => handleImageResizeStart(e, 'nw')}
        ></div>
        <div
          class="absolute w-3 h-3 bg-blue-500 border border-white cursor-nesw-resize pointer-events-auto shadow-sm"
          style="top: -6px; right: -6px;"
          on:pointerdown={(e) => handleImageResizeStart(e, 'ne')}
        ></div>
        <div
          class="absolute w-3 h-3 bg-blue-500 border border-white cursor-nesw-resize pointer-events-auto shadow-sm"
          style="bottom: -6px; left: -6px;"
          on:pointerdown={(e) => handleImageResizeStart(e, 'sw')}
        ></div>
        <div
          class="absolute w-3 h-3 bg-blue-500 border border-white cursor-nwse-resize pointer-events-auto shadow-sm"
          style="bottom: -6px; right: -6px;"
          on:pointerdown={(e) => handleImageResizeStart(e, 'se')}
        ></div>
      </div>
    {/if}

    {#if showPlayButton}
      <button
        class="play-segment-hover-btn absolute z-30 w-6 h-6 flex items-center justify-center bg-blue-600 hover:bg-blue-700 text-white rounded-full shadow-md transition-all duration-200 border-2 border-white dark:border-gray-800"
        style="top: {playButtonPosition.top}px; left: {playButtonPosition.left}px; transform: translateY(-50%);"
        on:click|stopPropagation={handlePlaySegmentClick}
        title="Play this segment"
      >
        <Play class="w-3.5 h-3.5 ml-0.5 fill-current" />
      </button>
    {/if}
  </div>

  {#if enableTableCellMenu}
    <TableCellActionMenu
      {editor}
      anchorElement={editorWrapper}
      bind:isOpen={showTableCellMenu}
      bind:cellNodeKey={activeTableCellKey}
      bind:position={tableCellMenuPosition}
      on:close={handleTableCellMenuClose}
    />
  {/if}

  <LinkModal
    bind:showModal={showLinkModal}
    initialUrl={currentModalUrl}
    isEditing={isEditingLink}
    on:confirm={handleLinkConfirm}
    on:delete={handleLinkDelete}
    on:close={() => (showLinkModal = false)}
  />

  <FindReplaceModal
    bind:showModal={showFindReplaceModal}
    bind:initialSearchTerm={searchTerm}
    currentMatchIndex={currentSearchResultIndex}
    totalMatches={searchResults.length}
    on:replace={handleReplace}
    on:replaceall={handleReplaceAll}
    on:findnext={navigateToNextResult}
    on:findprev={navigateToPreviousResult}
    on:findchange={(e) =>
      executeSearch(e.detail.term, {
        isCaseSensitive: e.detail.isCaseSensitive,
        isRegex: e.detail.isRegex,
        isWholeWord: e.detail.isWholeWord
      })}
    on:close={() => (showFindReplaceModal = false)}
  />

  <InsertTableModal
    bind:showModal={showInsertTableModal}
    on:confirm={handleInsertTableConfirm}
    on:close={() => (showInsertTableModal = false)}
  />

  <InsertImageModal
    bind:showModal={showInsertImageModal}
    {documentPath}
    on:insert_attached={handleInsertImageAttached}
    on:insert_external={handleInsertImageExternal}
  />

  <InsertEquationModal
    bind:showModal={showInsertEquationModal}
    initialEquation={equationInitialData.equation}
    initialInline={equationInitialData.inline}
    on:confirm={handleEquationConfirm}
    on:close={() => {
      showInsertEquationModal = false;
      equationNodeToEditKey = null;
    }}
  />

  <DatePromptModal
    bind:showModal={showDateModal}
    isEditing={!!dateNodeToEditKey}
    initialDate={dateInitialData.date}
    initialFormat={dateInitialData.format}
    initialShowTime={dateInitialData.showTime}
    initialTimeFormat={dateInitialData.timeFormat}
    on:confirm={handleDateConfirm}
    on:delete={handleDateDelete}
    on:cancel={() => {
      showDateModal = false;
      dateNodeToEditKey = null;
    }}
  />

  {#if enableFloatingToolbar}
    <FloatingModifyHighlightToolbar
      {editor}
      showToolbar={showModifyToolbar}
      toolbarPosition={modifyToolbarPosition}
      highlightId={clickedHighlightId}
      docType={$project.selectedDocumentPath === documentPath
        ? $project.selectedDocumentType
        : $project.currentStandaloneTranscriptPath === documentPath
          ? 'standalone_transcript'
          : $project.activeTranscriptPathInDataTab === documentPath
            ? $project.activeTranscriptTypeInDataTab || 'audio_transcript'
            : 'doc'}
      filePath={documentPath}
      on:close={() => {
        showModifyToolbar = false;
        clickedNodeKey = null;
      }}
      onChangeColor={(color) => {
        if (clickedNodeKey) {
          editor.update(() => {
            const clickedNode = _getNodeByKey(clickedNodeKey);
            if (!_isExtendedTextNode(clickedNode)) return;
            const highlightId = clickedNode.getHighlightId();
            if (!highlightId) return;

            const root = _getRoot();
            const nodesToVisit = [root];
            while (nodesToVisit.length > 0) {
              const currentNode = nodesToVisit.pop();
              if (
                _isExtendedTextNode(currentNode) &&
                currentNode.getHighlightId() === highlightId
              ) {
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
            while (nodesToVisit.length > 0) {
              const currentNode = nodesToVisit.pop();
              if (
                _isExtendedTextNode(currentNode) &&
                currentNode.getHighlightId() === highlightId
              ) {
                currentNode.setStyle('');
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

    <FloatingHighlightToolbar
      showToolbar={showCreateToolbar}
      toolbarPosition={createToolbarPosition}
      onHighlight={handleCreateHighlight}
      onRemoveHighlight={handleRemoveHighlightFromToolbar}
      onTagToggle={handleCreateTagDirectly}
    />
  {/if}
</div>

<style lang="postcss">
  .toolbar button.mini-toolbar-button,
  .toolbar select.mini-toolbar-select {
    @apply p-1 rounded inline-flex items-center justify-center
             focus:outline-none focus:ring-1 focus:ring-offset-1 focus:ring-blue-500
             dark:focus:ring-offset-[var(--app-bg)] transition duration-150 ease-in-out
             text-xs disabled:opacity-50 disabled:cursor-not-allowed;
    color: var(--ui-icon-color);
    border: 1px solid var(--ui-select-border);
    background-color: transparent; /* Default for light mode, will be overridden by dark mode or specific hover */
    margin-right: 2px;
    line-height: 1.2;
    min-height: 24px;
    height: 24px;
  }

  .toolbar button.mini-toolbar-button:hover:not(:disabled),
  .toolbar select.mini-toolbar-select:hover:not(:disabled) {
    background-color: var(--ui-icon-hover-bg);
    border-color: var(--ui-select-border);
  }

  html.dark .toolbar button.mini-toolbar-button,
  html.dark .toolbar select.mini-toolbar-select {
    color: #e5e5e5;
    border: 1px solid #404040;
    background-color: transparent;
  }

  html.dark .toolbar button.mini-toolbar-button:hover:not(:disabled),
  html.dark .toolbar select.mini-toolbar-select:hover:not(:disabled) {
    background-color: #404040;
    border-color: #404040;
  }

  html.dark .toolbar button.mini-toolbar-button.active {
    @apply bg-blue-500 text-white;
  }

  /* Unordered and Ordered List padding */
  :global(.lexical-ul),
  :global(.lexical-ol) {
    padding-left: 0;
    margin-left: 0;
  }

  :global(.lexical-ul > li.lexical-li),
  :global(.lexical-ol > li.lexical-li) {
    margin-left: 2em;
    padding-left: 0.25em;
  }

  /* Nested list items shouldn't inherit outer list styles */
  :global(.lexical-content .lexical-nested-listitem) {
    list-style-type: none !important;
  }
  :global(.lexical-content .lexical-nested-listitem::before),
  :global(.lexical-content .lexical-nested-listitem::after) {
    display: none !important;
  }

  /* Checklist item styles */
  :global(.lexical-content ul.list-none > li.lexical-li.list-item-checkbox) {
    padding-left: 1.5em;
    margin-left: 0;
    list-style-type: none;
    position: relative;
  }

  :global(.lexical-content ul.list-none > li.lexical-li.list-item-checkbox::before) {
    content: '';
    position: absolute;
    left: 0.125em;
    top: 0.25em;
    width: 1em;
    height: 1em;
    border: 1px solid #ccc;
    border-radius: 0.1875em;
    background-color: transparent;
    cursor: pointer;
  }

  :global(
    .lexical-content ul.list-none > li.lexical-li.list-item-checkbox[aria-checked='true']::before
  ) {
    background-color: #3b82f6; /* Tailwind blue-500 */
    border-color: #3b82f6;
  }

  :global(
    .lexical-content ul.list-none > li.lexical-li.list-item-checkbox[aria-checked='true']::after
  ) {
    content: '';
    position: absolute;
    left: 0.4375em;
    top: 0.375em;
    width: 0.3125em;
    height: 0.625em;
    border: solid white;
    border-width: 0 0.125em 0.125em 0;
    transform: rotate(45deg);
    pointer-events: none;
  }

  :global(.lexical-content ul.list-none > li.lexical-li.list-item-checkbox[aria-checked='true']) {
    text-decoration: line-through;
    color: #888;
  }

  .lexical-content {
    min-width: 150px; /* Prevent it from being too tiny when empty */
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .editor-table {
    border-collapse: collapse;
    width: 100%;
    table-layout: auto; /* Default to auto for standard documents */
  }

  /* Force fixed layout only when resizing is disabled (Transcripts) */
  .lexizing-disabled .editor-table,
  .resizing-disabled .editor-table {
    table-layout: fixed;
  }

  .editor-table-cell {
    border: 1px solid #ccc;
    padding: 8px;
    /* min-width is managed via theme to allow per-document flexibility */
    position: relative; /* Needed for resizer positioning */
    font-weight: normal;
  }

  .editor-table-cell-header {
    /* Background removed and weight normalized to match normal cells as per user request */
    font-weight: normal;
  }

  .resizer-line {
    /* The style is dynamically applied in the script */
  }

  .indent-outdent-icon {
    transform: scaleX(-1); /* Flips the icon horizontally */
  }

  :global(.editor-code-block) {
    font-family: Monaco, Consolas, 'Lucida Console', monospace;
    line-height: 1.6 !important;
  }

  button.active {
    @apply bg-gray-300 dark:bg-gray-500;
  }

  /* Make text black if it has a background color in dark mode (improves contrast) */
  :global(html.dark .lexical-content [style*='background-color']) {
    color: black;
  }
  :global(html.dark .lexical-content [style*='background-color: transparent']) {
    color: white; /* Revert if it's explicitly transparent */
  }

  /* Ensure link color applies to text - targeting anchor tags directly inside editor */
  :global(.lexical-content a),
  :global(.link-text) {
    color: #2563eb !important; /* blue-600 */
    text-decoration: underline !important;
  }
  /* Force children (like Lexical's inner spans) to inherit the blue color */
  :global(.lexical-content a *),
  :global(.link-text *) {
    color: inherit !important;
  }

  :global(html.dark .lexical-content a),
  :global(html.dark .link-text) {
    color: #60a5fa !important; /* blue-400 */
  }
  :global(.lexical-content a:hover),
  :global(.link-text:hover) {
    color: #1e40af !important; /* blue-800 */
  }
  :global(html.dark .lexical-content a:hover),
  :global(html.dark .link-text:hover) {
    color: #93c5fd !important; /* blue-300 */
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
    box-shadow:
      inset 0 0 4px 1px rgba(59, 130, 246, 0.5),
      0 0 4px 1px rgba(59, 130, 246, 0.5);
    background-color: rgba(59, 130, 246, 0.05);
    transition:
      box-shadow 0.2s ease,
      background-color 0.2s ease;
    z-index: 5;
    position: relative;
  }

  :global(html.dark .editor-table-row.cursor-row-glow) {
    box-shadow:
      inset 0 0 6px 1px rgba(96, 165, 250, 0.4),
      0 0 6px 1px rgba(96, 165, 250, 0.4);
    background-color: rgba(96, 165, 250, 0.1);
  }

  .play-segment-hover-btn {
    pointer-events: auto;
    cursor: pointer;
    opacity: 0.9;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .play-segment-hover-btn:hover {
    opacity: 1;
    transform: translateY(-50%) scale(1.1);
  }

  /* =================================================================== */
  /* LAYOUT SPECIFIC RENDERING RULES FOR TRANSCRIPT TABLES               */
  /* =================================================================== */

  /* STYLES FOR LAYOUT 1 (Detailed Table) - Only apply fixed logic if resizing is disabled */
  .lexical-editor-root.layout-Layout1.resizing-disabled :global(.lexical-content table) {
    table-layout: fixed !important;
    width: 100% !important;
  }
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table th:nth-child(1)),
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table td:nth-child(1)) {
    width: 5% !important;
    min-width: 40px;
  }
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table th:nth-child(2)),
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table td:nth-child(2)) {
    width: 15% !important;
    min-width: 100px;
  }
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table th:nth-child(3)),
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table td:nth-child(3)) {
    width: 15% !important;
    min-width: 100px;
  }
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table th:nth-child(4)),
  .lexical-editor-root.layout-Layout1.resizing-disabled
    :global(.lexical-content table td:nth-child(4)) {
    width: 65% !important;
  }

  /* STYLES FOR LAYOUT 2 (Segment Block) */
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table) {
    table-layout: auto;
    border: none;
  }
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table tr) {
    display: flex;
    flex-wrap: wrap;
    border: none;
  }
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table th),
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table td) {
    box-sizing: border-box;
    padding: 8px;
    border: 1px solid #ccc;
  }
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table th:nth-child(odd)),
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table td:nth-child(odd)) {
    flex: 1 0 25%;
  }
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table th:nth-child(even)),
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table td:nth-child(even)) {
    flex: 1 0 75%;
    margin-left: -1px;
  }
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table th:nth-child(n + 3)),
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table td:nth-child(n + 3)) {
    margin-top: -1px;
  }

  /* STYLES FOR LAYOUT 3 (Timestamped Paragraph) */
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table) {
    table-layout: auto;
    border: none;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table tr) {
    display: flex;
    flex-wrap: wrap;
    border: none;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table th:nth-child(1)),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table td:nth-child(1)) {
    display: none;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table th:nth-child(n + 2)),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table td:nth-child(n + 2)) {
    box-sizing: border-box;
    padding: 8px;
    border: 1px solid #ccc;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table th:nth-child(2)),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table td:nth-child(2)) {
    flex: 1 0 25%;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table th:nth-child(3)),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table td:nth-child(3)) {
    flex: 1 0 75%;
    margin-left: -1px;
  }
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table th:nth-child(4)),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table td:nth-child(4)) {
    flex: 1 0 100%;
    margin-top: -1px;
  }

  /* STYLES FOR LAYOUT 4 (Speaker & Text) */
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table) {
    table-layout: auto;
    border: none;
  }
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table tr) {
    display: flex;
    flex-wrap: nowrap;
    border: none;
  }
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table th:nth-child(-n + 2)),
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table td:nth-child(-n + 2)) {
    display: none;
  }
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table th:nth-child(n + 3)),
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table td:nth-child(n + 3)) {
    box-sizing: border-box;
    padding: 8px;
    border: 1px solid #ccc;
  }
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table th:nth-child(3)),
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table td:nth-child(3)) {
    flex: 1 0 25%;
  }
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table th:nth-child(4)),
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table td:nth-child(4)) {
    flex: 1 0 75%;
    margin-left: -1px;
  }

  /* STYLES FOR LAYOUT 5 (Plain Text) */
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table) {
    table-layout: auto;
    border: none;
  }
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table tr) {
    display: flex;
    flex-wrap: nowrap;
    border: none;
  }
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table th:nth-child(-n + 3)),
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table td:nth-child(-n + 3)) {
    display: none;
  }
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table th:nth-child(4)),
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table td:nth-child(4)) {
    flex: 1 0 100%;
    box-sizing: border-box;
    padding: 8px;
    border: 1px solid #ccc;
  }

  /* COMMON RULE TO COLLAPSE ROWS VERTICALLY */
  .lexical-editor-root.layout-Layout2 :global(.lexical-content table tr + tr),
  .lexical-editor-root.layout-Layout3 :global(.lexical-content table tr + tr),
  .lexical-editor-root.layout-Layout4 :global(.lexical-content table tr + tr),
  .lexical-editor-root.layout-Layout5 :global(.lexical-content table tr + tr) {
    margin-top: -1px;
  }
</style>
