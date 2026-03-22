<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { FileText, MessageSquareText, Image as ImageIcon, CalendarDays } from '@lucide/svelte';

    export let file;
    export let isTranscript = false;

    let blocks = [];
    let isLoading = true;
    let error = false;

    // Lexical text formats bitmask
    const IS_BOLD = 1;
    const IS_ITALIC = 2;
    const IS_STRIKETHROUGH = 4;
    const IS_UNDERLINE = 8;

    async function loadPreview() {
        if (!file || !file.full_path) {
            isLoading = false;
            error = true;
            return;
        }

        try {
            const content = await invoke('read_file_content', { path: file.full_path });
            if (content) {
                const data = JSON.parse(content);
                
                if (Array.isArray(data)) {
                    // Transcript format: array of { speaker, text, start_time, end_time }
                    blocks = data.slice(0, 40).map(seg => ({
                        type: 'paragraph',
                        spans: [
                            { text: seg.speaker ? `${seg.speaker}: ` : '', format: IS_BOLD },
                            { text: seg.text || seg.content || seg.body || '', format: 0 }
                        ]
                    })).filter(b => b.spans[1].text);
                } else {
                    // Document format: Lexical JSON structure
                    const root = data.root || (data.editorState && data.editorState.root) || data;
                    blocks = extractBlocks(root);
                    
                    // If structured parsing failed, try hyper-resilient flat extraction
                    if (blocks.length === 0) {
                        const flatText = hyperResilientExtract(data);
                        if (flatText) {
                            blocks = [{ type: 'paragraph', spans: [{ text: flatText, format: 0 }] }];
                        }
                    }
                }
            }
        } catch (e) {
            console.error('[DocumentThumbnail] Failed to load preview:', e);
            error = true;
        } finally {
            isLoading = false;
        }
    }

    function hyperResilientExtract(obj) {
        let chunks = [];
        let count = 0;
        function recurse(o) {
            if (!o || count > 30) return;
            if (typeof o === 'string' && o.length > 3 && !o.includes('{')) {
                chunks.push(o);
                count++;
            } else if (Array.isArray(o)) {
                o.forEach(recurse);
            } else if (typeof o === 'object') {
                Object.values(o).forEach(recurse);
            }
        }
        recurse(obj);
        return chunks.join(' ').substring(0, 800);
    }

    function extractBlocks(node, depth = 0) {
        if (!node || depth > 30) return [];
        let result = [];

        if (node.type === 'table') {
            let rows = [];
            const rowNodes = node.children || [];
            for (const rowNode of rowNodes) {
                if (rowNode.type === 'tablerow' && rowNode.children) {
                    let cells = [];
                    for (const cellNode of rowNode.children.slice(0, 10)) { // Up to 10 cols
                        let cellSpans = [];
                        collectSpans(cellNode, cellSpans);
                        cells.push({ spans: cellSpans.filter(s => s.text || s.type === 'date') });
                    }
                    rows.push({ cells });
                }
                if (rows.length >= 20) break; // Up to 20 rows
            }
            if (rows.length > 0) {
                result.push({ type: 'table', rows });
            }
        } else if (node.type === 'horizontalrule') {
            result.push({ type: 'hr' });
        } else if (node.type === 'image') {
            const filename = node.filename || (node.src ? node.src.split('/').pop() : null);
            if (filename) {
                result.push({
                    type: 'image',
                    filename: filename,
                    alt: node.altText || node.alt || ''
                });
            }
        } else {
            const isBlock = ['paragraph', 'heading', 'listitem', 'quote', 'code', 'image'].includes(node.type);
            if (isBlock) {
                let spans = [];
                collectSpans(node, spans);
                if (spans.length > 0 || node.type === 'image') { // Image blocks might not have spans
                    result.push({
                        type: node.type,
                        tag: node.tag || 'p',
                        spans: spans.filter(s => s.text || s.type === 'date')
                    });
                }
            }
        }

        // Search for more blocks up to a reasonable "page" limit
        if (result.length < 35 && node.children && Array.isArray(node.children)) {
            for (const child of node.children) {
                // For structure-level containers, recurse. For blocks, only if we explicitly want nested blocks.
                const handled = ['paragraph', 'heading', 'table', 'listitem', 'quote', 'code', 'horizontalrule', 'image'].includes(node.type);
                if (!handled || node.type === 'list') {
                    const childBlocks = extractBlocks(child, depth + 1);
                    result = [...result, ...childBlocks];
                }
                if (result.length >= 35) break;
            }
        } else if (node.root) {
            return extractBlocks(node.root, depth + 1);
        }

        return result;
    }

    function collectSpans(node, spans) {
        if (node.type === 'text' && node.text) {
            spans.push({ type: 'text', text: node.text, format: node.format || 0 });
        } else if (node.type === 'linebreak') {
            spans.push({ type: 'text', text: '\n', format: 0 });
        } else if (node.type === 'tab') {
            spans.push({ type: 'text', text: '    ', format: 0 });
        } else if (node.type === 'date') {
            spans.push({
                type: 'date',
                text: node.displayValue || node.date || 'Invalid Date',
                format: 0
            });
        } else if (node.children) {
            for (const child of node.children) {
                collectSpans(child, spans);
            }
        } else if (typeof node.text === 'string' && node.text) {
             spans.push({ type: 'text', text: node.text, format: node.format || 0 });
        }
    }

    onMount(loadPreview);
</script>

<div class="w-full h-full p-3 bg-white dark:bg-gray-950 overflow-hidden relative group/thumb border border-transparent transition-colors duration-200">
    {#if isLoading}
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="w-5 h-5 border-2 border-blue-500/10 border-t-blue-500/50 rounded-full animate-spin"></div>
        </div>
    {:else if error || blocks.length === 0}
        <div class="absolute inset-0 flex items-center justify-center text-gray-300 dark:text-gray-700">
            <svelte:component this={isTranscript ? MessageSquareText : FileText} size={40} strokeWidth={1} />
        </div>
    {:else}
        <div class="flex flex-col gap-1 select-none pointer-events-none">
            {#each blocks as block}
                {#if block.type === 'table'}
                    <div class="flex flex-col border-[0.5px] border-gray-200 dark:border-gray-800 rounded-sm mb-1 overflow-hidden">
                        {#each block.rows as row}
                            <div class="flex border-b-[0.5px] border-gray-100 dark:border-gray-900 last:border-0">
                                {#each row.cells as cell}
                                    <div class="flex-1 p-0.5 border-r-[0.5px] border-gray-100 dark:border-gray-900 last:border-0 min-w-0 overflow-hidden">
                                        <div class="text-[6px] text-gray-400 dark:text-gray-500 truncate leading-tight">
                                            {#each cell.spans as span}
                                                {#if span.type === 'date'}
                                                    <span class="inline-flex items-center gap-0.5 bg-blue-100 dark:bg-blue-900/50 text-blue-800 dark:text-blue-200 px-1 py-0.5 rounded-full text-[6px] font-medium leading-none">
                                                        <CalendarDays size={7} strokeWidth={1.5} />
                                                        {span.text}
                                                    </span>
                                                {:else}
                                                    <span class="{(span.format & IS_BOLD) ? 'font-bold text-gray-600 dark:text-gray-400' : ''}">{span.text}</span>
                                                {/if}
                                            {/each}
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        {/each}
                    </div>
                {:else if block.type === 'hr'}
                    <div class="h-px w-full bg-gray-100 dark:bg-gray-800 my-0.5"></div>
                {:else if block.type === 'image'}
                    <div class="flex items-center gap-1 text-[7px] text-gray-400 dark:text-gray-600 bg-gray-50 dark:bg-gray-900 px-1 py-0.5 rounded-sm w-fit">
                        <ImageIcon size={8} strokeWidth={1.5} />
                        <span>{block.filename}</span>
                    </div>
                {:else}
                    <div class="break-words leading-tight {block.type === 'heading' ? 'font-bold text-gray-700 dark:text-gray-300 text-[9px]' : 'text-gray-500 dark:text-gray-400 text-[8px]'} {block.type === 'code' ? 'font-mono bg-gray-50 dark:bg-gray-900 px-1 py-0.5 rounded-sm opacity-80' : ''}"
                         style={block.type === 'quote' ? 'border-left: 2px solid rgba(156, 163, 175, 0.4); padding-left: 5px; font-style: italic;' : ''}>
                        {#if block.type === 'listitem'}
                            <span class="mr-1 opacity-40 inline-block">•</span>
                        {/if}
                        {#each block.spans as span}
                            {#if span.type === 'date'}
                                <span class="inline-flex items-center gap-0.5 bg-blue-100 dark:bg-blue-900/50 text-blue-800 dark:text-blue-200 px-1 py-0.5 rounded-full text-[6px] font-medium leading-none">
                                    <CalendarDays size={7} strokeWidth={1.5} />
                                    {span.text}
                                </span>
                            {:else}
                                <span class="
                                    {(span.format & IS_BOLD) ? 'font-bold text-gray-700 dark:text-gray-300' : ''} 
                                    {(span.format & IS_ITALIC) ? 'italic' : ''} 
                                    {(span.format & IS_UNDERLINE) ? 'underline decoration-1' : ''} 
                                    {(span.format & IS_STRIKETHROUGH) ? 'line-through' : ''}
                                ">{span.text}</span>
                            {/if}
                        {/each}
                    </div>
                {/if}
            {/each}
        </div>
        <!-- Gradient overlay for fade effect at bottom -->
        <div class="absolute inset-x-0 bottom-0 h-12 bg-gradient-to-t from-white dark:from-gray-950 via-white/80 dark:via-gray-950/80 to-transparent pointer-events-none"></div>
    {/if}
</div>

<style>
    div {
        word-wrap: break-word;
        overflow-wrap: break-word;
    }
</style>
