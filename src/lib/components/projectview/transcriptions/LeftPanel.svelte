<!-- src/lib/components/projectview/transcriptions/LeftPanel.svelte -->
<script>
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js';
	import { transcriptStore, selectMedia } from '$lib/stores/transcriptStore.js';
	import { panelStateStore } from '$lib/stores/panelStateStore.js'; // Added import
	import { loadTranscriptFile, refreshProjectFiles, renameProjectItem, deleteProjectItem } from '$lib/services/projectService.js';
	import TreeNode from './TreeNode.svelte';
	import FileRenameModal from '../modals/FileRenameModal.svelte';
	import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { createEventDispatcher } from 'svelte';
    import CategoryTooltip from '../notes/CategoryTooltip.svelte';

    const dispatch = createEventDispatcher();

	// --- Tooltip State ---
	let tooltipVisible = false;
	let tooltipCategoryName = '';
	let tooltipItems = [];
	let tooltipX = 0;
	let tooltipY = 0;

	// --- Tooltip Functions ---
	function showTooltip(event, category, items) {
		const buttonRect = event.currentTarget.getBoundingClientRect();
		tooltipCategoryName = category;
		tooltipItems = items;
		tooltipX = buttonRect.right + 8; // Position to the right of the button
		tooltipY = buttonRect.top;
		tooltipVisible = true;
	}

	function hideTooltip() {
		tooltipVisible = false;
	}

	// --- State for Accordion Sections ---
	let openSection = 'files';

	// --- State for Rename Modal ---
	let showRenameModal = false;
	let itemToRename = null;

	// --- Accordion Click Handlers ---
	function toggleSection(sectionName) {
		openSection = openSection === sectionName ? null : sectionName;
	}

	// --- File Tree Logic ---
	$: selectedMediaPath = $transcriptStore.selectedMediaFile?.path;
    // NEW: Get current transcript path for highlighting
    // currentTranscriptPath is now sourced from transcriptStore.
    $: currentTranscriptPath = $transcriptStore.currentTranscriptPath;

	// --- projectFileTree now directly uses the XML-derived tree from the store ---
	$: projectFileTree = $project.files || [];

    // --- Function to handle opening a note ---
    function handleOpenNote(item) {
        if (!item || item.file_type !== 'note') return;
        console.log(`[LeftPanel] Requesting to open note: ${item.name} (${item.path})`);
        dispatch('requestopentab', { tabName: 'notes', notePath: item.path });
    }

	// --- Item Interaction Logic ---
	// MODIFIED: handleItemClick for better media/transcript loading sequence
	async function handleItemClick(event) {
		const item = event.detail;
		console.log('[LeftPanel] handleItemClick triggered for:', item);

		if (item.is_directory) {
			console.log('[LeftPanel] Clicked item is a directory, ignoring for selection/load.');
			return; // Ignore clicks on directories
		}

		if (item.file_type === 'media') {
			console.log('[LeftPanel] Clicked item is media, calling selectMedia.');
			// selectMedia handles loading the first associated transcript automatically
			selectMedia(item); // Select the media file
		}
        else if (item.file_type === 'transcript') {
			console.log('[LeftPanel] Clicked item is transcript, attempting to load.');
			const transcriptToLoadPath = item.path;
            const mediaIdentifier = item.media_xml_identifier; // Get associated media stem

            if (!mediaIdentifier) {
                console.warn('[LeftPanel] Transcript item missing media_xml_identifier. Cannot select associated media.');
                // Proceed to load the transcript anyway, but without guarantees media is correct
                if (get(transcriptStore).currentTranscriptPath !== transcriptToLoadPath) {
                    try {
                        await loadTranscriptFile(transcriptToLoadPath);
                        console.log('[LeftPanel] Transcript loaded (without associated media selection).');
                        await message(`Warning: Could not identify the media file associated with '${item.name}'. The transcript was loaded, but the player might not be synchronized.`, { title: 'Media Not Found', type: 'warning'});
                    } catch (error) {
                        console.error(`[LeftPanel] Error loading transcript ${item.name} (no media ID):`, error);
                        await message(`Error loading transcript ${item.name}: ${error.message || error}`, { title: 'Load Error', type: 'error'});
                        project.update((p) => ({ ...p, statusMessage: `Error loading ${item.name}` }));
                    }
                } else { console.log('[LeftPanel] Clicked transcript (no media ID) is already loaded.'); }
                return; // Stop further processing for this case
            }

            // Find the corresponding media file entry in the store's raw file list
            const allFiles = get(project).files;
            let foundMediaEntry = null;
            function findMediaInChildrenRecursive(nodes, identifier) {
                 if (!Array.isArray(nodes)) return null;
                 for (const node of nodes) {
                     // Check if the node itself is the media file
                     if (node.file_type === 'media' && node.media_xml_identifier === identifier) { return node; }
                     // If it's a directory, search its children
                     if (node.children && node.children.length > 0) {
                         const found = findMediaInChildrenRecursive(node.children, identifier);
                         if (found) return found;
                     }
                 }
                 return null;
             }
            foundMediaEntry = findMediaInChildrenRecursive(allFiles, mediaIdentifier);

            if (!foundMediaEntry) {
                console.warn('[LeftPanel] Could not find corresponding media file entry for transcript identifier:', mediaIdentifier);
                // Still try loading the transcript, similar to the no-identifier case
                 if (get(transcriptStore).currentTranscriptPath !== transcriptToLoadPath) {
                    try {
                        await loadTranscriptFile(transcriptToLoadPath);
                        console.log('[LeftPanel] Transcript loaded (associated media node not found).');
                        await message(`Warning: Could not find the media file entry associated with '${item.name}' in the project structure. The transcript was loaded, but the player might not be synchronized.`, { title: 'Media Entry Missing', type: 'warning'});
                    } catch (error) { /* ... error handling ... */
                         console.error(`[LeftPanel] Error loading transcript ${item.name} (media entry missing):`, error);
                         await message(`Error loading transcript ${item.name}: ${error.message || error}`, { title: 'Load Error', type: 'error'});
                         project.update((p) => ({ ...p, statusMessage: `Error loading ${item.name}` }));
                    }
                } else { console.log('[LeftPanel] Clicked transcript (media entry missing) is already loaded.'); }
                return; // Stop further processing
            }

            // Media entry found!
            console.log(`[LeftPanel] Found associated media entry: ${foundMediaEntry.name}`);
            const currentSelectedMediaPath = get(transcriptStore).selectedMediaFile?.path;

            // --- Sequence: Select Media FIRST, then load SPECIFIC transcript ---
            // 1. Select the associated media (if not already selected)
            // NOTE: Pass `true` to selectMedia to prevent it from auto-loading the *first* transcript,
            // because we are about to load a *specific* one. We need to modify selectMedia for this.
            // For now, let's stick to the original logic and accept the brief load of the first transcript.
            if (currentSelectedMediaPath !== foundMediaEntry.path) {
                console.log('[LeftPanel] Associated media is not currently selected. Calling selectMedia...');
                selectMedia(foundMediaEntry);
                // selectMedia will attempt to load the *primary* transcript. We wait for the next step.
            } else {
                console.log('[LeftPanel] Associated media is already selected.');
            }

            // 2. Load the *clicked* transcript (even if it overwrites the primary one just loaded by selectMedia)
            if (get(transcriptStore).currentTranscriptPath !== transcriptToLoadPath) {
                console.log(`[LeftPanel] Loading the *specifically clicked* transcript file: ${transcriptToLoadPath}`);
                try {
                    await loadTranscriptFile(transcriptToLoadPath);
                    console.log('[LeftPanel] Clicked transcript loaded successfully.');
                    project.update(p => ({ ...p, statusMessage: `Transcript loaded: ${item.name}` }));
                } catch (error) {
                    console.error(`[LeftPanel] Error loading clicked transcript ${item.name}:`, error);
                    await message(`Error loading transcript ${item.name}: ${error.message || error}`, { title: 'Load Error', type: 'error'});
                    project.update((p) => ({ ...p, statusMessage: `Error loading ${item.name}` }));
                }
            } else {
                console.log('[LeftPanel] Clicked transcript is already loaded.');
            }

		} else if (item.file_type === 'note') {
            handleOpenNote(item);
		} else {
			console.log('[LeftPanel] Clicked item is of type', item.file_type, '- no primary click action defined.');
		}
	}

	function handleItemDoubleClick(event) {
		const item = event.detail;
		if (!item.is_directory && item.file_type === 'media') {
			console.log('[LeftPanel] Double-clicked media, calling selectMedia.');
			selectMedia(item);
        } else if (!item.is_directory && item.file_type === 'note') {
            console.log('[LeftPanel] Double-clicked note, calling handleOpenNote.');
            handleOpenNote(item);
        }
	}

	// --- Context Menu Logic ---
	let contextMenuVisible = false; let contextMenuX = 0; let contextMenuY = 0; let contextMenuItem = null;
    let closeContextMenuListener = null;

	function handleContextMenu(event) {
        const { event: mouseEvent, item } = event.detail;
		if (item.is_directory) return; // Only allow on files
        if (contextMenuVisible) closeContextMenu();
		mouseEvent.preventDefault(); mouseEvent.stopPropagation();
		contextMenuItem = item; contextMenuX = mouseEvent.clientX; contextMenuY = mouseEvent.clientY; contextMenuVisible = true;
        setTimeout(() => {
             if (closeContextMenuListener) document.removeEventListener('click', closeContextMenuListener, { capture: true });
             closeContextMenuListener = (e) => {
                  const menuElement = document.getElementById('context-menu-div');
                  if (menuElement && !menuElement.contains(e.target)) closeContextMenu();
             };
             document.addEventListener('click', closeContextMenuListener, { capture: true, once: true });
        }, 0);
	}

	function closeContextMenu() {
		if (contextMenuVisible) { contextMenuVisible = false; contextMenuItem = null; if (closeContextMenuListener) closeContextMenuListener = null; }
	}

	async function handleMenuAction(action) {
		const item = contextMenuItem; if (!item) return;
        const itemPathForClosure = item.path; closeContextMenu();
		switch (action) {
			case 'Load':
				if (!item.is_directory && item.file_type === 'media') selectMedia(item);
                else console.warn("[LeftPanel] 'Load' action called on non-media item:", item);
				break;
            case 'OpenNote':
                if (!item.is_directory && item.file_type === 'note') handleOpenNote(item);
                else console.warn("[LeftPanel] 'OpenNote' action called on non-note item:", item);
                break;
			case 'Rename':
				if (!item.is_directory) {
                    itemToRename = { path: item.path, name: item.name, file_type: item.file_type, media_xml_identifier: item.media_xml_identifier };
					showRenameModal = true;
				} else console.warn("[LeftPanel] Rename requested on directory (not allowed):", item);
				break;
			case 'Delete': {
				if (!item.is_directory) {
                    let confirmMsg = '';
                    if (item.file_type === 'media') {
                        const stemName = item.media_xml_identifier || (item.name.includes('.') ? item.name.substring(0, item.name.lastIndexOf('.')) : item.name);
						confirmMsg = `Are you sure you want to delete the media file "${item.name}"?\n\nThis will permanently delete the entire folder for this media source ("${stemName}"), including associated transcripts and notes.\n\nThis action cannot be undone.`;
					} else if (item.file_type === 'transcript') {
                        const mediaStem = item.media_xml_identifier || item.name.replace(/\.[^/.]+$/, "");
                        confirmMsg = `Are you sure you want to delete the transcript file "${item.name}"?\n\nThis will remove it from the project.\n\nThis action cannot be undone.`;
					} else if (item.file_type === 'note') {
                        confirmMsg = `Are you sure you want to delete the note file "${item.name}"?\n\nThis action cannot be undone.`;
                    } else { confirmMsg = `Are you sure you want to delete the file "${item.name}"?\n\nThis cannot be undone.`; }
                    try {
                        const confirmed = await confirm(confirmMsg, { title: 'Confirm Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' });
                        if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(item.path); } catch (err) { console.error(`[LeftPanel] Delete service call failed:`, err); } }
                        else { project.update(p => ({ ...p, statusMessage: 'Deletion cancelled.' })); }
                    } catch (e) { await message(`An error occurred during deletion: ${e}`, {title: "Delete Error", type: "error"}); }
				} else console.warn("[LeftPanel] Delete requested on directory (not allowed):", item);
				break; // End Delete case
			}
			default: await message(`Action '${action}' not implemented yet.`, { title: 'Not Implemented', type: 'info' }); break;
		}
	}

	// Handle confirmation from the rename modal
	async function handleRenameConfirm(event) {
		const { newName } = event.detail; const item = itemToRename;
		if (!item || !newName || newName.trim() === '') { console.error("[LeftPanel] Rename confirmation failed: Missing item or new name."); showRenameModal = false; itemToRename = null; return; }
		const finalNewName = newName.trim(); showRenameModal = false;
        if (item.file_type === 'media') {
             const currentExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : ''; const fullNewMediaName = `${finalNewName}${currentExtension}`;
             const confirmRename = await confirm(`Renaming media '${item.name}' to '${fullNewMediaName}' will also rename the folder and primary transcript.\n\nProceed?`, { title: 'Confirm Media Rename', type: 'warning', okLabel: 'Rename', cancelLabel: 'Cancel' });
             if (!confirmRename) { itemToRename = null; return; }
        }
        else if (item.file_type === 'transcript') {
            const mediaStem = item.media_xml_identifier; const primaryTranscriptName = mediaStem ? `${mediaStem}.json` : null;
            if (item.name === primaryTranscriptName && finalNewName !== primaryTranscriptName) {
                 const confirmTranscriptRename = await confirm(`Renaming the primary transcript '${item.name}' to '${finalNewName}' may break automatic loading.\n\nProceed?`, { title: 'Confirm Primary Transcript Rename', type: 'warning', okLabel: 'Rename', cancelLabel: 'Cancel' });
                 if (!confirmTranscriptRename) { itemToRename = null; return; }
            }
        }
		try { await renameProjectItem(item.path, finalNewName, item.file_type); }
        catch (err) { console.error(`[LeftPanel] Rename service call failed:`, err); }
        finally { itemToRename = null; }
	}

	function handleRenameModalClose() { showRenameModal = false; itemToRename = null; }

	// --- UI Elements ---
	const CHEVRON_DOWN = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;
	const CHEVRON_RIGHT = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M8.22 5.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L11.94 10 8.22 6.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;

</script>

<!-- Main Container -->
<div class="h-full flex flex-col bg-inherit text-gray-800 dark:text-gray-200">

	{#if !$panelStateStore.leftCollapsed}
	<!-- Media Files Accordion Header -->
	<div class="border-b border-gray-300 dark:border-gray-700 flex-shrink-0">
		<div
			class="flex items-center px-2 py-2 cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700"
			on:click="{() => toggleSection('files')}"
			aria-expanded="{openSection === 'files'}" aria-controls="files-content" role="button" tabindex="0"
			on:keydown="{(e) => { if (e.key === 'Enter' || e.key === ' ') toggleSection('files'); }}"
		>
			<span class="mr-1.5 text-gray-600 dark:text-gray-400"> <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.227-.026.344-.026h15.812c.117 0 .232.009.344.026m-16.5 0a2.25 2.25 0 0 0-1.883 2.542l.857 6a2.25 2.25 0 0 0 2.227 1.932H19.5a2.25 2.25 0 0 0 2.227-1.932l.857-6a2.25 2.25 0 0 0-1.883-2.542m-16.5 0V6A2.25 2.25 0 0 1 6 3.75h3.879a1.5 1.5 0 0 1 1.06.44l2.122 2.12a1.5 1.5 0 0 0 1.06.44H18A2.25 2.25 0 0 1 20.25 9v.776" /></svg> </span>
			<h3 class="font-semibold text-sm flex-grow">Media Files</h3>
			<span class="ml-2 text-gray-500 dark:text-gray-400"> {@html openSection === 'files' ? CHEVRON_DOWN : CHEVRON_RIGHT} </span>
		</div>
	</div>

	<!-- Media Files Content (Tree) -->
	{#if openSection === 'files'}
		<div id="files-content" class="flex-grow overflow-y-auto min-h-0 pb-1 pt-1 px-1" role="region" aria-live="polite">
			{#if $project.isLoading && !$project.files?.length}
				<p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-2">Loading project...</p>
			{:else if !$project.files || projectFileTree.length === 0}
				<p class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-2">Import a media file to begin.</p>
			{:else}
				<ul class="space-y-0.5">
					{#each projectFileTree as node (node.path || node.relativePath) }
						<TreeNode
							{node}
							{selectedMediaPath}
                            currentTranscriptPath={currentTranscriptPath}
							on:itemclick={handleItemClick}
							on:itemcontextmenu={handleContextMenu}
							on:itemdblclick={handleItemDoubleClick}
						/>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}

	<!-- Shortcuts Accordion Header -->
	<div class="border-b border-gray-300 dark:border-gray-700 flex-shrink-0">
         <div
			class="flex items-center px-2 py-2 cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700"
			on:click="{() => toggleSection('shortcuts')}"
			aria-expanded="{openSection === 'shortcuts'}" aria-controls="shortcuts-content" role="button" tabindex="0"
			on:keydown="{(e) => { if (e.key === 'Enter' || e.key === ' ') toggleSection('shortcuts'); }}"
		>
			<span class="mr-1.5 text-gray-600 dark:text-gray-400"> <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" /></svg> </span>
			<h3 class="font-semibold text-sm flex-grow">Shortcuts</h3>
			<span class="ml-2 text-gray-500 dark:text-gray-400"> {@html openSection === 'shortcuts' ? CHEVRON_DOWN : CHEVRON_RIGHT} </span>
		</div>
	</div>

	<!-- Shortcuts Content -->
	{#if openSection === 'shortcuts'}
		<div id="shortcuts-content" class="flex-grow overflow-y-auto min-h-0 p-3 text-xs" role="region" aria-live="polite">
            <ul class="space-y-1.5 text-gray-700 dark:text-gray-300">
				<li class="flex items-center"> <span class="font-mono bg-gray-200 dark:bg-gray-600 px-1.5 py-0.5 rounded text-gray-800 dark:text-gray-200 mr-3 text-[11px] min-w-[60px] text-center">Ctrl + E</span> <span>Edit Segment</span> </li>
				<li class="flex items-center"> <span class="font-mono bg-gray-200 dark:bg-gray-600 px-1.5 py-0.5 rounded text-gray-800 dark:text-gray-200 mr-3 text-[11px] min-w-[60px] text-center">Ctrl + S</span> <span>Save Transcript</span> </li>
				<li class="flex items-center"> <span class="font-mono bg-gray-200 dark:bg-gray-600 px-1.5 py-0.5 rounded text-gray-800 dark:text-gray-200 mr-3 text-[11px] min-w-[60px] text-center">F8</span> <span>Play / Pause</span> </li>
			</ul>
		</div>
	{/if}
	{:else}
	<!-- Collapsed Panel Buttons -->
	<div class="flex flex-col items-center space-y-2 py-2 border-b border-gray-300 dark:border-gray-700 flex-shrink-0">
		<button
			class="p-2 rounded hover:bg-gray-200 dark:hover:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
			title="Media Files"
			on:mouseenter={(event) => showTooltip(event, 'Media Files', $project.files && $project.files.length > 0 ? $project.files.slice(0,5).map(f => ({name: f.name || f.path})) : [{name: 'No media files'}])}
			on:mouseleave={hideTooltip}
			on:focus={(event) => showTooltip(event, 'Media Files', $project.files && $project.files.length > 0 ? $project.files.slice(0,5).map(f => ({name: f.name || f.path})) : [{name: 'No media files'}])}
			on:blur={hideTooltip}
		>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-gray-600 dark:text-gray-400"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.227-.026.344-.026h15.812c.117 0 .232.009.344.026m-16.5 0a2.25 2.25 0 0 0-1.883 2.542l.857 6a2.25 2.25 0 0 0 2.227 1.932H19.5a2.25 2.25 0 0 0 2.227-1.932l.857-6a2.25 2.25 0 0 0-1.883-2.542m-16.5 0V6A2.25 2.25 0 0 1 6 3.75h3.879a1.5 1.5 0 0 1 1.06.44l2.122 2.12a1.5 1.5 0 0 0 1.06.44H18A2.25 2.25 0 0 1 20.25 9v.776" /></svg>
		</button>
		<button
			class="p-2 rounded hover:bg-gray-200 dark:hover:bg-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
			title="Shortcuts"
			on:mouseenter={(event) => showTooltip(event, 'Shortcuts', [{name: 'Ctrl + E: Edit Segment'}, {name: 'Ctrl + S: Save Transcript'}, {name: 'F8: Play/Pause'}])}
			on:mouseleave={hideTooltip}
			on:focus={(event) => showTooltip(event, 'Shortcuts', [{name: 'Ctrl + E: Edit Segment'}, {name: 'Ctrl + S: Save Transcript'}, {name: 'F8: Play/Pause'}])}
			on:blur={hideTooltip}
		>
			<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-gray-600 dark:text-gray-400"><path stroke-linecap="round" stroke-linejoin="round" d="m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z" /></svg>
		</button>
	</div>
	{/if}


	<!-- Context Menu -->
	{#if contextMenuVisible && contextMenuItem}
		<div
            id="context-menu-div"
			class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[120px]"
			style="left: {contextMenuX}px; top: {contextMenuY}px;"
			on:click|stopPropagation
		>
            {#if !contextMenuItem.is_directory}
                {#if contextMenuItem.file_type === 'media'}
				    <button on:click|stopPropagation="{(e) => handleMenuAction('Load')}" class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Load Media</button>
                    <hr class="my-1 border-gray-200 dark:border-gray-600" />
                {/if}
                {#if contextMenuItem.file_type === 'note'}
				    <button on:click|stopPropagation="{(e) => handleMenuAction('OpenNote')}" class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open Note</button>
                    <hr class="my-1 border-gray-200 dark:border-gray-600" />
                {/if}
				{#if ['media', 'transcript', 'note', 'other'].includes(contextMenuItem.file_type)}
					<button on:click|stopPropagation="{(e) => handleMenuAction('Rename')}" class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename…</button>
					<button on:click|stopPropagation="{(e) => handleMenuAction('Delete')}" class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete…</button>
				{/if}
			{:else}
                 <span class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic">No actions available</span>
            {/if}
		</div>
	{/if}

</div>

<!-- Rename Modal -->
<FileRenameModal
	bind:showModal={showRenameModal}
	currentName={itemToRename?.name || ''}
	itemType={itemToRename?.file_type || ''}
	on:confirm={handleRenameConfirm}
	on:close={handleRenameModalClose}
/>

<CategoryTooltip
	bind:visible={tooltipVisible}
	categoryName={tooltipCategoryName}
	items={tooltipItems}
	x={tooltipX}
	y={tooltipY}
/>

<style>
	.flex-grow.overflow-y-auto::-webkit-scrollbar { width: 6px; height: 6px; }
	.flex-grow.overflow-y-auto::-webkit-scrollbar-track { background: transparent; }
	.flex-grow.overflow-y-auto::-webkit-scrollbar-thumb { background-color: rgba(156, 163, 175, 0.5); border-radius: 3px; }
	.dark .flex-grow.overflow-y-auto::-webkit-scrollbar-thumb { background-color: rgba(107, 114, 128, 0.5); }
	.flex-grow.overflow-y-auto::-webkit-scrollbar-thumb:hover { background-color: rgba(107, 114, 128, 0.7); }
	.dark .flex-grow.overflow-y-auto::-webkit-scrollbar-thumb:hover { background-color: rgba(75, 85, 99, 0.7); }
	.flex-grow.overflow-y-auto { scrollbar-width: thin; scrollbar-color: rgba(156, 163, 175, 0.5) transparent; }
	.dark .flex-grow.overflow-y-auto { scrollbar-color: rgba(107, 114, 128, 0.5) transparent; }
	.min-h-0 { min-height: 0; }

    /* Removed highlight-transcript class */

</style>