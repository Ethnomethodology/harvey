<!-- src/lib/components/projectview/data/DataMiddlePanel.svelte -->
<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import {
        createEditor, $getRoot as _getRoot, $createParagraphNode as _createParagraphNode,
        $getSelection as _getSelection, $isRangeSelection as _isRangeSelection,
        $isElementNode as _isElementNode, $isTextNode as _isTextNode,
        FORMAT_TEXT_COMMAND, CLICK_COMMAND,
        // REMOVED CLEAR_FORMATTING_COMMAND - It doesn't exist for import
        COMMAND_PRIORITY_NORMAL, COMMAND_PRIORITY_LOW, COMMAND_PRIORITY_CRITICAL,
        TextNode, ParagraphNode, ElementNode, SELECTION_CHANGE_COMMAND,
        FORMAT_ELEMENT_COMMAND, INDENT_CONTENT_COMMAND, OUTDENT_CONTENT_COMMAND
    } from 'lexical';
    import { mergeRegister } from '@lexical/utils';
    import { $findMatchingParent as _findMatchingParent } from '@lexical/utils';

    import {
        HeadingNode, QuoteNode, $isHeadingNode as _isHeadingNode, $createHeadingNode as _createHeadingNode
    } from '@lexical/rich-text';
    import {
        ListNode, ListItemNode, $isListNode as _isListNode, $isListItemNode as _isListItemNode,
        INSERT_ORDERED_LIST_COMMAND, INSERT_UNORDERED_LIST_COMMAND, REMOVE_LIST_COMMAND
    } from '@lexical/list';
    // Import necessary selection utils - $clearFormatting is NOT available here
    import { $setBlocksType as _setBlocksType, $patchStyleText as _patchStyleText, $getSelectionStyleValueForProperty as _getSelectionStyleValueForProperty } from '@lexical/selection';

    import { registerRichText } from '@lexical/rich-text';
    import { registerHistory, createEmptyHistoryState } from '@lexical/history';
    import { registerList } from '@lexical/list';

    let editorWrapper;
    let editorContainer;
    let editor = null;
    let isReady = false;
    let unregisterMinimalListeners = () => {};
    let historyState = createEmptyHistoryState();

    // --- Toolbar State ---
    let isBold = false; let isItalic = false; let isUnderline = false; let isStrikethrough = false;
    let blockType = 'paragraph';
    let selectedAlignment = 'left';
    let selectedTextColor = '#000000';
    let selectedHighlightColor = 'transparent';

    // --- Dropdown Options (Unchanged) ---
    const blockTypeOptions = [ { value: 'paragraph', label: 'Normal' }, { value: 'h1', label: 'Heading 1' }, { value: 'h2', label: 'Heading 2' }, { value: 'h3', label: 'Heading 3' }, { value: 'ul', label: 'Bullet List' }, { value: 'ol', label: 'Numbered List' } ];
    const alignmentOptions = [ { value: 'left', label: 'Left' }, { value: 'center', label: 'Center' }, { value: 'right', label: 'Right' }, { value: 'justify', label: 'Justify' } ];
    const colorOptions = [ { value: '#000000', label: 'Black' }, { value: '#FF0000', label: 'Red' }, { value: '#0000FF', label: 'Blue' }, { value: '#008000', label: 'Green'}, { value: '#FFA500', label: 'Orange'} , { value: null, label: 'Default'} ];
    const highlightOptions = [ { value: null, label: 'None' }, { value: '#FFFF00', label: 'Yellow' }, { value: '#AEEFFA', label: 'Light Blue' }, { value: '#FFC0CB', label: 'Pink' }, { value: '#D3D3D3', label: 'Gray'}, { value: '#90EE90', label: 'Light Green'} ];


    onMount(() => { // (onMount logic unchanged)
        console.log('[DataMiddlePanel] Mounting Minimal Editor.');
        editor = createEditor({
            namespace: 'MinimalEditor',
            nodes: [ParagraphNode, TextNode, HeadingNode, QuoteNode, ListNode, ListItemNode],
            theme: {
                 paragraph: 'mb-1', text: { bold: 'font-bold', italic: 'italic', underline: 'underline', strikethrough: 'line-through' },
                 heading: { h1: 'text-2xl font-bold mb-1 mt-2', h2: 'text-xl font-semibold mb-1 mt-1', h3: 'text-lg font-semibold mb-1' },
                 list: { ul: 'list-disc list-inside mb-1 pl-4', ol: 'list-decimal list-inside mb-1 pl-4', listitem: 'mb-0.5 pl-1' },
                 quote: 'border-l-4 border-gray-300 pl-2 italic my-1',
                 align_left: 'text-left', align_center: 'text-center', align_right: 'text-right', align_justify: 'text-justify'
            },
            onError: (error) => { console.error('[DataMiddlePanel Minimal Editor] Error:', error); },
            historyState: historyState,
        });
        if (editorContainer) {
            editor.setRootElement(editorContainer);
            unregisterMinimalListeners = mergeRegister(
                editor.registerUpdateListener(({editorState}) => { editorState.read(() => { updateMinimalToolbarState(); }); }),
                editor.registerCommand(SELECTION_CHANGE_COMMAND, () => { if(isReady) updateMinimalToolbarState(); return false; }, COMMAND_PRIORITY_LOW),
                registerRichText(editor), registerHistory(editor, historyState, 200), registerList(editor),
                editor.registerCommand(CLICK_COMMAND, () => false, COMMAND_PRIORITY_LOW),
            );
            console.log('[DataMiddlePanel Minimal Editor] Registered Lexical listeners.');
            editor.update(() => { const root = _getRoot(); if (root.isEmpty()) { root.append(_createParagraphNode()); } });
            tick().then(() => {
                if (editor) {
                    editor.setEditable(true); const isNowEditable = editor.isEditable();
                    console.log(`[DataMiddlePanel Minimal Editor] editor.isEditable() returned: ${isNowEditable}`);
                    if (isNowEditable) { editor.focus(() => { console.log('[DataMiddlePanel Minimal Editor] Focus callback.'); }); isReady = true; editor.getEditorState().read(updateMinimalToolbarState); }
                    else { console.error('[DataMiddlePanel Minimal Editor] FAILED TO SET EDITABLE.'); }
                }
            });
        } else { console.error('[DataMiddlePanel Minimal Editor] editorContainer not found!'); }
        return () => { if (editor) { console.log('[DataMiddlePanel Minimal Editor] Destroying editor.'); unregisterMinimalListeners(); editor = null; } isReady = false; };
    }); // End onMount

    // --- Update Toolbar State (Unchanged) ---
    function updateMinimalToolbarState() {
         if (!editor || !isReady) return; const selection = _getSelection();
         isBold = false; isItalic = false; isUnderline = false; isStrikethrough = false;
         blockType = 'paragraph'; selectedAlignment = 'left';
         if (_isRangeSelection(selection)) {
             isBold = selection.hasFormat('bold'); isItalic = selection.hasFormat('italic'); isUnderline = selection.hasFormat('underline'); isStrikethrough = selection.hasFormat('strikethrough');
             selectedTextColor = _getSelectionStyleValueForProperty(selection, 'color', '#000000'); selectedHighlightColor = _getSelectionStyleValueForProperty(selection, 'background-color', 'transparent');
             const anchorNode = selection.anchor.getNode(); let element = _findMatchingParent(anchorNode, (node) => _isElementNode(node) && node.getType() !== 'root') || anchorNode;
             if (element) {
                 const type = element.getType();
                 if (_isHeadingNode(element)) { blockType = element.getTag(); }
                 else if (_isListItemNode(element)) { const parentList = _findMatchingParent(element, _isListNode); blockType = parentList ? parentList.getListType() : 'paragraph'; }
                 else { blockType = 'paragraph'; }
                 if (_isElementNode(element)) { selectedAlignment = element.getFormatType() || 'left'; }
             }
         }
         isBold=isBold; isItalic=isItalic; isUnderline=isUnderline; isStrikethrough=isStrikethrough; blockType=blockType; selectedAlignment=selectedAlignment; selectedTextColor=selectedTextColor; selectedHighlightColor=selectedHighlightColor;
    }

    // --- Toolbar Actions (Unchanged except clearFormatting) ---
    function formatText(formatType) { if (!editor || !isReady || !editor.isEditable()) { return; }; editor.focus(() => { editor.dispatchCommand(FORMAT_TEXT_COMMAND, formatType); }); }
    function alignElement(alignType) { if (!editor || !isReady || !editor.isEditable()) { return; } editor.focus(() => { editor.dispatchCommand(FORMAT_ELEMENT_COMMAND, alignType); }); }
    function indentContent() { if (!editor || !isReady || !editor.isEditable()) { return; } editor.focus(() => { editor.dispatchCommand(INDENT_CONTENT_COMMAND, undefined); }); }
    function outdentContent() { if (!editor || !isReady || !editor.isEditable()) { return; } editor.focus(() => { editor.dispatchCommand(OUTDENT_CONTENT_COMMAND, undefined); }); }
    function applyStyle(styleName, value) { if (!editor || !isReady || !editor.isEditable()) { return; } editor.focus(() => { editor.update(() => { const selection = _getSelection(); if (_isRangeSelection(selection)) { _patchStyleText(selection, { [styleName]: value }); } }); }); }
    function applyTextColor(event) { const color = event.target.value === 'null' ? null : event.target.value; applyStyle('color', color); }
    function applyHighlightColor(event) { const color = event.target.value === 'null' ? null : event.target.value; applyStyle('background-color', color); }
    function handleBlockTypeChange(event) {
         const type = event.target.value; if (!editor || !isReady || !editor.isEditable()) { return; }
         editor.focus(() => {
            if (type === 'paragraph' || type === 'h1' || type === 'h2' || type === 'h3') { const formatFunction = type === 'paragraph' ? () => _createParagraphNode() : type === 'h1' ? () => _createHeadingNode('h1') : type === 'h2' ? () => _createHeadingNode('h2') : () => _createHeadingNode('h3'); editor.update(() => { const selection = _getSelection(); if (_isRangeSelection(selection)) { _setBlocksType(selection, formatFunction); } }); }
            else if (type === 'ul') { editor.dispatchCommand(INSERT_UNORDERED_LIST_COMMAND, undefined); }
            else if (type === 'ol') { editor.dispatchCommand(INSERT_ORDERED_LIST_COMMAND, undefined); }
         });
    }

    // *** REVISED clearFormatting Action (Manual Implementation) ***
    function clearFormatting() {
        if (!editor || !isReady || !editor.isEditable()) { return; }
         console.log(`[DataMiddlePanel Minimal Editor] Clearing formatting MANUALLY`);
         editor.focus(() => {
             editor.update(() => {
                 const selection = _getSelection();
                 if (_isRangeSelection(selection)) {
                    try {
                         // 1. Clear inline formats/styles on the text nodes first
                         const selectedNodes = selection.getNodes();
                         selectedNodes.forEach(node => {
                             if (_isTextNode(node)) {
                                 // Check if node is still attached before modifying
                                 if(node.isAttached()){
                                     node.setFormat(0); // Clear bold, italic, underline, strikethrough etc.
                                     node.setStyle(''); // Clear inline styles (color, background-color)
                                 }
                             }
                             // Note: Link removal would need extra logic here ($isLinkNode, node.replace())
                         });

                         // 2. Convert selected block elements to paragraphs *after* clearing inline styles
                         // This might be less prone to the common ancestor error than doing it before getNodes()
                         _setBlocksType(selection, () => _createParagraphNode());

                         console.log(" -> Cleared inline formats/styles and attempted to set block to paragraph.");

                    } catch (error) {
                         // Log error if getNodes or subsequent operations fail
                         console.error("[DataMiddlePanel Minimal Editor] Error during manual clearFormatting:", error);
                    }
                 }
             });
         });
    }

</script>

<!-- Main container -->
<div class="h-full flex flex-col bg-white dark:bg-gray-800 rounded-md shadow p-2 space-y-2">
    <div class="text-xs text-gray-500 dark:text-gray-400">Minimal Test Editor (Dropdowns):</div>
    <!-- Minimal Toolbar (Unchanged) -->
    <div class="toolbar flex items-center flex-wrap gap-x-1 border-b border-gray-300 dark:border-border pb-1 mb-1 flex-shrink-0">
         <select class="mini-toolbar-select" value={blockType} on:change={handleBlockTypeChange} title="Block Type"> {#each blockTypeOptions as option} <option value={option.value}>{option.label}</option> {/each} </select> <div class="separator"></div>
         <button class="mini-toolbar-button font-bold" on:click={() => formatText('bold')} class:active={isBold} title="Bold">B</button> <button class="mini-toolbar-button italic" on:click={() => formatText('italic')} class:active={isItalic} title="Italic">I</button> <button class="mini-toolbar-button underline" on:click={() => formatText('underline')} class:active={isUnderline} title="Underline">U</button> <button class="mini-toolbar-button line-through" on:click={() => formatText('strikethrough')} class:active={isStrikethrough} title="Strikethrough">S</button> <div class="separator"></div>
         <select class="mini-toolbar-select" value={selectedAlignment} on:change={(e) => alignElement(e.target.value)} title="Alignment"> {#each alignmentOptions as option} <option value={option.value}>{option.label}</option> {/each} </select> <div class="separator"></div>
          <button class="mini-toolbar-button" on:click={outdentContent} title="Outdent">OD</button> <button class="mini-toolbar-button" on:click={indentContent} title="Indent">ID</button> <div class="separator"></div>
         <select class="mini-toolbar-select" value={selectedTextColor} on:change={applyTextColor} title="Text Color"> {#each colorOptions as option} <option value={option.value ?? 'null'} style="color: {option.value ?? 'inherit'};">{option.label}</option> {/each} </select> <div class="separator"></div>
         <select class="mini-toolbar-select" value={selectedHighlightColor} on:change={applyHighlightColor} title="Highlight Color"> {#each highlightOptions as option} <option value={option.value ?? 'null'} style="background-color: {option.value ?? 'inherit'}; color: {option.value ? 'black' : 'inherit'};">{option.label}</option> {/each} </select> <div class="separator"></div>
         <button class="mini-toolbar-button" on:click={clearFormatting} title="Clear Formatting">Clear</button>
    </div>
    <!-- Wrapper Div (Unchanged) -->
    <div class="lexical-wrapper flex-grow min-h-[100px] overflow-y-auto border border-dashed border-green-400 p-1" bind:this={editorWrapper}>
        <div bind:this={editorContainer} class="lexical-minimal-content focus:outline-none min-h-full" contenteditable="true" role="textbox" aria-multiline="true" spellcheck="true" ></div>
    </div>
</div>

<style lang="postcss">
    /* Styling (Unchanged) */
    .toolbar button.mini-toolbar-button, .toolbar select.mini-toolbar-select { @apply px-2 py-0.5 border border-gray-300 dark:border-border rounded bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-xs disabled:opacity-50 focus:outline-none focus:ring-1 focus:ring-blue-500; margin-right: 2px; } .toolbar select.mini-toolbar-select { height: 26px; } .toolbar button.mini-toolbar-button.active { @apply bg-blue-100 dark:bg-blue-800 border-blue-300 dark:border-blue-600 text-blue-800 dark:text-blue-200; } .separator { @apply w-px h-4 bg-gray-300 dark:bg-border mx-1 inline-block align-middle; } .lexical-wrapper { position: relative; } .lexical-minimal-content { outline: none; caret-color: currentcolor; height: 100%; }
    .lexical-minimal-content p { @apply mb-1; } .lexical-minimal-content h1 { @apply text-2xl font-bold mb-1 mt-2; } .lexical-minimal-content h2 { @apply text-xl font-semibold mb-1 mt-1; } .lexical-minimal-content h3 { @apply text-lg font-semibold mb-1; } .lexical-minimal-content ul { @apply list-disc list-inside mb-1 pl-4; } .lexical-minimal-content ol { @apply list-decimal list-inside mb-1 pl-4; } .lexical-minimal-content li { @apply mb-0.5 pl-1; }
    .lexical-minimal-content [style*="text-decoration-line: underline line-through"], .lexical-minimal-content [style*="text-decoration: underline line-through"] { text-decoration: underline line-through; }
    .lexical-minimal-content .text-left { text-align: left; } .lexical-minimal-content .text-center { text-align: center; } .lexical-minimal-content .text-right { text-align: right; } .lexical-minimal-content .text-justify { text-align: justify; }
	.lexical-wrapper::-webkit-scrollbar { @apply w-[8px] h-[8px]; } .lexical-wrapper::-webkit-scrollbar-track { @apply bg-gray-100 dark:bg-gray-800 rounded-lg; } .lexical-wrapper::-webkit-scrollbar-thumb { @apply bg-gray-400 dark:bg-gray-500 rounded-lg border-2 border-solid border-gray-100 dark:border-gray-800; } .lexical-wrapper::-webkit-scrollbar-thumb:hover { @apply bg-gray-500 dark:bg-gray-400; } .lexical-wrapper { scrollbar-width: thin; scrollbar-color: theme('colors.gray.400') theme('colors.gray.100'); } html.dark .lexical-wrapper { scrollbar-color: theme('colors.gray.500') theme('colors.gray.800'); }
</style>