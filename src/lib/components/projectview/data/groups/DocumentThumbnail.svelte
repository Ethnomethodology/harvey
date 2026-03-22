<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { FileText, MessageSquareText } from '@lucide/svelte';

    export let file;
    export let isTranscript = false;

    let textPreview = '';
    let isLoading = true;
    let error = false;

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
                textPreview = extractTextFromLexical(data);
            }
        } catch (e) {
            console.error('[DocumentThumbnail] Failed to load preview:', e);
            error = true;
        } finally {
            isLoading = false;
        }
    }

    function extractTextFromLexical(node) {
        if (!node) return '';
        let text = '';
        
        // Handle root wrapper
        if (node.root) {
            return extractTextFromLexical(node.root);
        }

        // Extract text from current node
        if (node.type === 'text' && node.text) {
            text += node.text;
        } else if (node.type === 'linebreak') {
            text += '\n';
        }

        // Recurse into children
        if (node.children && Array.isArray(node.children)) {
            for (const child of node.children) {
                const childText = extractTextFromLexical(child);
                if (childText) {
                    text += childText + (child.type === 'paragraph' ? '\n' : ' ');
                }
                if (text.length > 800) break;
            }
        }

        return text.trim();
    }

    onMount(loadPreview);
</script>

<div class="w-full h-full p-4 bg-white dark:bg-gray-950 overflow-hidden relative group/thumb border border-transparent transition-colors duration-200">
    {#if isLoading}
        <div class="absolute inset-0 flex items-center justify-center bg-gray-50/50 dark:bg-gray-950/50 backdrop-blur-[1px]">
            <div class="w-5 h-5 border-2 border-blue-500/30 border-t-blue-500 rounded-full animate-spin"></div>
        </div>
    {:else if error || !textPreview}
        <div class="absolute inset-0 flex items-center justify-center text-gray-300 dark:text-gray-700">
            <svelte:component this={isTranscript ? MessageSquareText : FileText} size={40} strokeWidth={1} />
        </div>
    {:else}
        <div class="text-[9px] leading-relaxed text-gray-500 dark:text-gray-400 font-sans break-words select-none pointer-events-none whitespace-pre-wrap">
            {textPreview}
        </div>
        <!-- Gradient overlay for fade effect at bottom -->
        <div class="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-white dark:from-gray-950 via-white/80 dark:via-gray-950/80 to-transparent pointer-events-none"></div>
    {/if}
</div>

<style>
    /* Ensure nice line breaking for text snippets */
    div {
        word-wrap: break-word;
        overflow-wrap: break-word;
    }
</style>
