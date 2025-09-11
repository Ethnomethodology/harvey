<!-- src/lib/components/projectview/transcriptions/TreeNode.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
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

    /**
     * Determines the correct SVG icon HTML string based on the node type.
     */
	function renderFileIcon() {
		const ext = node.name.split('.').pop()?.toLowerCase() ?? '';

        // Use file_type first for more specific identification
        if (node.file_type === 'media') {
            if (AUDIO_EXTENSIONS.has(ext)) return AUDIO_ICON;
			if (VIDEO_EXTENSIONS.has(ext)) return VIDEO_ICON;
        } else if (node.file_type === 'transcript' || node.file_type === 'imported_transcript') {
             return TRANSCRIPT_ICON;
        } else if (node.file_type === 'note') {
             return NOTE_ICON;
        }

        // Fallback for 'other' or if file_type is somehow missing
		return QUESTION_ICON; // Fallback for unknown file types
	}

    /* ---------- Highlighting Logic ---------- */
    $: shouldHighlight = !node.is_directory && (
        (node.file_type === 'media' && node.path === selectedMediaPath) ||
        (node.file_type === 'transcript' && node.path === currentTranscriptPath)
    );

/* ---------- SVG constants ---------- */
const FOLDER_CLOSE_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
<path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.958 0 1.76.56 2.311 1.184C7.985 3.648 8.48 4 9 4h4.5A1.5 1.5 0 0 1 15 5.5v7a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 1 12.5zM2.5 3a.5.5 0 0 0-.5.5V6h12v-.5a.5.5 0 0 0-.5-.5H9c-.964 0-1.71-.629-2.174-1.154C6.374 3.334 5.82 3 5.264 3zM14 7H2v5.5a.5.5 0 0 0 .5.5h11a.5.5 0 0 0 .5-.5z"/>
</svg>`;
const FOLDER_OPEN_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
<path d="M1 3.5A1.5 1.5 0 0 1 2.5 2h2.764c.958 0 1.76.56 2.311 1.184C7.985 3.648 8.48 4 9 4h4.5A1.5 1.5 0 0 1 15 5.5v.64c.57.265.94.876.856 1.546l-.64 5.124A2.5 2.5 0 0 1 12.733 15H3.266a2.5 2.5 0 0 1-2.481-2.19l-.64-5.124A1.5 1.5 0 0 1 1 6.14zM2 6h12v-.5a.5.5 0 0 0-.5-.5H9c-.964 0-1.71-.629-2.174-1.154C6.374 3.334 5.82 3 5.264 3H2.5a.5.5 0 0 0-.5.5zm-.367 1a.5.5 0 0 0-.496.562l.64 5.124A1.5 1.5 0 0 0 3.266 14h9.468a1.5 1.5 0 0 0 1.489-1.314l.64-5.124A.5.5 0 0 0 14.367 7z"/>
</svg>`;
const AUDIO_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
  <path d="M6 13c0 1.105-1.12 2-2.5 2S1 14.105 1 13s1.12-2 2.5-2 2.5.896 2.5 2m9-2c0 1.105-1.12 2-2.5 2s-2.5-.895-2.5-2 1.12-2 2.5-2 2.5.895 2.5 2"/>
  <path fill-rule="evenodd" d="M14 11V2h1v9zM6 3v10H5V3z"/>
  <path d="M5 2.905a1 1 0 0 1 .9-.995l8-.8a1 1 0 0 1 1.1.995V3L5 4z"/>
</svg>`;
const VIDEO_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
  <path d="M0 1a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H1a1 1 0 0 1-1-1zm4 0v6h8V1zm8 8H4v6h8zM1 1v2h2V1zm2 3H1v2h2zM1 7v2h2V7zm2 3H1v2h2zm-2 3v2h2v-2zM15 1h-2v2h2zm-2 3v2h2V4zm2 3h-2v2h2zm-2 3v2h2v-2zm2 3h-2v2h2z"/>
</svg>`;
const TRANSCRIPT_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
  <path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/>
  <path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/>
</svg>`;
const NOTE_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="w-4 h-4">
  <path d="M5 10.5a.5.5 0 0 1 .5-.5h2a.5.5 0 0 1 0 1h-2a.5.5 0 0 1-.5-.5m0-2a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5m0-2a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5m0-2a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/>
  <path d="M3 0h10a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2v-1h1v1a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1v1H1V2a2 2 0 0 1 2-2"/>
  <path d="M1 5v-.5a.5.5 0 0 1 1 0V5h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0V8h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0v.5h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1"/>
</svg>`;
const QUESTION_ICON = `
<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
<path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z" />
</svg>`;
</script>

<!-- List Item Structure -->
<li class="text-xs select-none">
    <!-- Clickable/Hoverable Row -->
    <div
        class="flex items-center group rounded hover:bg-gray-100 dark:hover:bg-gray-700"
        class:cursor-pointer="{!node.is_directory}"
        class:cursor-default="{node.is_directory}"
        class:bg-blue-100="{shouldHighlight}"
        class:dark:bg-blue-900="{shouldHighlight}"
        on:click="{handleRowClick}"
        on:dblclick="{handleRowDoubleClick}"
        on:contextmenu="{handleRowContextMenu}"
        title="{node.path}"
	>
		<!-- Indentation based on depth -->
        <!-- Base depth is 3 (stem folder). Subtract 3 to get nesting level (0 for stem, 1 for subdirs, 2 for files) -->
		<span class="flex-shrink-0" style:width={(node.depth > 3 ? node.depth - 3 : 0) * 1.25}rem;></span>

		<!-- Folder Toggle Icon OR File Icon -->
		{#if node.is_directory}
            <span on:click="{toggleExpand}" class="px-1 cursor-pointer flex-shrink-0 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
				{@html expanded ? FOLDER_OPEN_ICON : FOLDER_CLOSE_ICON}
			</span>
		{:else}
			<!-- File Icon -->
			<span class="px-1 flex-shrink-0 flex items-center justify-center text-gray-600 dark:text-gray-400">
				{@html renderFileIcon()}
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