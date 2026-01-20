// src/lib/services/projectService.js
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { open, confirm, message } from '@tauri-apps/plugin-dialog';
import { get } from 'svelte/store';
import { v4 as uuidv4 } from 'uuid';
import {
	$getRoot as _getRoot,
    $createParagraphNode as _createParagraphNode,
	$createTextNode as _createTextNode,
    $createLineBreakNode as _createLineBreakNode,
	$isElementNode as _isElementNode,
	$isTextNode as _isTextNode,
	$parseSerializedNode as _parseSerializedNode,
    ParagraphNode, RootNode, TextNode, LineBreakNode, ElementNode
} from 'lexical';
import {
    $createTableNode as _createTableNode,
    $createTableRowNode as _createTableRowNode,
    $createTableCellNode as _createTableCellNode,
    $isTableNode as _isTableNode,
    $isTableRowNode as _isTableRowNode,
    $isTableCellNode as _isTableCellNode,
    TableNode, TableRowNode, TableCellNode
} from '@lexical/table';
import {
    $createHeadingNode as _createHeadingNode,
    HeadingNode, QuoteNode,
    $isHeadingNode as _isHeadingNode
} from '@lexical/rich-text';
import {
    $isListNode as _isListNode,
    ListNode, ListItemNode,
    $isListItemNode as _isListItemNode
} from '@lexical/list';
import { createHeadlessEditor } from '@lexical/headless';
import {
    $generateHtmlFromNodes as _generateHtmlFromNodes,
    $generateNodesFromDOM as _generateNodesFromDOM
} from '@lexical/html';

import { LinkNode, $isLinkNode as _isLinkNode } from '@lexical/link';
import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';

import { dirname, basename, sep } from '@tauri-apps/api/path';

import {
	project,
    prepareDocumentView,
    setLoadedDocumentData,
    setDocumentLoadFailed,
    markDocumentAsSaved,
    markDocumentChangesDiscarded,
    clearDocumentEditorState,

    markDocumentMetadataAsSaved,
    markPdfAnnotationsAsSaved,

    prepareImportedTranscriptView,
    markImportedTranscriptChangesDiscarded,

	showUnsavedChangesPrompt,
    hideUnsavedChangesPrompt,
    setAssetImportStatus,
    showConversionPrompt,
    hideConversionPrompt,

    setLoadedPdfAnnotations,
    setPdfAnnotationsLoadFailed,

    prepareMediaNoteView,
    markMediaNoteTranscriptChangesDiscarded
} from '$lib/stores/projectStore.js';

import {
    transcriptStore,
    setTranscriptData,
    toggleTranscribeModal,
    setTranscriptionStatus,
    updateTranscriptionProgress,
    clearTranscriptionStatus,
    selectMedia, // Ensure selectMedia is imported
    clearTranscriptState,
    markTranscriptAsSaved,
    prepareForNewTranscription, // Import the function directly
    setTranslationStatus,
    toggleTranslateModal,
    updateTranslationProgress,
} from '$lib/stores/transcriptStore.js';

import notificationStore from '$lib/stores/notificationStore.js';

export function normalizePath(path) {
    if (typeof path !== 'string') {
        return path;
    }
    // On Windows, paths may start with the `\\?\` prefix. This removes it.
    let normalized = path.startsWith('\\\\?\\') ? path.substring(4) : path;

    // Normalize backslashes to forward slashes for consistent path handling.
    normalized = normalized.replace(/\\/g, '/');

    return normalized;
}

/**
 * Updates the name and description for a specific tag.
 * @param {string} projectId - The ID of the project.
 * @param {string} projectRootPath - The root path of the project.
 * @param {string} oldName - The current name of the tag.
 * @param {string} newName - The new name for the tag.
 * @param {string} newDescription - The new description for the tag.
 */
export async function updateTag(projectId, projectRootPath, oldName, newName, newDescription) {
    return await invoke('update_tag', {
        projectId,
        projectRootPathStr: projectRootPath,
        oldName,
        newName,
        newDescription,
    });
}

export async function saveTableLayoutPrefs(tablePath, layoutJson) {
    const currentProject = get(project);
    const projectId = currentProject.id;

    if (!tablePath || !layoutJson) {
        console.error('[ProjectService] saveTableLayoutPrefs: Missing tablePath or layoutJson.');
        throw new Error('Missing tablePath or layoutJson for saving table layout preferences.');
    }
    if (!projectId) {
        console.error('[ProjectService] saveTableLayoutPrefs: Missing projectId.');
        throw new Error('Missing projectId for saving table layout preferences.');
    }

    try {
        await invoke('save_table_layout_prefs', { projectId, tablePath, layoutJson });
    } catch (error) {
        console.error(`[ProjectService] Error saving table layout preferences for ${tablePath}:`, error);
        throw error;
    }
}

export async function loadHighlightsForFile(filePath, itemType) {
    if (!filePath || !itemType) {
        console.warn('[ProjectService] loadHighlightsForFile called with missing filePath or itemType.');
        return;
    }

    // Determine the correct loading function based on itemType
    if (itemType === 'doc' && filePath.toLowerCase().endsWith('.pdf')) {
        await loadPdfAnnotationsFromFile(filePath);
    } else if (itemType === 'images') {
        await loadImageAnnotations(filePath);
    } else if (itemType === 'tables' || itemType === 'table') {
        await loadTableHighlights(filePath);
    } else if (itemType === 'imported_transcript') {
        // Assuming there's a function to load highlights for imported transcripts
        // If not, this part needs to be implemented. For now, let's log it.
        console.log(`[ProjectService] Highlight loading for 'imported_transcript' is not yet implemented.`);
    } else { // 'doc' (non-PDF), etc.
        const metadata = await loadDocumentMetadata(filePath);
        if (metadata && metadata.highlights) {
            const { setDocumentHighlights } = await import('$lib/stores/projectStore.js');
            setDocumentHighlights(metadata.highlights);
        } else {
            console.log(`[ProjectService] No highlights found for document type '${itemType}'.`);
        }
    }
}

export async function deleteTableColumn(tablePath, columnName) {
    if (!tablePath || !columnName) {
        throw new Error("Missing required parameters for deleting table column.");
    }

    try {
        await invoke('delete_table_column', {
            tablePathStr: tablePath,
            columnNameToDelete: columnName
        });
    } catch (error) {
        const errorMessage = error.message || String(error);
        await message(`Error deleting column: ${errorMessage}`, { title: 'Delete Column Error', type: 'error' });
        throw error;
    }
}

/**
 * Saves the style information for a specific table.
 * @param {string} tablePath - The absolute path to the table file.
 * @param {object} styles - The style object to save.
 * @returns {Promise<void>}
 */
export async function saveTableStyles(tablePath, styles) {
    try {
        await invoke('save_table_styles', { filePath: tablePath, styles: JSON.stringify(styles) });
    } catch (error) {
        console.error(`Failed to save styles for table ${tablePath}:`, error);
        throw error;
    }
}

/**
 * Loads the style information for a specific table.
 * @param {string} tablePath - The absolute path to the table file.
 * @returns {Promise<object|null>} The loaded style object, or null if not found.
 */
export async function loadTableStyles(tablePath) {
    try {
        const styles = await invoke('load_table_styles', { filePath: tablePath });
        if (styles) {
            if (typeof styles === 'string') {
                const parsedStyles = JSON.parse(styles);
                return parsedStyles;
            }
            return styles;
        }
        return null;
    } catch (error) {
        console.error(`Failed to load styles for table ${tablePath}:`, error);
        // It's common for styles to not exist, so we don't re-throw.
        // We just log the error and return null.
        return null;
    }
}

import { setLoadedTableHighlights, setTableHighlightsLoadFailed, markTableHighlightsAsSaved } from '$lib/stores/projectStore.js';

export async function loadTableHighlights(filePath) {
    if (!filePath) {
        setLoadedTableHighlights([]);
        return;
    }
    try {
        const highlights = await invoke("load_table_styles", { filePath });
        setLoadedTableHighlights(highlights || []);
    } catch (error) {
        console.error(`Error loading table highlights for ${filePath}:`, error);
        setTableHighlightsLoadFailed(filePath, error.message || String(error));
    }
}

export async function saveTableHighlights() {
    const projState = get(project);
    const tablePath = projState.selectedDocumentPath;
    const highlights = projState.currentTableHighlights;

    if (!tablePath || !projState.isTableHighlightsDirty) {
        return;
    }

    try {
        await invoke('save_table_styles', {
            filePath: tablePath,
            styles: JSON.stringify(highlights)
        });
        markTableHighlightsAsSaved();
        console.log(`[ProjectService] Table highlights saved for ${tablePath}`);
    } catch (error) {
        console.error(`[ProjectService] Error saving table highlights for ${tablePath}:`, error);
        notificationStore.add(`Error saving table highlights: ${error.message || error}`, 'error');
    }
}

export async function createNewDocument(projectXmlPath) {
    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot create document: Project XML path is missing.');
        await message('Project data is not fully loaded. Cannot create documents.', { title: 'Create Error', type: 'error' });
        return;
    }

    try {
        const newDocument = await invoke('create_new_document', {
            projectXmlPath: projectXmlPath,
            documentName: "Untitled.json"
        });

        await refreshProjectFiles();

        prepareDocumentView(newDocument, 'documents');
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        await message(`Error creating document: ${errorMessage}`, { title: 'Create Error', type: 'error' });
    }
}

export async function loadTableLayoutPrefs(tablePath) {
    const currentProject = get(project);
    const projectId = currentProject.id;

    if (!tablePath) {
        console.error('[ProjectService] loadTableLayoutPrefs: Missing tablePath.');
        return null;
    }
    if (!projectId) {
        console.error('[ProjectService] loadTableLayoutPrefs: Missing projectId.');
        return null;
    }

    try {
        const layoutJson = await invoke('load_table_layout_prefs', { projectId, tablePath });
        if (layoutJson) {
            return JSON.parse(layoutJson);
        }
        return null;
    } catch (error) {
        console.error(`[ProjectService] Error loading table layout preferences for ${tablePath}:`, error);
        return null;
    }
}

function findMediaPathByName(nodes, filename) {
  if (!Array.isArray(nodes)) return null;
  for (const node of nodes) {
    if (node.file_type === 'media' && !node.is_directory && node.name === filename) {
      return node.path;
    }
    if (node.children) {
      const found = findMediaPathByName(node.children, filename);
      if (found) return found;
    }
  }
  return null;
}

const HARVEY_FILES_DIR = 'harvey_files';
const MEDIA_DIR_NAME = 'Media';
const MEDIA_SUBDIR = 'media';
const DOCS_DIR_NAME = 'Documents';
const TRANSCRIPTS_SUBDIR_MEDIA = 'transcripts';
const TRANSCRIPTS_DIR_IMPORTED = 'Transcripts';
const TABLES_DIR_NAME = 'Tables';
const IMAGES_DIR_NAME = 'Images';

const audioExtensions = ['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac'];
const videoExtensions = ['mp4', 'mov', 'avi', 'mkv', 'webm'];
const allMediaExtensions = [...audioExtensions, ...videoExtensions];
const audioFilter = { name: 'Audio Files', extensions: audioExtensions };
const videoFilter = { name: 'Video Files', extensions: videoExtensions };
const allMediaFilter = { name: 'All Supported Media', extensions: allMediaExtensions };

const documentExtensions = ['docx', 'txt', 'md', 'pdf', 'rtf'];
const documentFilter = { name: 'Documents', extensions: documentExtensions };

const tableExtensions = ['csv', 'xlsx'];
const tableFilter = { name: 'Table Files', extensions: tableExtensions };

const imageExtensions = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff'];
const imageFilter = { name: 'Image Files', extensions: imageExtensions };

const wordDocumentFilter = { name: 'Word Documents', extensions: ['docx'] };


const ALL_EDITOR_NODES = [
    RootNode, ParagraphNode, TextNode, ExtendedTextNode, LineBreakNode,
    HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode,
    TableNode, TableRowNode, TableCellNode
];

function createConversionEditor(instanceId) {
    return createHeadlessEditor({
        nodes: ALL_EDITOR_NODES,
        namespace: `html-converter-${instanceId}-${Math.random()}`,
        onError: (e) => console.error(`[Lexical HTML Converter ${instanceId}] Error:`, e)
    });
}

export async function loadProjectDataAndUpdateStore(projectXmlPath, targetPathToSelect = null) {
    if (!projectXmlPath || projectXmlPath.trim() === '') {
        console.error('[ProjectService] loadProjectDataAndUpdateStore called without a valid projectXmlPath');
        project.update((current) => ({ ...current, isLoading: false, error: 'Project path is missing.', statusMessage: 'Error: Project path is missing.' }));
        throw new Error('projectXmlPath is required');
    }
    try {
        // Clear any existing transcript state to prevent leakage from previous projects
        clearTranscriptState();

        const loadedData = await invoke('load_project_data', { projectXmlPath });
        
        const normalizedBaseDirectory = normalizePath(loadedData.base_directory);

        if (Array.isArray(loadedData.files)) {
          const attachTranscripts = (nodes) => {
            for (const node of nodes) {
              // Normalize the node's own path
              if (node.path) {
                node.path = normalizePath(node.path);
              }

              if (node.file_type === 'media') {
                // Ensure node.associated_transcripts is an array before mapping
                node.associated_transcripts = Array.isArray(node.associated_transcripts) ? node.associated_transcripts : [];
                node.associated_transcripts = node.associated_transcripts.map(t => {
                    let absolutePath = null;
                    let name = t.name; // Preserve existing name if available
                    if (normalizedBaseDirectory && typeof normalizedBaseDirectory === 'string' &&
                        t.relativePath && typeof t.relativePath === 'string') {
                        // Ensure no double slashes if base_directory ends with one and relativePath starts with one (though unlikely for relativePath)
                        const base = normalizedBaseDirectory.endsWith('/') || normalizedBaseDirectory.endsWith('\\')
                                   ? normalizedBaseDirectory.slice(0, -1)
                                   : normalizedBaseDirectory;
                        const rel = t.relativePath.startsWith('/') || t.relativePath.startsWith('\\')
                                    ? t.relativePath.substring(1)
                                    : t.relativePath;
                        absolutePath = normalizePath(`${base}/${rel}`);
                        if (!name) { // If name is not provided by backend, derive from relativePath
                            name = t.relativePath.split(/[\\/]/).pop();
                        }
                    } else {
                        // If base_directory or relativePath is missing, we can't form a full path.
                        // Log this, as it indicates an issue with the data from the backend or project structure.
                        console.warn(`[ProjectService] Cannot construct absolute path for transcript. Base dir: ${normalizedBaseDirectory}, Relative path: ${t.relativePath}`);
                        if (!name) { // If name is not provided and path construction failed, use relativePath as fallback
                            name = t.relativePath;
                        }
                    }
                    return {
                        path: normalizePath(absolutePath), // Normalize absolutePath here
                        relativePath: t.relativePath, // Always preserve the original relativePath
                        language_code: t.language_code, // Pass the language code
                        name: name // Add the name property
                    };
                });
              }
              if (Array.isArray(node.children)) {
                attachTranscripts(node.children);
              }
            }
          };
          attachTranscripts(loadedData.files);
        }

        const dataToSet = {
            name: loadedData.project_name,
            id: loadedData.project_uuid,
            xmlPath: loadedData.project_xml_path,
            baseDirectory: normalizedBaseDirectory,
            files: loadedData.files || [],
            documentFiles: loadedData.document_files || [],
            tableFiles: loadedData.table_files || [],
            imageFiles: loadedData.image_files || [],
            importedTranscriptFiles: loadedData.imported_transcript_files || [],
            documentMetadataFiles: loadedData.document_metadata_files || [],
            isLoading: false,
            error: null,
            statusMessage: `Loaded project: ${loadedData.project_name}`
        };
        project.update((current) => ({
            ...current,
            ...dataToSet
        }));

        // Update project groups list
        try {
            const { updateProjectGroupsList } = await import('$lib/stores/projectStore.js');
            if (loadedData.project_uuid) { // Ensure project_uuid (as id) is available
                await updateProjectGroupsList(loadedData.project_uuid);
            } else {
                console.warn("[ProjectService] Project UUID not available after loading, cannot update groups list.");
            }
        } catch (e) {
            console.error("[ProjectService] Error importing or calling updateProjectGroupsList:", e);
        }

        await emit('project-view-ready', { projectXmlPath: projectXmlPath });

        let mediaFileToSelect = null;

        function findMediaByPathRecursive(nodes, path) {
            if (!Array.isArray(nodes) || !path) return null;
            for (const node of nodes) {
                if (node.file_type === 'media' && !node.is_directory && node.path === path) {
                    return node;
                }
                if (node.children && node.children.length > 0) {
                    const found = findMediaByPathRecursive(node.children, path);
                    if (found) return found;
                }
            }
            return null;
        }

        function findFirstMediaRecursive(nodes) {
            if (!Array.isArray(nodes)) return null;
            for (const node of nodes) {
                if (node.file_type === 'media' && !node.is_directory) { return node; }
                if (node.children && node.children.length > 0) {
                    const found = findFirstMediaRecursive(node.children);
                    if (found) return found;
                }
            }
            return null;
        }

        if (targetPathToSelect) {
            mediaFileToSelect = findMediaByPathRecursive(loadedData.files || [], targetPathToSelect);
            if (!mediaFileToSelect) {
                console.warn(`[ProjectService] Target media path ${targetPathToSelect} provided but not found. Falling back to first media.`);
                mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
            }
        } else {
            mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
        }

        if (mediaFileToSelect) {
            
            selectMedia(mediaFileToSelect);
        
            
        }
    } catch (error) {
        console.error('[ProjectService] Failed to load project data:', error);
        project.update((current) => ({ ...current, isLoading: false, error: error?.message || 'Unknown error loading project.', statusMessage: `Error loading project.` }));
        throw error;
    }
}

export async function silentlyRefreshProjectData(projectXmlPath) {
    if (!projectXmlPath || projectXmlPath.trim() === '') {
        console.error('[ProjectService] silentlyRefreshProjectData called without a valid projectXmlPath');
        project.update((current) => ({ ...current, isLoading: false, error: 'Project path is missing for silent refresh.', statusMessage: 'Error: Project path missing.' }));
        return;
    }
    project.update((current) => ({ ...current, isLoading: true, error: null, statusMessage: 'Refreshing project data silently...' }));
    try {
        const loadedData = await invoke('load_project_data', { projectXmlPath });

        if (Array.isArray(loadedData.files)) {
          const attachTranscripts = (nodes) => {
            for (const node of nodes) {
              if (node.file_type === 'media' && node.transcripts) {
                 node.transcripts = node.transcripts.map(t => {
                    let absolutePath = null;
                    let name = t.name; // Preserve existing name if available
                    if (loadedData.base_directory && typeof loadedData.base_directory === 'string' &&
                        t.relativePath && typeof t.relativePath === 'string') {
                        // Ensure no double slashes if base_directory ends with one and relativePath starts with one (though unlikely for relativePath)
                        const base = loadedData.base_directory.endsWith('/') || loadedData.base_directory.endsWith('\\')
                                   ? loadedData.base_directory.slice(0, -1)
                                   : loadedData.base_directory;
                        const rel = t.relativePath.startsWith('/') || t.relativePath.startsWith('\\')
                                    ? t.relativePath.substring(1)
                                    : t.relativePath;
                        absolutePath = normalizePath(`${base}/${rel}`);
                        if (!name) { // If name is not provided by backend, derive from relativePath
                            name = t.relativePath.split(/[\\/]/).pop();
                        }
                    } else {
                        // If base_directory or relativePath is missing, we can't form a full path.
                        // Log this, as it indicates an issue with the data from the backend or project structure.
                        console.warn(`[ProjectService] Cannot construct absolute path for transcript. Base dir: ${loadedData.base_directory}, Relative path: ${t.relativePath}`);
                        if (!name) { // If name is not provided and path construction failed, use relativePath as fallback
                            name = t.relativePath;
                        }
                    }
                    return {
                        path: absolutePath, // This will be null if construction failed
                        relativePath: t.relativePath, // Always preserve the original relativePath
                        language_code: t.language_code, // Pass the language code
                        name: name // Add the name property
                    };
                });
              }
              if (Array.isArray(node.children)) {
                attachTranscripts(node.children);
              }
            }
          };
          attachTranscripts(loadedData.files);
        }

        const preRefreshSelectedPath = get(transcriptStore).selectedMediaFile?.path;
        let foundMediaFileObjectFromNewList = null;

        if (preRefreshSelectedPath) {
            function findMediaByPathRecursive(nodes, path) {
                if (!Array.isArray(nodes) || !path) return null;
                for (const node of nodes) {
                    if (node.file_type === 'media' && !node.is_directory && node.path === path) {
                        return node;
                    }
                    if (node.children && node.children.length > 0) {
                        const foundChild = findMediaByPathRecursive(node.children, path);
                        if (foundChild) return foundChild;
                    }
                }
                return null;
            }
            foundMediaFileObjectFromNewList = findMediaByPathRecursive(loadedData.files || [], preRefreshSelectedPath);
        }

        if (foundMediaFileObjectFromNewList) {
            transcriptStore.update(ts => {
                if (get(transcriptStore).selectedMediaFile?.path === preRefreshSelectedPath || !get(transcriptStore).selectedMediaFile) {
                    return { ...ts, selectedMediaFile: foundMediaFileObjectFromNewList };
                }
                return ts;
            });
        } else if (preRefreshSelectedPath) {
             transcriptStore.update(ts => {
                if (ts.selectedMediaFile?.path === preRefreshSelectedPath) {
                    return { ...ts, selectedMediaFile: null };
                }
                return ts;
            });
        }

        const dataToSet = {
            name: loadedData.project_name,
            id: loadedData.project_uuid,
            xmlPath: loadedData.project_xml_path,
            baseDirectory: loadedData.base_directory,
            files: loadedData.files || [],
            documentFiles: loadedData.document_files || [],
            tableFiles: loadedData.table_files || [],
            imageFiles: loadedData.image_files || [],
            importedTranscriptFiles: loadedData.imported_transcript_files || [],
            documentMetadataFiles: loadedData.document_metadata_files || [],
            isLoading: false,
            error: null,
            statusMessage: 'File list updated.'
        };
        project.update((current) => ({
            ...current,
            ...dataToSet,
        }));

    } catch (error) {
        console.error('[ProjectService] Failed to silently refresh project data:', error);
        project.update((current) => ({ ...current, isLoading: false, error: error?.message || 'Unknown error refreshing project data.', statusMessage: 'Error refreshing project data.' }));
        throw error;
    }
}

export async function importMediaFile(importType = null) {
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import media: Project XML path missing.');
        await message('Project data is not fully loaded. Cannot import media.', { title: 'Import Error', type: 'error' });
        return;
    }
    try {
        let filters = [allMediaFilter, audioFilter, videoFilter];
        let dialogTitle = 'Import Media File';
        if (importType === 'audio') {
            filters = [audioFilter];
            dialogTitle = 'Import Audio File';
        } else if (importType === 'video') {
            filters = [videoFilter];
            dialogTitle = 'Import Video File';
        }
        const selected = await open({
            multiple: false,
            directory: false,
            filters: filters,
            title: dialogTitle
        });

        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Media import cancelled.' }));
            return;
        }

        const sourceFilePath = selected;
        const filename = await basename(sourceFilePath);

        const canProceed = await checkUnsavedChangesThenProceed(null, `importing media: ${filename}`);
        if (!canProceed) {
            setAssetImportStatus(false, 'Media import cancelled by user.');
            return;
        }

        setAssetImportStatus(true, `Importing ${filename}...`);

        const newlyImportedFileEntry = await invoke('import_media', { // backendResponse is now newlyImportedFileEntry
            sourceFilePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });

        if (!newlyImportedFileEntry || typeof newlyImportedFileEntry !== 'object' || !newlyImportedFileEntry.path) {
            console.error('[ProjectService] import_media returned invalid FileEntry:', newlyImportedFileEntry);
            setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);
            await message(`Error importing ${filename}: Backend returned invalid data.`, { title: 'Import Error', type: 'error' });
            // Attempt a refresh as a fallback, as the file might exist even if the entry wasn't returned correctly
            await refreshProjectFiles();
            return;
        }

        // Refresh the main file list in projectStore, and select the newly imported file.
        // This prevents the UI from selecting the first media file by default, which might have transcripts
        // and cause the UI to hang trying to load them for a file that doesn't have any.
        await refreshProjectFiles(newlyImportedFileEntry.path);

        // Ensure the correct view is active for the newly imported media.
        prepareMediaNoteView(newlyImportedFileEntry.path);
        console.log(`[ProjectService] Media imported. The new file has been selected. Path: ${newlyImportedFileEntry.path}`);

        setAssetImportStatus(false, `${filename} imported successfully.`);
        return newlyImportedFileEntry.path;

    } catch (error) {
        console.error('[ProjectService] Failed to import media file:', error);
        const errorMessage = error.message || String(error);
        await message(`Error importing media: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error importing media.`);
        // Ensure loading states are reset on error
        project.update(p => ({
            ...p,
            isImportingAsset: false,
            isLoading: false
        }));
    }
}

export async function importDocumentFile() {
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    const projectBaseDir = currentProject.baseDirectory;

    if (!projectXmlPath || !projectBaseDir) {
        console.error('[ProjectService] Cannot import document: Project data not fully loaded.');
        await message('Project data is not fully loaded. Cannot import documents.', { title: 'Import Error', type: 'error' });
        return;
    }

     const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a document");
     if (!canProceedDialog) {
         setAssetImportStatus(false, 'Document import cancelled by user.');
         return;
     }

    let sourceFilePath = '';
    let backendResultPathAndOriginalFilename = '';
    let finalJsonPath = '';
    let finalJsonName = '';

    try {
        const selected = await open({ multiple: false, directory: false, filters: [documentFilter], title: 'Import Document File' });
        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Document import cancelled.' }));
            return;
        }
        sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);
        const sourceFilenameStem = sourceFilename.includes('.') ? sourceFilename.substring(0, sourceFilename.lastIndexOf('.')) : sourceFilename;
        const sourceExtension = (sourceFilename.includes('.') ? sourceFilename.substring(sourceFilename.lastIndexOf('.') + 1) : '').toLowerCase();

        const needsConversionPrompt = ['docx', 'rtf'].includes(sourceExtension);
        if (needsConversionPrompt) {
            const conversionConfirmed = await new Promise((resolve) => {
                showConversionPrompt(sourceFilename, () => { hideConversionPrompt(); resolve(true); }, () => { hideConversionPrompt(); resolve(false); });
            });
            if (!conversionConfirmed) {
                 project.update(p => ({ ...p, statusMessage: 'Document import cancelled.' }));
                 return;
            }
        }

        setAssetImportStatus(true, `Importing ${sourceFilename}...`);

        backendResultPathAndOriginalFilename = await invoke('import_document', { sourcePathStr: sourceFilePath, projectXmlPathStr: projectXmlPath });
        let tempHtmlPath = backendResultPathAndOriginalFilename;
        if (backendResultPathAndOriginalFilename.includes("|original_filename:")) {
            tempHtmlPath = backendResultPathAndOriginalFilename.split("|original_filename:")[0];
        }

        if (tempHtmlPath && tempHtmlPath.toLowerCase().endsWith('.pdf')) {
            await refreshProjectFiles();
            const importedPdfName = await basename(tempHtmlPath);
            setAssetImportStatus(false, `Document "${importedPdfName}" imported successfully.`);
            prepareDocumentView(tempHtmlPath, 'documents');
            return;
        }
        if (!tempHtmlPath || !tempHtmlPath.toLowerCase().endsWith('.html')) throw new Error("Backend did not return expected temporary HTML path.");

        const htmlContent = await invoke('read_file_content', { path: tempHtmlPath });
        try { await invoke('delete_temporary_file', { path: tempHtmlPath }); } catch(delErr) { console.warn(`[ProjectService] Failed to delete temp HTML: ${tempHtmlPath}`); }

        let lexicalJsonString = '';
        const conversionEditor = createConversionEditor('import-doc');
        try {
            const domParser = new DOMParser(); const dom = domParser.parseFromString(htmlContent, 'text/html');
            await conversionEditor.update(() => { const nodes = _generateNodesFromDOM(conversionEditor, dom); _getRoot().clear(); _getRoot().append(...nodes); });
            const editorState = conversionEditor.getEditorState();
            if (editorState.isEmpty()) {
                 conversionEditor.update(() => { _getRoot().clear(); const para = _createParagraphNode(); para.append(_createTextNode(`[Content from ${sourceFilename} could not be fully parsed] `)); _getRoot().append(para); });
            }
            lexicalJsonString = JSON.stringify(conversionEditor.getEditorState().toJSON(), null, 2);
        } catch (lexicalError) {
            const errorEditor = createConversionEditor('import-error');
            errorEditor.update(() => { _getRoot().clear(); const p = _createParagraphNode(); p.append(_createTextNode(`Error importing content from ${sourceFilename}: ${lexicalError.message || lexicalError}`)); _getRoot().append(p); });
            lexicalJsonString = JSON.stringify(errorEditor.getEditorState().toJSON(), null, 2);
        }
        if (!lexicalJsonString) throw new Error("Failed to generate Lexical JSON from HTML.");

        const docsFolderPath = `${projectBaseDir}/${HARVEY_FILES_DIR}/${DOCS_DIR_NAME}/${sourceFilenameStem}`;
        finalJsonPath = `${docsFolderPath}/${sourceFilenameStem}.json`;
        finalJsonName = await basename(finalJsonPath);
        await invoke('save_document_and_update_xml', {
            projectXmlPath: projectXmlPath,
            targetPath: finalJsonPath,
            documentName: finalJsonName,
            jsonContent: lexicalJsonString
        });
        await refreshProjectFiles();
        setAssetImportStatus(false, `Document "${sourceFilename}" imported as "${finalJsonName}".`);
        prepareDocumentView(finalJsonPath, 'documents');
        return finalJsonPath;

    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        await message(`Error importing document: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error importing: ${errorMessage}`);
        if (backendResultPathAndOriginalFilename && !backendResultPathAndOriginalFilename.toLowerCase().endsWith('.pdf') && backendResultPathAndOriginalFilename.includes('.html')) {
            let pathToClean = backendResultPathAndOriginalFilename.split("|original_filename:")[0];
            try { await invoke('delete_temporary_file', { path: pathToClean }); } catch(delErr) {}
        }
    }
}

export async function importImageFile() {
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error("[ProjectService] Cannot import image: Project data not fully loaded.");
        await message('Project data is not fully loaded. Cannot import images.', { title: 'Import Error', type: 'error' });
        return;
    }
    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing an image");
    if (!canProceedDialog) {
        setAssetImportStatus(false, 'Image import cancelled by user.'); return;
    }
    try {
        const selected = await open({ multiple: false, directory: false, filters: [imageFilter], title: 'Import Image File'});
        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Image import cancelled.' })); return;
        }
        const sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);
        setAssetImportStatus(true, `Importing image ${sourceFilename}...`);
        const finalImagePath = await invoke('import_image_file', { sourcePathStr: sourceFilePath, projectXmlPathStr: projectXmlPath });
        await refreshProjectFiles();
        const importedImageName = await basename(finalImagePath);
        setAssetImportStatus(false, `Image "${importedImageName}" imported successfully.`);
        prepareDocumentView(finalImagePath, 'images');
        return finalImagePath;
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        await message(`Error importing image: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during image import: ${errorMessage}`);
    }
}

export async function importTranscriptFile(sourceType = 'msWord') {
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error("[ProjectService] Cannot import transcript: Project data not fully loaded.");
        await message('Project data is not fully loaded. Cannot import transcripts.', { title: 'Import Error', type: 'error' });
        return;
    }
    const canProceedDialog = await checkUnsavedChangesThenProceed(null, `importing a ${sourceType} transcript`);
    if (!canProceedDialog) {
        setAssetImportStatus(false, 'Transcript import cancelled by user.'); return;
    }
    try {
        if (sourceType === 'msWord') {
            const selected = await open({ multiple: false, directory: false, filters: [wordDocumentFilter], title: 'Import MS Word Transcript (.docx)'});
            if (!selected || typeof selected !== 'string') {
                project.update(p => ({ ...p, statusMessage: 'Transcript import cancelled.' })); return;
            }
            const sourceDocxPath = selected;
            const sourceFilename = await basename(sourceDocxPath);
            setAssetImportStatus(true, `Importing transcript from ${sourceFilename}...`);
            const newTranscriptJsonPath = await invoke('import_word_transcript', { sourceDocxPathStr: sourceDocxPath, projectXmlPathStr: projectXmlPath });
            await refreshProjectFiles();
            const importedTranscriptName = await basename(newTranscriptJsonPath);
            setAssetImportStatus(false, `Transcript "${importedTranscriptName}" imported successfully.`);
            prepareImportedTranscriptView(newTranscriptJsonPath);
            return newTranscriptJsonPath;
        } else {
            throw new Error(`Unsupported transcript source type: ${sourceType}`);
        }
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        await message(`Error importing transcript: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during transcript import: ${errorMessage}`);
    }
}

export async function deleteImportedTranscript(transcriptAbsolutePath) {
    return deleteProjectItem(transcriptAbsolutePath);
}

export async function importTableFile(hasHeaders) {
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    console.log(`[ProjectService] importTableFile: projectXmlPath = ${projectXmlPath}`);

    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import table: Project data not fully loaded.');
        await message('Project data is not fully loaded. Cannot import tables.', { title: 'Import Error', type: 'error' });
        return null;
    }

    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a table");
    if (!canProceedDialog) {
        setAssetImportStatus(false, 'Table import cancelled by user.');
        return null;
    }

    try {
        const selected = await open({
            multiple: false,
            directory: false,
            filters: [tableFilter],
            title: 'Import Table File'
        });

        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Table import cancelled.' }));
            return null;
        }

        const sourceFilePath = selected;
        console.log(`[ProjectService] importTableFile: sourceFilePath = ${sourceFilePath}`);
        const sourceFilename = await basename(sourceFilePath);
        setAssetImportStatus(true, `Importing table ${sourceFilename}...`);

        console.log(`[ProjectService] Invoking 'import_table_file' with sourcePathStr: ${sourceFilePath}, projectXmlPathStr: ${projectXmlPath}`);
        const result = await invoke('import_table_file', {
            sourcePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });
        console.log(`[ProjectService] Result from 'import_table_file':`, result);

        if (result && result.table_path && result.preview_data) {
            setAssetImportStatus(false, `${sourceFilename} imported successfully.`);
            return { ...result, filename: sourceFilename };
        } else {
            throw new Error('Invalid response from backend during table import.');
        }
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        await message(`Error importing table: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during table import: ${errorMessage}`);
        return null;
    }
}






export async function loadTableData(tablePath, hasHeaders) {
    if (!tablePath) throw new Error('tablePath is required');
    try {
        const tableData = await invoke('load_table_data', {
            tablePathStr: tablePath,
            hasHeaders: hasHeaders
        });

        // Check if the response is an object with 'headers' and 'data' arrays
        if (typeof tableData !== 'object' || tableData === null || !Array.isArray(tableData.headers) || !Array.isArray(tableData.data)) {
            throw new Error("Backend returned invalid data format for table.");
        }

        // Sanitize data: remove carriage returns from all cell values
        const sanitizedData = tableData.data.map(row => {
            const newRow = {};
            for (const key in row) {
                if (typeof row[key] === 'string') {
                    newRow[key] = row[key].replace(/\r/g, '');
                } else {
                    newRow[key] = row[key];
                }
            }
            return newRow;
        });

        return { headers: tableData.headers, data: sanitizedData };
    } catch (error) {
        const errorMessage = error.message || String(error);
        await message(`Error loading table data: ${errorMessage}`, { title: 'Load Table Error', type: 'error' });
        throw error;
    }
}
function parseTimestampStringToSeconds(timestampStr) { if (!timestampStr || typeof timestampStr !== 'string') return 0; const cleanedStr = timestampStr.trim(); const parts = cleanedStr.split(':'); let seconds = 0; try { if (parts.length === 3) { seconds = parseInt(parts[0], 10) * 3600 + parseInt(parts[1], 10) * 60 + parseFloat(parts[2]); } else if (parts.length === 2) { seconds = parseInt(parts[0], 10) * 60 + parseFloat(parts[1]); } else if (parts.length === 1) { seconds = parseFloat(parts[0]); } else { return 0; } } catch (e) { return 0; } return isNaN(seconds) ? 0 : parseFloat(seconds.toFixed(3)); }
function extractPlainTextFromLexicalNode(node) { if (!node) return ''; if (node.type === 'text' || node.type === 'extended-text') return node.text || ''; let text = ''; if (node.children && Array.isArray(node.children)) { for (const child of node.children) text += extractPlainTextFromLexicalNode(child); } if (node.type === 'linebreak') return '\n'; return text; }
export function parseLexicalTableToSegments(lexicalTableJsonString) { let parsedFullEditorState; try { parsedFullEditorState = JSON.parse(lexicalTableJsonString); if (!parsedFullEditorState?.root?.children) return []; } catch (error) { return []; } const segmentsArray = []; try { const tableNode = parsedFullEditorState.root.children.find(node => node.type === 'table'); if (!tableNode?.children) return []; for (let i = 1; i < tableNode.children.length; i++) { const rowNode = tableNode.children[i]; if (rowNode.type !== 'tablerow' || !rowNode.children || !rowNode.children.length || rowNode.children.length < 4) continue; try { let startTime = 0, endTime = 0, speakerName = "Unknown", segmentTextJsonString = "{}"; const timestampCellNode = rowNode.children[1]; if (timestampCellNode.type !== 'tablecell') continue; let timestampFullText = ''; if (timestampCellNode.children) timestampCellNode.children.forEach(child => timestampFullText += extractPlainTextFromLexicalNode(child)); const timeParts = timestampFullText.split(' - '); startTime = parseTimestampStringToSeconds(timeParts[0]); endTime = timeParts.length > 1 ? parseTimestampStringToSeconds(timeParts[1]) : startTime; const speakerCellNode = rowNode.children[2]; if (speakerCellNode.type !== 'tablecell') continue; let tempSpeakerName = ''; if (speakerCellNode.children) speakerCellNode.children.forEach(child => tempSpeakerName += extractPlainTextFromLexicalNode(child)); speakerName = tempSpeakerName.trim() || "Unknown"; const textContentCellNode = rowNode.children[3]; if (textContentCellNode.type !== 'tablecell') continue; const deepClonedCellChildren = JSON.parse(JSON.stringify(textContentCellNode.children || [])); segmentTextJsonString = JSON.stringify({ root: { type: 'root', children: deepClonedCellChildren, direction: null, format: '', indent: 0, version: 1 }}); segmentsArray.push({ start_time: startTime, end_time: endTime, speaker: speakerName, text: segmentTextJsonString }); } catch (cellProcessingError) { segmentsArray.push({ start_time: 0, end_time: 0, speaker: "Error Processing Row", text: JSON.stringify({ root: { type: 'root', children:[], direction:null, format:'', indent:0, version:1 } }) }); } } } catch (tableProcessingError) { return []; } return segmentsArray; }

export async function getAssetMetadata(assetRelativePath) {
    const currentProject = get(project);
    const projectId = currentProject.id;

    if (!assetRelativePath) {
        console.error('[ProjectService] getAssetMetadata: Missing assetRelativePath.');
        return null;
    }
    if (!projectId) {
        console.error('[ProjectService] getAssetMetadata: Missing projectId.');
        return null;
    }

    try {
        const metadata = await invoke('get_asset_metadata_command', { projectId, assetRelativePath });
        return metadata;
    } catch (error) {
        console.error(`[ProjectService] Error getting asset metadata for ${assetRelativePath}:`, error);
        return null;
    }
}

export async function loadTranscriptFile(transcriptFilePath) {
    if (!transcriptFilePath) {
        project.update(p => ({ ...p, error: "Transcript file path is missing."}));
        throw new Error("Transcript file path is required.");
    }
    if (!transcriptFilePath.toLowerCase().endsWith('.json')) {}
    const filename = transcriptFilePath.split(/[\\/]/).pop();
    project.update(p => ({ ...p, statusMessage: `Loading transcript ${filename}...` }));
    try {
        const normalizedPath = normalizePath(transcriptFilePath);
        const fullLexicalJsonString = await invoke('load_transcript_json', { transcriptPath: normalizedPath });
        const segmentsArray = parseLexicalTableToSegments(fullLexicalJsonString);
        const currentProject = get(project);
        const projectBaseDir = currentProject.baseDirectory;
        let relativeTranscriptPath = transcriptFilePath;
        if (projectBaseDir && transcriptFilePath.startsWith(projectBaseDir)) {
            relativeTranscriptPath = transcriptFilePath.substring(projectBaseDir.length);
            if (relativeTranscriptPath.startsWith(sep) || relativeTranscriptPath.startsWith('/') || relativeTranscriptPath.startsWith('\\')) {
                relativeTranscriptPath = relativeTranscriptPath.substring(1);
            }
        }
        setTranscriptData(relativeTranscriptPath, segmentsArray, false);
    } catch (error) {
        let errorMessage = "Unknown error";
        if (error && typeof error === 'object') {
            if (error.__tauriCore__ && typeof error.__tauriCore__.message === 'string') {
                errorMessage = error.__tauriCore__.message;
            } else if (typeof error.message === 'string') {
                errorMessage = error.message;
            } else {
                errorMessage = String(error); // Fallback to String(error) if no specific message found
            }
        } else if (typeof error === 'string') {
            errorMessage = error;
        }

        project.update(p => ({ ...p, error: `Transcript load failed: ${errorMessage}`, statusMessage: `Error loading transcript ${filename}.`}));
        throw new Error(`Failed to load transcript: ${errorMessage}`);
    }
}
export async function saveTranscriptData() {
    const projData = get(project);
    const tsData = get(transcriptStore);
    const transcriptPath = tsData.currentTranscriptPath;
    const transcriptSegments = tsData.segments;
    const projectXmlPath = projData.xmlPath;

    if (!transcriptPath) throw new Error("Cannot save, no transcript loaded.");
    if (!projectXmlPath) throw new Error("Cannot save, project path unknown.");
    if (!transcriptPath.toLowerCase().endsWith('.json')) throw new Error("Transcript must be saved as .json.");
    const filename = transcriptPath.split(/[\\/]/).pop();
    project.update(p => ({ ...p, statusMessage: `Saving transcript ${filename}...` }));
    let fullLexicalTableJsonString = "";
    try {
        const editorForTableAssembly = createHeadlessEditor({ nodes: ALL_EDITOR_NODES, namespace: `table-assembly-editor-${Date.now()}`, onError: (e) => console.error("[TableAssemblyEditor] Error:", e), });
        await editorForTableAssembly.update(() => { const root = _getRoot(); root.clear(); const tableNode = _createTableNode(); const headerRow = _createTableRowNode(); const headers = ["#", "Timestamp", "Speaker", "Text"]; for (const headerText of headers) { const cell = _createTableCellNode({ headerState: 'column' }); const paragraph = _createParagraphNode(); paragraph.append(_createTextNode(headerText)); cell.append(paragraph); headerRow.append(cell); } tableNode.append(headerRow); for (let i = 0; i < transcriptSegments.length; i++) { const segment = transcriptSegments[i]; const dataRow = _createTableRowNode(); const cellNum = _createTableCellNode(); const pNum = _createParagraphNode(); pNum.append(_createTextNode(String(i + 1))); cellNum.append(pNum); dataRow.append(cellNum); const cellTime = _createTableCellNode(); const pTime = _createParagraphNode(); const startTime = formatTimestampHtml(segment.start_time || 0); const endTime = formatTimestampHtml(segment.end_time || 0); pTime.append(_createTextNode(`${startTime} - ${endTime}`)); cellTime.append(pTime); dataRow.append(cellTime); const cellSpeaker = _createTableCellNode(); const pSpeaker = _createParagraphNode(); pSpeaker.append(_createTextNode(segment.speaker || "Unknown")); cellSpeaker.append(pSpeaker); dataRow.append(cellSpeaker); const cellText = _createTableCellNode(); if (segment.text && typeof segment.text === 'string') { let parsedSegmentState; try { parsedSegmentState = JSON.parse(segment.text); } catch (e) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Malformed cell JSON]")); cellText.append(pError); dataRow.append(cellText); tableNode.append(dataRow); continue; } function flattenNodes(nodes) { return nodes.flatMap(n => n.type === 'root' && Array.isArray(n.children) ? flattenNodes(n.children) : [n]); } const rawChildren = parsedSegmentState?.root?.children || []; const serializedChildNodes = flattenNodes(rawChildren); if (serializedChildNodes.length > 0) { serializedChildNodes.forEach(serializedNodeObject => { if (typeof serializedNodeObject !== 'object' || serializedNodeObject === null) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Invalid node object found]")); cellText.append(pError); return; } try { const liveNode = _parseSerializedNode(serializedNodeObject); if (liveNode) { if (typeof liveNode.clone === 'function') cellText.append(liveNode.clone()); else if (typeof liveNode.constructor?.clone === 'function') cellText.append(liveNode.constructor.clone(liveNode)); else { const pError = _createParagraphNode(); pError.append(_createTextNode(`[Error V6: Clone totally failed on type ${liveNode.getType()}]`)); cellText.append(pError);}} else { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Parsed node is null before clone attempt]")); cellText.append(pError);}} catch (e) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: _parseSerializedNode exception]")); cellText.append(pError);}}); } else cellText.append(_createParagraphNode()); } else cellText.append(_createParagraphNode()); dataRow.append(cellText); tableNode.append(dataRow); } root.append(tableNode); root.append(_createParagraphNode()); });
        fullLexicalTableJsonString = JSON.stringify(editorForTableAssembly.getEditorState().toJSON());

        // Add validation here
        const parsedJson = JSON.parse(fullLexicalTableJsonString);
        if (!parsedJson || !parsedJson.root || !Array.isArray(parsedJson.root.children)) {
            throw new Error("Generated Lexical JSON is invalid: missing root or children.");
        }

    } catch (assemblyError) {
        project.update(p => ({ ...p, error: `Save failed: Error preparing data. ${assemblyError.message}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to prepare transcript data for saving: ${assemblyError.message}`);
    }
    try {
        await invoke('save_transcript_json', { projectXmlPath: projectXmlPath, transcriptPath: transcriptPath, lexicalTableJsonString: fullLexicalTableJsonString });
        markTranscriptAsSaved();
    } catch (error) {
        const errorMessage = error?.message || String(error);
        project.update(p => ({ ...p, error: `Save failed: ${errorMessage}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to save transcript: ${errorMessage}`);
    }
}

export async function refreshProjectFiles(targetPathToSelect = null) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) return; project.update(p => ({ ...p, statusMessage: 'Refreshing file list...', isLoading: true })); try { await loadProjectDataAndUpdateStore(projectXmlPath, targetPathToSelect); project.update(p => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false })); } catch (error) { const errorMessage = error?.message || String(error); project.update(p => ({ ...p, error: `Refresh failed: ${errorMessage}`, statusMessage: 'Error refreshing file list.', isLoading: false })); } }
export async function renameProjectItem(itemPath, newName, itemType) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) { await message('Project data not loaded. Cannot rename.', { title: 'Rename Error', type: 'error' }); throw new Error('Project path missing.'); } if (!itemPath || !newName) { await message('Missing item path or new name.', { title: 'Rename Error', type: 'error' }); throw new Error('Missing parameters.'); } const oldFilename = await basename(itemPath); project.update(p => ({ ...p, statusMessage: `Renaming ${oldFilename} to ${newName}...`, isLoading: true })); try {
    const newPath = await invoke('rename_project_item', { itemPath: itemPath, newName: newName, itemType: itemType, projectXmlPath: projectXmlPath });
    await refreshProjectFiles(); // Refresh the file list after rename
    project.update(p => ({ ...p, statusMessage: `Renamed ${oldFilename} to ${newName}.`, fileRenamed: { oldPath: itemPath, newPath: newPath } }));
} catch (error) {
    const errorMessage = error?.message || String(error);
    await message(`Error renaming item: ${errorMessage}`, { title: 'Rename Failed', type: 'error' });
    project.update(p => ({ ...p, error: `Rename failed: ${errorMessage}`, statusMessage: `Error renaming ${oldFilename}.`, isLoading: false }));
    throw error;
} }
export async function deleteProjectItem(itemPath) {
    const currentProj = get(project);
    const currentTs = get(transcriptStore);
    const projectXmlPath = currentProj.xmlPath;
    if (!projectXmlPath) { await message('Project data not loaded. Cannot delete.', { title: 'Delete Error', type: 'error' }); throw new Error('Project path missing.'); }
    if (!itemPath) { await message('Missing item path.', { title: 'Delete Error', type: 'error' }); throw new Error('Missing parameters.'); }
    const filename = await basename(itemPath);
    project.update(p => ({ ...p, statusMessage: `Deleting ${filename}...`, isLoading: true }));
    try {
        await invoke('delete_project_item', { itemPath: itemPath, projectXmlPath: projectXmlPath });

        const wasSelectedMedia = currentTs.selectedMediaFile?.path === itemPath;
        const wasCurrentTranscript = currentTs.currentTranscriptPath === itemPath;
        const wasSelectedDocument = currentProj.selectedDocumentPath === itemPath;
        const wasSelectedImportedTranscript = currentProj.currentImportedTranscriptPath === itemPath;
        const wasSelectedMediaNote = currentProj.selectedMediaNotePath === itemPath;

        if (wasSelectedMedia) selectMedia(null);
        else if (wasCurrentTranscript) clearTranscriptState();
        else if (wasSelectedDocument) prepareDocumentView(null);
        else if (wasSelectedImportedTranscript) prepareImportedTranscriptView(null);
        else if (wasSelectedMediaNote) prepareMediaNoteView(null);

        await refreshProjectFiles();
        project.update(p => ({ ...p, statusMessage: `Deleted ${filename}.`}));
 } catch (error) { const errorMessage = error?.message || String(error); await message(`Error deleting item: ${errorMessage}`, { title: 'Delete Failed', type: 'error' }); project.update(p => ({ ...p, error: `Delete failed: ${errorMessage}`, statusMessage: `Error deleting ${filename}.`, isLoading: false })); throw error; } }
export async function handleTrimMediaConfirm(originalMediaPath, startTime, endTime) { if (!originalMediaPath || typeof startTime !== 'number' || typeof endTime !== 'number' || startTime < 0 || endTime <= startTime) throw new Error(`Invalid trim parameters provided.`); const filename = await basename(originalMediaPath); project.update(p => ({ ...p, isImportingAsset: true, statusMessage: `Trimming ${filename}...` })); try { const updatedFiles = await invoke('trim_media', { originalMediaPath, startTime, endTime }); if (Array.isArray(updatedFiles)) { project.update(p => ({ ...p, files: updatedFiles, isImportingAsset: false, error: null, statusMessage: 'Media trimmed successfully.', isLoading: false })); let trimmedEntry = null; const originalFilename = await basename(originalMediaPath); const originalExtension = originalFilename.includes('.') ? originalFilename.substring(originalFilename.lastIndexOf('.')) : ''; function findTrimmedRecursive(nodes, stemPrefix, extension) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory && node.name.startsWith(stemPrefix) && node.name.includes('_trimmed_') && node.name.endsWith(extension)) return node; if (node.children && node.children.length > 0) { const found = findTrimmedRecursive(node.children, stemPrefix, extension); if (found) return found; } } return null; } const originalStem = originalFilename.includes('.') ? originalFilename.substring(0, originalFilename.lastIndexOf('.')) : originalFilename; trimmedEntry = findTrimmedRecursive(updatedFiles, originalStem, originalExtension); if (trimmedEntry) selectMedia(trimmedEntry); else { let firstMedia = null; function findFirstMediaRecursive(nodes) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory) return node; if (node.children && node.children.length > 0) { const found = findFirstMediaRecursive(node.children); if (found) return found; } } return null; } firstMedia = findFirstMediaRecursive(updatedFiles); if (firstMedia) selectMedia(firstMedia); } } else { await refreshProjectFiles(); throw new Error("Received invalid data from trim process."); } } catch (error) { const errorMessage = error?.message || String(error); project.update(p => ({ ...p, isImportingAsset: false, error: `Trim failed: ${errorMessage}`, statusMessage: `Error trimming media.`, isLoading: false })); throw new Error(`Trim failed: ${errorMessage}`); } }

export let transcribeModalInstance = null; export function registerTranscribeModal(instance) { transcribeModalInstance = instance; }
export async function requestTranscription() {
    const storeState = get(transcriptStore);
    console.log(`[JULES-DEBUG PS requestTranscription] Called. Current store state: isTranscribing=${storeState.isTranscribing}, showModal=${storeState.showTranscribeModal}, jobStatus=${storeState.transcriptionJobStatus}`);
    const currentTs = get(transcriptStore);
    const currentProj = get(project);
    if (!currentTs.selectedMediaFile?.path) { await message('Please select a media file first.', { title: 'Transcription Request', type: 'info'}); return; }
    if (storeState.isTranscribing) {
        toggleTranscribeModal(true);
        return;
    }
    prepareForNewTranscription(); // Call the function directly
    toggleTranscribeModal(true); // Ensure modal is shown after preparing for new transcription
}
export async function handleConfirmStartTranscription(transcriptionMode) {
    const currentTs = get(transcriptStore);
    const currentProj = get(project);
    // const jobId = uuidv4();
    const translateToEnglish = currentTs.translateToEnglish;
    const diarize = currentTs.diarizationEnabledForNextJob;

    let numSpeakersForPayload = 0;
    if (diarize) {
        if (currentTs.speakers.count > 0) {
            numSpeakersForPayload = currentTs.speakers.count;
        } else {
            numSpeakersForPayload = 2; // Default to 2 speakers if diarize is checked but no count is set
        }
    }
    
    const mediaPathForJob = currentTs.selectedMediaFile?.path;
    const modelNameForJob = currentTs.selectedModelName; // This is the one selected in UI

    console.log(`[JULES-DEBUG] projectService.handleConfirmStartTranscription: modelNameForJob = ${modelNameForJob}`);

    if (!mediaPathForJob || !modelNameForJob) {
        // Use notification store for error
        notificationStore.add('Error: Missing media file or model selection.', 'error', 0);
        // Call setTranscriptionStatus to reflect the error state in the modal and keep it open
        setTranscriptionStatus(false, null, { // isTranscribing is false
            status: 'error',
            errorMessage: 'Missing media file or model selection.'
        });
        return;
    }

    const selectedModelIdentifier = currentTs.selectedModelName;
    // const isCloudModel = selectedModelIdentifier.startsWith('google-') || selectedModelIdentifier.startsWith('gemini-'); // Cloud model logic will be handled by Rust or removed if not supported by transcribe_media_command

    // Consolidate arguments for the unified 'transcribe_media_command'
    const payload = {
        project_xml_path: currentProj.xmlPath,
        media_path_str: mediaPathForJob,
        num_speakers: numSpeakersForPayload, // Use the adjusted num_speakers value
        language_code: (currentTs.selectedLanguage === 'auto' || !currentTs.selectedLanguage) ? null : currentTs.selectedLanguage,
        model_name: modelNameForJob,
        translate_to_english: translateToEnglish, // Use variable defined above
        speaker_names: currentTs.speakers.names || [],
        translated_speaker_names: translateToEnglish ? (currentTs.speakers.translatedNames || []) : [],
        transcription_mode: transcriptionMode,
    };

    // Step 1: Set status to 'initiating'. JobId is null at this point.
    // This makes isTranscribing=true, and the modal should show an "Initiating..." state.
    setTranscriptionStatus(true, null, { // jobIdToSet is null
        status: 'initiating',
        initialProgressMessage: `Initiating with ${modelNameForJob}...`,
        mediaPath: mediaPathForJob
    });

    try {
        // Always call the unified command
        const initiatedPayload = await invoke('transcribe_media_command', { payload: payload });

        if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
            throw new Error("Backend did not return a valid job_id.");
        }
        const backendJobId = initiatedPayload.job_id;

        // Immediately set the job ID in the store
        transcriptStore.update(ts => ({ ...ts, transcriptionJobId: backendJobId }));

        // Step 2: Update status to 'running' with the actual job ID from the backend.
        setTranscriptionStatus(true, backendJobId, { // Pass the backendJobId
            status: 'running',
            // The progress message might be quickly updated by the first actual progress event.
            initialProgressMessage: `Transcription started (Job: ${backendJobId.substring(0,8)})...`,
            mediaPath: mediaPathForJob // Can be redundant if already set in step 1, but harmless
        });
        // The progress listener should now be able to match events to backendJobId.

    } catch (error) {
        let displayMessage = 'An unknown error occurred during transcription initiation.';
        let finalStatus = 'error';

        if (typeof error === 'string') {
            displayMessage = error;
        } else if (error && typeof error.message === 'string') {
            displayMessage = error.message;
        } else {
            // Fallback if error is an object without a message property
            displayMessage = 'The operation failed, and the error details could not be displayed.';
            try {
                // Attempt to stringify, but this can be verbose or circular
                const stringifiedError = JSON.stringify(error);
                if (stringifiedError !== '{}') { // Avoid empty object stringification
                     displayMessage = `Operation failed: ${stringifiedError}`;
                }
            } catch (stringifyError) {
                // Ignore if stringify fails
            }
        }

        // Check if the error message indicates cancellation
        const lowerCaseMessage = displayMessage.toLowerCase();
        if (lowerCaseMessage.includes("cancel") || lowerCaseMessage.includes("cancelled") || lowerCaseMessage.includes("canceled")) {
            finalStatus = 'cancelled';
            // Override display message for a cleaner UI if it's a cancellation
            displayMessage = 'Transcription Cancelled';
        }

        setTranscriptionStatus(false, get(transcriptStore).transcriptionJobId, {
            status: finalStatus,
            errorMessage: displayMessage
        });
        console.error(`[ProjectService] Error during transcribe_media_command invocation:`, error); // Keep original error for console
    }
}
export async function handleCancelTranscriptionRequest() {
    const currentProj = get(project);
    const currentTs = get(transcriptStore);
    const jobId = currentTs.transcriptionJobId; // Reading from transcriptStore where it's set

    if (!jobId || !currentTs.isTranscribing) {
        console.warn("[ProjectService handleCancel] No active job ID or not transcribing. JobID:", jobId, "IsTranscribing:", currentTs.isTranscribing);
        return;
    }

    // Update UI to "cancelling" state immediately
    transcriptStore.update(ts => ({ ...ts, transcriptionJobStatus: 'cancelling' }));

    // const modelUsedForJob = currentTs.selectedModelName; // selectedModelName might not be the one used for the *current* job if UI changed
    // Rely on backend to know which type of job it is, if necessary.
    // For now, assuming a single 'cancel_transcription' command.
    // const isCloudJob = modelUsedForJob && (modelUsedForJob.startsWith('google-') || modelUsedForJob.startsWith('gemini-'));
    // const cancelCommand = isCloudJob ? 'cancel_cloud_transcription' : 'cancel_transcription';
    const cancelCommand = 'cancel_transcription'; // Assuming one command for now

    // transcribeModalInstance?.setStatusCancelling('Requesting cancellation...'); // Modal now reacts to store

    try {
        await invoke(cancelCommand, { jobId });
        // Backend will emit TRANSCRIPTION_PROGRESS with a cancellation message,
        // then custom_transcription_job_completed with status 'cancelled'.
        // Store listener will update transcriptionJobStatus again based on that event.
    } catch (error) {
        const errorMessage = error?.message || String(error);
        // If cancel invoke fails, revert status from 'cancelling' to 'error' or 'running'
        transcriptStore.update(ts => ({
            ...ts,
            transcriptionJobStatus: 'error', // Or back to 'running' if cancel failed meaning job might still be running
            transcriptionErrorMessage: `Failed to send cancel request: ${errorMessage}`
        }));
        // project.update(p => ({ ...p, error: `Cancellation request failed: ${errorMessage}` })); // Project store error might be too broad
        notificationStore.add(`Cancellation request failed: ${errorMessage}`, 'error');
    }
}
export let progressListenerInitialized = false;
export let progressUnlistenFn = null;
export async function initializeProgressListener() {
    // console.log('[JULES-DEBUG] initializeProgressListener called');
    if (progressListenerInitialized) return;
    try {
        progressUnlistenFn = await listen('TRANSCRIPTION_PROGRESS', (event) => {
            // console.log('[JULES-DEBUG] projectService: TRANSCRIPTION_PROGRESS event received:', event);
            const payload = event.payload;
            if (!payload || typeof payload !== 'object') {
                // console.log('[JULES-DEBUG] projectService: Payload empty or not an object');
                return;
            }
            const eventJobId = payload.jobId ?? payload.job_id; // Prefer 'jobId', fallback to 'job_id'
            // Directly call updateTranscriptionProgress. It will handle matching and state transitions.
            updateTranscriptionProgress({
                jobId: eventJobId, // Pass the event's job ID
                percent: payload.percent ?? 0,
                message: payload.message ?? ''
            });
        });
        progressListenerInitialized = true;
    } catch (e) {
        console.error("[ProjectService] Failed to initialize progress listener:", e);
        project.update(p => ({ ...p, error: "Failed to initialize progress listener." }));
    }
}
export function cleanupProgressListener() { if (progressUnlistenFn) { progressUnlistenFn(); progressUnlistenFn = null; } progressListenerInitialized = false; }

export function formatTimestampHtml(seconds) { if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00.000'; const totalMs = Math.round(seconds * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const totalS = Math.floor(totalMs / 1000); const sec = String(totalS % 60).padStart(2, '0'); const min = String(Math.floor(totalS / 60)).padStart(2, '0'); return `${min}:${sec}.${ms}`; }
export function isLexicalJson(jsonString) { if (!jsonString || typeof jsonString !== 'string') return false; try { const parsed = JSON.parse(jsonString); return parsed && typeof parsed === 'object' && parsed.root && typeof parsed.root === 'object' && Array.isArray(parsed.root.children); } catch (e) { return false; } }

export async function convertAndSaveTranscriptAsDoc() {
    const projData = get(project);
    const tsData = get(transcriptStore);
    const transcriptPath = tsData.currentTranscriptPath;
    const selectedMedia = tsData.selectedMediaFile;
    const projectXmlPath = projData.xmlPath;
    const projectBaseDir = projData.baseDirectory;
    if (!transcriptPath) throw new Error("No transcript file loaded.");
    if (!selectedMedia?.path) throw new Error("No media file selected.");
    if (!projectBaseDir) throw new Error("Project base directory not found.");
    if (!projectXmlPath) throw new Error("Project XML path not found.");
    project.update(p => ({ ...p, statusMessage: `Converting transcript to table document...` }));
    const finalTableEditor = createHeadlessEditor({ nodes: ALL_EDITOR_NODES, namespace: `doc-table-finalizer-${Date.now()}`, onError: (error) => console.error(error), });
    let finalLexicalJsonString = "";
    try {
        const fullLexicalTableString = await invoke('load_transcript_json', { transcriptPath: transcriptPath });
        if (!fullLexicalTableString) throw new Error("Transcript file content is empty.");
        finalLexicalJsonString = fullLexicalTableString;
                const originalTranscriptFilename = await basename(transcriptPath); // e.g., "20130922_1.json"
                console.debug(`[ProjectService] originalTranscriptFilename: ${originalTranscriptFilename}`);
                const originalTranscriptStem = originalTranscriptFilename.includes('.')
                    ? originalTranscriptFilename.substring(0, originalTranscriptFilename.lastIndexOf('.'))
                    : originalTranscriptFilename; // e.g., "20130922_1"
                console.debug(`[ProjectService] originalTranscriptStem: ${originalTranscriptStem}`);
            
                // The base name for the new document file will be the original transcript's stem
                const docFilenameBase = originalTranscriptStem; // Use the original stem
                console.debug(`[ProjectService] docFilenameBase: ${docFilenameBase}`);
            
                // Construct the full target directory path for the new document
                // This should be projectBaseDir/HARVEY_FILES_DIR/DOCS_DIR_NAME/originalTranscriptStem
                const targetDocumentDir = `${projectBaseDir}${sep()}${HARVEY_FILES_DIR}${sep()}${DOCS_DIR_NAME}${sep()}${originalTranscriptStem}`;
                console.debug(`[ProjectService] targetDocumentDir: ${targetDocumentDir}`);
            
                project.update(p => ({ ...p, statusMessage: `Saving transcript document...` }));
            
                const targetFullPath = await invoke('get_unique_document_path', {
                    targetDirStr: targetDocumentDir, // Pass the correctly constructed target directory
                    baseName: docFilenameBase,
                    extension: 'json'
                });
                console.debug(`[ProjectService] targetFullPath from get_unique_document_path: ${targetFullPath}`);        const docFilename = await basename(targetFullPath);
        await invoke('save_document_and_update_xml', { projectXmlPath: projectXmlPath, targetPath: targetFullPath, documentName: docFilename, jsonContent: finalLexicalJsonString });

        const relativePath = targetFullPath.substring(projectBaseDir.length + 1).replace(/\\/g, '/');
        const fileMetadata = {
            file_name: docFilename,
            file_path: targetFullPath,
            last_modified: new Date().toISOString(),
            title: "",
            description: "",
            summary: "",
            duration_seconds: null,
            width: null,
            height: null,
            frame_rate: null,
            bit_rate: null,
            audio_codec: null,
            video_codec: null,
            created_at: new Date().toISOString(),
            original_import_path: null,
            speaker_names: null,
            waveform_data: null,
        };

        await invoke('update_asset_metadata_command', {
            projectXmlPathStr: projectXmlPath,
            assetRelativePath: relativePath,
            metadataPayload: fileMetadata,
            customFieldsPayload: null,
            assetType: 'document',
        });

        project.update(p => ({ ...p, statusMessage: `Document file created: ${docFilename}` }));
        await refreshProjectFiles();
        return targetFullPath;
    } catch (error) {
        project.update(p => ({ ...p, statusMessage: `Error converting transcript: ${error.message || error}` }));
        throw error;
    }
}
export async function loadActiveDocumentContent() { const currentProj = get(project); const filePath = currentProj.selectedDocumentPath; if (!filePath) { project.update(p => ({...p, isDocumentLoading: false, documentError: null })); return; } const filename = await basename(filePath); project.update(p => ({ ...p, isDocumentLoading: true, documentError: null })); try { const jsonContent = await invoke('load_note_json', { filePath }); if (!jsonContent || jsonContent.trim() === '') throw new Error("Loaded document content empty/invalid."); try { JSON.parse(jsonContent); } catch (e) { throw new Error(`Loaded document content not valid JSON.`); } setLoadedDocumentData(filePath, jsonContent); } catch (error) { const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); setDocumentLoadFailed(filePath, errorMessage); await message(`Error loading document '${filename}': ${errorMessage}`, { title: 'Load Document Error', type: 'error' }); } }
export async function saveCurrentPdfAnnotations() {
    const projState = get(project);
    if (!projState.selectedDocumentPath || !projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')) return;
    if (!projState.isPdfAnnotationsDirty) return;

    const projectBaseDir = projState.baseDirectory;
    if (!projectBaseDir) {
        console.error("[ProjectService] saveCurrentPdfAnnotations: Project base directory is missing.");
        notificationStore.add('Error: Project base directory is missing. Cannot save PDF annotations.', 'error');
        return;
    }
    if (!projState.id || typeof projState.id !== 'string' || projState.id.trim() === '') { // project_uuid is stored as 'id' in projectStore
        console.error("[ProjectService] saveCurrentPdfAnnotations: project_uuid (project.id) is missing or invalid.", projState);
        await message('Cannot save annotations: Project identifier is missing or invalid. Please ensure the project is fully loaded.', { title: 'Save Error', type: 'error' });
        return;
    }
    const projectId = projState.id;

    let relativePdfPath = projState.selectedDocumentPath;
    if (relativePdfPath.startsWith(projectBaseDir + sep) || relativePdfPath.startsWith(projectBaseDir + '/')) {
        relativePdfPath = relativePdfPath.substring(projectBaseDir.length + 1);
    } else if (relativePdfPath.startsWith(projectBaseDir)) {
        relativePdfPath = relativePdfPath.substring(projectBaseDir.length);
        if (relativePdfPath.startsWith(sep) || relativePdfPath.startsWith('/') || relativePdfPath.startsWith('\\')) {
            relativePdfPath = relativePdfPath.substring(1);
        }
    }
    relativePdfPath = relativePdfPath.replace(/\\/g, '/');

    try {
        const annList = projState.currentPdfAnnotations ?? [];
        await invoke('save_pdf_annotations', {
            projectId: projectId,
            originalPdfRelativePathStr: relativePdfPath,
            annotationsJsonContent: JSON.stringify(annList)
        });
        markPdfAnnotationsAsSaved();
        console.log(`[ProjectService] PDF annotations saved for ${relativePdfPath} in project ${projectId}`);
    } catch (error) {
        console.error(`[ProjectService] Error saving PDF annotations for ${relativePdfPath} in project ${projectId}:`, error);
        notificationStore.add(`Error saving PDF annotations: ${error.message || error}`, 'error');
        // Do not throw here to avoid unhandled promise rejections if the caller doesn't catch.
    }
}
export async function saveTableData(tablePath, tableData, orderedHeaders) {
    if (!tablePath) {
        throw new Error("Cannot save, no table path specified.");
    }
    if (!tableData) {
        throw new Error("Cannot save, no table data provided.");
    }

    const filename = await basename(tablePath);
    project.update(p => ({ ...p, statusMessage: `Saving table ${filename}...` }));

    try {
        await invoke('save_table_data', { tablePathStr: tablePath, tableData: tableData, headers: orderedHeaders });
        project.update(p => ({ ...p, isDocumentDirty: false, statusMessage: `Table saved: ${filename}` }));
    } catch (error) {
        const errorMessage = error?.message || String(error);
        project.update(p => ({ ...p, documentError: `Failed to save table: ${errorMessage}`, statusMessage: `Error saving ${filename}.` }));
        await message(`Error saving table: ${errorMessage}`, { title: 'Save Table Error', type: 'error' });
        throw error;
    }
}
export async function saveDocumentContent(filePath, jsonContent) {
    if (filePath && filePath.toLowerCase().endsWith('.pdf')) {
        project.update(p => ({...p, documentError: "PDF content cannot be saved this way.", statusMessage: 'Save failed (PDF type).'}));
        throw new Error("PDF content saving is not handled by saveDocumentContent.");
    }
    if (!filePath || jsonContent === null || typeof jsonContent !== 'string') {
        const errorMsg = "Cannot save document: Missing path or invalid/missing JSON content.";
        await message(errorMsg, { title: 'Save Error', type: 'error' });
        project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed.'}));
        throw new Error(errorMsg);
    }
    try {
        const parsed = JSON.parse(jsonContent);
        if (!parsed.root?.children) throw new Error("Invalid Lexical JSON structure.");
    } catch (e) {
        const errorMsg = `Cannot save document: Content not valid JSON or invalid structure. ${e.message}`;
        await message(errorMsg, { title: 'Save Error', type: 'error' });
        project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed (invalid content).'}));
        throw new Error(errorMsg);
    }

    const projState = get(project);
    const filename = await basename(filePath);
    project.update(p => ({ ...p, statusMessage: `Saving document ${filename}...` }));

    let mainContentSaveError = null;
    try {
        const highlights_json = (projState.isDocumentMetadataDirty && projState.currentDocumentHighlights?.length > 0)
            ? JSON.stringify(projState.currentDocumentHighlights)
            : null;

        await invoke('save_note_json', {
            targetPath: filePath,
            jsonContent: jsonContent,
            highlightsJson: highlights_json,
        });

        // Mark content as saved
        if (projState.selectedDocumentPath === filePath) {
            markDocumentAsSaved(jsonContent);
        } else if (projState.selectedMediaNotePath) {
            // This is a media note, use its specific save marker
            const { markMediaNoteTranscriptAsSaved } = await import('$lib/stores/projectStore.js');
            markMediaNoteTranscriptAsSaved(projState.selectedMediaNotePath, jsonContent);
        }

        // Mark metadata (highlights) as saved
        if (highlights_json) {
            markDocumentMetadataAsSaved(projState.currentDocumentFileLevelMetadata);
        }

    } catch (error) {
        mainContentSaveError = error;
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        project.update(p => ({ ...p, documentError: `Failed save document: ${errorMessage}`, statusMessage: `Error saving ${filename}.` }));
    }

    // This block is now mostly redundant for highlights, but might handle other metadata.
    // Let's keep it but ensure it doesn't run for media notes to avoid errors.
    let metadataSaveError = null;
    if (projState.selectedDocumentPath === filePath && projState.isDocumentMetadataDirty) {
        try {
            await saveDocumentMetadata(filePath);
        } catch (error) {
            metadataSaveError = error;
        }
    }

    if (mainContentSaveError) {
        await message(`Error saving document '${filename}': ${mainContentSaveError.message || mainContentSaveError}`, { title: 'Save Document Error', type: 'error' });
        throw mainContentSaveError;
    }
    if (metadataSaveError) {
        // We don't throw here because the main content saved successfully.
        // The error will be handled by saveDocumentMetadata itself.
    }
}

export async function saveHighlightChanges(highlight) {
    if (!highlight || !highlight.id || !highlight.source || !highlight.source.file_path) {
        console.error("[ProjectService] saveHighlightChanges: Invalid highlight object provided.", highlight);
        throw new Error("Invalid highlight object provided for saving.");
    }

    const { source, ...highlightData } = highlight;
    const filePath = source.file_path;
    const docType = source.file_type;
    const proj = get(project);

    if (!proj.id) {
        console.error("[ProjectService] saveHighlightChanges: Project ID is missing.");
        throw new Error("Project ID is missing.");
    }

    try {
        await invoke('save_highlight_changes', {
            projectId: proj.id,
            filePath: filePath,
            docType: docType,
            highlight: highlightData,
        });
        console.log(`[ProjectService] Highlight changes saved for ${filePath}`);
    } catch (error) {
        console.error(`[ProjectService] Error saving highlight changes for ${filePath}:`, error);
        notificationStore.add(`Error saving highlight changes: ${error.message || error}`, 'error');
        throw error;
    }
}

export async function saveImportedTranscriptContent(filePath, jsonContent) {
    if (!filePath || jsonContent === null || typeof jsonContent !== 'string') {
        const errorMsg = "Cannot save transcript: Missing path or invalid/missing JSON content.";
        await message(errorMsg, { title: 'Save Error', type: 'error' });
        project.update(p => ({...p, importedTranscriptError: errorMsg, statusMessage: 'Save failed.'}));
        throw new Error(errorMsg);
    }
    try {
        const parsed = JSON.parse(jsonContent);
        if (!parsed.root?.children) throw new Error("Invalid Lexical JSON structure.");
    } catch (e) {
        const errorMsg = `Cannot save transcript: Content not valid JSON or invalid structure. ${e.message}`;
        await message(errorMsg, { title: 'Save Error', type: 'error' });
        project.update(p => ({...p, importedTranscriptError: errorMsg, statusMessage: 'Save failed (invalid content).'}));
        throw new Error(errorMsg);
    }

    const projState = get(project);
    const filename = await basename(filePath);
    project.update(p => ({ ...p, statusMessage: `Saving transcript ${filename}...` }));

    try {
        const highlights_json = (projState.isImportedTranscriptMetadataDirty && projState.currentImportedTranscriptHighlights?.length > 0)
            ? JSON.stringify(projState.currentImportedTranscriptHighlights)
            : null;

        await invoke('save_note_json', {
            targetPath: filePath,
            jsonContent: jsonContent,
            highlightsJson: highlights_json,
        });

        const { markImportedTranscriptAsSaved } = await import('$lib/stores/projectStore.js');
        markImportedTranscriptAsSaved(filePath, jsonContent);

    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        project.update(p => ({ ...p, importedTranscriptError: `Failed save transcript: ${errorMessage}`, statusMessage: `Error saving ${filename}.` }));
        await message(`Error saving transcript '${filename}': ${errorMessage}`, { title: 'Save Transcript Error', type: 'error' });
        throw error;
    }
}
export async function loadDocumentMetadata(originalDocumentAbsPath) {
    const proj = get(project);
    if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) return null;
    let relativePath = "";
    const base = proj.baseDirectory;
    const absPath = originalDocumentAbsPath;
    if (absPath.startsWith(base)) {
        relativePath = absPath.substring(base.length);
        if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length);
        if (relativePath.startsWith('/') || relativePath.startsWith('\\')) relativePath = relativePath.substring(1);
    } else {
        return null;
    }
    const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/');

    try {
        const result = await invoke('load_document_metadata', {
            projectXmlPathStr: proj.xmlPath,
            originalDocumentRelativePathStr: originalDocumentRelativePathStr
        });

        if (result) {
            if (result.highlights && typeof result.highlights === 'string') {
                try {
                    result.highlights = JSON.parse(result.highlights);
                } catch (e) {
                    console.error("Failed to parse highlights JSON from backend:", e);
                    result.highlights = [];
                }
            } else {
                result.highlights = [];
            }
            return result;
        }
        return null;
    } catch (error) {
        console.error("Error loading document metadata:", error);
        return null;
    }
}
export async function saveDocumentMetadata(originalDocumentAbsPath) {
    const proj = get(project);
    if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) {
        console.error("[ProjectService saveDocMeta] Pre-condition failed: Missing project data or path.");
        return;
    }
    // If not dirty and it's the currently selected document, no need to save.
    if (!proj.isDocumentMetadataDirty && originalDocumentAbsPath === proj.selectedDocumentPath) {
        console.log("[ProjectService saveDocMeta] No metadata changes to save for current document.");
        return;
    }

    let relativePath = "";
    const base = proj.baseDirectory;
    const absPath = originalDocumentAbsPath;
    const docFilename = await basename(absPath);

    if (absPath.startsWith(base)) {
        relativePath = absPath.substring(base.length);
        if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length);
        // Normalize path separators for consistency, though backend might do this too
        if (relativePath.startsWith('/') || relativePath.startsWith('\\')) relativePath = relativePath.substring(1);
    } else {
        await message(`Internal error: Could not determine relative path for metadata saving. Path ${absPath} not in base ${base}`, { title: 'Save Metadata Error', type: 'error' });
        throw new Error("Failed to construct relative path for metadata saving.");
    }
    const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/');

    // Prepare the metadata fields from the store for the payload
    // This fullMetadataToSave structure is slightly different from what's directly passed.
    // We'll use its components to build the metadataPayload.
    const fullMetadataToSave = {
        metadata: {
            file_name: docFilename,
            last_modified: proj.currentDocumentFileLevelMetadata.last_modified || new Date().toISOString(),
            title: proj.currentDocumentFileLevelMetadata.title || "",
            description: proj.currentDocumentFileLevelMetadata.description || "",
            summary: proj.currentDocumentFileLevelMetadata.summary || "",
        },
        highlights: proj.currentDocumentHighlights || [] // This becomes customFieldsPayload
    };

    try {
        const metadataPayload = {
            file_name: docFilename, // Already available from basename(absPath)
            file_path: originalDocumentAbsPath, // Absolute path
            last_modified: fullMetadataToSave.metadata.last_modified, // Backend will set its own, but good to pass
            title: fullMetadataToSave.metadata.title,
            description: fullMetadataToSave.metadata.description,
            summary: fullMetadataToSave.metadata.summary,
            // Optional fields from Rust's FileMetadata struct (duration_seconds, width, height, etc.)
            // are intentionally omitted. The backend should treat missing fields as None
            // and not update them, preserving existing technical metadata. `created_at` also not sent.
        };

        await invoke('update_asset_metadata_command', {
            projectXmlPathStr: proj.xmlPath,
            assetRelativePath: originalDocumentRelativePathStr, // Key for DB lookup
            metadataPayload: metadataPayload,
            customFieldsPayload: null, // Ensure this is null
            assetType: "doc" // Explicitly set asset type
        });

        markDocumentMetadataAsSaved(fullMetadataToSave.metadata); // Update UI state
        console.log(`[ProjectService saveDocMeta] Document metadata saved for: ${originalDocumentRelativePathStr}`);

    } catch (error) {
        const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown error saving metadata.");
        console.error(`[ProjectService saveDocMeta] Error for ${originalDocumentRelativePathStr}:`, errorMsg);
        await message(`Error saving document metadata: ${errorMsg}`, { title: 'Save Metadata Error', type: 'error' });
        throw new Error(errorMsg); // Re-throw to indicate failure
    }
}

export async function loadImageAnnotations(imageAbsPath) {
    const { setLoadedImageAnnotations, setImageAnnotationsLoadFailed } = await import('$lib/stores/projectStore.js');

    const currentProj = get(project);
    const projectBaseDir = currentProj.baseDirectory;
    const projectId = currentProj.id;

    if (!imageAbsPath) {
        setLoadedImageAnnotations([]);
        return;
    }

    if (!projectBaseDir || !projectId) {
        const errorMsg = "Project data not fully loaded.";
        console.error(`[ProjectService] Cannot load image annotations: ${errorMsg}`);
        setImageAnnotationsLoadFailed(imageAbsPath, errorMsg);
        return;
    }

    let relativeImagePath = imageAbsPath;
    if (imageAbsPath.startsWith(projectBaseDir)) {
        relativeImagePath = imageAbsPath.substring(projectBaseDir.length).replace(/^[\\/]/, '');
    }
    relativeImagePath = relativeImagePath.replace(/\\/g, '/');

    try {
        const annotationsJsonString = await invoke('load_image_annotations', {
            projectId,
            imageRelativePathStr: relativeImagePath
        });
        const annotations = annotationsJsonString ? JSON.parse(annotationsJsonString) : [];
        setLoadedImageAnnotations(annotations);
    } catch (err) {
        console.error(`[ProjectService] Error loading annotations for ${relativeImagePath}:`, err);
        setImageAnnotationsLoadFailed(imageAbsPath, err.message || String(err));
    }
}

export async function saveImageAnnotations() {
    const { markImageAnnotationsAsSaved } = await import('$lib/stores/projectStore.js');
    const projState = get(project);
    const imagePath = projState.selectedDocumentPath;
    const annotations = projState.currentImageAnnotations;

    if (!imagePath || !projState.isImageAnnotationsDirty) {
        return;
    }

    const projectBaseDir = projState.baseDirectory;
    const projectId = projState.id;

    if (!projectBaseDir || !projectId) {
        console.error("[ProjectService] saveImageAnnotations: Project data not fully loaded.");
        notificationStore.add('Error: Project not fully loaded. Cannot save annotations.', 'error');
        return;
    }

    let relativeImagePath = imagePath;
    if (imagePath.startsWith(projectBaseDir)) {
        relativeImagePath = imagePath.substring(projectBaseDir.length).replace(/^[\\/]/, '');
    }
    relativeImagePath = relativeImagePath.replace(/\\/g, '/');

    try {
        await invoke('save_image_annotations', {
            projectId,
            imageRelativePathStr: relativeImagePath,
            annotationsJsonString: JSON.stringify(annotations, null, 2)
        });
        markImageAnnotationsAsSaved();
        console.log(`[ProjectService] Image annotations saved for ${relativeImagePath}`);
    } catch (error) {
        console.error(`[ProjectService] Error saving image annotations for ${relativeImagePath}:`, error);
        notificationStore.add(`Error saving image annotations: ${error.message || error}`, 'error');
    }
}


export async function checkUnsavedChangesThenProceed(newPathToLoad, providedActionContextDescription) {
    const projState = get(project);
    const tsState = get(transcriptStore); // Get transcript store state
    let itemIsDirty = false;
    let itemPath = null;
    let itemName = '';
    let itemTypeForPrompt = '';
    let saveFunction = null;
    let discardFunction = null;
    let resetEditorFunction = null; // Not always used, depends on editor
    let initialContentForReset = null; // Not always used

    // const pathDescForLog = newPathToLoad ? await basename(newPathToLoad) : "NO_PATH_PROVIDED";
    // const typeDescForLog = providedActionContextDescription || "unknown action";

    // Check order: Media Notes -> PDF Annotations -> JSON Documents -> Imported Transcripts -> Main Transcript
    if (projState.selectedMediaNotePath && projState.isMediaNoteTranscriptDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedMediaNotePath;
        itemTypeForPrompt = 'media notes';
        if (projState.activeMediaNoteEditorRef?.ref && typeof projState.activeMediaNoteEditorRef.ref.save === 'function') {
            saveFunction = projState.activeMediaNoteEditorRef.ref.save;
            discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
            initialContentForReset = projState.initialMediaNoteTranscriptJson;
            resetEditorFunction = projState.activeMediaNoteEditorRef.ref.resetEditorState;
        } else {
            console.warn(`[checkUnsavedChanges] Media note for ${itemPath} is dirty but editor ref missing.`);
            discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
        }
    } else if (projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') && projState.isPdfAnnotationsDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'PDF annotations';
        saveFunction = async () => saveCurrentPdfAnnotations();
        discardFunction = () => markDocumentChangesDiscarded();
        initialContentForReset = projState.initialPdfAnnotations;
    } else if (projState.selectedDocumentType === 'images' && projState.isImageAnnotationsDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'image annotations';
        saveFunction = async () => saveImageAnnotations();
        discardFunction = () => markDocumentChangesDiscarded(); // This should also clear image annotations
        initialContentForReset = projState.initialImageAnnotations;
    }
    else if (projState.selectedDocumentType == 'tables' && projState.isDocumentDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'table';
        saveFunction = async () => saveTableData(itemPath, projState.tableData);
        discardFunction = () => {};
    } else if (projState.selectedDocumentPath && (projState.isDocumentDirty || projState.isDocumentMetadataDirty)) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'document';
        if (projState.activeDocumentEditorRef?.ref && typeof projState.activeDocumentEditorRef.ref.save === 'function') {
            saveFunction = projState.activeDocumentEditorRef.ref.save;
        } else {
            if (projState.isDocumentDirty || projState.isDocumentMetadataDirty) {
                saveFunction = () => saveDocumentContent(itemPath, projState.currentDocumentJson);
            }
        }
        discardFunction = () => markDocumentChangesDiscarded();
        initialContentForReset = projState.initialDocumentJson;
        resetEditorFunction = projState.activeDocumentEditorRef?.ref?.resetEditorState;
    } else if (projState.currentImportedTranscriptPath && (projState.isImportedTranscriptDirty || projState.isImportedTranscriptMetadataDirty)) {
        itemIsDirty = true;
        itemPath = projState.currentImportedTranscriptPath;
        itemTypeForPrompt = 'imported transcript';
        if (projState.activeImportedTranscriptEditorRef?.ref && typeof projState.activeImportedTranscriptEditorRef.ref.save === 'function') {
            saveFunction = projState.activeImportedTranscriptEditorRef.ref.save;
            discardFunction = () => markImportedTranscriptChangesDiscarded(itemPath);
            initialContentForReset = projState.initialImportedTranscriptLexicalJson;
            resetEditorFunction = projState.activeImportedTranscriptEditorRef.ref.resetEditorState;
        } else {
            discardFunction = () => markImportedTranscriptChangesDiscarded(itemPath);
        }
    } else if (tsState.currentTranscriptPath && tsState.transcriptDirty) {
        itemIsDirty = true;
        itemPath = tsState.currentTranscriptPath;
        itemTypeForPrompt = 'main transcript';
        saveFunction = async () => saveTranscriptData();
        discardFunction = () => {
            const undoStack = get(transcriptStore).transcriptUndoStack;
            transcriptStore.update(ts => ({ ...ts, segments: undoStack.length > 0 ? undoStack[0] : ts.segments, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [] }));
        };
    }

    if (itemIsDirty && itemPath === newPathToLoad) {
        return true;
    }

    if (!itemIsDirty) {
        return true;
    }

    itemName = itemPath ? await basename(itemPath) : 'current item';
    const actionContextDisplay = newPathToLoad ? `load '${await basename(newPathToLoad)}'` : (providedActionContextDescription || "perform this action");

    if (itemTypeForPrompt === 'media notes' && projState.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND") {
        return true;
    }

    if (projState.autosaveEnabled) {
        if (projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') && projState.isPdfAnnotationsDirty) {
            try {
                await saveCurrentPdfAnnotations();
                return true;
            } catch (error) {
                console.error('[checkUnsavedChanges] Implicit save for PDF annotations failed:', error);
                const proceedAfterFail = await confirm(
                    `Failed to automatically save changes for PDF annotations on "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
                    { title: 'Autosave Failed', type: 'error', okLabel: 'Discard and Continue', cancelLabel: 'Cancel Action' }
                );
                if (proceedAfterFail) {
                    markDocumentChangesDiscarded();
                    return true;
                } else {
                    return false;
                }
            }
        }

        if (saveFunction) {
            try {
                await saveFunction();
                return true;
            } catch (error) {
                console.error(`[checkUnsavedChanges] Implicit save failed for "${itemName}":`, error);
                const proceedAfterFail = await confirm(
                    `Failed to automatically save changes for "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
                    { title: 'Autosave Failed', type: 'error', okLabel: 'Discard and Continue', cancelLabel: 'Cancel Action' }
                );
                if (proceedAfterFail) {
                    if (discardFunction) discardFunction();
                    if (resetEditorFunction && typeof resetEditorFunction === 'function' && initialContentForReset !== null && itemTypeForPrompt !== 'PDF annotations') {
                         resetEditorFunction(initialContentForReset);
                    }
                    return true;
                } else {
                    return false;
                }
            }
        } else {
            console.warn(`[checkUnsavedChanges] Autosave ON, but save method missing for dirty item "${itemName}" (${itemTypeForPrompt}). Blocking action.`);
            await message(`Cannot ${actionContextDisplay}: Unsaved changes exist for "${itemName}", but an automatic save could not be performed (missing save capability for this item type). Please save or discard changes manually.`, { title: 'Autosave Error', type: 'error'});
            return false;
        }
    } else {
        return new Promise((resolve) => {
            showUnsavedChangesPrompt(itemName, itemTypeForPrompt,
                async () => {
                    hideUnsavedChangesPrompt();
                    if (saveFunction) {
                        try { await saveFunction(); resolve(true); }
                        catch (error) { console.error("[UnsavedChangesModal callback] Save failed:", error); await message(`Failed to save "${itemName}": ${error.message || error}`, {title: "Save Error", type: "error"}); resolve(false); }
                    } else { console.error("[UnsavedChangesModal callback] Save chosen, but save function missing."); await message('Cannot save: Editor reference or save method is missing.', { title: 'Internal Error', type: 'error' }); resolve(false); }
                },
                () => {
                    hideUnsavedChangesPrompt();
                    if (discardFunction) discardFunction();
                    if (resetEditorFunction && typeof resetEditorFunction === 'function' && initialContentForReset !== null) resetEditorFunction(initialContentForReset);
                    resolve(true);
                },
                () => {
                    hideUnsavedChangesPrompt();
                    resolve(false);
                }
            );
        });
    }
}

export async function loadPdfAnnotationsFromFile(pdfAbsPath) {
    const currentProj = get(project);
    const projectBaseDir = currentProj.baseDirectory;

    if (!pdfAbsPath) {
        setLoadedPdfAnnotations([]);
        project.update(p => {
            if (p.selectedDocumentPath === pdfAbsPath && p.isDocumentLoading) {
                return { ...p, isDocumentLoading: false, isLoading: false };
            }
            return p;
        });
        return;
    }

    if (!projectBaseDir) {
        console.error('[ProjectService] Cannot load PDF annotations: Project base directory is missing.');
        setPdfAnnotationsLoadFailed(pdfAbsPath, "Project base directory not found.");
        return;
    }

    let relativePdfPath = pdfAbsPath;
    if (pdfAbsPath.startsWith(projectBaseDir)) {
        relativePdfPath = pdfAbsPath.substring(projectBaseDir.length);
        if (relativePdfPath.startsWith(sep) || relativePdfPath.startsWith('/') || relativePdfPath.startsWith('\\')) {
            relativePdfPath = relativePdfPath.substring(1);
        }
    } else {
        console.warn(`[ProjectService] pdfAbsPath "${pdfAbsPath}" does not seem to be within projectBaseDir "${projectBaseDir}". Using it as is, but this might be an issue for DB lookup.`);
    }
    relativePdfPath = relativePdfPath.replace(/\\/g, '/');

    const filename = await basename(pdfAbsPath);
    project.update(p => ({ ...p, statusMessage: `Loading annotations for ${filename}...`}));

    try {
        if (!currentProj || !currentProj.id || typeof currentProj.id !== 'string' || currentProj.id.trim() === '') {
            console.error('[ProjectService] loadPdfAnnotationsFromFile: project ID (from $project.id) is missing or invalid.', currentProj);
            setPdfAnnotationsLoadFailed(pdfAbsPath, "Project identifier is missing or invalid."); // Assuming pdfAbsPath is available
            return; // Or throw error
        }
        const projectId = currentProj.id;
        const annotationsJsonString = await invoke('load_pdf_annotations', { projectId: projectId, originalPdfRelativePathStr: relativePdfPath });

        if (annotationsJsonString && typeof annotationsJsonString === 'string') {
            try {
                const parsedAnnotations = JSON.parse(annotationsJsonString);
                setLoadedPdfAnnotations(parsedAnnotations || []);
            } catch (parseError) {
                console.error(`[ProjectService] Failed to parse annotations for ${relativePdfPath}:`, parseError);
                setPdfAnnotationsLoadFailed(pdfAbsPath, `Failed to parse loaded annotations: ${parseError.message}`);
            }
        } else if (annotationsJsonString === null) {
            setLoadedPdfAnnotations([]);
        } else {
            console.warn(`[ProjectService] Unexpected response from load_pdf_annotations for ${relativePdfPath}:`, annotationsJsonString);
            setLoadedPdfAnnotations([]);
        }
    } catch (e) {
        const errorMessage = e.message || String(e);
        console.error(`[ProjectService] Error loading annotations for ${relativePdfPath}:`, errorMessage);
        setPdfAnnotationsLoadFailed(pdfAbsPath, `Service call failed: ${errorMessage}`);
    }
}

// Function to clear all project-related data from stores
export async function clearProjectDataStore() {
    console.log('[ProjectService] Clearing project data store.');
    // Need to dynamically import stores here because this is a .js file, not a .svelte component
    // and to avoid circular dependencies if projectStore itself imports projectService.
    const projectStoreModule = await import('$lib/stores/projectStore.js');
    const transcriptStoreModule = await import('$lib/stores/transcriptStore.js');

    projectStoreModule.project.set({ ...projectStoreModule.initialState });
    projectStoreModule.currentProjectGroupsList.set([]);

    transcriptStoreModule.clearTranscriptState();

    // Optionally, inform other parts of the app that the project has been cleared
    // await emit('project-cleared');
    console.log('[ProjectService] Project data store cleared.');
}

export async function renameTableHeader(tablePath, oldHeader, newHeader) {
    if (!tablePath || !oldHeader || !newHeader) {
        throw new Error("Missing required parameters for renaming table header.");
    }

    try {
        await invoke('rename_table_header', {
            tablePathStr: tablePath,
            oldHeader: oldHeader,
            newHeader: newHeader
        });
    } catch (error) {
        const errorMessage = error.message || String(error);
        await message(`Error renaming header: ${errorMessage}`, { title: 'Rename Header Error', type: 'error' });
        throw error;
    }
}

export let translationProgressListenerInitialized = false;
export let translationProgressUnlistenFn = null;

export async function initializeTranslationProgressListener() {
    if (translationProgressListenerInitialized) return;
    try {
        translationProgressUnlistenFn = await listen('TRANSLATION_PROGRESS', (event) => {
            const payload = event.payload;
            if (!payload || typeof payload !== 'object') {
                return;
            }
            updateTranslationProgress(payload);
        });
        translationProgressListenerInitialized = true;
    } catch (e) {
        console.error("[ProjectService] Failed to initialize translation progress listener:", e);
    }
}

export async function requestTranslation(transcriptPath, modelName) {
    const currentProject = get(project);
    const ts = get(transcriptStore);

    if (ts.isTranslating) {
        toggleTranslateModal(true);
        return;
    }

    if (!currentProject.xmlPath) {
        await message('Cannot translate: Project path is not set.', { title: 'Translation Error', type: 'error' });
        return;
    }

    setTranslationStatus(true, null, { status: 'initiating' });

    try {
        const initiatedPayload = await invoke('translate_transcript_command', {
            projectXmlPath: currentProject.xmlPath,
            transcriptPath,
            modelName: modelName,
            targetLanguage: ts.selectedLanguage,
        });

        if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
            throw new Error("Backend did not return a valid job_id for translation.");
        }

        setTranslationStatus(true, initiatedPayload.job_id, { status: 'running' });
    } catch (error) {
        const errorMessage = error.message || String(error);
        setTranslationStatus(false, null, { status: 'error', errorMessage });
        console.error(`[ProjectService] Error during translate_transcript_command invocation:`, error);
    }
}

export async function handleCancelTranslationRequest() {
    const ts = get(transcriptStore);
    const jobId = ts.translationJobId;

    if (!jobId || !ts.isTranslating) {
        console.warn("[ProjectService] No active translation job to cancel.");
        return;
    }

    transcriptStore.update(s => ({ ...s, translationJobStatus: 'cancelling' }));

    try {
        await invoke('cancel_translation_command', { jobId });
    } catch (error) {
        const errorMessage = error.message || String(error);
        transcriptStore.update(s => ({
            ...s,
            translationJobStatus: 'error',
            translationErrorMessage: `Failed to send cancel request: ${errorMessage}`
        }));
        notificationStore.add(`Cancellation request failed: ${errorMessage}`, 'error');
    }
}
