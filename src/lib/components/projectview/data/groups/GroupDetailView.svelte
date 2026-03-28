<script>
    import { onMount, onDestroy } from 'svelte';
    import { project as projectStore, prepareDocumentView, prepareStandaloneTranscriptView, prepareMediaNoteView, updateProjectStoreState, setSelectedGroup, currentProjectGroupsList, updateProjectGroupsList, groupContentNotification } from '$lib/stores/projectStore.js';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { get, writable } from 'svelte/store';
    import { createEventDispatcher } from 'svelte';
    import { confirm, message } from '@tauri-apps/plugin-dialog';
    import { type as getOsType } from '@tauri-apps/plugin-os'; // Added for OS detection
    import EditGroupModal from '$lib/components/projectview/modals/EditGroupModal.svelte';
    import FileContextMenu from '$lib/components/projectview/shared/FileContextMenu.svelte';
    import CreateGroupModal from '$lib/components/projectview/modals/CreateGroupModal.svelte';
    import FileRenameModal from '$lib/components/projectview/modals/FileRenameModal.svelte';
    import { renameProjectItem, deleteProjectItem } from '$lib/services/projectService.js';
    import { Music, Film, FileText, Image as ImageIcon, Sheet, MessageSquareText, File, MoreHorizontal, MoreVertical, SquarePen, ChevronDown } from '@lucide/svelte';
    import DocumentThumbnail from './DocumentThumbnail.svelte';
    import TableThumbnail from './TableThumbnail.svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';
    import { Table, TableBody, TableBodyCell, TableBodyRow, TableHead, TableHeadCell, Search, Dropdown, Checkbox, Button } from 'flowbite-svelte';

    // Props
    export let groupData; // Expected: { id, name, description, project_id }

    const dispatch = createEventDispatcher();

    // "Add to Group" Submenu State
    let showAddToGroupSubMenu = false;
    let addToGroupSubMenuX = 0;
    let addToGroupSubMenuY = 0;
    let itemForAddToGroup = null;
    let projectGroupsForMenu = [];
    let showCreateGroupModalFromGroupView = false;
    let closeAddToGroupSubMenuListener = null;

    // Context Menu State
    let contextMenuVisible = false;
    let contextMenuItem = null;
    let contextMenuX = 0;
    let contextMenuY = 0;
    let closeContextMenuListener = null;

    // Rename Modal State
    let showRenameModal = false;
    let itemToRename = null;

    let revealButtonLabelGroupView = 'Open File Location'; // Default reveal label

    // Internal State
    let categorizedFiles = {
        audios: [],
        documents: [],
        images: [],
        tables: [],
        standalone_transcripts: [],
        videos: [],
        others: [] // For any files that don't fit predefined categories
    };
    let allFiles = []; // Flat list for table view
    let isLoading = false;
    let errorMessage = null;
    let isEditGroupModalOpen = false;

    // Table List View State
    let searchQuery = '';
    let sortKey = 'type';
    let sortDirection = 1;

    const LS_COLUMNS_KEY = 'harveyGroupListColumns';
    const defaultColumns = [
        { key: 'type', label: 'Type', visible: true, disabled: true },
        { key: 'name', label: 'File Name', visible: true, disabled: true },
        { key: 'title', label: 'Title', visible: false, disabled: false },
        { key: 'description', label: 'Description', visible: false, disabled: false },
        { key: 'createdAt', label: 'Created At', visible: true, disabled: false },
        { key: 'lastModified', label: 'Last Modified', visible: true, disabled: false }
    ];

    let columns = [...defaultColumns];

    if (typeof window !== 'undefined') {
        try {
            const savedColumns = localStorage.getItem(LS_COLUMNS_KEY);
            if (savedColumns) {
                const parsed = JSON.parse(savedColumns);
                // Merge saved visibility state into the default configuration to ensure all columns exist and correct labels/disabled states are preserved.
                columns = defaultColumns.map(defCol => {
                    const savedCol = parsed.find(c => c.key === defCol.key);
                    if (savedCol && !defCol.disabled) {
                        return { ...defCol, visible: savedCol.visible };
                    }
                    return defCol;
                });
            }
        } catch (e) {
            console.warn("[GroupDetailView] Failed to load column preferences:", e);
        }
    }

    // Reactively save to localStorage when columns change
    $: {
        if (typeof window !== 'undefined') {
            try {
                // Save only the key and visibility state to reduce storage payload
                const stateToSave = columns.map(c => ({ key: c.key, visible: c.visible }));
                localStorage.setItem(LS_COLUMNS_KEY, JSON.stringify(stateToSave));
            } catch (e) {
                console.warn("[GroupDetailView] Failed to save column preferences:", e);
            }
        }
    }

    $: visibleColumnsCount = columns.filter(c => c.visible).length;

    const CATEGORY_ORDER = [
        { key: 'audios', name: 'Audios', singularName: 'Audio', icon: Music },
        { key: 'documents', name: 'Documents', singularName: 'Document', icon: FileText },
        { key: 'images', name: 'Images', singularName: 'Image', icon: ImageIcon },
        { key: 'tables', name: 'Tables', singularName: 'Table', icon: Sheet },
        { key: 'standalone_transcripts', name: 'Transcripts', singularName: 'Transcript', icon: MessageSquareText },
        { key: 'videos', name: 'Videos', singularName: 'Video', icon: Film },
        { key: 'others', name: 'Others', singularName: 'Other', icon: File }
    ];

    function getCategoryInfo(fileType) {
        switch (fileType) {
            case 'audio': return CATEGORY_ORDER.find(c => c.key === 'audios');
            case 'video': return CATEGORY_ORDER.find(c => c.key === 'videos');
            case 'document': return CATEGORY_ORDER.find(c => c.key === 'documents');
            case 'image': return CATEGORY_ORDER.find(c => c.key === 'images');
            case 'table': return CATEGORY_ORDER.find(c => c.key === 'tables');
            case 'standalone_transcript': return CATEGORY_ORDER.find(c => c.key === 'standalone_transcripts');
            default: return CATEGORY_ORDER.find(c => c.key === 'others');
        }
    }

    async function fetchGroupContents() {
        // Use get(project) to access store values if outside reactive context or component markup
        const currentProject = get(projectStore);
        if (!groupData || !groupData.id || !currentProject || !currentProject.id || !currentProject.xmlPath) {
            errorMessage = "Group data or project context is missing.";
            console.error("fetchGroupContents precondition failed:", { groupData, currentProject });
            return;
        }
        isLoading = true;
        errorMessage = null;
        try {
            const files = await invoke('get_group_contents', {
                projectXmlPathStr: currentProject.xmlPath,
                groupId: groupData.id
            });

            const newCategorizedFiles = { audios: [], documents: [], images: [], tables: [], standalone_transcripts: [], videos: [], others: [] };
            allFiles = files || [];
            (files || []).forEach(file => { // Ensure files is an array
                switch (file.file_type) {
                    case 'audio': newCategorizedFiles.audios.push(file); break;
                    case 'video': newCategorizedFiles.videos.push(file); break; // Added video to switch
                    case 'document': newCategorizedFiles.documents.push(file); break;
                    case 'image': newCategorizedFiles.images.push(file); break;
                    case 'table': newCategorizedFiles.tables.push(file); break;
                    case 'standalone_transcript': newCategorizedFiles.standalone_transcripts.push(file); break;
                    default: newCategorizedFiles.others.push(file); break;
                }
            });
            categorizedFiles = newCategorizedFiles;
            sortFiles();
        } catch (err) {
            console.error("Error fetching group contents:", err);
            errorMessage = typeof err === 'string' ? err : "Failed to load group contents.";
        } finally {
            isLoading = false;
        }
    }

    function handleFileDoubleClick(file) {
        if (!file || !file.relative_path || !file.file_type) return;

        const filePathToOpen = file.full_path; // Use full_path from AssociatedFile

        if (file.file_type === 'document') {
            prepareDocumentView(filePathToOpen, 'documents');
        } else if (file.file_type === 'table') {
            prepareDocumentView(filePathToOpen, 'tables');
        } else if (file.file_type === 'image') {
            prepareDocumentView(filePathToOpen, 'images');
        } else if (file.file_type === 'standalone_transcript') {
            prepareStandaloneTranscriptView(filePathToOpen);
        } else if (file.file_type === 'audio' || file.file_type === 'video' || file.file_type === 'media_other') {
            prepareMediaNoteView(filePathToOpen);
        } else {
            console.warn("Unknown file type for double click:", file.file_type);
        }
    }

    onMount(async () => {
        try {
            const currentOs = await getOsType();
            if (currentOs === 'windows') revealButtonLabelGroupView = 'Reveal in Explorer';
            else if (currentOs === 'macos') revealButtonLabelGroupView = 'Reveal in Finder';
            // Default 'Open File Location' is already set
        } catch (e) {
            console.error("[GroupDetailView] Error getting OS type for reveal label:", e);
        }
    });

    function handleSort(key) {
        if (sortKey === key) {
            sortDirection *= -1;
        } else {
            sortKey = key;
            sortDirection = 1;
        }
        sortFiles();
    }

    function sortFiles() {
        allFiles = [...allFiles].sort((a, b) => {
            let valA, valB;
            if (sortKey === 'type') {
                valA = getCategoryInfo(a.file_type)?.singularName || '';
                valB = getCategoryInfo(b.file_type)?.singularName || '';
                if (valA === valB) {
                    // secondary sort by name
                    valA = a.name.toLowerCase();
                    valB = b.name.toLowerCase();
                }
            } else if (sortKey === 'name') {
                valA = a.name.toLowerCase();
                valB = b.name.toLowerCase();
            } else if (sortKey === 'createdAt') {
                valA = a.created_at ? new Date(a.created_at).getTime() : 0;
                valB = b.created_at ? new Date(b.created_at).getTime() : 0;
            } else if (sortKey === 'lastModified') {
                valA = a.last_modified ? new Date(a.last_modified).getTime() : 0;
                valB = b.last_modified ? new Date(b.last_modified).getTime() : 0;
            } else if (sortKey === 'title') {
                valA = (a.title || '').toLowerCase();
                valB = (b.title || '').toLowerCase();
            } else if (sortKey === 'description') {
                valA = (a.description || '').toLowerCase();
                valB = (b.description || '').toLowerCase();
            }

            if (valA < valB) return -1 * sortDirection;
            if (valA > valB) return 1 * sortDirection;
            return 0;
        });
    }

    $: filteredAllFiles = allFiles.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()));

    // Reactive watch on groupData and specific project properties
    // Using get(projectStore) inside the reactive block might be redundant if $projectStore is used,
    // but ensures access if the block's timing is tricky with store updates.
    // For simplicity and directness, direct $: subscription to $projectStore.id and $projectStore.xmlPath is cleaner.
    $: if (groupData && groupData.id && $projectStore.id && $projectStore.xmlPath) {
        fetchGroupContents();
    } else if (!groupData || !$projectStore.id || !$projectStore.xmlPath) { // Added condition to clear if context is lost
        categorizedFiles = { audios: [], documents: [], images: [], tables: [], standalone_transcripts: [], videos: [], others: [] };
        allFiles = [];
        isLoading = false;
        errorMessage = null;
    }

    // Listen for external notifications to refresh group content
    $: if ($groupContentNotification && groupData && $groupContentNotification.groupId === groupData.id) {
        console.log('[GroupDetailView] groupContentNotification received for current group, refreshing contents...', $groupContentNotification);
        fetchGroupContents();
        // Resetting the notification store after processing to prevent re-triggering
        // This is a common pattern, but ensure it fits the overall design (e.g., if other components also need to react).
        // If multiple components need to react independently, this reset should be handled more carefully,
        // perhaps by having components acknowledge the notification or by using event-based logic.
        // For a simple refresh, immediate reset is often fine.
        // groupContentNotification.set(null);
        // Edit: Per discussion, if the store value uses a timestamp, downstream components can decide if the notification is "new" enough to act on.
        // So, direct reset here might not be needed if consumers check the timestamp.
        // However, for this specific component, if it acts on any notification for its ID, resetting might still be useful if it shouldn't re-fetch for the exact same timestamped event.
        // Let's defer resetting for now, assuming consumers will be smart or the notification implies a definite state change needing refresh.
    }

    async function handleGroupDetailsUpdated(event) { // Make it async if calling await
        const updatedGroup = event.detail;
        groupData = { ...groupData, ...updatedGroup }; // Update local prop
        isEditGroupModalOpen = false;

        // Update project store's selectedGroupData
        projectStore.update(p => {
            if (p.selectedGroupData && p.selectedGroupData.id === updatedGroup.id) {
                return { ...p, selectedGroupData: { ...p.selectedGroupData, ...updatedGroup } };
            }
            return p;
        });

        // New line: Refresh the list of all project groups
        if (updatedGroup.project_id) { // Ensure we have a project_id
            console.log('[GroupDetailView] Group details updated, refreshing project groups list...');
            await updateProjectGroupsList(updatedGroup.project_id);
        } else {
            console.warn('[GroupDetailView] project_id not available in updatedGroup, cannot refresh project groups list.');
        }
    }

    async function handleGroupDeleted(event) {
        const deletedGroupId = event.detail;
        const currentProjectId = $projectStore.id;
        console.log('[GroupDetailView] Group deleted:', deletedGroupId);
        isEditGroupModalOpen = false;

        // If the deleted group was the one we're viewing, clear the selection
        if (groupData && groupData.id === deletedGroupId) {
            setSelectedGroup(null);
            groupData = null;
        }

        // Refresh the sidebar
        if (currentProjectId) {
            await updateProjectGroupsList(currentProjectId);
        }

        updateProjectStoreState({ statusMessage: "Group deleted successfully." });
    }

    function handleFileContextMenu(event, file) {
      event.preventDefault();
      event.stopPropagation();
      if (contextMenuVisible) {
        closeContextMenu();
      }
      contextMenuItem = file;

      let x = event.clientX;
      let y = event.clientY;

      // Estimate menu size to keep it in viewport
      const menuWidthEstimate = 200;
      const menuHeightEstimate = 250;

      if (x + menuWidthEstimate > window.innerWidth) {
          x = window.innerWidth - menuWidthEstimate - 10;
      }
      if (y + menuHeightEstimate > window.innerHeight) {
          y = window.innerHeight - menuHeightEstimate - 10;
      }

      contextMenuX = Math.max(10, x);
      contextMenuY = Math.max(10, y);

      contextMenuVisible = true;
      // Add listener to close on outside click
      if (closeContextMenuListener) document.removeEventListener('click', closeContextMenuListener, { capture: true });
      closeContextMenuListener = (e) => {
        const menuElement = document.getElementById('group-detail-context-menu'); // Ensure unique ID for this menu
        if (menuElement && !menuElement.contains(e.target)) {
          closeContextMenu();
        }
      };
      setTimeout(() => document.addEventListener('click', closeContextMenuListener, { capture: true, once: true }), 0);
    }

    function closeContextMenu() {
      contextMenuVisible = false;
      contextMenuItem = null;
      showAddToGroupSubMenu = false; // also close submenu
      itemForAddToGroup = null;
      if (closeContextMenuListener) {
        document.removeEventListener('click', closeContextMenuListener, { capture: true });
        closeContextMenuListener = null;
      }
    }

    async function handleContextMenuRemoveFromGroup(event) {
      const item = event.detail.item;
      if (!item || !groupData || !groupData.id) return;

      const confirmed = await confirm(`Are you sure you want to remove "${item.name}" from the group "${groupData.name}"?`, {
        title: 'Confirm Removal',
        type: 'warning'
      });
      if (!confirmed) return;

      try {
        const currentProject = get(projectStore);
        await invoke('remove_file_from_group', {
          projectId: currentProject.id,
          groupId: groupData.id,
          fileAssetRelativePath: item.relative_path
        });
        updateProjectStoreState({ statusMessage: `File "${item.name}" removed from group "${groupData.name}".` });
        await fetchGroupContents(); // Refresh the current view
      } catch (err) {
        console.error('Error removing file from group:', err);
        await message(`Failed to remove file from group: ${err}`, { title: 'Error', type: 'error' });
      }
    }

    async function handleContextMenuOpen(event) {
      const item = event.detail.item;
      if (!item) return;
      console.log('Open action for:', item);
      handleFileDoubleClick(item); // Uses existing logic to open the file
      closeContextMenu();
    }

    async function handleContextMenuReveal(event) {
      const item = event.detail.item;
      if (!item || !item.full_path) return;
      console.log('Reveal action for:', item);
      try {
        await invoke('reveal_in_file_explorer', { filePathStr: item.full_path });
      } catch (err) {
        console.error("Error revealing file:", err);
        await message(`Could not reveal file: ${err.message || err}`, { title: 'Error', type: 'error' });
      }
      closeContextMenu();
    }

    function handleContextMenuRename(event) {
      const item = event.detail.item;
      if (!item) return;
      console.log('Rename action for:', item);
      itemToRename = item; // item should have { name, relative_path, file_type, full_path }
      showRenameModal = true;
      closeContextMenu();
    }

    async function handleRenameModalConfirm(event) {
        const newNameFromModal = event.detail.newName; // This is the (potentially) stem name from the modal
        const originalFullName = itemToRename.name; // Full original name, e.g., "OldName.png"
        const itemType = itemToRename.file_type;

        if (!itemToRename || !newNameFromModal || newNameFromModal.trim() === '') {
            showRenameModal = false;
            itemToRename = null;
            return;
        }

        let finalNewName = newNameFromModal.trim();
        const originalExtension = originalFullName.includes('.') ? originalFullName.substring(originalFullName.lastIndexOf('.')) : '';

        // Check if the name from the modal already includes an extension.
        // The FileRenameModal is designed to send only the stem for item types like 'image', 'doc', 'table', etc.
        // So, newNameFromModal is not expected to have an extension for these types.
        // We need to append the original extension if one existed.
        if (originalExtension && !finalNewName.endsWith(originalExtension)) {
            // A simple check for whether newNameFromModal already contains *any* dot can also be used
            // if we want to allow users to change extensions, but the current modal doesn't facilitate that for stem input mode.
            // For now, assume we always preserve the original extension if the modal sends a stem.
            // More robustly, check if the modal sent a name that *doesn't* have an extension part.
            const newNameHasExtension = finalNewName.includes('.') && finalNewName.lastIndexOf('.') > 0; // Basic check
            if (!newNameHasExtension) {
                finalNewName += originalExtension;
            }
        }

        // Optional: Add a console log to verify the names
        console.log(`[GroupDetailView] Rename Confirm: Original='${originalFullName}', FromModal='${newNameFromModal}', Final='${finalNewName}'`);

        const currentProj = get(projectStore);
        try {
            // Call renameProjectItem with the finalNewName that includes the extension
            await renameProjectItem(itemToRename.full_path, finalNewName, itemType, currentProj.xmlPath, currentProj.baseDirectory);
            updateProjectStoreState({ statusMessage: `Item "${originalFullName}" renamed to "${finalNewName}" successfully.` });
            await fetchGroupContents(); // Refresh this group's view
        } catch (error) {
            console.error('Error renaming item:', error);
            await message(`Failed to rename item: ${error.message || error}`, { title: 'Error', type: 'error' });
        } finally {
            showRenameModal = false;
            itemToRename = null;
        }
    }

    function handleRenameModalClose() {
      showRenameModal = false;
      itemToRename = null;
    }

    async function handleContextMenuDelete(event) {
      const item = event.detail.item;
      if (!item || !item.full_path) return;
      console.log('Delete action for:', item);

      const confirmed = await confirm(`Are you sure you want to delete "${item.name}"? This action cannot be undone.`, {
        title: 'Confirm Deletion',
        type: 'warning'
      });
      if (!confirmed) {
        closeContextMenu();
        return;
      }

      const currentProj = get(projectStore);
      try {
        await deleteProjectItem(item.full_path, currentProj.xmlPath); // deleteProjectItem expects full path
        updateProjectStoreState({ statusMessage: `Item "${item.name}" deleted successfully.` });
        await fetchGroupContents(); // Refresh this group's view
        // projectService.deleteProjectItem should have updated the main projectStore files list
      } catch (error) {
        console.error('Error deleting item:', error);
        await message(`Failed to delete item: ${error.message || error}`, { title: 'Error', type: 'error' });
      }
      closeContextMenu();
    }

    async function handleContextMenuTranscribe(event) {
      const itemFromGroup = event.detail.item;
      if (!itemFromGroup || !itemFromGroup.full_path) {
        await message("Cannot transcribe: media path is missing.", { title: "Error", type: "error" });
        closeContextMenu();
        return;
      }

      console.log('[GroupDetailView] Transcribe action for (from group):', itemFromGroup);
      const projectFiles = get(projectStore).files; // projectStore is already imported
      const mediaPathToFind = itemFromGroup.full_path;

      // Local helper function, could be moved to a utility if used elsewhere
      function findMediaByPathRecursiveLocal(nodes, path) {
        if (!Array.isArray(nodes)) return null;
        const targetPathNormalized = path?.replace(/\\/g, '/');
        for (const node of nodes) {
            const nodePathNormalized = node.path?.replace(/\\/g, '/');
            if (node.file_type === 'media' && !node.is_directory && nodePathNormalized === targetPathNormalized) {
                return node;
            }
            if (node.children && node.children.length > 0) {
                const found = findMediaByPathRecursiveLocal(node.children, path);
                if (found) return found;
            }
        }
        return null;
      }

      const canonicalFileEntry = findMediaByPathRecursiveLocal(projectFiles, mediaPathToFind);

      let pathForDispatch;
      if (canonicalFileEntry) {
        console.log('[GroupDetailView] Found canonical FileEntry, dispatching requestmediaselection with its path:', canonicalFileEntry.path);
        pathForDispatch = canonicalFileEntry.path;
         // DO NOT call selectMedia here. Let ProjectView.handleRequestMediaSelection do it.
      } else {
        console.error(`[GroupDetailView] Canonical FileEntry not found for path: ${mediaPathToFind}. Falling back to itemFromGroup.full_path for dispatch.`);
        pathForDispatch = itemFromGroup.full_path; // Fallback to the original path from group content
      }

      dispatch('requestmediaselection', { mediaPath: pathForDispatch });
      updateProjectStoreState({ statusMessage: 'Transcription requested for ' + itemFromGroup.name });
      closeContextMenu();
    }

    function handleContextMenuAddToGroup(event) { // Renamed from original placeholder to avoid conflict
      const item = event.detail.item;
      if (!item) return;
      itemForAddToGroup = item;
      // Position submenu relative to the main context menu click coordinates
      openAddToGroupSubMenu(contextMenuX + 5, contextMenuY + 5); // Offset slightly
      // Main context menu closes itself via its own click handler now
    }

    async function fetchProjectGroupsForMenu(forceRefresh = false) {
      const currentProject = get(projectStore); // projectStore is the alias for project
      if (!currentProject || !currentProject.id) {
        projectGroupsForMenu = [];
        return;
      }

      const groupsFromStore = get(currentProjectGroupsList);
      if (!forceRefresh && groupsFromStore && groupsFromStore.length > 0) {
        projectGroupsForMenu = [...groupsFromStore].sort((a, b) => a.name.localeCompare(b.name)); // Ensure sort if store isn't pre-sorted or copy needed
        // console.log('[GroupDetailView] Using groups from store for submenu.');
        return;
      }

      // console.log('[GroupDetailView] Forcing refresh or store empty, calling updateProjectGroupsList.');
      await updateProjectGroupsList(currentProject.id); // This updates the store
      projectGroupsForMenu = [...get(currentProjectGroupsList)].sort((a, b) => a.name.localeCompare(b.name)); // Read from store after update
    }

    async function openAddToGroupSubMenu(x_pos, y_pos) {
        await fetchProjectGroupsForMenu(); // Ensure groups are loaded
        addToGroupSubMenuX = x_pos;
        addToGroupSubMenuY = y_pos;
        showAddToGroupSubMenu = true;

        if (closeAddToGroupSubMenuListener) document.removeEventListener('click', closeAddToGroupSubMenuListener, { capture: true });
        closeAddToGroupSubMenuListener = (e) => {
        const subMenuElement = document.getElementById('group-detail-add-to-group-submenu');
        if (subMenuElement && !subMenuElement.contains(e.target)) {
            closeAddToGroupSubMenu();
        }
        };
        setTimeout(() => {
        if(showAddToGroupSubMenu) document.addEventListener('click', closeAddToGroupSubMenuListener, { capture: true, once: true });
        }, 0);
    }

    function closeAddToGroupSubMenu(preserveItemForGroupCreation = false) {
        showAddToGroupSubMenu = false;
        if (!preserveItemForGroupCreation) {
            itemForAddToGroup = null;
        }
        if (closeAddToGroupSubMenuListener) {
        document.removeEventListener('click', closeAddToGroupSubMenuListener, { capture: true });
        closeAddToGroupSubMenuListener = null;
        }
    }

    async function handleAddFileToExistingGroupInGroupView(group) {
        if (!itemForAddToGroup || !group || !group.id) return;
        const currentProject = get(projectStore);
        if (!currentProject || !currentProject.id) {
        await message('Project context is missing. Cannot add to group.', { title: 'Error', type: 'error'});
        return;
        }

        try {
        await invoke('add_file_to_existing_group', {
            projectId: currentProject.id,
            groupId: group.id,
            fileAssetRelativePath: itemForAddToGroup.relative_path
        });
        updateProjectStoreState({ statusMessage: `File "${itemForAddToGroup.name}" added to group "${group.name}".` });
        if (group.id === groupData.id) { // 'group' is the target group, 'groupData' is the currently viewed group
            console.log('[GroupDetailView] File added to current group, refreshing contents...');
            await fetchGroupContents();
        }
        } catch (err) {
        console.error('Error adding file to group:', err);
        await message(`Failed to add file to group: ${err}`, { title: 'Error', type: 'error' });
        } finally {
        closeAddToGroupSubMenu();
        }
    }

    function handleNewGroupClickInGroupView() {
        // itemForAddToGroup is already set when the submenu was opened.
        showCreateGroupModalFromGroupView = true;
        closeAddToGroupSubMenu(true);
    }

    function handleModalGroupCreated() { // When group is created, but file might not have been added if itemForAddToGroup was null
        fetchProjectGroupsForMenu(true); // Refresh group list
        showCreateGroupModalFromGroupView = false;
        itemForAddToGroup = null; // Reset
    }

    function handleModalGroupCreatedAndFileAdded(event) { // When group is created AND current file added
        fetchProjectGroupsForMenu(true); // Refresh group list
        showCreateGroupModalFromGroupView = false;
        updateProjectStoreState({ statusMessage: `File "${itemForAddToGroup?.name}" added to new group "${event.detail.group?.name}".` });

        const newGroupData = event.detail.group;
        if (newGroupData && itemForAddToGroup) { // Ensure itemForAddToGroup was processed
           // groupContentNotification.set({ groupId: newGroupData.id, action: 'file_added', timestamp: Date.now() }); // Line 511: removed .set call
        }
        itemForAddToGroup = null; // Reset
    }

</script>

<div class="p-4 h-full flex flex-col bg-white dark:bg-gray-900">
    {#if groupData}
        <!-- Header -->
        <div class="mb-4 pb-2 border-b border-gray-300 dark:border-gray-700">
            <div class="flex items-center space-x-2">
                <h2 class="text-xl font-semibold text-gray-800 dark:text-gray-100">{groupData.name}</h2>
                <button
                    on:click={() => isEditGroupModalOpen = true}
                    title="Edit group details"
                    class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500 flex items-center justify-center"
                >
                    <SquarePen class="w-4 h-4" />
                </button>
            </div>
            {#if groupData.description && groupData.description.trim() !== ''}
                <p class="text-sm text-gray-600 dark:text-gray-400 mt-1 max-h-20 overflow-y-auto pr-2">{groupData.description}</p>
            {:else}
                <p class="text-sm text-gray-400 dark:text-gray-500 mt-1 italic h-5">No description provided.</p>
            {/if}
        </div>

        <!-- Toolbar (Below Header Rule) -->
        {#if $panelStateStore.groupDetailViewMode === 'list' && !isLoading}
            <div class="mb-4 flex justify-between items-center">
                <div class="relative inline-block text-left">
                    <Button color="alternative" size="sm" class="flex items-center space-x-1">
                        <span>{visibleColumnsCount} Columns</span>
                        <ChevronDown class="w-4 h-4" />
                    </Button>
                    <Dropdown class="w-48 p-3 space-y-2">
                        {#each columns as col}
                            <li>
                                <Checkbox bind:checked={col.visible} disabled={col.disabled} class="cursor-pointer">{col.label}</Checkbox>
                            </li>
                        {/each}
                    </Dropdown>
                </div>
                <div class="w-64">
                    <Search size="sm" class="bg-gray-50 dark:bg-gray-800" placeholder="Search..." bind:value={searchQuery} />
                </div>
            </div>
        {/if}

        <!-- Body -->
        <div class="flex-grow overflow-y-auto pr-2">
            {#if isLoading}
                <p class="text-gray-500 dark:text-gray-400 text-center py-8">Loading group contents...</p>
            {:else if errorMessage}
                <p class="text-red-500 dark:text-red-400 text-center py-8">Error: {errorMessage}</p>
            {:else}
                {#if $panelStateStore.groupDetailViewMode === 'grid'}
                    {#each CATEGORY_ORDER as category}
                        {@const filesInCategory = categorizedFiles[category.key]}
                        {#if filesInCategory && filesInCategory.length > 0}
                        <div class="mb-6">
                            <h3 class="text-lg font-medium text-gray-700 dark:text-gray-200 mb-2">{category.name}</h3>
                            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                                {#each filesInCategory as file (file.relative_path)}
                                    <div
                                        class="group relative flex flex-col cursor-pointer"
                                        on:dblclick={() => handleFileDoubleClick(file)}
                                        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleFileDoubleClick(file); }}
                                        on:contextmenu={(e) => handleFileContextMenu(e, file)}
                                        role="button"
                                        tabindex="0"
                                    >
                                        <!-- Preview Area -->
                                        <div class="aspect-square w-full relative bg-gray-50 dark:bg-gray-950 border border-gray-200 dark:border-gray-800 rounded-xl overflow-hidden mb-2 transition-colors duration-200 group-hover:border-gray-300 dark:group-hover:border-gray-600">
                                            {#if file.file_type === 'image' && file.full_path}
                                                <img 
                                                    src={convertFileSrc(file.full_path)} 
                                                    alt={file.name} 
                                                    class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                                                    loading="lazy"
                                                />
                                            {:else if file.file_type === 'video' && file.full_path}
                                                <video
                                                    src={convertFileSrc(file.full_path) + '#t=0.1'}
                                                    preload="metadata"
                                                    muted
                                                    playsinline
                                                    class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                                                ></video>
                                            {:else if (file.file_type === 'document' || file.file_type.includes('transcript')) && file.full_path && file.full_path.endsWith('.json')}
                                                <DocumentThumbnail {file} isTranscript={file.file_type.includes('transcript')} />
                                            {:else if file.file_type === 'table' && file.full_path}
                                                <TableThumbnail {file} />
                                            {:else}
                                                <div class="absolute inset-0 flex items-center justify-center transition-transform duration-300 group-hover:scale-110 text-gray-400 dark:text-gray-500">
                                                    <svelte:component this={category.icon} class="w-12 h-12" />
                                                </div>
                                            {/if}

                                            <!-- Actions (More options) -->
                                            <button
                                                on:click|stopPropagation|preventDefault={(e) => handleFileContextMenu(e, file)}
                                                class="absolute top-2 right-2 p-1.5 bg-white/90 dark:bg-gray-800/90 border border-gray-200 dark:border-gray-700 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 opacity-0 group-hover:opacity-100 transition-all duration-200 shadow-sm z-10"
                                                title="More options for {file.name}"
                                            >
                                                <MoreHorizontal class="w-3.5 h-3.5" />
                                            </button>
                                        </div>

                                        <!-- Filename Area -->
                                        <p class="text-[11px] font-medium text-gray-600 dark:text-gray-400 truncate text-center px-1 group-hover:text-gray-900 dark:group-hover:text-gray-200" title={file.name}>
                                            {file.name}
                                        </p>
                                    </div>
                                {/each}
                            </div>
                        </div>
                        {/if}
                    {/each}
                {:else}
                    <!-- List View -->
                    {#if filteredAllFiles.length > 0}
                        <Table hoverable={true}>
                            <TableHead>
                                {#each columns as col}
                                    {#if col.visible}
                                        <TableHeadCell on:click={() => handleSort(col.key)} class="cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 select-none">
                                            <div class="flex items-center space-x-1">
                                                <span>{col.label}</span>
                                                {#if sortKey === col.key}
                                                    <span class="text-xs">{sortDirection === 1 ? '▲' : '▼'}</span>
                                                {/if}
                                            </div>
                                        </TableHeadCell>
                                    {/if}
                                {/each}
                                <TableHeadCell class="w-10"><span class="sr-only">Actions</span></TableHeadCell>
                            </TableHead>
                            <TableBody>
                                {#each filteredAllFiles as file (file.relative_path)}
                                    <TableBodyRow
                                        class="cursor-pointer group"
                                        on:dblclick={() => handleFileDoubleClick(file)}
                                        on:contextmenu={(e) => handleFileContextMenu(e, file)}
                                    >
                                        {#if columns.find(c => c.key === 'type').visible}
                                            <TableBodyCell class="w-48 whitespace-nowrap" title={getCategoryInfo(file.file_type)?.singularName || 'Unknown'}>
                                                <div class="flex items-center space-x-2 text-gray-600 dark:text-gray-300">
                                                    <svelte:component this={getCategoryInfo(file.file_type)?.icon || File} class="w-4 h-4" />
                                                    <span>{getCategoryInfo(file.file_type)?.singularName || 'Unknown'}</span>
                                                </div>
                                            </TableBodyCell>
                                        {/if}
                                        {#if columns.find(c => c.key === 'name').visible}
                                            <TableBodyCell class="whitespace-nowrap font-medium text-gray-900 dark:text-white" title={file.name}>
                                                {file.name}
                                            </TableBodyCell>
                                        {/if}
                                        {#if columns.find(c => c.key === 'title').visible}
                                            <TableBodyCell class="whitespace-nowrap text-gray-500 dark:text-gray-400 truncate max-w-[150px]" title={file.title || ''}>
                                                {file.title || ''}
                                            </TableBodyCell>
                                        {/if}
                                        {#if columns.find(c => c.key === 'description').visible}
                                            <TableBodyCell class="whitespace-nowrap text-gray-500 dark:text-gray-400 truncate max-w-[200px]" title={file.description || ''}>
                                                {file.description || ''}
                                            </TableBodyCell>
                                        {/if}
                                        {#if columns.find(c => c.key === 'createdAt').visible}
                                            <TableBodyCell class="whitespace-nowrap text-gray-500 dark:text-gray-400" title={file.created_at ? new Date(file.created_at).toLocaleString() : 'Unknown'}>
                                                {#if file.created_at}
                                                    {new Date(file.created_at).toLocaleString()}
                                                {:else}
                                                    Unknown
                                                {/if}
                                            </TableBodyCell>
                                        {/if}
                                        {#if columns.find(c => c.key === 'lastModified').visible}
                                            <TableBodyCell class="whitespace-nowrap text-gray-500 dark:text-gray-400" title={file.last_modified ? new Date(file.last_modified).toLocaleString() : 'Unknown'}>
                                                {#if file.last_modified}
                                                    {new Date(file.last_modified).toLocaleString()}
                                                {:else}
                                                    Unknown
                                                {/if}
                                            </TableBodyCell>
                                        {/if}
                                        <TableBodyCell class="text-right">
                                            <button
                                                on:click|stopPropagation|preventDefault={(e) => handleFileContextMenu(e, file)}
                                                class="p-1 rounded text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:hover:text-white dark:hover:bg-gray-700"
                                                title="More options"
                                            >
                                                <MoreVertical class="w-4 h-4" />
                                            </button>
                                        </TableBodyCell>
                                    </TableBodyRow>
                                {/each}
                            </TableBody>
                        </Table>
                    {:else}
                        <p class="text-gray-500 dark:text-gray-400 text-center py-8">No files match your search.</p>
                    {/if}
                {/if}

                {@const totalFiles = Object.values(categorizedFiles).reduce((sum, arr) => sum + arr.length, 0)}
                {#if totalFiles === 0 && !isLoading}
                     <p class="text-gray-500 dark:text-gray-400 text-center py-8">This group is empty.</p>
                {/if}
            {/if}
        </div>
    {:else}
            <p class="text-gray-500 dark:text-gray-400 text-center py-8">No group selected.</p>
    {/if}
</div>

<EditGroupModal
    bind:showModal={isEditGroupModalOpen}
    groupData={groupData}
    on:groupUpdated={handleGroupDetailsUpdated}
    on:groupDeleted={handleGroupDeleted}
    on:close={() => isEditGroupModalOpen = false}
/>

<FileContextMenu
  bind:isVisible={contextMenuVisible}
  item={contextMenuItem}
  x={contextMenuX}
  y={contextMenuY}
  revealLabel={revealButtonLabelGroupView}
  on:open={handleContextMenuOpen}
  on:reveal={handleContextMenuReveal}
  on:rename={handleContextMenuRename}
  on:delete={handleContextMenuDelete}
  on:transcribe={handleContextMenuTranscribe}
  on:addToGroup={handleContextMenuAddToGroup}
  on:removeFromGroup={handleContextMenuRemoveFromGroup}
  id="group-detail-context-menu"
/>

{#if showAddToGroupSubMenu && itemForAddToGroup}
  <div
    id="group-detail-add-to-group-submenu"
    class="fixed z-[101] bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 py-1 text-xs min-w-[180px]"
    style="left: {addToGroupSubMenuX}px; top: {addToGroupSubMenuY}px;"
    on:click|stopPropagation
    role="menu"
  >
    <button on:click|stopPropagation={handleNewGroupClickInGroupView} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200">New group...</button>
    {#if projectGroupsForMenu.length > 0}
      <hr class="my-1 border-gray-200 dark:border-gray-700" />
      {#each projectGroupsForMenu as group (group.id)}
        <button on:click|stopPropagation={() => handleAddFileToExistingGroupInGroupView(group)} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-800 dark:text-gray-200 truncate" title={group.name}>
          {group.name}
        </button>
      {/each}
    {:else}
      <span class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic">No other groups</span>
    {/if} <!-- This closes the #if projectGroupsForMenu.length > 0 -->
  </div>
{/if} <!-- This closes the #if showAddToGroupSubMenu && itemForAddToGroup -->

<CreateGroupModal
    bind:showModal={showCreateGroupModalFromGroupView}
    projectUuid={$projectStore?.id}
    fileToAdd={itemForAddToGroup}
    on:close={() => { showCreateGroupModalFromGroupView = false; itemForAddToGroup = null; }}
    on:groupCreated={handleModalGroupCreated}
    on:groupCreatedAndFileAdded={handleModalGroupCreatedAndFileAdded}
/>

{#if showRenameModal && itemToRename}
    <FileRenameModal
        bind:showModal={showRenameModal}
        currentName={itemToRename.name}
        itemType={itemToRename.file_type}
        isMediaRename={itemToRename.file_type === 'audio' || itemToRename.file_type === 'video'}
        on:confirm={handleRenameModalConfirm}
        on:close={handleRenameModalClose}
    />
{/if}
<!-- Ensure all {#if} and {#each} blocks above this line are properly closed -->
<style>
    /* Ensure grid items don't overflow their container excessively if names are too long */
    .grid div > p {
        max-width: 100%; /* Or specific width like '8rem' or '120px' */
    }
</style>
