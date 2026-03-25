<!-- src/lib/components/projectview/transcription/LeftPanel.svelte -->
<script>
	import { get } from "svelte/store";
	import {
		project,
		HARVEY_FILES_DIR,
		MEDIA_DIR_NAME,
	} from "$lib/stores/projectStore.js";
	import {
		transcriptStore,
		selectMedia,
	} from "$lib/stores/transcriptStore.js";
	import {
		loadTranscriptFile,
		refreshProjectFiles,
		renameProjectItem,
		deleteProjectItem,
		normalizePath,
	} from "$lib/services/projectService.js";
	import TreeNode from "./TreeNode.svelte";
	import FileRenameModal from "../modals/FileRenameModal.svelte";
	import { confirm, message } from "@tauri-apps/plugin-dialog";
	import { createEventDispatcher } from "svelte";
	import { sep } from "@tauri-apps/api/path";

	const dispatch = createEventDispatcher();

	// --- State for Accordion Sections ---
	let openSection = "files";

	// --- State for Rename Modal ---
	let showRenameModal = false;
	let itemToRename = null;

	// --- Accordion Click Handlers ---
	function toggleSection(sectionName) {
		openSection = openSection === sectionName ? null : sectionName;
	}

	// Determine platform-specific modifier key name
	const isMac =
		typeof window !== "undefined" &&
		navigator.platform.toUpperCase().indexOf("MAC") >= 0;
	const modKeyName = isMac ? "⌘" : "Ctrl";
	const ctrlKeyName = isMac ? "⌃" : "Ctrl";
	const optKeyName = isMac ? "⌥" : "Alt";
	const shiftKeyName = isMac ? "⇧" : "Shift";
	const enterKeyName = isMac ? "↵" : "Enter";

	// --- File Tree Logic ---
	$: selectedMediaPath = $transcriptStore.selectedMediaFile?.path;
	// NEW: Get current transcript path for highlighting
	// currentTranscriptPath is now sourced from transcriptStore.
	$: currentTranscriptPath = $transcriptStore.activeTranscript?.path;

	// --- projectFileTree now directly uses the XML-derived tree from the store ---
	$: projectFileTree = $project.files || [];

	$: uniqueProjectFileTree = (() => {
		const normalizedBaseDirectory = normalizePath($project.baseDirectory);
		const mediaPathPrefix = normalizePath(
			`${normalizedBaseDirectory}${sep()}${HARVEY_FILES_DIR}${sep()}${MEDIA_DIR_NAME}`,
		);
		const seen = new Set();

		const filtered = projectFileTree.filter((node) => {
			const normalizedNodePath = normalizePath(node.path);
			const key = normalizedNodePath || normalizePath(node.relativePath);

			if (seen.has(key)) {
				return false;
			}
			seen.add(key);

			const isMediaFileOrDirectory =
				node.file_type === "media" ||
				node.file_type === "directory_media_stem" ||
				node.file_type === "transcript";
			const isWithinMediaPath =
				normalizedNodePath &&
				normalizedNodePath.startsWith(mediaPathPrefix);
			const isRootMediaDirectory = normalizedNodePath === mediaPathPrefix;

			return (
				isRootMediaDirectory ||
				(isWithinMediaPath && isMediaFileOrDirectory)
			);
		});

		return filtered;
	})();

	// --- Function to handle opening a data ---
	function handleOpenData(item) {
		if (!item || item.file_type !== "data") return;
		console.log(
			`[LeftPanel] Requesting to open data: ${item.name} (${item.path})`,
		);
		dispatch("requestopentab", { tabName: "data", dataPath: item.path });
	}

	// --- Item Interaction Logic ---
	// MODIFIED: handleItemClick to dispatch a generic request
	async function handleItemClick(event) {
		const item = event.detail;
		console.log("[LeftPanel] handleItemClick triggered for:", item);

		if (item.is_directory) {
			console.log(
				"[LeftPanel] Clicked item is a directory, ignoring for selection/load.",
			);
			return; // Ignore clicks on directories
		}

		// Dispatch a generic request to the parent (TranscriptionView) to handle the item loading
		dispatch("requestLoadItem", item);
	}

	function handleItemDoubleClick(event) {
		const item = event.detail;
		if (!item.is_directory && item.file_type === "media") {
			console.log(
				"[LeftPanel] Double-clicked media, calling selectMedia.",
			);
			selectMedia(item, item.path);
		} else if (!item.is_directory && item.file_type === "data") {
			console.log(
				"[LeftPanel] Double-clicked data, calling handleOpenData.",
			);
			handleOpenData(item);
		}
	}

	// --- Context Menu Logic ---
	let contextMenuVisible = false;
	let contextMenuX = 0;
	let contextMenuY = 0;
	let contextMenuItem = null;
	let closeContextMenuListener = null;

	function handleContextMenu(event) {
		const { event: mouseEvent, item } = event.detail;
		if (item.is_directory) return; // Only allow on files
		if (contextMenuVisible) closeContextMenu();
		mouseEvent.preventDefault();
		mouseEvent.stopPropagation();
		contextMenuItem = item;
		contextMenuX = mouseEvent.clientX;
		contextMenuY = mouseEvent.clientY;
		contextMenuVisible = true;
		setTimeout(() => {
			if (closeContextMenuListener)
				document.removeEventListener(
					"click",
					closeContextMenuListener,
					{ capture: true },
				);
			closeContextMenuListener = (e) => {
				const menuElement = document.getElementById("context-menu-div");
				if (menuElement && !menuElement.contains(e.target))
					closeContextMenu();
			};
			document.addEventListener("click", closeContextMenuListener, {
				capture: true,
				once: true,
			});
		}, 0);
	}

	function closeContextMenu() {
		if (contextMenuVisible) {
			contextMenuVisible = false;
			contextMenuItem = null;
			if (closeContextMenuListener) closeContextMenuListener = null;
		}
	}

	async function handleMenuAction(action) {
		const item = contextMenuItem;
		if (!item) return;
		const itemPathForClosure = item.path;
		closeContextMenu();
		switch (action) {
			case "Load":
				if (!item.is_directory && item.file_type === "media")
					selectMedia(item);
				else
					console.warn(
						"[LeftPanel] 'Load' action called on non-media item:",
						item,
					);
				break;
			case "OpenData":
				if (!item.is_directory && item.file_type === "data")
					handleOpenData(item);
				else
					console.warn(
						"[LeftPanel] 'OpenData' action called on non-data item:",
						item,
					);
				break;
			case "Rename":
				if (!item.is_directory) {
					itemToRename = {
						path: item.path,
						name: item.name,
						file_type: item.file_type,
						media_xml_identifier: item.media_xml_identifier,
					};
					showRenameModal = true;
				} else
					console.warn(
						"[LeftPanel] Rename requested on directory (not allowed):",
						item,
					);
				break;
			case "Delete": {
				if (item.is_directory) {
					console.warn(
						"[LeftPanel] Delete requested on directory (not allowed via context menu):",
						item,
					);
					break;
				}

				let confirmMsg = "";

				if (item.file_type === "media") {
					const stemName =
						item.media_xml_identifier ||
						(item.name.includes(".")
							? item.name.substring(0, item.name.lastIndexOf("."))
							: item.name);
					confirmMsg = `Are you sure you want to delete the media file "${item.name}"?\n\nThis will permanently delete the entire folder for this media source ("${stemName}"), including associated transcripts and data.\n\nThis action cannot be undone.`;
				} else if (item.file_type === "transcript") {
					confirmMsg = `Are you sure you want to delete the transcript file "${item.name}"?\n\nThis will remove it from the project.\n\nThis action cannot be undone.`;
				} else if (item.file_type === "data") {
					confirmMsg = `Are you sure you want to delete the data file "${item.name}"?\n\nThis action cannot be undone.`;
				} else {
					confirmMsg = `Are you sure you want to delete the file "${item.name}"?\n\nThis cannot be undone.`;
				}

				try {
					const confirmed = await confirm(confirmMsg, {
						title: "Confirm Deletion",
						type: "warning",
						okLabel: "Delete",
						cancelLabel: "Cancel",
					});
					if (confirmed) {
						project.update((p) => ({
							...p,
							statusMessage: `Deleting ${item.name}...`,
						}));
						try {
							await deleteProjectItem(item.path);
						} catch (err) {
							console.error(
								`[LeftPanel] Delete service call failed:`,
								err,
							);
						}
					} else {
						project.update((p) => ({
							...p,
							statusMessage: "Deletion cancelled.",
						}));
					}
				} catch (e) {
					await message(
						`An error occurred during the deletion process: ${e.message || e}`,
						{ title: "Delete Error", type: "error" },
					);
				}
				break;
			}
			default:
				await message(`Action '${action}' not implemented yet.`, {
					title: "Not Implemented",
					type: "info",
				});
				break;
		}
	}

	// Handle confirmation from the rename modal
	async function handleRenameConfirm(event) {
		const { newName } = event.detail;
		const item = itemToRename;
		if (!item || !newName || newName.trim() === "") {
			console.error(
				"[LeftPanel] Rename confirmation failed: Missing item or new name.",
			);
			showRenameModal = false;
			itemToRename = null;
			return;
		}
		const finalNewName = newName.trim();
		showRenameModal = false;
		if (item.file_type === "media") {
			const currentExtension = item.name.includes(".")
				? item.name.substring(item.name.lastIndexOf("."))
				: "";
			const fullNewMediaName = `${finalNewName}${currentExtension}`;
			const confirmRename = await confirm(
				`Renaming media '${item.name}' to '${fullNewMediaName}' will also rename the folder and primary transcript.\n\nProceed?`,
				{
					title: "Confirm Media Rename",
					type: "warning",
					okLabel: "Rename",
					cancelLabel: "Cancel",
				},
			);
			if (!confirmRename) {
				itemToRename = null;
				return;
			}
		} else if (item.file_type === "transcript") {
			const mediaStem = item.media_xml_identifier;
			const primaryTranscriptName = mediaStem
				? `${mediaStem}.json`
				: null;
			if (
				item.name === primaryTranscriptName &&
				finalNewName !== primaryTranscriptName
			) {
				const confirmTranscriptRename = await confirm(
					`Renaming the primary transcript '${item.name}' to '${finalNewName}' may break automatic loading.\n\nProceed?`,
					{
						title: "Confirm Primary Transcript Rename",
						type: "warning",
						okLabel: "Rename",
						cancelLabel: "Cancel",
					},
				);
				if (!confirmTranscriptRename) {
					itemToRename = null;
					return;
				}
			}
		}
		try {
			await renameProjectItem(item.path, finalNewName, item.file_type);
		} catch (err) {
			console.error(`[LeftPanel] Rename service call failed:`, err);
		} finally {
			itemToRename = null;
		}
	}

	function handleRenameModalClose() {
		showRenameModal = false;
		itemToRename = null;
	}

	// --- UI Elements ---
	const CHEVRON_DOWN = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;
	const CHEVRON_RIGHT = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M8.22 5.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L11.94 10 8.22 6.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;
</script>

<!-- Main Container -->
<div class="h-full flex flex-col bg-inherit text-gray-800 dark:text-gray-200">
	<!-- Media Files Accordion Header -->
	<h2
		class="flex items-center justify-between text-sm font-semibold text-gray-700 dark:text-gray-400 px-1 h-9 border-b border-gray-200 dark:border-gray-800 cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-800 flex-shrink-0"
		on:click={() => toggleSection("files")}
		aria-expanded={openSection === "files"}
		aria-controls="files-content"
		role="button"
		tabindex="0"
		on:keydown={(e) => {
			if (e.key === "Enter" || e.key === " ") toggleSection("files");
		}}
	>
		<div
			class="flex items-center space-x-2 transition-opacity duration-200"
		>
			<span class="pl-2">Media Files</span>
		</div>
		<span class="pr-1 text-gray-500 dark:text-gray-400">
			{@html openSection === "files" ? CHEVRON_DOWN : CHEVRON_RIGHT}
		</span>
	</h2>

	<!-- Media Files Content (Tree) -->
	{#if openSection === "files"}
		<div
			id="files-content"
			class="flex-grow overflow-y-auto min-h-0 pb-1 pt-1 px-1 mb-3 text-xs"
			role="region"
			aria-live="polite"
		>
			{#if $project.isLoading && !$project.files?.length}
				<p
					class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-2"
				>
					Loading project...
				</p>
			{:else if !$project.files || projectFileTree.length === 0}
				<p
					class="text-xs text-gray-500 dark:text-gray-400 italic px-2 py-2"
				>
					Import a media file to begin.
				</p>
			{:else}
				<ul class="space-y-0.5">
					{#each uniqueProjectFileTree as node (node.path || node.relativePath)}
						<TreeNode
							{node}
							{selectedMediaPath}
							{currentTranscriptPath}
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
	<h2
		class="flex items-center justify-between text-sm font-semibold text-gray-700 dark:text-gray-400 px-1 h-9 border-b border-gray-200 dark:border-gray-800 cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-800 flex-shrink-0 {openSection !==
		'files'
			? ''
			: 'border-t'}"
		on:click={() => toggleSection("shortcuts")}
		aria-expanded={openSection === "shortcuts"}
		aria-controls="shortcuts-content"
		role="button"
		tabindex="0"
		on:keydown={(e) => {
			if (e.key === "Enter" || e.key === " ") toggleSection("shortcuts");
		}}
	>
		<div
			class="flex items-center space-x-2 transition-opacity duration-200"
		>
			<span class="pl-2">Shortcuts</span>
		</div>
		<span class="pr-1 text-gray-500 dark:text-gray-400">
			{@html openSection === "shortcuts" ? CHEVRON_DOWN : CHEVRON_RIGHT}
		</span>
	</h2>

	<!-- Shortcuts Content -->
	{#if openSection === "shortcuts"}
		<div
			id="shortcuts-content"
			class="flex-grow overflow-y-auto min-h-0 p-3 text-xs"
			role="region"
			aria-live="polite"
		>
			<ul class="space-y-2 text-gray-700 dark:text-gray-300">
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;E</span
					> <span>Switch Edit / Read Mode</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{shiftKeyName}&nbsp;&nbsp;Space</span
					> <span>Play / Pause</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{shiftKeyName}&nbsp;&nbsp;←</span
					> <span>Rewind</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{shiftKeyName}&nbsp;&nbsp;→</span
					> <span>Forward</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{optKeyName}&nbsp;&nbsp;↑</span
					> <span>Previous Segment</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{optKeyName}&nbsp;&nbsp;↓</span
					> <span>Next Segment</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{shiftKeyName}&nbsp;&nbsp;{enterKeyName}</span
					> <span>Insert New Segment</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{shiftKeyName}&nbsp;&nbsp;J&nbsp;/&nbsp;K</span
					> <span>Change Speaker</span>
				</li>
				<li class="flex items-center">
					<span
						class="font-mono bg-gray-200 dark:bg-gray-600 px-2 py-1 rounded text-gray-800 dark:text-gray-200 mr-3 text-[13px] min-w-[100px] text-center"
						>{modKeyName}&nbsp;&nbsp;{shiftKeyName}&nbsp;&nbsp;,&nbsp;/&nbsp;.</span
					> <span>Speed Down / Up</span>
				</li>
			</ul>
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
				{#if contextMenuItem.file_type === "media"}
					<button
						on:click|stopPropagation={(e) =>
							handleMenuAction("Load")}
						class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
						>Load Media</button
					>
					<hr class="my-1 border-gray-200 dark:border-gray-600" />
				{/if}
				{#if contextMenuItem.file_type === "note"}
					<button
						on:click|stopPropagation={(e) =>
							handleMenuAction("OpenNote")}
						class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
						>Open Note</button
					>
					<hr class="my-1 border-gray-200 dark:border-gray-600" />
				{/if}
				{#if ["media", "transcript", "data", "other"].includes(contextMenuItem.file_type)}
					<button
						on:click|stopPropagation={(e) =>
							handleMenuAction("Rename")}
						class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
						>Rename…</button
					>
					<button
						on:click|stopPropagation={(e) =>
							handleMenuAction("Delete")}
						class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500"
						>Delete…</button
					>
				{/if}
			{:else}
				<span
					class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic"
					>No actions available</span
				>
			{/if}
		</div>
	{/if}
</div>

<!-- Rename Modal -->
<FileRenameModal
	bind:showModal={showRenameModal}
	currentName={itemToRename?.name || ""}
	itemType={itemToRename?.file_type || ""}
	on:confirm={handleRenameConfirm}
	on:close={handleRenameModalClose}
/>

<style>
	.flex-grow.overflow-y-auto::-webkit-scrollbar {
		width: 6px;
		height: 6px;
	}
	.flex-grow.overflow-y-auto::-webkit-scrollbar-track {
		background: transparent;
	}
	.flex-grow.overflow-y-auto::-webkit-scrollbar-thumb {
		background-color: rgba(156, 163, 175, 0.5);
		border-radius: 3px;
	}
	.dark .flex-grow.overflow-y-auto::-webkit-scrollbar-thumb {
		background-color: rgba(107, 114, 128, 0.5);
	}
	.flex-grow.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background-color: rgba(107, 114, 128, 0.7);
	}
	.dark .flex-grow.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background-color: rgba(75, 85, 99, 0.7);
	}
	.flex-grow.overflow-y-auto {
		scrollbar-width: thin;
		scrollbar-color: rgba(156, 163, 175, 0.5) transparent;
	}
	.dark .flex-grow.overflow-y-auto {
		scrollbar-color: rgba(107, 114, 128, 0.5) transparent;
	}
	.min-h-0 {
		min-height: 0;
	}

	/* Removed highlight-transcript class */
</style>
