<!-- src/lib/components/projectview/notes/NotesLeftPanel.svelte -->
<script>
	import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView } from '$lib/stores/projectStore.js'; // Added prepareMediaNoteView
	import { get } from 'svelte/store';
	import { renameProjectItem, deleteProjectItem, importMediaFile, importDocumentFile, importTableFile, importImageFile, importTranscriptFile, deleteImportedTranscript } from '$lib/services/projectService.js';
	import FileRenameModal from '../modals/FileRenameModal.svelte';
	import ImportTranscriptSourceModal from '../modals/ImportTranscriptSourceModal.svelte';
	import { confirm, message } from '@tauri-apps/plugin-dialog';
	import { readTextFile, writeTextFile, rename } from '@tauri-apps/plugin-fs';
	import { dirname, basename, sep, extname } from '@tauri-apps/api/path';
	import * as openerPlugin from '@tauri-apps/plugin-opener';
	import { createEventDispatcher, onMount } from 'svelte';
    import { convertFileSrc } from '@tauri-apps/api/core';

    const dispatch = createEventDispatcher();

    let prevAutoOpenPath = null;
    let showImportTranscriptModal = false;

    let currentFileMetadata = null; // Holds only the 'metadata' part of the JSON
    let fullLoadedMetadataObject = null; // Holds the entire parsed JSON object
    let isEditing = false;
    let editableMetadata = {
        file_name: '',
        title: '',
        description: '',
        summary: ''
    };

    let categoryContextMenuVisible = false;
    let categoryContextMenuX = 0;
    let categoryContextMenuY = 0;
    let categoryContextMenuType = null;

    function handleCategoryContextMenu(event, categoryType) {
      event.preventDefault();
      event.stopPropagation();
      if (categoryContextMenuVisible) {
        closeCategoryContextMenu();
      }
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

    onMount(() => {
      const listener = () => {
        if (categoryContextMenuVisible) closeCategoryContextMenu();
      };
      document.addEventListener('click', listener);
      return () => document.removeEventListener('click', listener);
    });

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
    let categoryOpenState = {}; const LS_KEY_NOTES_PANEL_STATE = 'harveyNotesPanelCategoryState';

    onMount(() => { const defaultState = {}; CATEGORIES_BASE.forEach(cat => { defaultState[cat.type] = true; }); try { const savedState = localStorage.getItem(LS_KEY_NOTES_PANEL_STATE); if (savedState) { const parsedState = JSON.parse(savedState); categoryOpenState = { ...defaultState, ...parsedState }; } else { categoryOpenState = defaultState; } } catch (e) { console.error("[NotesLeftPanel] Failed load category state:", e); categoryOpenState = defaultState; } });
    function toggleCategory(categoryType) { if (categoryOpenState.hasOwnProperty(categoryType)) { categoryOpenState[categoryType] = !categoryOpenState[categoryType]; categoryOpenState = categoryOpenState; } else { console.warn(`[NotesLeftPanel] Toggle unknown category: ${categoryType}`); } }
    $: if (Object.keys(categoryOpenState).length > 0) { try { localStorage.setItem(LS_KEY_NOTES_PANEL_STATE, JSON.stringify(categoryOpenState)); } catch (e) { console.error("[NotesLeftPanel] Failed save category state:", e); } }

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
                    const mediaData = { name: node.name, path: node.path, media_xml_identifier: node.media_xml_identifier || '', file_type: 'media' };
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
      const q = searchQuery.trim().toLowerCase();
      if (!showSearchBox || q === '') return displayCategories;
      return displayCategories.map(cat => ({
        ...cat,
        files: cat.files.filter(file => file.name.toLowerCase().includes(q)),
      }));
    })();

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
                    console.log(`[NotesLeftPanel] Auto open path ${autoPath} matches category ${itemCategoryType}. NotesView will handle it.`);
                    prevAutoOpenPath = autoPath;
                } else {
                    console.warn(`[NotesLeftPanel] Auto open path ${autoPath} (type ${itemCategoryType}) NOT FOUND in current displayCategories.`);
                }
            }
        }
    }

    async function handleImportClick(categoryType) {
        console.log(`[NotesLeftPanel] Import clicked for: ${categoryType}`);
        const categoryInfo = CATEGORIES_BASE.find(c => c.type === categoryType);
        if (!categoryInfo || !categoryInfo.importEnabled) {
            message(`Import for ${categoryInfo?.name || 'this category'} not available.`, { title: 'Not Implemented', type: 'info' });
            return;
        }

        if (categoryType === 'video' || categoryType === 'audio') {
            try { 
                await importMediaFile(categoryType); 
            } catch (e) { console.error(`[NotesLeftPanel] Error importMediaFile ${categoryType}:`, e); }
        } else if (categoryType === 'document') {
            try { await importDocumentFile(); } catch (e) { console.error(`[NotesLeftPanel] Error importDocumentFile:`, e); }
        } else if (categoryType === 'table') {
            try { await importTableFile(); } catch (e) { console.error(`[NotesLeftPanel] Error importTableFile:`, e); }
        } else if (categoryType === 'image') {
            try { await importImageFile(); } catch (e) { console.error(`[NotesLeftPanel] Error importImageFile:`, e); }
        } else if (categoryType === 'imported_transcript') { 
            showImportTranscriptModal = true;
        } else {
            message(`Specific import for ${categoryInfo.name} not implemented.`, { title: 'Coming Soon', type: 'info' });
        }
    }
    
    async function handleImportTranscriptConfirm(event) {
        const { sourceType } = event.detail;
        showImportTranscriptModal = false;
        if (sourceType === 'msWord') {
            try {
                await importTranscriptFile(sourceType); 
            } catch (e) {
                console.error(`[NotesLeftPanel] Error importTranscriptFile (msWord):`, e);
            }
        } else {
            console.warn(`[NotesLeftPanel] Unknown transcript import source type: ${sourceType}`);
            await message(`Import from "${sourceType}" is not supported.`, { title: 'Import Error', type: 'error' });
        }
    }

    function handleItemContextMenu(event, item) { event.preventDefault(); event.stopPropagation(); console.log(`[NotesLeftPanel] Context menu for ${item.file_type} item:`, item); if (contextMenuVisible) { closeContextMenu(); } contextMenuItem = item; contextMenuX = event.clientX; contextMenuY = event.clientY; contextMenuVisible = true; setTimeout(() => { if (closeContextMenuListener) { document.removeEventListener('click', closeContextMenuListener, { capture: true }); } closeContextMenuListener = (e) => { const menuElement = document.getElementById('notes-left-panel-context-menu'); if (menuElement && !menuElement.contains(e.target)) { closeContextMenu(); } }; document.addEventListener('click', closeContextMenuListener, { capture: true, once: true }); }, 0); }
    function closeContextMenu() { if (contextMenuVisible) { contextMenuVisible = false; contextMenuItem = null; if (closeContextMenuListener) { document.removeEventListener('click', closeContextMenuListener, { capture: true }); closeContextMenuListener = null; } } }

    async function handleContextMenuAction(action) {
        console.log(`[NotesLeftPanel] Context action: "${action}"`);
        const item = contextMenuItem;
        if (!item) { console.error("[NotesLeftPanel] Context item null."); closeContextMenu(); return; }

        const itemPathForClosure = item.path;
        const itemType = item.file_type; 
        const isPdf = item.name?.toLowerCase().endsWith('.pdf');
        closeContextMenu();

        if (itemType === 'media') { 
            switch (action) {
                case 'Open': 
                    console.log(`[NotesLeftPanel] 'Open' action for media: ${item.name}`);
                    dispatch('requestviewchange', { viewType: 'media_note', itemPath: item.path });
                    break;
                case 'Rename': itemToRename = { path: item.path, name: item.media_xml_identifier, file_type: 'media', media_xml_identifier: item.media_xml_identifier }; showRenameModal = true; break;
                case 'Delete': const stemName = item.media_xml_identifier || (item.name.includes('.') ? item.name.substring(0, item.name.lastIndexOf('.')) : item.name); const confirmMsg = `Delete media "${stemName}"? This deletes the entire folder (media, transcripts, notes). Cannot be undone.`; const options = { title: 'Confirm Media Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmMsg, options); if (confirmed) { project.update(p => ({ ...p, statusMessage: `Deleting ${stemName}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[NotesLeftPanel] Delete failed for ${stemName}:`, err); } } else { project.update(p => ({ ...p, statusMessage: 'Deletion cancelled.' })); } } catch (e) { console.error("[NotesLeftPanel] Error confirm/delete:", e); await message(`Error deleting: ${e}`, {title: "Delete Error", type: "error"}); } break;
                case 'Transcribe': if (!item.path) { console.error("[NotesLeftPanel] Cannot transcribe: Media path missing."); await message("Cannot transcribe: path unknown.", { title: "Error", type: "error"}); break; } dispatch('requestmediaselection', { mediaPath: item.path }); break;
                default: console.warn(`[NotesLeftPanel] Unknown action for media: ${action}`); await message(`Action '${action}' not yet implemented for media.`, { title: 'Not Implemented', type: 'info' });
            }
        } else if (itemType === 'doc') {
            switch (action) {
                case 'Open': if (isPdf) { console.log(`[NotesLeftPanel] 'Open Externally' action for PDF: ${item.name}`); try { await openerPlugin.open(item.path); } catch (e) { console.error(`[NotesLeftPanel] Failed to open PDF externally: ${e}`); await message(`Could not open PDF externally: ${e}`, { title: 'Open Error', type: 'error'}); } } else { console.log(`[NotesLeftPanel] 'Open' action for JSON document: ${item.name}`); dispatch('requestviewchange', { viewType: 'documents', itemPath: item.path }); } break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'doc', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmDocMsg = `Delete document "${item.name}"? Cannot be undone.`; const docOptions = { title: 'Confirm Document Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmDocMsg, docOptions); if (confirmed) { console.log(`[NotesLeftPanel] Deleting document: ${itemPathForClosure}`); project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[NotesLeftPanel] Delete failed for ${item.name}:`, err); } } else { project.update(p => ({ ...p, statusMessage: 'Deletion cancelled.' })); } } catch (e) { console.error("[NotesLeftPanel] Error confirm/delete doc:", e); await message(`Error deleting: ${e}`, {title: "Delete Error", type: "error"}); } break;
                default: console.warn(`[NotesLeftPanel] Unknown action for document: ${action}`); await message(`Action '${action}' not implemented for documents.`, { title: 'Not Implemented', type: 'info' });
            }
        } else if (itemType === 'table') {
             switch (action) {
                case 'Open': console.log(`[NotesLeftPanel] 'Open' action for table: ${item.name}`); dispatch('requestviewchange', { viewType: 'tables', itemPath: item.path }); break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'table', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmTableMsg = `Delete table "${item.name}"? This cannot be undone.`; const tableOptions = { title: 'Confirm Table Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmTableMsg, tableOptions); if (confirmed) { console.log(`[NotesLeftPanel] Deleting table: ${itemPathForClosure}`); project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[NotesLeftPanel] Delete failed for table ${item.name}:`, err); await message(`Error deleting table: ${err}`, { title: "Delete Error", type: "error" }); } } else { project.update(p => ({ ...p, statusMessage: 'Table deletion cancelled.' })); } } catch (e) { console.error("[NotesLeftPanel] Error during confirm/delete table:", e); await message(`Error deleting table: ${e}`, { title: "Delete Error", type: "error" }); } break;
                default: console.warn(`[NotesLeftPanel] Unknown action for table: ${action}`); await message(`Action '${action}' not implemented for tables.`, { title: 'Not Implemented', type: 'info' });
            }
        } else if (itemType === 'image') {
             switch (action) {
                case 'Open': console.log(`[NotesLeftPanel] 'Open' action for image: ${item.name}`); dispatch('requestviewchange', { viewType: 'images', itemPath: item.path }); break;
                case 'Rename': itemToRename = { path: item.path, name: item.name, file_type: 'image', media_xml_identifier: null }; showRenameModal = true; break;
                case 'Delete': const confirmImageMsg = `Delete image "${item.name}"? This cannot be undone.`; const imageOptions = { title: 'Confirm Image Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' }; try { const confirmed = await confirm(confirmImageMsg, imageOptions); if (confirmed) { console.log(`[NotesLeftPanel] Deleting image: ${itemPathForClosure}`); project.update(p => ({ ...p, statusMessage: `Deleting ${item.name}...` })); try { await deleteProjectItem(itemPathForClosure); } catch (err) { console.error(`[NotesLeftPanel] Delete failed for image ${item.name}:`, err); await message(`Error deleting image: ${err}`, { title: "Delete Error", type: "error" }); } } else { project.update(p => ({ ...p, statusMessage: 'Image deletion cancelled.' })); } } catch (e) { console.error("[NotesLeftPanel] Error during confirm/delete image:", e); await message(`Error deleting image: ${e}`, { title: "Delete Error", type: "error" }); } break;
                default: console.warn(`[NotesLeftPanel] Unknown action for image: ${action}`); await message(`Action '${action}' not implemented for images.`, { title: 'Not Implemented', type: 'info' });
            }
        } else if (itemType === 'imported_transcript') { 
            switch (action) {
                case 'Open': 
                    console.log(`[NotesLeftPanel] 'Open' action for imported transcript: ${item.name}`);
                    dispatch('requestviewchange', { viewType: 'imported_transcript', itemPath: item.path }); 
                    break;
                case 'Rename': 
                    const nameWithoutExt = item.name.toLowerCase().endsWith('.json') 
                                            ? item.name.slice(0, -5) 
                                            : item.name;
                    itemToRename = { path: item.path, name: nameWithoutExt, file_type: 'imported_transcript', media_xml_identifier: null }; 
                    showRenameModal = true; 
                    break;
                case 'Delete':
                    const confirmTranscriptMsg = "Are you sure you want to delete this transcript? This cannot be undone.";
                    const transcriptOptions = { title: 'Confirm Transcript Deletion', type: 'warning', okLabel: 'Delete', cancelLabel: 'Cancel' };
                    try {
                        const confirmed = await confirm(confirmTranscriptMsg, transcriptOptions);
                        if (confirmed) {
                            try {
                                await deleteImportedTranscript(item.path);
                                // Clear any open transcript view so the panel updates
                                project.update(p => ({ ...p, currentImportedTranscriptPath: null }));
                            } catch (err) {
                                console.error('[NotesLeftPanel] Error deleting imported transcript:', err);
                                await message(`Error deleting transcript: ${err}`, { title: 'Delete Error', type: 'error' });
                            }
                        } else {
                            project.update(p => ({ ...p, statusMessage: 'Transcript deletion cancelled.' }));
                        }
                    } catch (e) {
                        console.error('[NotesLeftPanel] Error during confirm/delete imported transcript:', e);
                        await message(`Error deleting transcript: ${e}`, { title: 'Delete Error', type: 'error' });
                    }
                    break;
                default: 
                    console.warn(`[NotesLeftPanel] Unknown action for imported transcript: ${action}`); 
                    await message(`Action '${action}' not implemented for imported transcripts.`, { title: 'Not Implemented', type: 'info' });
            }
        }
    }

    async function handleRenameConfirm(event) {
        const { newName } = event.detail; 
        const item = itemToRename;
        if (!item || !newName || newName.trim() === '') { console.error("[NotesLeftPanel] Rename failed: Invalid input."); showRenameModal = false; itemToRename = null; return; }

        const finalNewNameFromModal = newName.trim(); 
        showRenameModal = false;

        if (item.file_type === 'media') {
            const finalNewStemName = finalNewNameFromModal;
            const confirmRename = await confirm(`Rename media '${item.media_xml_identifier}' to '${finalNewStemName}'? Renames folder & primary transcript.`, { title: 'Confirm Media Rename', type: 'warning', okLabel: 'Rename', cancelLabel: 'Cancel' });
            if (!confirmRename) { console.log("[NotesLeftPanel] Media rename cancelled."); itemToRename = null; return; }
            try { await renameProjectItem(item.path, finalNewStemName, item.file_type); } catch (err) { console.error(`[NotesLeftPanel] Rename failed for ${item.media_xml_identifier}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'doc') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : ''; // e.g. ".pdf"

            if (!originalExtension) {
                await message(`Error: Original file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' });
                itemToRename = null; return;
            }

            const allowedExts = ['.json', '.pdf', '.md', '.txt'];
            if (!allowedExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original file type '${originalExtension}' cannot be renamed via this interface.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;

            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[NotesLeftPanel] Rename failed for ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'table') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : ''; // e.g. ".csv"

            if (!originalExtension) {
                await message(`Error: Original table file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' });
                itemToRename = null; return;
            }

            const allowedTableExts = ['.csv', '.xlsx'];
            if (!allowedTableExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original table file type '${originalExtension}' cannot be renamed like this.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;

            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[NotesLeftPanel] Rename failed for table ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'image') {
            const stemNameFromModal = finalNewNameFromModal;
            const originalExtension = item.name.includes('.') ? item.name.substring(item.name.lastIndexOf('.')) : ''; // e.g. ".png"

            if (!originalExtension) {
                await message(`Error: Original image file '${item.name}' appears to have no extension. Cannot rename.`, { title: 'Rename Error', type: 'error' });
                itemToRename = null; return;
            }

            const allowedImageExts = ['.jpg', '.jpeg', '.png', '.gif', '.bmp', '.webp', '.tiff'];
            if (!allowedImageExts.includes(originalExtension.toLowerCase())) { await message(`Error: Original image file type '${originalExtension}' cannot be renamed like this.`, { title: 'Rename Error', type: 'error' }); itemToRename = null; return; }
            
            const newNameWithOriginalExt = `${stemNameFromModal}${originalExtension}`;

            try { await renameProjectItem(item.path, newNameWithOriginalExt, item.file_type); } catch (err) { console.error(`[NotesLeftPanel] Rename failed for image ${item.name}:`, err); } finally { itemToRename = null; }
        } else if (item.file_type === 'imported_transcript') { 
            const nameForBackend = finalNewNameFromModal; 
            try { 
                await renameProjectItem(item.path, nameForBackend, item.file_type); 
            } 
            catch (err) { console.error(`[NotesLeftPanel] Rename failed for imported transcript ${item.name}:`, err); } 
            finally { itemToRename = null; }
        } else {
            console.warn("[NotesLeftPanel] Rename unhandled type:", item.file_type);
            itemToRename = null;
        }
    }
    function handleRenameModalClose() { showRenameModal = false; itemToRename = null; }

    async function handleItemClick(item) {
        if (item.file_type === 'doc' || item.file_type === 'table' || item.file_type === 'image' || item.file_type === 'imported_transcript' || item.file_type === 'media') { 
            console.log(`[NotesLeftPanel] Clicked ${item.file_type}: ${item.name}. Requesting view change.`);
            let viewType = item.file_type; 
            if (item.file_type === 'doc') viewType = 'documents';
            else if (item.file_type === 'table') viewType = 'tables';
            else if (item.file_type === 'image') viewType = 'images';
            else if (item.file_type === 'media') viewType = 'media_note'; 

            dispatch('requestviewchange', { viewType, itemPath: item.path });
        }
    }

    let showSearchBox = false;
    let searchQuery = '';

    function handleSearchClick(event) {
      event.stopPropagation();
      showSearchBox = true;
      setTimeout(() => document.getElementById('notes-search-input')?.focus(), 0);
    }

    function handleSearchClear(event) {
      event.stopPropagation();
      searchQuery = '';
      setTimeout(() => document.getElementById('notes-search-input')?.focus(), 0);
    }

    onMount(() => {
      const listener = (e) => {
        const input = document.getElementById('notes-search-input');
        if (!showSearchBox) return;
        if (input && (e.target === input || input.contains(e.target))) return;
        if (searchQuery.trim() === '') {
          showSearchBox = false;
        }
      };
      document.addEventListener('click', listener);
      return () => document.removeEventListener('click', listener);
    });

    $: selectedItemPathInStore = $project.selectedDocumentPath || $project.currentImportedTranscriptPath || $project.selectedMediaNotePath;

    // Reactive statement to load metadata when selectedItemPathInStore changes
    $: {
        if (selectedItemPathInStore) {
            // When selected item changes, exit editing mode.
            if (isEditing) isEditing = false;
            loadMetadata(selectedItemPathInStore);
        } else {
            currentFileMetadata = null;
            fullLoadedMetadataObject = null;
            isEditing = false; // Ensure editing is off if no file selected
        }
    }

    async function loadMetadata(filePath) {
        fullLoadedMetadataObject = null; // Reset before loading
        currentFileMetadata = null; // Reset before loading
        let metadataPath; // Declare here to be accessible in catch
        try {
            console.log(`[NotesLeftPanel DEBUG] loadMetadata called with filePath: ${filePath}`);
            const dirName = await dirname(filePath);
            console.log(`[NotesLeftPanel DEBUG] dirname resolved to: ${dirName}`);
            const baseName = await basename(filePath); // e.g., "Test.png"
            console.log(`[NotesLeftPanel DEBUG] basename resolved to: ${baseName}`);
            const originalExtension = await extname(baseName); // e.g., "png" (no dot)
            console.log(`[NotesLeftPanel DEBUG] originalExtension resolved to: ${originalExtension}`);
            let fileNameWithoutExtension;
            if (originalExtension.length > 0) {
                // Remove the extension and the dot before it
                fileNameWithoutExtension = baseName.substring(0, baseName.length - originalExtension.length - 1);
            } else {
                // No extension, use baseName as is
                fileNameWithoutExtension = baseName;
            }
            console.log(`[NotesLeftPanel DEBUG] fileNameWithoutExtension: ${fileNameWithoutExtension}`);

            const currentSep = sep();
            console.log(`[NotesLeftPanel DEBUG] path.sep resolved to: ${currentSep}`);

            const metadataFileName = `.${fileNameWithoutExtension}.metadata.json`; // e.g., ".Test.metadata.json"
            console.log(`[NotesLeftPanel DEBUG] constructed metadataFileName: ${metadataFileName}`);

            metadataPath = `${dirName}${currentSep}${metadataFileName}`; // Ensure metadataPath is declared to be accessible in catch

            console.log(`[NotesLeftPanel] Loading metadata from: ${metadataPath}`);

            const fileContents = await readTextFile(metadataPath);
            const parsed = JSON.parse(fileContents);


            if (parsed && parsed.metadata) {
                fullLoadedMetadataObject = parsed;
                currentFileMetadata = fullLoadedMetadataObject.metadata;
                console.log('[NotesLeftPanel] Full metadata object loaded. Metadata part:', currentFileMetadata);
            } else {
                console.warn('[NotesLeftPanel] Metadata file does not contain a "metadata" property or is empty:', metadataPath);
            }
        } catch (error) {
            let errorMessage = typeof error === 'string' ? error : (error.message || JSON.stringify(error));
            if (errorMessage.includes('os error 2') || errorMessage.toLowerCase().includes('no such file or directory')) {
                 console.log(`[NotesLeftPanel] Metadata file not found for ${filePath} at actual path: ${metadataPath}. This is normal if no metadata has been saved yet. Error: ${errorMessage}`);
            } else {
                console.error(`[NotesLeftPanel] Error loading or parsing metadata for ${filePath} at actual path: ${metadataPath}. Error:`, error);
            }
        }
        if (!currentFileMetadata) isEditing = false; // Turn off editing if metadata load failed
    }

    // Populate editableMetadata when entering edit mode
    $: if (isEditing && currentFileMetadata) {
        editableMetadata.file_name = currentFileMetadata.file_name || '';
        editableMetadata.title = currentFileMetadata.title || '';
        editableMetadata.description = currentFileMetadata.description || '';
        editableMetadata.summary = currentFileMetadata.summary || '';
    } else if (!isEditing) {
        // Optional: Clear or reset editableMetadata when not editing, though it's repopulated on edit start
        editableMetadata = { file_name: '', title: '', description: '', summary: '' };
    }

    function toggleEditMode() {
        isEditing = !isEditing;
    }

    async function handleSaveMetadata() {
        if (!currentFileMetadata || !currentFileMetadata.file_path || !fullLoadedMetadataObject) {
            console.error('[NotesLeftPanel] Save error: Missing critical metadata or file path info.');
            await message('Cannot save: Critical metadata information is missing. Please try reloading the file.', { title: 'Save Error', type: 'error' });
            return;
        }

        try {
            const originalFilePath = currentFileMetadata.file_path;
            const originalFileNameWithExtension = currentFileMetadata.file_name;
            const originalFileExtension = await extname(originalFileNameWithExtension); // e.g., "png"

            let originalFileNameWithoutExtension;
            if (originalFileExtension.length > 0) {
                originalFileNameWithoutExtension = originalFileNameWithExtension.substring(0, originalFileNameWithExtension.length - originalFileExtension.length - 1);
            } else {
                originalFileNameWithoutExtension = originalFileNameWithExtension;
            }

            const editedFileNameWithoutExtension = editableMetadata.file_name.trim();
            // Construct new full filename carefully based on whether an extension exists
            const newFileNameWithExtension = originalFileExtension.length > 0
                ? editedFileNameWithoutExtension + "." + originalFileExtension
                : editedFileNameWithoutExtension;

            const originalDir = await dirname(originalFilePath);
            const originalMetadataPath = `${originalDir}${sep()}.${originalFileNameWithoutExtension}.metadata.json`; // Corrected

            let wasRenamed = false;
            let finalFilePath = originalFilePath;
            let finalMetadataPath = originalMetadataPath;

            if (editedFileNameWithoutExtension !== originalFileNameWithoutExtension) {
                if (!editedFileNameWithoutExtension) {
                    await message('File name cannot be empty.', { title: 'Invalid File Name', type: 'error' });
                    return; // Keep editing mode
                }
                const userConfirmedRename = await confirm(
                    `The file name has changed.\nOld: '${originalFileNameWithoutExtension}'\nNew: '${editedFileNameWithoutExtension}'\n\nDo you want to rename the actual file and its metadata file?`,
                    { title: 'Confirm File Rename', okLabel: 'Rename', cancelLabel: 'Cancel' }
                );

                if (userConfirmedRename) {
                    const newFilePath = `${originalDir}${sep()}${newFileNameWithExtension}`;
                    // editedFileNameWithoutExtension is already available
                    const newMetadataPath = `${originalDir}${sep()}.${editedFileNameWithoutExtension}.metadata.json`; // Corrected
                    try {
                        console.log(`[NotesLeftPanel] Renaming actual file from ${originalFilePath} to ${newFilePath}`);
                        await rename(originalFilePath, newFilePath);
                        console.log(`[NotesLeftPanel] Renaming metadata file from ${originalMetadataPath} to ${newMetadataPath}`);
                        await rename(originalMetadataPath, newMetadataPath);

                        wasRenamed = true;
                        finalFilePath = newFilePath;
                        finalMetadataPath = newMetadataPath;

                        console.log('[NotesLeftPanel] File and metadata renamed successfully.');
                        // projectService.renameItem would be better here to update all project XMLs etc.
                        // For now, direct store update for selected path:
                         if ($project.selectedDocumentPath === originalFilePath) project.update(p => ({...p, selectedDocumentPath: finalFilePath}));
                         if ($project.currentImportedTranscriptPath === originalFilePath) project.update(p => ({...p, currentImportedTranscriptPath: finalFilePath}));
                         if ($project.selectedMediaNotePath === originalFilePath) project.update(p => ({...p, selectedMediaNotePath: finalFilePath}));

                        // dispatch('projectfileschanged'); // Ideal scenario to trigger full refresh if paths changed globally

                    } catch (err) {
                        console.error(`[NotesLeftPanel] Error renaming file/metadata:`, err);
                        await message(`Error renaming file: ${err.message || err}. Other changes have not been saved.`, { title: 'Rename Failed', type: 'error' });
                        return; // Keep editing mode
                    }
                } else {
                    // User cancelled rename, revert filename in form and abort save.
                    editableMetadata.file_name = originalFileNameWithoutExtension;
                    await message('Rename cancelled. Other changes were not saved.', { title: 'Rename Cancelled', type: 'info' });
                    isEditing = false;
                    return;
                }
            }

            let updatedFileMetadata = { ...currentFileMetadata };
            updatedFileMetadata.title = editableMetadata.title.trim();
            updatedFileMetadata.description = editableMetadata.description.trim();
            updatedFileMetadata.summary = editableMetadata.summary.trim();
            updatedFileMetadata.last_modified = new Date().toISOString();

            if (wasRenamed) {
                updatedFileMetadata.file_name = newFileNameWithExtension;
                updatedFileMetadata.file_path = finalFilePath;
            }

            let objectToWrite = { ...fullLoadedMetadataObject };
            objectToWrite.metadata = updatedFileMetadata;
            objectToWrite.version = objectToWrite.version || "1.0"; // Ensure version if missing
            objectToWrite.last_modified_harvey = new Date().toISOString(); // Update overall object mod time

            await writeTextFile(finalMetadataPath, JSON.stringify(objectToWrite, null, 2));

            // Update current state after successful save
            currentFileMetadata = { ...updatedFileMetadata }; // Make sure it's a new object for reactivity if needed elsewhere
            fullLoadedMetadataObject = objectToWrite; // Keep the full object in sync

            isEditing = false;
            await message('Metadata saved successfully!', { title: 'Success' });

        } catch (err) {
            console.error('[NotesLeftPanel] Error saving metadata:', err);
            await message(`Error saving metadata: ${err.message || err}. Please check console for details.`, { title: 'Save Failed', type: 'error' });
            // Do not exit editing mode if save fails
        }
    }

    const EDIT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-pencil-square" viewBox="0 0 16 16"><path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/><path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/></svg>`;
    const CANCEL_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16"><path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/></svg>`;

</script>

<div class="h-full bg-white dark:bg-gray-800 rounded-md shadow p-3 flex flex-col overflow-hidden">
	<h2 class="relative flex items-center text-sm font-semibold mb-3 border-b pb-1 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300">
		<span>Fieldnotes</span>
    {#if !showSearchBox}
      <button
        type="button"
        class="absolute inset-y-0 right-0 p-2 flex items-center justify-center z-20 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
        on:click|stopPropagation={handleSearchClick}
        title="Search Fieldnotes"
      >
        {@html `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-search" viewBox="0 0 16 16"><path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001q.044.06.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1 1 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0"/></svg>`}
      </button>
    {:else}
      {#if searchQuery.trim() !== ''}
        <button
          type="button"
          class="absolute inset-y-0 right-0 p-2 flex items-center justify-center z-20 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
          on:click|stopPropagation={handleSearchClear}
          title="Clear Search"
        >
          {@html `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-x" viewBox="0 0 16 16"><path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/></svg>`}
        </button>
      {/if}
    {/if}
    <input
      id="notes-search-input"
      bind:value={searchQuery}
      type="text"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="off"
      spellcheck="false"
      placeholder="Search..."
      class="absolute inset-y-0 left-0 right-0 z-10 transition-all duration-300 ease-out border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-xs pl-2 pr-10 {showSearchBox ? 'opacity-100 w-full' : 'opacity-0 w-0'}"
      on:click|stopPropagation
    />
	</h2>

	<div class="flex-grow overflow-y-auto min-h-0 -mr-2 pr-2">
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
                 {#if category.type !== 'Videos'} <hr class="border-gray-200 dark:border-gray-700 my-1"> {/if}
            {/each}
        </ul>
        {#if $project.isLoading} <p class="text-xs text-gray-500 dark:text-gray-400 italic px-1 py-2">Loading project data...</p> {/if}
	</div>

    <!-- Metadata Display Section -->
    <div class="mt-4 pt-4 border-t border-gray-300 dark:border-gray-600 overflow-y-auto text-xs relative">
        {#if currentFileMetadata}
            <button
                on:click={toggleEditMode}
                class="absolute top-2 right-1 p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                title={isEditing ? 'Cancel Edit' : 'Edit Metadata'}
            >
                {@html isEditing ? CANCEL_ICON_SVG : EDIT_ICON_SVG}
            </button>

            <div class="p-1 space-y-2">
                <h3 class="font-semibold text-gray-700 dark:text-gray-300 mb-2">File Metadata</h3>

                <div class="mb-2">
                    <span class="font-medium text-gray-600 dark:text-gray-400 block mb-0.5">File Name:</span>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.file_name} class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs" />
                    {:else}
                        <span class="ml-1 text-gray-800 dark:text-gray-200">{currentFileMetadata.file_name || 'N/A'}</span>
                    {/if}
                </div>

                <div>
                    <span class="font-medium text-gray-600 dark:text-gray-400">File Path:</span>
                    <span class="ml-1 text-gray-800 dark:text-gray-200 break-all">{currentFileMetadata.file_path || 'N/A'}</span>
                </div>

                <div>
                    <span class="font-medium text-gray-600 dark:text-gray-400">Last Modified:</span>
                    <span class="ml-1 text-gray-800 dark:text-gray-200">{currentFileMetadata.last_modified ? new Date(currentFileMetadata.last_modified).toLocaleString() : 'N/A'}</span>
                </div>

                <div class="mb-2">
                    <span class="font-medium text-gray-600 dark:text-gray-400 block mb-0.5">Title:</span>
                    {#if isEditing}
                        <input type="text" bind:value={editableMetadata.title} class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs" />
                    {:else}
                        <span class="ml-1 text-gray-800 dark:text-gray-200">{currentFileMetadata.title || 'N/A'}</span>
                    {/if}
                </div>

                <div class="mb-2">
                    <span class="font-medium text-gray-600 dark:text-gray-400 block mb-0.5">Description:</span>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.description} rows="3" class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs"></textarea>
                    {:else}
                        <span class="ml-1 text-gray-800 dark:text-gray-200 whitespace-pre-wrap">{currentFileMetadata.description || 'N/A'}</span>
                    {/if}
                </div>

                <div class="mb-2">
                    <span class="font-medium text-gray-600 dark:text-gray-400 block mb-0.5">Summary:</span>
                    {#if isEditing}
                        <textarea bind:value={editableMetadata.summary} rows="2" class="mt-0.5 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white p-1 text-xs"></textarea>
                    {:else}
                        <span class="ml-1 text-gray-800 dark:text-gray-200 whitespace-pre-wrap">{currentFileMetadata.summary || 'N/A'}</span>
                    {/if}
                </div>

                {#if isEditing}
                    <div class="mt-3 flex justify-end">
                        <button
                            on:click={handleSaveMetadata}
                            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-xs font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
                        >
                            Save
                        </button>
                    </div>
                {/if}
            </div>
        {:else}
            <p class="text-gray-500 dark:text-gray-400 italic px-1 py-2">No metadata loaded or file not selected.</p>
        {/if}
    </div>

	{#if contextMenuVisible && contextMenuItem}
		<div id="notes-left-panel-context-menu" class="fixed z-50 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[120px]" style="left: {contextMenuX}px; top: {contextMenuY}px;" on:click|stopPropagation>
            {#if contextMenuItem.file_type === 'media'}
                <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Transcribe'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Transcribe</button>
                <hr class="my-1 border-gray-200 dark:border-gray-600" />
                <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'doc'}
                 {#if contextMenuItem.name?.toLowerCase().endsWith('.pdf')}
                     <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open Externally</button>
                 {:else}
                     <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 {/if}
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'table'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'image'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else if contextMenuItem.file_type === 'imported_transcript'}
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Open'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
                 <hr class="my-1 border-gray-200 dark:border-gray-600" />
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Rename'); }} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename</button>
                 <button on:click|stopPropagation={() => { handleContextMenuAction('Delete'); }} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete</button>
            {:else}
                 <span class="block w-full text-left px-3 py-1.5 text-gray-400 dark:text-gray-500 italic">No actions available</span>
            {/if}
		</div>
	{/if}
    {#if categoryContextMenuVisible}
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

<FileRenameModal bind:showModal={showRenameModal} currentName="{itemToRename?.name || ''}" itemType="{itemToRename?.file_type || ''}" isMediaRename="{itemToRename?.file_type === 'media'}" on:confirm={handleRenameConfirm} on:close={handleRenameModalClose} />
<ImportTranscriptSourceModal bind:showModal={showImportTranscriptModal} on:confirm={handleImportTranscriptConfirm} on:close={() => showImportTranscriptModal = false} />


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