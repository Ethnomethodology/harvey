<!-- src/lib/components/projectview/data/DataLeftPanel.svelte -->
<script>
	import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView, setSelectedGroup, currentProjectGroupsList, updateProjectGroupsList } from '$lib/stores/projectStore.js'; // Added setSelectedGroup, currentProjectGroupsList, and updateProjectGroupsList
	import { get } from 'svelte/store';
	import panelStateStore from '$lib/stores/panelStateStore.js';
	import { renameProjectItem, deleteProjectItem, importMediaFile, importDocumentFile, importTableFile, importImageFile, importTranscriptFile, deleteImportedTranscript } from '$lib/services/projectService.js';
	import FileRenameModal from '../modals/FileRenameModal.svelte';
	import ImportTranscriptSourceModal from '../modals/ImportTranscriptSourceModal.svelte';
    import CreateGroupModal from '../modals/CreateGroupModal.svelte';
    import GroupRenameModal from '../modals/GroupRenameModal.svelte'; // Added GroupRenameModal
	import { confirm, message } from '@tauri-apps/plugin-dialog';
	import * as openerPlugin from '@tauri-apps/plugin-opener';
	import { createEventDispatcher, onMount } from 'svelte';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { type as getOsType } from '@tauri-apps/plugin-os';
    import CategoryTooltip from './CategoryTooltip.svelte';
    import { searchQuery, showSearchBox } from '$lib/stores/searchStore.js';


    const dispatch = createEventDispatcher();

    const JOURNAL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-journals" viewBox="0 0 16 16"><path d="M5 0h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2 2 2 0 0 1-2 2H3a2 2 0 0 1-2-2h1a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1H1a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v9a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1H3a2 2 0 0 1 2-2"/><path d="M1 6v-.5a.5.5 0 0 1 1 0V6h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 3v-.5a.5.5 0 0 1 1 0V9h.5a.5.5 0 0 1 0 1h-2a.5.5 0 0 1 0-1zm0 2.5v.5H.5a.5.5 0 0 0 0 1h2a.5.5 0 0 0 0-1H2v-.5a.5.5 0 0 0-1 0"/></svg>`;

    function handleGroupItemContextMenu(event, group) {
        event.preventDefault();
        event.stopPropagation();
        if (groupContextMenuVisible) closeGroupContextMenu();
        // Close other menus if open
        if (contextMenuVisible) closeContextMenu();
        if (categoryContextMenuVisible) closeCategoryContextMenu();
        if (showGroupSubMenu) closeGroupSubMenu();


        groupContextMenuItem = group;
        groupContextMenuX = event.clientX;
        groupContextMenuY = event.clientY;
        groupContextMenuVisible = true;

        // Add listener to close on outside click
        setTimeout(() => {
            if (closeGroupContextMenuListener) document.removeEventListener('click', closeGroupContextMenuListener, { capture: true });
            closeGroupContextMenuListener = (e) => {
                const menuElement = document.getElementById('notes-left-panel-group-item-context-menu');
                if (menuElement && !menuElement.contains(e.target)) {
                    closeGroupContextMenu();
                }
            };
            document.addEventListener('click', closeGroupContextMenuListener, { capture: true });
        }, 0);
    }

    function closeGroupContextMenu() {
        if (groupContextMenuVisible) {
            groupContextMenuVisible = false;
            groupContextMenuItem = null;
            if (closeGroupContextMenuListener) {
                document.removeEventListener('click', closeGroupContextMenuListener, { capture: true });
                closeGroupContextMenuListener = null;
            }
        }
    }

    async function handleGroupContextMenuAction(action) { // Made this function async
        const group = groupContextMenuItem;
        closeGroupContextMenu();
        if (!group) {
            console.error("[NotesLeftPanel] Group context item is null for action:", action);
            return;
        }
        console.log(`[NotesLeftPanel] Group context menu action: '${action}' for group:`, group.name);

        if (action === 'Open') {
            if (group && group.id) {
                setSelectedGroup(group.id, group); // setSelectedGroup is already imported
                // Optionally, dispatch an event if ProjectView needs to react beyond store changes,
                // but setSelectedGroup should trigger NotesView to show GroupDetailView.
                project.update(p => ({ ...p, statusMessage: `Opened group: ${group.name}` }));
            } else {
                console.error("[NotesLeftPanel] 'Open' action called with invalid group data:", group);
                message("Cannot open group: Invalid group data.", {title: "Error", type: "error"});
            }
        } else if (action === 'Rename') {
            if (group && group.id) {
                groupToRename = { ...group }; // Store a copy of the group data
                showGroupRenameModal = true;
            } else {
                console.error("[NotesLeftPanel] 'Rename' action called with invalid group data:", group);
                message("Cannot rename group: Invalid group data.", {title: "Error", type: "error"});
            }
        } else if (action === 'Delete') {
            if (group && group.id) {
                const confirmed = await confirm(`Are you sure you want to delete the group "${group.name}"? This action cannot be undone, but the files within the group will not be deleted.`, {
                    title: 'Confirm Group Deletion',
                    type: 'warning',
                    okLabel: 'Delete Group',
                    cancelLabel: 'Cancel'
                });

                if (confirmed) {
                    const currentProjectId = get(project).id;
                    if (!currentProjectId) {
                        message('Project ID is missing. Cannot delete group.', { title: 'Error', type: 'error' });
                        return;
                    }
                    try {
                        // Backend command to be implemented: delete_project_group
                        await invoke('delete_project_group', {
                            projectId: currentProjectId,
                            groupId: group.id
                        });

                        await updateProjectGroupsList(currentProjectId); // Refresh the list

                        if (get(project).selectedGroupId === group.id) {
                            setSelectedGroup(null, null); // Clear selection if the deleted group was active
                             // Additionally, make NotesView show the placeholder
                            dispatch('requestviewchange', { viewType: 'placeholder', itemPath: null });
                        }
                        project.update(p => ({ ...p, statusMessage: `Group "${group.name}" deleted.` }));

                    } catch (err) {
                        console.error(`[NotesLeftPanel] Error deleting group ${group.id}:`, err);
                        await message(`Failed to delete group "${group.name}": ${err}`, { title: 'Delete Error', type: 'error' });
                        project.update(p => ({ ...p, statusMessage: `Failed to delete group "${group.name}".` }));
                    }
                }
            } else {
                console.error("[NotesLeftPanel] 'Delete' action called with invalid group data:", group);
                message("Cannot delete group: Invalid group data.", {title: "Error", type: "error"});
            }
        }
    }

    function handleToggleDataLeftPanel() {
        tooltipVisible = false;
        panelStateStore.toggleDataLeftPanel();
    }

    function handleItemContextMenu(event, item) {
        event.preventDefault();
        event.stopPropagation();
        if (contextMenuVisible) closeContextMenu();
        contextMenuItem = item;
        contextMenuX = event.clientX;
        contextMenuY = event.clientY;
        contextMenuVisible = true;

        // Add listener to close on outside click
        setTimeout(() => {
            if (closeContextMenuListener) document.removeEventListener('click', closeContextMenuListener, { capture: true });
            closeContextMenuListener = (e) => {
                const menuElement = document.getElementById('notes-left-panel-context-menu');
                if (menuElement && !menuElement.contains(e.target)) {
                    closeContextMenu();
                }
            };
            document.addEventListener('click', closeContextMenuListener, { capture: true });
        }, 0);
    }

    function closeContextMenu() {
        if (contextMenuVisible) {
            contextMenuVisible = false;
            contextMenuItem = null;
            if (closeContextMenuListener) {
                document.removeEventListener('click', closeContextMenuListener, { capture: true });
                closeContextMenuListener = null;
            }
        }
    }

    async function handleContextMenuAction(action) {
        console.log(`[DataLeftPanel] Context action: "${action}"`);
        const item = contextMenuItem;
        if (!item) { console.error("[DataLeftPanel] Context item null."); closeContextMenu(); return; }

        const itemPathForClosure = item.path;
        const itemType = item.file_type; 
        const isPdf = item.name?.toLowerCase().endsWith('.pdf');
        // Do not close main context menu here if "Add to Group" is clicked, as it will be handled by handleShowAddToGroupSubMenu
        if (action !== 'AddToGroup') { // Assuming 'AddToGroup' would be a specific action string if passed here
            closeContextMenu();
        }


        if (action === 'Reveal') {
            if (!itemPathForClosure) {
                console.error("[DataLeftPanel] Reveal error: Item path is missing.");
                await message("Cannot reveal item: Path is missing.", { title: 'Error', type: 'error' });
                return;
            }
            try {
                console.log(`[DataLeftPanel] Invoking reveal_in_file_explorer for: ${itemPathForClosure}`);
                await invoke('reveal_in_file_explorer', { filePathStr: itemPathForClosure });
            } catch (err) {
                console.error(`[DataLeftPanel] Error revealing item ${itemPathForClosure}:`, err);
                await message(`Could not reveal item: ${err}`, { title: 'Error', type: 'error' });
            }
            return;
        }

        if (itemType === 'media') { 
            switch (action) {
                case 'Open': 
                    dispatch('requestviewchange', { viewType: 'media_note', itemPath: item.path });
                    break;
                case 'Rename': itemToRename = { path: item.path, name: item.media_xml_identifier, file_type: 'media', media_xml_identifier: item.media_xml_identifier }; showRenameModal = true; break;
                case 'Delete': const stemName = item.media_xml_identifier || (item.name.includes('.') ? item.name.substring(0, item.name.lastIndexOf('.')) : item.name); const confirmMsg = `Delete media "${stemName}"? This deletes the entire folder (media, transcripts, data). Cannot be undone.`; const options = { title: 'Confirm Media Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmMsg, options); if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${stemName}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[DataLeftPanel] Delete failed for ${stemName}:`, err); } } else { project.update(p => ({ ...p, statusMessage: 'Deletion cancelled.' })); } } catch (e) { console.error("[DataLeftPanel] Error confirm/delete:", e); await message(`Error deleting: ${e}`, {title: "Delete Error", type: "error"}); } break;
                case 'Transcribe': if (!item.path) { await message("Cannot transcribe: path unknown.", { title: "Error", type: "error"}); break; } dispatch('requestTranscriptionTabWithMediaAndDialog', { mediaPath: item.path }); break;
                default: console.warn(`[DataLeftPanel] Unknown action for media: ${action}`);
            }
        } else if (itemType === 'doc') {
            switch (action) {
                case 'Open': if (isPdf) { try { await openerPlugin.openPath(item.path); } catch (e) { await message(`Could not open PDF externally: ${e}`, { title: 'Open Error', type: 'error'}); } } else { dispatch('requestviewchange', { viewType: 'documents', itemPath: item.path }); } break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'doc', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmDocMsg = `Delete document "${item.name}"? Cannot be undone.`; const docOptions = { title: 'Confirm Document Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmDocMsg, docOptions); if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[DataLeftPanel] Delete failed for ${item.name}:`, err); } } else { project.update(p => ({ ...p, statusMessage: 'Deletion cancelled.' })); } } catch (e) { await message(`Error deleting: ${e}`, {title: "Delete Error", type: "error"}); } break;
                default: console.warn(`[DataLeftPanel] Unknown action for document: ${action}`);
            }
        } else if (itemType === 'table') {
             switch (action) {
                case 'Open': dispatch('requestviewchange', { viewType: 'tables', itemPath: item.path }); break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'table', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmTableMsg = `Delete table "${item.name}"? This cannot be undone.`; const tableOptions = { title: 'Confirm Table Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmTableMsg, tableOptions); if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { await message(`Error deleting table: ${err}`, { title: "Delete Error", type: "error" }); } } else { project.update(p => ({ ...p, statusMessage: 'Table deletion cancelled.' })); } } catch (e) { await message(`Error deleting table: ${e}`, { title: "Delete Error", type: "error" }); } break;
                default: console.warn(`[DataLeftPanel] Unknown action for table: ${action}`);
            }
        } else if (itemType === 'image') {
             switch (action) {
                case 'Open': dispatch('requestviewchange', { viewType: 'images', itemPath: item.path }); break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'image', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmImageMsg = `Delete image "${item.name}"? This cannot be undone.`; const imageOptions = { title: 'Confirm Image Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmImageMsg, imageOptions); if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { await message(`Error deleting image: ${err}`, { title: "Delete Error", type: "error" }); } } else { project.update(p => ({ ...p, statusMessage: 'Image deletion cancelled.' })); } } catch (e) { await message(`Error deleting image: ${e}`, { title: "Delete Error", type: "error" }); } break;
                default: console.warn(`[DataLeftPanel] Unknown action for image: ${action}`);
            }
        } else if (itemType === 'imported_transcript') { 
            switch (action) {
                case 'Open': 
                    dispatch('requestviewchange', { viewType: 'imported_transcript', itemPath: item.path }); 
                    break;
                case 'Import':
                    handleImportClick('imported_transcript');
                    break;
                case 'Rename': 
                    const nameWithoutExt = item.name.toLowerCase().endsWith('.json') ? item.name.slice(0, -5) : item.name;
                    itemToRename = { path: item.path, name: nameWithoutExt, file_type: 'imported_transcript', media_xml_identifier: null }; 
                    showRenameModal = true; 
                    break;
                case 'Delete':
                    const confirmTranscriptMsg = "Are you sure you want to delete this transcript? This cannot be undone.";
                    const transcriptOptions = { title: 'Confirm Transcript Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' };
                    try {
                        const confirmed = await confirm(confirmTranscriptMsg, transcriptOptions);
                        if (confirmed) {
                            try { await deleteImportedTranscript(item.path); project.update(p => ({ ...p, currentImportedTranscriptPath: null })); }
                            catch (err) { await message(`Error deleting transcript: ${err}`, { title: 'Delete Error', type: 'error' }); }
                        } else { project.update(p => ({ ...p, statusMessage: 'Transcript deletion cancelled.' })); }
                    } catch (e) { await message(`Error deleting transcript: ${e}`, { title: 'Delete Error', type: 'error' });}
                    break;
                                default: console.warn(`[DataLeftPanel] Unknown action for imported transcript: ${action}`);
            }
        }
    }
    let prevAutoOpenPath = null;
    let showImportTranscriptModal = false;
    let categoryContextMenuVisible = false;
    let categoryContextMenuX = 0;
    let categoryContextMenuY = 0;
    let categoryContextMenuType = null;
    let revealButtonLabel = 'Open File Location';

    // --- Group Sub-Menu State & Handlers (Step III.3 & III.4) ---
    // let projectGroups = []; // Replaced by $currentProjectGroupsList
    $: localProjectGroupsForSubMenu = $currentProjectGroupsList || []; // Use store for submenu
    let showGroupSubMenu = false;
    let groupSubMenuX = 0;
    let groupSubMenuY = 0;
    let groupSubMenuItem = null;
    let showCreateGroupModal = false; // For Step III.4
    let addToGroupHoverTimer = null; // For hover effect
    const FOLDER_PLUS_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-folder-plus mr-2" viewBox="0 0 16 16"><path d="m.5 3 .04.87a1.99 1.99 0 0 0-.342 1.311l.637 7A2 2 0 0 0 2.826 14H9v-1H2.826a1 1 0 0 1-.995-.91l-.637-7A1 1 0 0 1 2.19 4h11.62a1 1 0 0 1 .996 1.09L14.54 8h1.005l.256-2.819A2 2 0 0 0 13.81 3H9.828a2 2 0 0 1-1.414-.586l-.828-.828A2 2 0 0 0 6.172 1H2.5a2 2 0 0 0-2 2m5.672-1a1 1 0 0 1 .707.293L7.586 3H2.19c-.24 0-.47.042-.683.12L1.5 2.98A1 1 0 0 1 2.5 2h3.672z"/><path d="M13.5 10a.5.5 0 0 1 .5.5V12h1.5a.5.5 0 0 1 0 1H14v1.5a.5.5 0 0 1-1 0V13h-1.5a.5.5 0 0 1 0-1H13v-1.5a.5.5 0 0 1 .5-.5"/></svg>`;

    function handleCategoryContextMenu(event, categoryType) {
      event.preventDefault();
      event.stopPropagation();
      if (categoryContextMenuVisible) closeCategoryContextMenu();
      if (showGroupSubMenu) closeGroupSubMenu();

      categoryContextMenuType = categoryType;
      const rect = event.currentTarget.getBoundingClientRect();
      categoryContextMenuX = rect.right + 4;
      categoryContextMenuY = rect.top;
      categoryContextMenuVisible = true;
    }

    function closeCategoryContextMenu() {
      categoryContextMenuVisible = false;
      categoryContextMenuType = null;
    }

    function closeGroupSubMenu() {
        showGroupSubMenu = false;
    }

    function handleGroupSelected(group) {
        if (group && group.id) {
            console.log(`[NotesLeftPanel] Group selected: ${group.name} (ID: ${group.id})`);
            // setSelectedGroup action will clear other selections (docs, media)
            // and set selectedGroupId and selectedGroupData in the store.
            setSelectedGroup(group.id, group);
        } else {
            console.warn("[NotesLeftPanel] Attempted to select an invalid group:", group);
        }
    }

    async function handleImportClick(categoryType) {
        console.log(`[DataLeftPanel] Import clicked for: ${categoryType}`);
        const categoryInfo = CATEGORIES_BASE.find(c => c.type === categoryType);
        if (!categoryInfo || !categoryInfo.importEnabled) {
            message(`Import for ${categoryInfo?.name || 'this category'} not available.`, { title: 'Not Implemented', type: 'info' });
            return;
        }

        if (categoryType === 'video' || categoryType === 'audio') {
            if (categoryType === 'video' || categoryType === 'audio') {
        try { 
            await importMediaFile(categoryType);
        } catch (e) {
            console.error(`[DataLeftPanel] Error importMediaFile ${categoryType}:`, e); 
        }
    }
        } else if (categoryType === 'document') {
            try { await importDocumentFile(); } catch (e) { console.error(`[DataLeftPanel] Error importDocumentFile:`, e); }
        } else if (categoryType === 'table') {
            try { await importTableFile(); } catch (e) { console.error(`[DataLeftPanel] Error importTableFile:`, e); }
        } else if (categoryType === 'image') {
            try { await importImageFile(); } catch (e) { console.error(`[DataLeftPanel] Error importImageFile:`, e); }
        } else if (categoryType === 'imported_transcript') { 
            showImportTranscriptModal = true;
        } else {
            message(`Specific import for ${categoryInfo.name} not implemented.`, { title: 'Coming Soon', type: 'info' });
        }
    }

    async function handleRenameConfirm(event) {
        const { newName } = event.detail; 
        const item = itemToRename;
        if (!item || !newName || newName.trim() === '') { console.error("[DataLeftPanel] Rename failed: Invalid input."); showRenameModal = false; itemToRename = null; return; }
        const finalNewNameFromModal = newName.trim(); 
        showRenameModal = false;
        if (item.file_type === 'media') {
            const finalNewStemName = finalNewNameFromModal;
            const confirmRename = await confirm(`Rename media '${item.media_xml_identifier}' to '${finalNewStemName}'? Renames folder & primary transcript.`, { title: 'Confirm Media Rename', type: 'warning', okLabel: 'Rename', cancelLabel: 'Cancel' });
            if (!confirmRename) { itemToRename = null; return; }
            try { await renameProjectItem(item.path, finalNewStemName, item.file_type); } catch (err) { console.error(`[DataLeftPanel] Rename failed for ${item.media_xml_identifier}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'doc') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : '';
            if (!originalExtension) { await message(`Error: Original file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const allowedExts = ['.json', '.pdf', '.md', '.txt'];
            if (!allowedExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original file type '${originalExtension}' cannot be renamed via this interface.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;
            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[DataLeftPanel] Rename failed for ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'table') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : '';
            if (!originalExtension) { await message(`Error: Original table file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const allowedTableExts = ['.csv', '.xlsx'];
            if (!allowedTableExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original table file type '${originalExtension}' cannot be renamed like this.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;
            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[DataLeftPanel] Rename failed for table ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'image') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : '';
            if (!originalExtension) { await message(`Error: Original image file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const allowedImageExts = ['.jpg', '.jpeg', '.png', '.gif', '.bmp', '.webp', '.tiff'];
            if (!allowedImageExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original image file type '${originalExtension}' cannot be renamed like this.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;
            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[DataLeftPanel] Rename failed for image ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'imported_transcript') { 
            const nameForBackend = finalNewNameFromModal; 
            try { await renameProjectItem(item.path, nameForBackend, item.file_type); }
            catch (err) { console.error(`[DataLeftPanel] Rename failed for imported transcript ${item.name}:`, err); } 
            finally { itemToRename = null; }
        } else {
            console.warn("[DataLeftPanel] Rename unhandled type:", item.file_type);
            itemToRename = null;
        }
    }
    function handleRenameModalClose() { showRenameModal = false; itemToRename = null; }

    // Removed local fetchProjectGroups function and reactive call
    // projectGroups is now $currentProjectGroupsList

    onMount(async () => {
      // projectStore's loadProjectDataAndUpdateStore should call updateProjectGroupsList
      // So, direct call to fetchProjectGroups() or updateProjectGroupsList() might be redundant here
      // if $project.id change reliably triggers it in projectStore.
      // However, to ensure groups are available if project is already loaded,
      // we can check $currentProjectGroupsList or trigger an update if $project.id exists.
      if (get(project).id && get(currentProjectGroupsList).length === 0) {
        // This scenario is less likely if loadProjectDataAndUpdateStore works as intended
        // but can be a fallback. Or simply rely on projectStore to handle it.
        // For now, let's assume projectStore handles initial load.
      }

      try {
        const currentOs = await getOsType();
        if (currentOs === 'windows') revealButtonLabel = 'Reveal in Explorer';
        else if (currentOs === 'macos') revealButtonLabel = 'Reveal in Finder';
        else revealButtonLabel = 'Open File Location';
      } catch (e) { console.error("Error getting OS type:", e); }

      const listener = (event) => {
        // Close main context menu if open and click is outside
        const mainContextMenuElement = document.getElementById('notes-left-panel-context-menu');
        if (contextMenuVisible && mainContextMenuElement && !mainContextMenuElement.contains(event.target)) {
            closeContextMenu();
        }

        // Close category context menu if open and click is outside
        const categoryMenuElement = document.getElementById('notes-left-panel-category-context-menu'); // Corrected ID
        if (categoryContextMenuVisible && categoryMenuElement && !categoryMenuElement.contains(event.target)) {
            // Check if the click was on the category button itself, if so, don't close
            const categoryButton = event.target.closest(`[aria-controls="category-content-${categoryContextMenuType}"]`);
            if (!categoryButton) {
                 closeCategoryContextMenu();
            }
        }

        // Close group sub-menu if open and click is outside
        const groupSubMenuElement = document.getElementById('notes-left-panel-group-sub-menu');
        if (showGroupSubMenu && groupSubMenuElement && !groupSubMenuElement.contains(event.target)) {
            closeGroupSubMenu();
        }

        // Close group item context menu if open and click is outside
        const groupItemContextMenuElement = document.getElementById('notes-left-panel-group-item-context-menu');
        if (groupContextMenuVisible && groupItemContextMenuElement && !groupItemContextMenuElement.contains(event.target)) {
            closeGroupContextMenu();
        }
      };
      document.addEventListener('click', listener, { capture: true });

      return () => document.removeEventListener('click', listener, { capture: true });
    });

    function handleShowAddToGroupSubMenu(event, item) {
        if (addToGroupHoverTimer) clearTimeout(addToGroupHoverTimer);
        if (categoryContextMenuVisible) closeCategoryContextMenu();

        groupSubMenuItem = item;
        const buttonRect = event.currentTarget.getBoundingClientRect();

        // Position submenu to the right of the "Add to Group" button
        groupSubMenuX = buttonRect.right + 2;
        groupSubMenuY = buttonRect.top;

        const menuWidthEstimate = 160; // Approx width of the submenu
        const menuHeightEstimate = (localProjectGroupsForSubMenu.length * 28) + 40 + 10; // Estimate height based on items

        // Adjust if submenu goes off-screen
        if (groupSubMenuX + menuWidthEstimate > window.innerWidth) {
            groupSubMenuX = buttonRect.left - menuWidthEstimate -2; // Position to the left
        }
        if (groupSubMenuY + menuHeightEstimate > window.innerHeight) {
             groupSubMenuY = Math.max(5, window.innerHeight - menuHeightEstimate - 5); // Adjust vertically
        }

        showGroupSubMenu = true;
        // Do not close main context menu immediately, allow mouse to travel
        // closeContextMenu();
    }

    function handleLeaveAddToGroupButton() {
        if (addToGroupHoverTimer) clearTimeout(addToGroupHoverTimer);
        addToGroupHoverTimer = setTimeout(() => {
            if (showGroupSubMenu) { // Check if mouse is not over submenu
                 const subMenuEl = document.getElementById('data-left-panel-group-sub-menu');
                 if (subMenuEl && !subMenuEl.matches(':hover')) {
                    closeGroupSubMenu();
                 }
            }
        }, 200); // Adjust delay as needed
    }

    function handleEnterGroupSubMenu() {
        if (addToGroupHoverTimer) clearTimeout(addToGroupHoverTimer);
    }

    function handleLeaveGroupSubMenu() {
        closeGroupSubMenu();
    }

    async function handleItemClick(item) {
        if (item.file_type === 'doc' || item.file_type === 'table' || item.file_type === 'image' || item.file_type === 'imported_transcript' || item.file_type === 'media') { 
            let viewType = item.file_type; 
            if (item.file_type === 'doc') viewType = 'documents';
            else if (item.file_type === 'table') viewType = 'tables';
            else if (item.file_type === 'image') viewType = 'images';
            else if (item.file_type === 'media') viewType = 'media_note'; 
            dispatch('requestviewchange', { viewType, itemPath: item.path });
        }
    }

    function handleNewGroupClick() { // For Step III.4
        // groupSubMenuItem is set when the "Add to Group" submenu is opened,
        // and this "New group..." button is inside that submenu.
        // Thus, groupSubMenuItem should typically be set here.
        // The CreateGroupModal's fileToAdd prop will use groupSubMenuItem.
        // If groupSubMenuItem were null, fileToAdd would be null, which is acceptable for creating an empty group.

        if (!$project || !$project.id || String($project.id).trim() === "" || String($project.id) === "null") { // Changed to $project.id
            message('Project ID is not available from $project.id. Cannot create a new group. Please ensure the project is fully loaded.', { title: 'Error', type: 'error' }); // Updated message
            closeGroupSubMenu(); // Ensure submenu is closed
            return;
        }
        closeGroupSubMenu();
        showCreateGroupModal = true;
    }

    async function handleAddFileToExistingGroup(group) {
        if (!groupSubMenuItem || !$project || !$project.id) { // Changed from project_uuid to id
            await message('Cannot add to group: Missing item or project context (project ID).', { title: 'Error', type: 'error' }); // Updated message
            closeGroupSubMenu();
            return;
        }
        const relativePath = groupSubMenuItem.relativePath;
        if (!relativePath) {
            await message('Cannot add to group: Item relative path is missing.', { title: 'Error', type: 'error' });
            closeGroupSubMenu();
            return;
        }
        try {
            await invoke('add_file_to_existing_group', {
                projectId: $project.id, // Changed from project_uuid to id
                groupId: group.id,
                fileAssetRelativePath: relativePath
            });
            // Removed explicit success message dialog
            project.update(p => ({ ...p, statusMessage: `File ${groupSubMenuItem.name} added to group ${group.name}.` }));
            // Also trigger group content notification for the GroupDetailView to refresh if it's open
            // Assuming groupContentNotification is imported and group.id is the ID of the group modified
            // const { groupContentNotification } = await import('$lib/stores/projectStore.js');
            // groupContentNotification.set({ groupId: group.id, action: 'file_added', timestamp: Date.now() });

        } catch (err) {
            await message(`Failed to add file to group: ${err}`, { title: 'Error', type: 'error' });
        } finally {
            closeGroupSubMenu();
        }
    }

    const CATEGORIES_BASE = [
        { name: 'Audios', type: 'audio', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-music-note-beamed w-4 h-4" viewBox="0 0 16 16"><path d="M6 13c0 1.105-1.12 2-2.5 2S1 14.105 1 13s1.12-2 2.5-2 2.5.896 2.5 2m9-2c0 1.105-1.12 2-2.5 2s-2.5-.895-2.5-2 1.12-2 2.5-2 2.5.895 2.5 2"/><path fill-rule="evenodd" d="M14 11V2h1v9zM6 3v10H5V3z"/><path d="M5 2.905a1 1 0 0 1 .9-.995l8-.8a1 1 0 0 1 1.1.995V3L5 4z"/></svg>`, importEnabled: true },
        { name: 'Documents', type: 'document', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-files w-4 h-4" viewBox="0 0 16 16"><path d="M13 0H6a2 2 0 0 0-2 2 2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2 2 2 0 0 0 2-2V2a2 2 0 0 0-2-2m0 13V4a2 2 0 0 0-2-2H5a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1M3 4a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/></svg>`, importEnabled: true },
        { name: 'Images', type: 'image', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-images w-4 h-4" viewBox="0 0 16 16"><path d="M4.502 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3"/><path d="M14.002 13a2 2 0 0 1-2 2h-10a2 2 0 0 1-2-2V5A2 2 0 0 1 2 3a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v8a2 2 0 0 1-1.998 2M14 2H4a1 1 0 0 0-1 1h9.002a2 2 0 0 1 2 2v7A1 1 0 0 0 15 11V3a1 1 0 0 0-1-1M2.002 4a1 1 0 0 0-1 1v8l2.646-2.354a.5.5 0 0 1 .63-.062l2.66 1.773 3.71-3.71a.5.5 0 0 1 .577-.094l1.777 1.947V5a1 1 0 0 0-1-1z"/></svg>`, importEnabled: true },
        { name: 'Tables', type: 'table', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-table w-4 h-4" viewBox="0 0 16 16"><path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm15 2h-4v3h4zm0 4h-4v3h4zm0 4h-4v3h3a1 1 0 0 0 1-1zm-5 3v-3H6v3zm-5 0v-3H1v2a1 1 0 0 0 1 1zm-4-4h4V8H1zm0-4h4V4H1zm5-3v3h4V4zm4 4H6v3h4z"/></svg>`, importEnabled: true },
        { name: 'Transcripts', type: 'imported_transcript', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chat-square-text w-4 h-4" viewBox="0 0 16 16"><path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/><path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/></svg>`, importEnabled: true },
        { name: 'Videos', type: 'video', icon: `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-film w-4 h-4" viewBox="0 0 16 16"><path d="M0 1a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H1a1 1 0 0 1-1-1zm4 0v6h8V1zm8 8H4v6h8zM1 1v2h2V1zm2 3H1v2h2zM1 7v2h2V7zm2 3H1v2h2zm-2 3v2h2v-2zM15 1h-2v2h2zm-2 3v2h2V4zm2 3h-2v2h2zm-2 3v2h2v-2zm2 3h-2v2h2z"/></svg>`, importEnabled: true },
    ];
    const IMPORT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-file-earmark-plus w-4 h-4" viewBox="0 0 16 16"><path d="M8 6.5a.5.5 0 0 1 .5.5v1.5H10a.5.5 0 0 1 0 1H8.5V11a.5.5 0 0 1-1 0V9.5H6a.5.5 0 0 1 0-1h1.5V7a.5.5 0 0 1 .5-.5"/><path d="M14 4.5V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h5.5zm-3 0A1.5 1.5 0 0 1 9.5 3V1H4a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4.5z"/></svg>`;
    const CONTEXT_MENU_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-three-dots-vertical w-4 h-4" viewBox="0 0 16 16"><path d="M9.5 13a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0m0-5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/></svg>`;
    const AUDIO_EXTENSIONS = new Set(['mp3','wav','m4a','ogg','aac','flac']);
	const VIDEO_EXTENSIONS = new Set(['mp4','mov','avi','mkv','webm']);
    const IMAGE_EXTENSIONS = new Set(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff']);

	const CHEVRON_DOWN_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;
	const CHEVRON_RIGHT_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M8.22 5.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L11.94 10 8.22 6.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd" /></svg>`;

	let showRenameModal = false; let itemToRename = null; let contextMenuVisible = false; let contextMenuX = 0; let contextMenuY = 0; let contextMenuItem = null; let closeContextMenuListener = null;
    let categoryOpenState = {}; const LS_KEY_DATA_PANEL_STATE = 'harveyDataPanelCategoryState';

    // --- Group & Tooltip States (moved from reactive block) ---
    let tooltipVisible = false;
    let tooltipCategoryName = '';
    let tooltipFiles = [];
    let groupContextMenuVisible = false;
    let groupContextMenuX = 0;
    let groupContextMenuY = 0;
    let groupContextMenuItem = null;
    let closeGroupContextMenuListener = null;
    let showGroupRenameModal = false;
    let groupToRename = null;
    let tooltipX = 0;
    let tooltipY = 0;
    let activeCollapsedCategoryType = null;

    onMount(() => { const defaultState = {}; CATEGORIES_BASE.forEach(cat => { defaultState[cat.type] = true; }); try { const savedState = localStorage.getItem(LS_KEY_DATA_PANEL_STATE); if (savedState) { const parsedState = JSON.parse(savedState); categoryOpenState = { ...defaultState, ...parsedState }; } else { categoryOpenState = defaultState; } } catch (e) { console.error("[DataLeftPanel] Failed load category state:", e); categoryOpenState = defaultState; } });
    function toggleCategory(categoryType) { if (categoryOpenState.hasOwnProperty(categoryType)) { categoryOpenState[categoryType] = !categoryOpenState[categoryType]; categoryOpenState = categoryOpenState; } else { console.warn(`[DataLeftPanel] Toggle unknown category: ${categoryType}`); } }
    $: if (Object.keys(categoryOpenState).length > 0) { try { localStorage.setItem(LS_KEY_DATA_PANEL_STATE, JSON.stringify(categoryOpenState)); } catch (e) { console.error("[DataLeftPanel] Failed save category state:", e); } }

    $: displayCategories = (() => {
        const projectFilesTree = $project.files || [];
        const projectDocumentFiles = $project.documentFiles || [];
        const projectTableFiles = $project.tableFiles || [];
        const projectImageFiles = $project.imageFiles || [];
        const projectImportedTranscriptFiles = $project.importedTranscriptFiles || []; 

        let videos = [];
        let audios = [];

        let documents = projectDocumentFiles.map(docXml => {
            const fullPath = $project.baseDirectory ? `${$project.baseDirectory}/${docXml.relativePath}` : docXml.relativePath;
            return { name: docXml.name, path: fullPath, relativePath: docXml.relativePath, file_type: 'doc' };
        }).sort((a, b) => a.name.localeCompare(b.name));

        let tables = projectTableFiles.map(tableXml => {
            const fullPath = $project.baseDirectory ? `${$project.baseDirectory}/${tableXml.relativePath}` : tableXml.relativePath;
            return { name: tableXml.name, path: fullPath, relativePath: tableXml.relativePath, file_type: 'table' };
        }).sort((a, b) => a.name.localeCompare(b.name));

        let images = projectImageFiles.map(imageXml => {
            const fullPath = $project.baseDirectory ? `${$project.baseDirectory}/${imageXml.relativePath}` : imageXml.relativePath;
            const assetUrl = fullPath ? convertFileSrc(fullPath) : null;
            return { name: imageXml.name, path: fullPath, relativePath: imageXml.relativePath, file_type: 'image', assetUrl };
        }).sort((a, b) => a.name.localeCompare(b.name));

        let importedTranscripts = projectImportedTranscriptFiles.map(tsXml => {
            const fullPath = $project.baseDirectory ? `${$project.baseDirectory}/${tsXml.relativePath}` : tsXml.relativePath;
            return { name: tsXml.name, path: fullPath, relativePath: tsXml.relativePath, file_type: 'imported_transcript' };
        }).sort((a,b) => a.name.localeCompare(b.name));


        function findMediaFilesRecursive(nodes) {
            if (!Array.isArray(nodes)) return;
            for (const node of nodes) {
                if (node.file_type === 'media' && !node.is_directory && node.path) {
                    const ext = node.name.split('.').pop()?.toLowerCase() ?? '';
                    const relativePath = node.relativePath || ($project.baseDirectory && node.path.startsWith($project.baseDirectory) ? node.path.substring($project.baseDirectory.length + 1) : node.path);
                    const mediaData = { name: node.name, path: node.path, relativePath: relativePath.replace(/\\/g, '/'), media_xml_identifier: node.media_xml_identifier || '', file_type: 'media' };
                    if (VIDEO_EXTENSIONS.has(ext)) { videos.push(mediaData); }
                    else if (AUDIO_EXTENSIONS.has(ext)) { audios.push(mediaData); }
                }
                if (node.children && node.children.length > 0) {
                    findMediaFilesRecursive(node.children);
                }
            }
        }
        findMediaFilesRecursive(projectFilesTree);
        videos.sort((a, b) => a.name.localeCompare(b.name));
        audios.sort((a, b) => a.name.localeCompare(b.name));

        return CATEGORIES_BASE.map(cat => {
            if (cat.type === 'video') { return { ...cat, files: videos }; }
            else if (cat.type === 'audio') { return { ...cat, files: audios }; }
            else if (cat.type === 'document') { return { ...cat, files: documents }; }
            else if (cat.type === 'table') { return { ...cat, files: tables }; }
            else if (cat.type === 'image') { return { ...cat, files: images }; }
            else if (cat.type === 'imported_transcript') { return { ...cat, files: importedTranscripts }; } 
            else { return { ...cat, files: [] }; }
        });
    })();

    $: filteredCategories = (() => {
      const q = $searchQuery.trim().toLowerCase();
      if (!$showSearchBox || q === '') return displayCategories;
      return displayCategories.map(cat => ({
        ...cat,
        files: cat.files.filter(file => file.name.toLowerCase().includes(q)),
      }));
    })();

    $: selectedItemPathInStore = $project.selectedMediaNotePath || $project.selectedDocumentPath || $project.selectedTablePath || $project.selectedImagePath || $project.currentImportedTranscriptPath || null;

    $: { 
        let autoPath = null;
        if ($project.selectedMediaNotePath) {
            autoPath = $project.selectedMediaNotePath;
        } else if ($project.selectedDocumentPath) {
            autoPath = $project.selectedDocumentPath;
        } else if ($project.currentImportedTranscriptPath) {
            autoPath = $project.currentImportedTranscriptPath;
        }

        if (autoPath && autoPath !== prevAutoOpenPath) {
            const lowerPath = autoPath.toLowerCase();
            let itemCategoryType = null;
            const extension = lowerPath.split('.').pop();

            if (AUDIO_EXTENSIONS.has(extension) || VIDEO_EXTENSIONS.has(extension)) {
                itemCategoryType = 'media_note'; 
            } else if ($project.importedTranscriptFiles?.some(f => f.relativePath && `${$project.baseDirectory}/${f.relativePath}` === autoPath)) {
                itemCategoryType = 'imported_transcript';
            } else if (lowerPath.endsWith('.pdf') || (lowerPath.endsWith('.json') && !itemCategoryType) || lowerPath.endsWith('.txt') || lowerPath.endsWith('.md')) {
                itemCategoryType = 'document';
            } else if (lowerPath.endsWith('.csv') || lowerPath.endsWith('.xlsx')) {
                itemCategoryType = 'table';
            } else if (IMAGE_EXTENSIONS.has(extension)) {
                itemCategoryType = 'image';
            }

            if (itemCategoryType) {
                const itemCategory = displayCategories.find(c => {
                            if (itemCategoryType === 'media_note') {
                                return (c.type === 'audio' || c.type === 'video');
                            }
                            return c.type === itemCategoryType;
                        });

                         if (itemCategory && itemCategory.files.some(f => f.path === autoPath)) {
                            console.log(`[DataLeftPanel] Auto open path ${autoPath} matches category ${itemCategoryType}. NotesView will handle it.`);
                            prevAutoOpenPath = autoPath;
                        } else {
                            console.warn(`[DataLeftPanel] Auto open path ${autoPath} (type ${itemCategoryType}) NOT FOUND in current displayCategories.`);
                        }
                    }
                }

            async function handleImportTranscriptConfirm(event) {
                const { sourceType } = event.detail;
                showImportTranscriptModal = false;
                if (sourceType === 'msWord') {
                    try {
                        await importTranscriptFile(sourceType); 
                    } catch (e) {
                        console.error(`[DataLeftPanel] Error importTranscriptFile (msWord):`, e);
                    }
                } else {
                    console.warn(`[DataLeftPanel] Unknown transcript import source type: ${sourceType}`);
                    await message(`Import from "${sourceType}" is not supported.`, { title: 'Import Error', type: 'error' });
                }
            }

            

            

            

            function handleSearchClick(event) {
              event.stopPropagation();
              showSearchBox.set(true);
              setTimeout(() => document.getElementById('data-search-input')?.focus(), 0);
            }

            function handleSearchClear(event) {
              event.stopPropagation();
              searchQuery.set('');
              setTimeout(() => document.getElementById('data-search-input')?.focus(), 0);
            }

            onMount(() => {
              const listener = (e) => {
                const input = document.getElementById('data-search-input');
                if (!$showSearchBox) return;
                if (input && (e.target === input || input.contains(e.target))) return;
                if ($searchQuery.trim() === '') {
                  showSearchBox.set(false);
                }
              };
              document.addEventListener('click', listener);
              return () => document.removeEventListener('click', listener);
            });

            function showTooltip(event, category) {
                const buttonRect = event.currentTarget.getBoundingClientRect();
                const fullCategoryData = filteredCategories.find(fc => fc.type === category.type);
                tooltipCategoryName = category.name;
                tooltipFiles = fullCategoryData ? fullCategoryData.files || [] : [];
                tooltipX = buttonRect.right + 8;
                tooltipY = buttonRect.top;
                tooltipVisible = true;
            }

            function hideTooltip() {
                tooltipVisible = false;
            }

        {
            if (selectedItemPathInStore && $project.baseDirectory) {
                const path = selectedItemPathInStore;
                const extension = path.split('.').pop()?.toLowerCase() || '';
                let determinedItemType = null;

                if (AUDIO_EXTENSIONS.has(extension)) determinedItemType = 'audio';
                else if (VIDEO_EXTENSIONS.has(extension)) determinedItemType = 'video';
                else {
                    const TABLE_EXTENSIONS = new Set(['csv', 'xlsx']);
                    const DOC_JSON_EXTENSIONS = new Set(['json']);
                    const TRANSCRIPT_EXTENSIONS = new Set(['json']);
                    const projectFileListsForOthers = [
                        { files: $project.importedTranscriptFiles, type: 'imported_transcript', isRelative: true, exts: TRANSCRIPT_EXTENSIONS },
                        { files: $project.imageFiles, type: 'image', isRelative: true, exts: IMAGE_EXTENSIONS },
                        { files: $project.tableFiles, type: 'table', isRelative: true, exts: TABLE_EXTENSIONS },
                        { files: $project.documentFiles, type: 'document', isRelative: true, exts: new Set(['pdf', 'txt', 'md', ...DOC_JSON_EXTENSIONS]) }
                    ];
                    for (const listInfo of projectFileListsForOthers) {
                        if (listInfo.files?.some(f => {
                            const filePathToCheck = listInfo.isRelative ? `${$project.baseDirectory}/${f.relativePath}` : f.path;
                            return filePathToCheck === path && (listInfo.exts ? listInfo.exts.has(extension) : true);
                        })) {
                            determinedItemType = listInfo.type;
                            break;
                        }
                    }
                    if (!determinedItemType) {
                        if (extension === 'pdf' || extension === 'txt' || extension === 'md') determinedItemType = 'document';
                        else if (DOC_JSON_EXTENSIONS.has(extension)) determinedItemType = 'document';
                    }
                }
                activeCollapsedCategoryType = determinedItemType;
            } else {
                activeCollapsedCategoryType = null;
            }
        }
    }
</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow flex flex-col overflow-hidden">
	<h2 class="relative flex items-center text-sm font-semibold text-gray-700 dark:text-gray-300 px-1 h-9 border-b border-gray-200 dark:border-gray-600"
        class:mb-3={!$panelStateStore.dataLeftPanelCollapsed}
        class:mb-0={$panelStateStore.dataLeftPanelCollapsed}
        class:justify-between={!$panelStateStore.dataLeftPanelCollapsed && !$showSearchBox}
        class:justify-center={$panelStateStore.dataLeftPanelCollapsed || $showSearchBox}>
    {#if !$showSearchBox}
        <div class="flex items-center space-x-2">
            <button
                type="button"
                class="p-1 text-gray-600 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 z-20"
                on:click={handleToggleDataLeftPanel}
                                title={$panelStateStore.dataLeftPanelCollapsed ? 'Expand Data Panel' : 'Collapse Data Panel'}
            >
                {@html JOURNAL_ICON_SVG}
            </button>
            {#if !$panelStateStore.dataLeftPanelCollapsed}
                <span>Data</span>
            {/if}
        </div>
    {/if}
        {#if !$panelStateStore.dataLeftPanelCollapsed}
            {#if !showSearchBox}
            <button
                type="button"
                class="absolute inset-y-0 right-0 p-1 flex items-center justify-center z-20 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
                on:click|stopPropagation={handleSearchClick}
                title="Search Data"
            >
                {@html `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16"><path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001q.044.06.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1 1 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0"/></svg>`}
            </button>
            {:else}
            {#if $searchQuery.trim() !== ''}
                <button
                type="button"
                class="absolute inset-y-0 right-0 p-1 flex items-center justify-center z-20 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
                on:click|stopPropagation={handleSearchClear}
                title="Clear Search"
                >
                {@html `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x" viewBox="0 0 16 16"><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/></svg>`}
                </button>
            {/if}
            {/if}
            <input
            id="data-search-input"
            bind:value={$searchQuery}
            type="text"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck="false"
            placeholder="Search..."
            class="absolute inset-y-0 left-0 right-0 z-10 transition-all duration-300 ease-out border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-sm {$showSearchBox ? 'opacity-100 w-full pl-2 pr-10 py-0.5' : 'opacity-0 w-0 pl-12 pr-10 py-0.5'}"
            />
        {/if}
	</h2>

            {#if !$panelStateStore.dataLeftPanelCollapsed}
    <div class="flex flex-col flex-grow overflow-hidden">
        <!-- Top 2/3 for Categories -->
        <div class="flex-grow overflow-y-auto min-h-0 px-2" style="flex-basis: 66.66%;">
            <ul class="space-y-2 text-xs">
                {#each filteredCategories as category (category.type)}
                    <li>
                        <div
                            class="flex items-center justify-between group mb-1 pr-1 py-1 cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-600 rounded {categoryContextMenuVisible && categoryContextMenuType === category.type ? 'bg-gray-100 dark:bg-gray-600' : ''}"
                            on:click={() => toggleCategory(category.type)} role="button" aria-expanded={categoryOpenState[category.type] ?? true} aria-controls={`category-content-${category.type}`} tabindex="0" on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleCategory(category.type); }}>
                            <div class="flex items-center space-x-1.5 text-gray-600 dark:text-gray-400">
                                <span class="flex-shrink-0 w-4 h-4 flex items-center justify-center"> {@html categoryOpenState[category.type] ? CHEVRON_DOWN_SVG : CHEVRON_RIGHT_SVG} </span>
                                <span class="flex-shrink-0">{@html category.icon}</span>
                                <span class="font-medium text-gray-700 dark:text-gray-300">{category.name}</span>
                            </div>
                            <button
                              type="button"
                              class="ml-2 flex-shrink-0 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity {categoryContextMenuVisible && categoryContextMenuType === category.type ? 'opacity-100' : ''}"
                              title="Options"
                              on:click|stopPropagation={(e) => handleCategoryContextMenu(e, category.type)}
                              disabled={!category.importEnabled}
                            >
                              {@html CONTEXT_MENU_ICON_SVG}
                            </button>
                        </div>

                        {#if categoryOpenState[category.type]}
                            <div id={`category-content-${category.type}`} role="region">
                                {#if (category.type === 'video' || category.type === 'audio' || category.type === 'document' || category.type === 'table' || category.type === 'image' || category.type === 'imported_transcript') && category.files.length > 0}
                                    <ul class="ml-2 space-y-0.5 border-l border-gray-200 dark:border-gray-600">
                                        {#each category.files as fileItem (fileItem.path || fileItem.relativePath)}
                                            <li class="group">
                                                <div class="flex items-center justify-between w-full rounded px-1.5 py-1 text-left {fileItem.path === selectedItemPathInStore ? 'bg-blue-50 dark:bg-blue-900' : 'hover:bg-gray-100 dark:hover:bg-gray-700'}" title="{fileItem.path || fileItem.relativePath}" role="button" tabindex="0"
                                                     on:click={() => handleItemClick(fileItem) }
                                                     on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleItemClick(fileItem); }}>
                                                    <span class="flex items-center space-x-1 {fileItem.path === selectedItemPathInStore ? 'text-blue-600 dark:text-blue-300' : 'text-gray-800 dark:text-gray-200'} truncate">
                                                        <span>{fileItem.name}</span>
                                                    </span>
                                                    <button type="button" class="ml-2 flex-shrink-0 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity" title="Options for {fileItem.name}" on:click|stopPropagation={(e) => handleItemContextMenu(e, fileItem)}> {@html CONTEXT_MENU_ICON_SVG} </button>
                                                </div>
                                            </li>
                                        {/each}
                                    </ul>
                                {:else if (category.type === 'video' || category.type === 'audio' || category.type === 'document' || category.type === 'table' || category.type === 'image' || category.type === 'imported_transcript')}
                                    <p class="ml-9 text-xs text-gray-400 dark:text-gray-500 italic py-1">No {category.name.toLowerCase()} found.</p>
                                {:else}
                                     <p class="ml-9 text-xs text-gray-400 dark:text-gray-500 italic py-1">No files in this category.</p>
                                {/if}
                             </div>
                        {/if}
                    </li>
                    <!-- {#if category.type !== 'Videos'} <hr class="border-gray-200 dark:border-gray-700 my-1"> {/if} -->
                {/each}
            </ul>
            {#if $project.isLoading} <p class="text-xs text-gray-500 dark:text-gray-400 italic px-1 py-2">Loading project data...</p> {/if}
        </div>

        <!-- Separator 1: Below Data section -->
        <hr class="border-gray-200 dark:border-gray-600 my-2 mx-0">

        <!-- Bottom 1/3 for Groups -->
        <div class="flex-grow overflow-y-auto min-h-0 px-2 pt-2" style="flex-basis: 33.33%;">
            <h3 class="flex items-center text-xs font-semibold text-gray-500 dark:text-gray-400 px-1 mb-1.5">
                <span class="mr-1.5 flex-shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-collection w-3.5 h-3.5" viewBox="0 0 16 16">
                        <path d="M2.5 3.5a.5.5 0 0 1 0-1h11a.5.5 0 0 1 0 1zm2-2a.5.5 0 0 1 0-1h7a.5.5 0 0 1 0 1zM0 13a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 16 13V6a1.5 1.5 0 0 0-1.5-1.5h-13A1.5 1.5 0 0 0 0 6zm1.5.5A.5.5 0 0 1 1 13V6a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 .5.5v7a.5.5 0 0 1-.5.5z"/>
                    </svg>
                </span>
                Groups
            </h3>
            <ul class="ml-2 space-y-0.5 border-l border-gray-200 dark:border-gray-600 text-xs">
                {#if $currentProjectGroupsList && $currentProjectGroupsList.length > 0}
                    {#each $currentProjectGroupsList as group (group.id)}
                        <li class="group">
                            <div
                                class="flex items-center justify-between w-full rounded px-1.5 py-1 text-left hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
                                class:bg-blue-100={$project.selectedGroupId === group.id}
                                class:dark:bg-blue-800={$project.selectedGroupId === group.id}
                                on:click={() => handleGroupSelected(group)}
                                role="button"
                                tabindex="0"
                                on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleGroupSelected(group); }}
                                title={group.name}
                            >
                                <span class="flex items-center text-gray-800 dark:text-gray-200 truncate"
                                    class:!text-blue-700={$project.selectedGroupId === group.id}
                                    class:dark:!text-blue-200={$project.selectedGroupId === group.id}
                                >
                                    <!-- Icon span removed -->
                                    <span>{group.name}</span>
                                </span>
                                <button
                                    type="button"
                                    class="ml-2 flex-shrink-0 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity"
                                    title="Options for {group.name}"
                                    on:click|stopPropagation={(e) => handleGroupItemContextMenu(e, group)}
                                >
                                    {@html CONTEXT_MENU_ICON_SVG}
                                </button>
                            </div>
                        </li>
                    {/each}
                {:else}
                    <p class="ml-2.5 text-xs text-gray-400 dark:text-gray-500 italic py-1">No groups created yet.</p>
                {/if}
            </ul>
        </div>
    </div>
{:else}
    <!-- Collapsed Content (Vertical Icons) -->
    <div class="flex flex-col items-center space-y-2 pt-2 flex-grow overflow-y-auto min-h-0">
        {#each CATEGORIES_BASE as category (category.type)}
            <button
                type="button"
                class="p-1.5 rounded-md focus:outline-none dark:focus:ring-offset-gray-800 focus:ring-offset-1"
                class:hover:bg-gray-200={category.type !== activeCollapsedCategoryType}
                class:dark:hover:bg-gray-700={category.type !== activeCollapsedCategoryType}
                class:focus:ring-2={category.type !== activeCollapsedCategoryType}
                class:focus:ring-blue-500={category.type !== activeCollapsedCategoryType}
                class:bg-blue-200={category.type === activeCollapsedCategoryType}
                class:dark:bg-blue-700={category.type === activeCollapsedCategoryType}
                class:text-gray-500={category.type !== activeCollapsedCategoryType}
                class:dark:text-gray-400={category.type !== activeCollapsedCategoryType}
                class:text-blue-600={category.type === activeCollapsedCategoryType}
                class:dark:text-blue-400={category.type === activeCollapsedCategoryType}
                class:hover:bg-blue-300={category.type === activeCollapsedCategoryType}
                class:dark:hover:bg-blue-600={category.type === activeCollapsedCategoryType}
                on:click={handleToggleDataLeftPanel}
                on:mouseenter={(event) => showTooltip(event, category)}
                on:mouseleave={hideTooltip}
                on:focus={(event) => showTooltip(event, category)}
                on:blur={hideTooltip}
            >
                {@html category.icon}
            </button>
        {/each}
    </div>
{/if}

				{#if contextMenuVisible && contextMenuItem && !$panelStateStore.dataLeftPanelCollapsed}
		<div id="notes-left-panel-context-menu" class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[150px]" style="left: {contextMenuX}px; top: {contextMenuY}px;" on:click|stopPropagation>
            {#if contextMenuItem.file_type === 'media'}
                <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Transcribe'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Transcribe</button>
                <button
                    on:mouseenter={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:mouseleave={handleLeaveAddToGroupButton}
                    on:focus={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:click|stopPropagation={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    class="flex items-center justify-between w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">
                    Add to Group
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right w-3 h-3" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
                </button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Reveal'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealButtonLabel}</button>
                <hr class="my-1 border-gray-200 dark:border-gray-600" />
                <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'doc'}
                 {#if contextMenuItem.name?.toLowerCase().endsWith('.pdf')}
                     <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open Externally</button>
                 {:else}
                     <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 {/if}
                <button
                    on:mouseenter={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:mouseleave={handleLeaveAddToGroupButton}
                    on:focus={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:click|stopPropagation={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    class="flex items-center justify-between w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">
                    Add to Group
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right w-3 h-3" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
                </button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Reveal'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealButtonLabel}</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'table'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                <button
                    on:mouseenter={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:mouseleave={handleLeaveAddToGroupButton}
                    on:focus={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:click|stopPropagation={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    class="flex items-center justify-between w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">
                    Add to Group
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right w-3 h-3" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
                </button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Reveal'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealButtonLabel}</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'image'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                <button
                    on:mouseenter={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:mouseleave={handleLeaveAddToGroupButton}
                    on:focus={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:click|stopPropagation={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    class="flex items-center justify-between w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">
                    Add to Group
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right w-3 h-3" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
                </button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Reveal'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealButtonLabel}</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'imported_transcript'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Import'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Import</button>
                <button
                    on:mouseenter={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:mouseleave={handleLeaveAddToGroupButton}
                    on:focus={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    on:click|stopPropagation={(e) => { handleShowAddToGroupSubMenu(e, contextMenuItem); }}
                    class="flex items-center justify-between w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">
                    Add to Group
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-right w-3 h-3" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708"/></svg>
                </button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Reveal'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealButtonLabel}</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else}
                 <span class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic">No actions available</span>
            {/if}
		</div>
	{/if}

    {#if showGroupSubMenu && groupSubMenuItem}
        <div
            id="notes-left-panel-group-sub-menu"
            class="fixed z-[51] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[150px]"
            style="left: {groupSubMenuX}px; top: {groupSubMenuY}px;"
            on:mouseenter={handleEnterGroupSubMenu}
            on:mouseleave={handleLeaveGroupSubMenu}
            on:click|stopPropagation>
            <button on:click|stopPropagation={() => { handleNewGroupClick(); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">New group...</button>
            <hr class="my-1 border-gray-200 dark:border-gray-600" />
            {#if localProjectGroupsForSubMenu && localProjectGroupsForSubMenu.length > 0}
                {#each localProjectGroupsForSubMenu as group (group.id)}
                    <button on:click|stopPropagation={() => { handleAddFileToExistingGroup(group); }} class="flex items-center w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200 truncate" title="{group.name} {group.description ? '(' + group.description + ')' : ''}">
                        <!-- Removed folder icon from here -->
                        <span class="ml-1">{group.name}</span> <!-- Added ml-1 for slight indent if needed, or adjust as per design -->
                    </button>
                {/each}
            {:else}
                <span class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic">No existing groups</span>
            {/if}
        </div>
    {/if}

        {#if categoryContextMenuVisible && !$panelStateStore.dataLeftPanelCollapsed}
      <div
        id="notes-left-panel-category-context-menu"
        class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[120px]"
        style="left: {categoryContextMenuX}px; top: {categoryContextMenuY}px;"
        on:click|stopPropagation
      >
        {#if categoryContextMenuType === 'document' || categoryContextMenuType === 'table'}
          <button
            on:click|stopPropagation={() => {}}
            class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
            title="Create New"
          >
            Create New
          </button>
        {/if}
        <button
          on:click|stopPropagation={() => { handleImportClick(categoryContextMenuType); closeCategoryContextMenu(); }}
          class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200"
          disabled={!CATEGORIES_BASE.find(c => c.type === categoryContextMenuType)?.importEnabled}
          title="Import {CATEGORIES_BASE.find(c => c.type === categoryContextMenuType)?.name}"
        >
          Import {CATEGORIES_BASE.find(c => c.type === categoryContextMenuType)?.name}
        </button>
      </div>
    {/if}
</div>

<CategoryTooltip
    visible={tooltipVisible}
    categoryName={tooltipCategoryName}
    files={tooltipFiles}
    x={tooltipX}
    y={tooltipY}
/>

<FileRenameModal bind:showModal={showRenameModal} currentName="{itemToRename?.name || ''}" itemType="{itemToRename?.file_type || ''}" isMediaRename="{itemToRename?.file_type === 'media'}" on:confirm={handleRenameConfirm} on:close={handleRenameModalClose} />
<ImportTranscriptSourceModal bind:showModal={showImportTranscriptModal} on:confirm={handleImportTranscriptConfirm} on:close={() => showImportTranscriptModal = false} />
<CreateGroupModal
    bind:showModal={showCreateGroupModal}
    projectUuid={$project?.id}
    fileToAdd={groupSubMenuItem}
    on:close={() => { showCreateGroupModal = false; groupSubMenuItem = null; }}
    on:groupCreatedAndFileAdded={(event) => {
        console.log('[NotesLeftPanel] Event: groupCreatedAndFileAdded', event.detail);
        showCreateGroupModal = false;
        groupSubMenuItem = null;
        project.update(p => ({ ...p, statusMessage: `File ${event.detail.file.name} added to new group ${event.detail.group.name}.` }));
        // Notify GroupDetailView if it's open for the new group
        // const { groupContentNotification } = import('$lib/stores/projectStore.js'); // Line 1114: problematic import
        // groupContentNotification.set({ groupId: event.detail.group.id, action: 'file_added', timestamp: Date.now() }); // Line 1115: removed .set call
    }}
    on:groupCreated={(event) => {
        console.log('[NotesLeftPanel] Event: groupCreated', event.detail);
        showCreateGroupModal = false;
        groupSubMenuItem = null;
        project.update(p => ({ ...p, statusMessage: `Group ${event.detail.group.name} created.` }));
    }}
/>

<GroupRenameModal
    bind:showModal={showGroupRenameModal}
    groupData={groupToRename}
    on:close={() => {
        showGroupRenameModal = false;
        groupToRename = null;
    }}
    on:save={async (event) => {
        const { groupId, newName, newDescription } = event.detail;
        const currentProjectId = get(project).id;
        const oldGroupName = groupToRename?.name || 'the group'; // For messages

        if (!currentProjectId || !groupId) {
            message('Project or Group ID is missing. Cannot rename.', { title: 'Error', type: 'error' });
            showGroupRenameModal = false;
            groupToRename = null;
            return;
        }

        try {
            // Backend command to be implemented: rename_project_group
            // It should return the updated GroupData object
            const updatedGroupData = await invoke('rename_project_group', {
                projectId: currentProjectId,
                groupId: groupId,
                newName: newName,
                newDescription: newDescription
            });

            await updateProjectGroupsList(currentProjectId); // Refresh the list

            // If the renamed group is currently selected, update its data in the project store
            if (get(project).selectedGroupId === groupId) {
                project.update(p => ({
                    ...p,
                    selectedGroupData: updatedGroupData, // Use data returned from backend
                    statusMessage: `Group '${oldGroupName}' renamed to '${newName}'.`
                }));
            } else {
                project.update(p => ({ ...p, statusMessage: `Group '${oldGroupName}' renamed to '${newName}'.` }));
            }
             // Also trigger groupContentNotification for the GroupDetailView to refresh if it's open for this group
            const { groupContentNotification } = await import('$lib/stores/projectStore.js');
            groupContentNotification.set({ groupId: groupId, action: 'details_updated', timestamp: Date.now() });


        } catch (err) {
            console.error(`[NotesLeftPanel] Error renaming group ${groupId}:`, err);
            await message(`Failed to rename group '${oldGroupName}': ${err}`, { title: 'Rename Error', type: 'error' });
            project.update(p => ({ ...p, statusMessage: `Failed to rename group '${oldGroupName}'.` }));
        } finally {
            showGroupRenameModal = false;
            groupToRename = null;
        }
    }}
/>

<!-- Group Item Context Menu -->
{#if groupContextMenuVisible && groupContextMenuItem}
    <div
        id="notes-left-panel-group-item-context-menu"
        class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[120px]"
        style="left: {groupContextMenuX}px; top: {groupContextMenuY}px;"
        on:click|stopPropagation
    >
        <button on:click={() => handleGroupContextMenuAction('Open')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
        <button on:click={() => handleGroupContextMenuAction('Rename')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename...</button>
        <hr class="my-1 border-gray-200 dark:border-gray-600" />
        <button on:click={() => handleGroupContextMenuAction('Delete')} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete...</button>
    </div>
{/if}

<style lang="postcss">
	.overflow-y-auto::-webkit-scrollbar { @apply w-[6px] h-[6px]; }
	.overflow-y-auto::-webkit-scrollbar-track { @apply bg-transparent; }
	.overflow-y-auto::-webkit-scrollbar-thumb { @apply rounded bg-gray-400/50 dark:bg-gray-500/50; }
	.overflow-y-auto::-webkit-scrollbar-thumb:hover { @apply bg-gray-500/70 dark:bg-gray-400/70; }
	.overflow-y-auto { scrollbar-width: thin; scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
	:root { --scrollbar-thumb: rgba(156, 163, 175, 0.5); --scrollbar-track: transparent; }
	html.dark { --scrollbar-thumb: rgba(107, 114, 128, 0.5); }
	.min-h-0 { min-height: 0; } .w-4 { width: 1rem; } .h-4 { height: 1rem; }
    .ml-9 { margin-left: 2.25rem; }
</style>
