<!-- src/lib/components/projectview/transcriptions/TreeNode.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
    import { FolderOpen, FolderClosed, MessageSquareText, Music, Film, CircleHelp, FileText } from '@lucide/svelte';
	// Use relative path for recursive import within the same directory (if this file is TreeNode.svelte)
	// If TreeNode is imported elsewhere, this might need adjustment, but assuming it's self-recursive
	import TreeNode from './TreeNode.svelte';

	/* ---------- props ---------- */
	export let node; // The FileEntry object for this node
	export let selectedMediaPath; // Full path of the selected media file for highlighting
    export let currentTranscriptPath; // Full path of the currently loaded transcript file for highlighting
    // REMOVED: highlightTranscript prop

	const dispatch = createEventDispatcher();

	/* ---------- event handling ---------- */
    // Combined click handler for the main div
    function handleRowClick() {
        // Only forward click if it's a file node (not a directory)
        if (!node.is_directory) {
             dispatch('itemclick', node);
        }
	}

    // Combined double-click handler for the main div
	function handleRowDoubleClick() {
        // Only forward double-click if it's a file node
        if (!node.is_directory) {
		    dispatch('itemdblclick', node);
        }
	}

    // Context menu handler for the main div
	function handleRowContextMenu(event) {
		event.preventDefault(); // Prevent default browser context menu
		// Forward the browser event and the associated item data
		dispatch('itemcontextmenu', { event, item: node });
	}

	/* ---------- expand / collapse ---------- */
	// Default expanded state: Expand media stem (depth 3) and subdirs (depth 4)
    // Assuming depths are now: harvey_files(1)/Media(2)/<stem>(3)/media|transcripts|notes(4)/file(5)
	let expanded = node.depth <= 4; // Expand depths 3 and 4 by default
	function toggleExpand(event) {
		event.stopPropagation(); // Prevent click from bubbling to handleRowClick
		if (node.is_directory) expanded = !expanded;
	}

	/* ---------- helpers ---------- */
	const AUDIO_EXTENSIONS = new Set(['mp3','wav','m4a','ogg','aac','flac']);
	const VIDEO_EXTENSIONS = new Set(['mp4','mov','avi','mkv','webm']);
    // Define extensions for note types for icon matching
    const NOTE_EXTENSIONS = new Set(['json', 'md', 'txt']); // Adjusted to include json

    /* ---------- Highlighting Logic ---------- */
    $: shouldHighlight = !node.is_directory && (
        (node.file_type === 'media' && node.path === selectedMediaPath) ||
        (node.file_type === 'transcript' && node.path === currentTranscriptPath)
    );

</script>

<!-- List Item Structure -->
<li class="text-xs select-none">
    <!-- Clickable/Hoverable Row -->
    <div
        class="flex items-center group rounded hover:bg-gray-100 dark:hover:bg-gray-800"
        class:cursor-pointer="{!node.is_directory}"
        class:cursor-default="{node.is_directory}"
        class:bg-blue-100="{shouldHighlight}"
        class:dark:bg-blue-900="{shouldHighlight}"
        on:click="{handleRowClick}"
        on:dblclick="{handleRowDoubleClick}"
        on:contextmenu="{handleRowContextMenu}"
        title="{node.name}"
	>
		<!-- Indentation based on depth -->
        <!-- Base depth is 3 (stem folder). Subtract 3 to get nesting level (0 for stem, 1 for subdirs, 2 for files) -->
		<span class="flex-shrink-0" style:width={(node.depth > 3 ? node.depth - 3 : 0) * 1.25}rem;></span>

		<!-- Folder Toggle Icon OR File Icon -->
		{#if node.is_directory}
            <span on:click="{toggleExpand}" class="px-1 cursor-pointer flex-shrink-0 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
                {#if expanded}
                    <FolderOpen class="w-4 h-4" />
                {:else}
                    <FolderClosed class="w-4 h-4" />
                {/if}
			</span>
		{:else}
			<!-- File Icon -->
			<span class="px-1 flex-shrink-0 flex items-center justify-center text-gray-600 dark:text-gray-400">
                {#if node.file_type === 'media' && AUDIO_EXTENSIONS.has(node.name.split('.').pop()?.toLowerCase() ?? '')}
                    <Music class="w-4 h-4" />
                {:else if node.file_type === 'media' && VIDEO_EXTENSIONS.has(node.name.split('.').pop()?.toLowerCase() ?? '')}
                    <Film class="w-4 h-4" />
                {:else if node.file_type === 'transcript' || node.file_type === 'imported_transcript'}
                    <MessageSquareText class="w-4 h-4" />
                {:else if node.file_type === 'note'}
                    <FileText class="w-4 h-4" />
                {:else}
                    <CircleHelp class="w-4 h-4" />
                {/if}
			</span>
		{/if}

		<!-- Filename -->
		<span
			class="text-left w-full px-1 py-0.5 truncate"
            class:text-blue-700="{shouldHighlight}"
            class:dark:text-blue-300="{shouldHighlight}"
            class:font-medium="{node.is_directory}"
            class:text-gray-800="{!shouldHighlight && !node.is_directory}"
            class:dark:text-gray-200="{!shouldHighlight && !node.is_directory}"
		>
			{node.name}
		</span>
	</div>

	<!-- Recursive Rendering for Children -->
	{#if node.is_directory && expanded && node.children && node.children.length}
		<ul class="space-y-0.5 mt-0.5">
			{#each node.children as child (child.path || child.name)}
				<svelte:self
                    node="{child}"
                    selectedMediaPath="{selectedMediaPath}"
                    currentTranscriptPath="{currentTranscriptPath}"
					on:itemclick
					on:itemcontextmenu
					on:itemdblclick
				/>
			{/each}
		</ul>
	{/if}
</li>
<style>
    /* Removed highlight-transcript class */
</style>