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
    // Functions moved to transcriptStore:
	// setTranscriptData,
	// toggleTranscribeModal,
	// setTranscriptionStatus,
	// updateTranscriptionProgress,
	// clearTranscriptionStatus,
	// selectMedia,
	// clearTranscriptState,
	// markTranscriptAsSaved,

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
    markMediaNoteTranscriptChangesDiscarded // For media notes
} from '$lib/stores/projectStore.js';

import {
    transcriptStore,
    setTranscriptData,
    toggleTranscribeModal,
    setTranscriptionStatus,
    updateTranscriptionProgress,
    clearTranscriptionStatus,
    selectMedia, // For main transcriptions view
    clearTranscriptState,
    markTranscriptAsSaved
} from '$lib/stores/transcriptStore.js';

import { getCloudConfig } from './configureActions.js';

export async function saveTableLayoutPrefs(tablePath, layoutJson) {
    if (!tablePath || !layoutJson) {
        console.error('[ProjectService] saveTableLayoutPrefs: Missing tablePath or layoutJson.');
        throw new Error('Missing tablePath or layoutJson for saving table layout preferences.');
    }
    try {
        await invoke('save_table_layout_prefs', { tablePath, layoutJson });
        console.info(`[ProjectService] Table layout preferences saved for ${tablePath}`);
    } catch (error) {
        console.error(`[ProjectService] Error saving table layout preferences for ${tablePath}:`, error);
        // Optionally, notify the user via a toast or silent fail
        throw error; // Re-throw if the caller needs to handle it
    }
}

export async function loadTableLayoutPrefs(tablePath) {
    if (!tablePath) {
        console.error('[ProjectService] loadTableLayoutPrefs: Missing tablePath.');
        return null; // Or throw error, depending on desired handling
    }
    try {
        const layoutJson = await invoke('load_table_layout_prefs', { tablePath });
        if (layoutJson) {
            console.info(`[ProjectService] Table layout preferences loaded for ${tablePath}`);
            return JSON.parse(layoutJson); // Parse it before returning
        }
        console.info(`[ProjectService] No saved table layout preferences found for ${tablePath}`);
        return null;
    } catch (error) {
        console.error(`[ProjectService] Error loading table layout preferences for ${tablePath}:`, error);
        // Optionally, notify the user or silent fail
        return null; // Return null on error to allow table to load with defaults
    }
}

// Helper to locate the imported media's actual path in the project tree
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
    console.info('[ProjectService] Calling load_project_data:', projectXmlPath); // INFO
    project.update((current) => ({ ...current, isLoading: true, error: null, statusMessage: 'Loading project data...' }));
    try {
        const loadedData = await invoke('load_project_data', { projectXmlPath });
        console.log('[ProjectService] loadProjectDataAndUpdateStore - loadedData.project_uuid:', loadedData.project_uuid);
        console.debug('[ProjectService] Raw Data received from backend:', loadedData); // DEBUG for potentially large object

        // --- Inject transcript paths from XML into media nodes ---
        if (Array.isArray(loadedData.files)) {
          const attachTranscripts = (nodes) => {
            for (const node of nodes) {
              if (node.file_type === 'media' && node.transcripts) {
                // Map XML transcripts to full and relative paths
                node.transcripts = node.transcripts.map(t => ({
                  path: loadedData.base_directory
                    ? `${loadedData.base_directory}/${t.relativePath}`
                    : t.relativePath,
                  relativePath: t.relativePath
                }));
              }
              if (Array.isArray(node.children)) {
                attachTranscripts(node.children);
              }
            }
          };
          attachTranscripts(loadedData.files);
        }
        // --- End transcript injection ---

        console.log('[ProjectService] loadProjectDataAndUpdateStore - dataToSet will include id:', loadedData.project_uuid);
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
            // pdfAnnotationFiles is no longer loaded from project XML/backend data structure
            isLoading: false, // isLoading false after successful load
            error: null,
            statusMessage: `Loaded project: ${loadedData.project_name}`
        };
        project.update((current) => ({ ...current, ...dataToSet }));
        console.log('[ProjectService] loadProjectDataAndUpdateStore - project.id from get(project).id AFTER update:', get(project).id);
        console.info('[ProjectService] Project store updated with core data.'); // INFO

        await emit('project-view-ready', { projectXmlPath: projectXmlPath });
        console.info("[ProjectService] 'project-view-ready' emitted."); // INFO

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
            if (mediaFileToSelect) {
                console.info(`[ProjectService] Prioritizing selection of target media: ${targetPathToSelect}`);
            } else {
                console.warn(`[ProjectService] Target media path ${targetPathToSelect} provided but not found. Falling back to first media.`);
                mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
            }
        } else {
            mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
        }

        if (mediaFileToSelect) {
            console.info(`[ProjectService] Selecting media file: ${mediaFileToSelect.name} (Path: ${mediaFileToSelect.path})`);
            selectMedia(mediaFileToSelect);
        } else {
            console.info('[ProjectService] No media files found or specified to select. Clearing media selection.');
            selectMedia(null);
        }
    } catch (error) {
        console.error('[ProjectService] Failed to load project data:', error); // ERROR
        project.update((current) => ({ ...current, isLoading: false, error: error?.message || 'Unknown error loading project.', statusMessage: `Error loading project.` }));
        throw error;
    }
}

export async function importMediaFile(importType = null) {
    console.info(`[ProjectService] Starting media import... (Type: ${importType || 'generic'})`); // INFO
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import media: Project XML path missing.'); // ERROR
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
            console.info('[ProjectService] Media import cancelled by user at file selection.'); // INFO
            project.update(p => ({ ...p, statusMessage: 'Media import cancelled.' }));
            return;
        }

        const sourceFilePath = selected;
        const filename = await basename(sourceFilePath);

        const canProceed = await checkUnsavedChangesThenProceed(null, `importing media: ${filename}`);
        if (!canProceed) {
            console.info('[ProjectService] Media import cancelled due to unsaved changes check.'); // INFO
            setAssetImportStatus(false, 'Media import cancelled by user.'); // Ensure loading is off
            return;
        }

        setAssetImportStatus(true, `Importing ${filename}...`);

        const backendResponse = await invoke('import_media', {
            sourceFilePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });

        // Guard against undefined or invalid backend response
        if (!backendResponse || typeof backendResponse !== 'object') {
            console.warn('[ProjectService] import_media returned invalid response:', backendResponse);
            // Refresh the project so the newly imported file appears
            await refreshProjectFiles();
            project.update(p => ({
                ...p,
                isImportingAsset: false,
                isLoading: false,
                statusMessage: `${filename} imported (no metadata returned).`
            }));
            // Auto-select imported media
            await refreshProjectFiles(); // ensure project.files is updated
            const proj = get(project);
            const realPath = findMediaPathByName(proj.files, filename);
            if (realPath) {
              console.info('[ProjectService] Auto-selecting imported media at real path:', realPath); // INFO
              prepareMediaNoteView(realPath);
            }
            return;
        }

        const updatedFiles = backendResponse.files || backendResponse.updatedFiles;
        const newMediaPath = backendResponse.new_media_path || backendResponse.newMediaPath;

        // If backend did not return an updated files array, just refresh and exit gracefully
        if (!Array.isArray(updatedFiles)) {
            console.warn('[ProjectService] import_media returned no updatedFiles. Falling back to refresh.'); // WARN
            await refreshProjectFiles();
            project.update(p => ({
                ...p,
                isImportingAsset: false,
                isLoading: false,
                statusMessage: `${filename} imported (refresh applied).`
            }));
            // Auto-select imported media
            await refreshProjectFiles(); // ensure project.files is updated
            const proj = get(project);
            const realPath = findMediaPathByName(proj.files, filename);
            if (realPath) {
              console.info('[ProjectService] Auto-selecting imported media at real path:', realPath); // INFO
              prepareMediaNoteView(realPath);
            }
            return;
        }

        console.info('[ProjectService] Import finished. Received updated file list and new media path:', newMediaPath); // INFO (path is useful)

        if (Array.isArray(updatedFiles)) {
            project.update(p => ({
                ...p,
                files: updatedFiles,
                isImportingAsset: false, // Explicitly false
                isLoading: false, // Explicitly false
                error: null,
                statusMessage: `${filename} imported.`
            }));

            if (newMediaPath) {
                console.info(`[ProjectService] Auto-selecting imported media note for path: ${newMediaPath}`); // INFO
                prepareMediaNoteView(newMediaPath);
            } else {
                console.warn('[ProjectService] Successfully imported media, but backend did not return new_media_path. Cannot auto-select.'); // WARN
            }

        } else {
            console.error('[ProjectService] Backend import_media returned invalid data:', updatedFiles); // ERROR
            setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);
            throw new Error("Received invalid data from import process.");
        }
    } catch (error) {
        console.error('[ProjectService] Failed to import media file:', error); // ERROR
        const errorMessage = error.message || String(error);
        await message(`Error importing media: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error importing media.`); // Ensure loading is off
        // throw error; // Re-throwing might not be necessary if user is already alerted
    }
}

export async function importDocumentFile() {
    console.info('[ProjectService] Starting document import...'); // INFO
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    const projectBaseDir = currentProject.baseDirectory;

    if (!projectXmlPath || !projectBaseDir) {
        console.error('[ProjectService] Cannot import document: Project data not fully loaded.'); // ERROR
        await message('Project data is not fully loaded. Cannot import documents.', { title: 'Import Error', type: 'error' });
        return;
    }

     const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a document");
     if (!canProceedDialog) {
         console.info('[ProjectService] Document import cancelled due to unsaved changes check before dialog.'); // INFO
         setAssetImportStatus(false, 'Document import cancelled by user.');
         return;
     }

    let sourceFilePath = '';
    let backendResultPathAndOriginalFilename = '';
    let finalJsonPath = '';
    let finalJsonName = '';
    let originalSourceFilenameForMeta = '';

    try {
        const selected = await open({ multiple: false, directory: false, filters: [documentFilter], title: 'Import Document File' });
        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Document import cancelled.' }));
            return;
        }
        sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);
        originalSourceFilenameForMeta = sourceFilename;
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

        setAssetImportStatus(true, `Importing ${sourceFilename}...`); // Store update handles user feedback

        backendResultPathAndOriginalFilename = await invoke('import_document', { sourcePathStr: sourceFilePath, projectXmlPathStr: projectXmlPath });
        let tempHtmlPath = backendResultPathAndOriginalFilename;
        if (backendResultPathAndOriginalFilename.includes("|original_filename:")) {
            tempHtmlPath = backendResultPathAndOriginalFilename.split("|original_filename:")[0];
        }

        if (tempHtmlPath && tempHtmlPath.toLowerCase().endsWith('.pdf')) {
            await refreshProjectFiles();
            const importedPdfName = await basename(tempHtmlPath);
            setAssetImportStatus(false, `Document "${importedPdfName}" imported successfully.`); // Store update
            prepareDocumentView(tempHtmlPath, 'documents');
            return;
        }
        if (!tempHtmlPath || !tempHtmlPath.toLowerCase().endsWith('.html')) throw new Error("Backend did not return expected temporary HTML path.");

        setAssetImportStatus(true, `Reading converted HTML...`); // Store update
        const htmlContent = await invoke('read_file_content', { path: tempHtmlPath });
        try { await invoke('delete_temporary_file', { path: tempHtmlPath }); } catch(delErr) { console.warn(`[ProjectService] Failed to delete temp HTML: ${tempHtmlPath}`); } // WARN

        setAssetImportStatus(true, `Parsing HTML...`); // Store update
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

        // Place JSON in its own folder under Documents
        const docsFolderPath = `${projectBaseDir}/${HARVEY_FILES_DIR}/${DOCS_DIR_NAME}/${sourceFilenameStem}`;
        // Backend will create folder if needed
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
        if (finalJsonPath) prepareDocumentView(finalJsonPath, 'documents');

    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); // ERROR
        await message(`Error importing document: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error importing: ${errorMessage}`); // Store update
        if (backendResultPathAndOriginalFilename && !backendResultPathAndOriginalFilename.toLowerCase().endsWith('.pdf') && backendResultPathAndOriginalFilename.includes('.html')) {
            let pathToClean = backendResultPathAndOriginalFilename.split("|original_filename:")[0];
            try { await invoke('delete_temporary_file', { path: pathToClean }); } catch(delErr) {}
        }
    }
}

// ... (importTableFile, importImageFile, importTranscriptFile remain similar but ensure setAssetImportStatus(false, ...) is called in catch blocks too)
export async function importTableFile() {
    console.info('[ProjectService] Starting table import...'); // INFO
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;

    if (!projectXmlPath) {
        console.error("[ProjectService] Cannot import table: Project data not fully loaded."); // ERROR
        await message('Project data is not fully loaded. Cannot import tables.', { title: 'Import Error', type: 'error' });
        return;
    }
    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a table");
    if (!canProceedDialog) {
        console.info('[ProjectService] Table import cancelled by user.'); // INFO
        setAssetImportStatus(false, 'Table import cancelled by user.'); return;
    }
    try {
        const selected = await open({ multiple: false, directory: false, filters: [tableFilter], title: 'Import Table File (CSV or XLSX)'});
        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Table import cancelled.' })); return;
        }
        const sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);
        setAssetImportStatus(true, `Importing table ${sourceFilename}...`); // Store update
        const finalTablePath = await invoke('import_table_file', { sourcePathStr: sourceFilePath, projectXmlPathStr: projectXmlPath });
        await refreshProjectFiles();
        const importedTableName = await basename(finalTablePath);
        setAssetImportStatus(false, `Table "${importedTableName}" imported successfully.`); // Store update
        if (finalTablePath) prepareDocumentView(finalTablePath, 'tables');
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); // ERROR
        await message(`Error importing table: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during table import: ${errorMessage}`); // Store update
    }
}

export async function importImageFile() {
    console.info('[ProjectService] Starting image import...'); // INFO
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error("[ProjectService] Cannot import image: Project data not fully loaded."); // ERROR
        await message('Project data is not fully loaded. Cannot import images.', { title: 'Import Error', type: 'error' });
        return;
    }
    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing an image");
    if (!canProceedDialog) {
        console.info('[ProjectService] Image import cancelled by user.'); // INFO
        setAssetImportStatus(false, 'Image import cancelled by user.'); return;
    }
    try {
        const selected = await open({ multiple: false, directory: false, filters: [imageFilter], title: 'Import Image File'});
        if (!selected || typeof selected !== 'string') {
            project.update(p => ({ ...p, statusMessage: 'Image import cancelled.' })); return;
        }
        const sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);
        setAssetImportStatus(true, `Importing image ${sourceFilename}...`); // Store update
        const finalImagePath = await invoke('import_image_file', { sourcePathStr: sourceFilePath, projectXmlPathStr: projectXmlPath });
        await refreshProjectFiles();
        const importedImageName = await basename(finalImagePath);
        setAssetImportStatus(false, `Image "${importedImageName}" imported successfully.`); // Store update
        if (finalImagePath) prepareDocumentView(finalImagePath, 'images');
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); // ERROR
        await message(`Error importing image: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during image import: ${errorMessage}`); // Store update
    }
}

export async function importTranscriptFile(sourceType = 'msWord') {
    console.info(`[ProjectService] Starting transcript import (Source Type: ${sourceType})...`); // INFO
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    if (!projectXmlPath) {
        console.error("[ProjectService] Cannot import transcript: Project data not fully loaded."); // ERROR
        await message('Project data is not fully loaded. Cannot import transcripts.', { title: 'Import Error', type: 'error' });
        return;
    }
    const canProceedDialog = await checkUnsavedChangesThenProceed(null, `importing a ${sourceType} transcript`);
    if (!canProceedDialog) {
        console.info('[ProjectService] Transcript import cancelled by user.'); // INFO
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
            setAssetImportStatus(true, `Importing transcript from ${sourceFilename}...`); // Store update
            const newTranscriptJsonPath = await invoke('import_word_transcript', { sourceDocxPathStr: sourceDocxPath, projectXmlPathStr: projectXmlPath });
            await refreshProjectFiles();
            const importedTranscriptName = await basename(newTranscriptJsonPath);
            setAssetImportStatus(false, `Transcript "${importedTranscriptName}" imported successfully.`); // Store update
            if (newTranscriptJsonPath) prepareImportedTranscriptView(newTranscriptJsonPath);
        } else {
            throw new Error(`Unsupported transcript source type: ${sourceType}`);
        }
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); // ERROR
        await message(`Error importing transcript: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during transcript import: ${errorMessage}`); // Store update
    }
}

/**
 * Deletes an imported transcript JSON file and its containing folder,
 * and updates the project manifest accordingly.
 *
 * @param {string} transcriptAbsolutePath - Full path to the imported transcript JSON file.
 */
export async function deleteImportedTranscript(transcriptAbsolutePath) {
    return deleteProjectItem(transcriptAbsolutePath);
}


export async function loadTableData(tablePath) { if (!tablePath) throw new Error('tablePath is required'); try { const tableData = await invoke('load_table_data', { tablePathStr: tablePath }); if (!Array.isArray(tableData)) throw new Error("Backend returned invalid data format for table."); return tableData; } catch (error) { const errorMessage = error.message || String(error); await message(`Error loading table data: ${errorMessage}`, { title: 'Load Table Error', type: 'error' }); throw error; } }
function parseTimestampStringToSeconds(timestampStr) { if (!timestampStr || typeof timestampStr !== 'string') return 0; const cleanedStr = timestampStr.trim(); const parts = cleanedStr.split(':'); let seconds = 0; try { if (parts.length === 3) { seconds = parseInt(parts[0], 10) * 3600 + parseInt(parts[1], 10) * 60 + parseFloat(parts[2]); } else if (parts.length === 2) { seconds = parseInt(parts[0], 10) * 60 + parseFloat(parts[1]); } else if (parts.length === 1) { seconds = parseFloat(parts[0]); } else { return 0; } } catch (e) { return 0; } return isNaN(seconds) ? 0 : parseFloat(seconds.toFixed(3)); }
function extractPlainTextFromLexicalNode(node) { if (!node) return ''; if (node.type === 'text' || node.type === 'extended-text') return node.text || ''; let text = ''; if (node.children && Array.isArray(node.children)) { for (const child of node.children) text += extractPlainTextFromLexicalNode(child); } if (node.type === 'linebreak') return '\n'; return text; }
export function parseLexicalTableToSegments(lexicalTableJsonString) { let parsedFullEditorState; try { parsedFullEditorState = JSON.parse(lexicalTableJsonString); if (!parsedFullEditorState?.root?.children) return []; } catch (error) { return []; } const segmentsArray = []; try { const tableNode = parsedFullEditorState.root.children.find(node => node.type === 'table'); if (!tableNode?.children) return []; for (let i = 1; i < tableNode.children.length; i++) { const rowNode = tableNode.children[i]; if (rowNode.type !== 'tablerow' || !rowNode.children || rowNode.children.length < 4) continue; try { let startTime = 0, endTime = 0, speakerName = "Unknown", segmentTextJsonString = "{}"; const timestampCellNode = rowNode.children[1]; if (timestampCellNode.type !== 'tablecell') continue; let timestampFullText = ''; if (timestampCellNode.children) timestampCellNode.children.forEach(child => timestampFullText += extractPlainTextFromLexicalNode(child)); const timeParts = timestampFullText.split(' - '); startTime = parseTimestampStringToSeconds(timeParts[0]); endTime = timeParts.length > 1 ? parseTimestampStringToSeconds(timeParts[1]) : startTime; const speakerCellNode = rowNode.children[2]; if (speakerCellNode.type !== 'tablecell') continue; let tempSpeakerName = ''; if (speakerCellNode.children) speakerCellNode.children.forEach(child => tempSpeakerName += extractPlainTextFromLexicalNode(child)); speakerName = tempSpeakerName.trim() || "Unknown"; const textContentCellNode = rowNode.children[3]; if (textContentCellNode.type !== 'tablecell') continue; const deepClonedCellChildren = JSON.parse(JSON.stringify(textContentCellNode.children || [])); segmentTextJsonString = JSON.stringify({ root: { type: 'root', children: deepClonedCellChildren, direction: null, format: '', indent: 0, version: 1 }}); segmentsArray.push({ start_time: startTime, end_time: endTime, speaker: speakerName, text: segmentTextJsonString }); } catch (cellProcessingError) { segmentsArray.push({ start_time: 0, end_time: 0, speaker: "Error Processing Row", text: JSON.stringify({ root: { type: 'root', children:[], direction:null, format:'', indent:0, version:1 } }) }); } } } catch (tableProcessingError) { return []; } return segmentsArray; }
export async function loadTranscriptFile(transcriptFilePath) {
    // project.update for isTranscriptLoading, error, statusMessage should be handled by transcriptStore now or via updateProjectStoreState
    if (!transcriptFilePath) {
        // transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: false, error: "Transcript file path is missing." }));
        project.update(p => ({ ...p, error: "Transcript file path is missing."})); // Keep global error for now
        throw new Error("Transcript file path is required.");
    }
    if (!transcriptFilePath.toLowerCase().endsWith('.json')) {}
    const filename = transcriptFilePath.split(/[\\/]/).pop();
    // transcriptStore.update(ts => ({ ...ts, isTranscriptLoading: true, error: null }));
    project.update(p => ({ ...p, statusMessage: `Loading transcript ${filename}...` })); // Global status
    try {
        const fullLexicalJsonString = await invoke('load_transcript_json', { transcriptPath: transcriptFilePath });
        const segmentsArray = parseLexicalTableToSegments(fullLexicalJsonString);
        setTranscriptData(transcriptFilePath, segmentsArray, false); // This will update transcriptStore, including isTranscriptLoading: false
    } catch (error) {
        const errorMessage = error?.message || String(error);
        // transcriptStore.update(ts => ({ ...ts, segments: [], currentTranscriptPath: null, transcriptDirty: false, isTranscriptLoading: false, error: `Transcript load failed: ${errorMessage}` }));
        project.update(p => ({ ...p, error: `Transcript load failed: ${errorMessage}`, statusMessage: `Error loading transcript ${filename}.`})); // Global error
        throw new Error(`Failed to load transcript: ${errorMessage}`);
    }
}
export async function saveTranscriptData() {
    const projData = get(project); // For projectXmlPath
    const tsData = get(transcriptStore); // For transcript data
    const transcriptPath = tsData.currentTranscriptPath;
    const transcriptSegments = tsData.segments;
    const projectXmlPath = projData.xmlPath;

    if (!transcriptPath) throw new Error("Cannot save, no transcript loaded.");
    if (!projectXmlPath) throw new Error("Cannot save, project path unknown.");
    if (!transcriptPath.toLowerCase().endsWith('.json')) throw new Error("Transcript must be saved as .json.");
    const filename = transcriptPath.split(/[\\/]/).pop();
    project.update(p => ({ ...p, statusMessage: `Saving transcript ${filename}...` })); // Global status
    let fullLexicalTableJsonString = "";
    try {
        const editorForTableAssembly = createHeadlessEditor({ nodes: ALL_EDITOR_NODES, namespace: `table-assembly-editor-${Date.now()}`, onError: (e) => console.error("[TableAssemblyEditor] Error:", e), });
        await editorForTableAssembly.update(() => { const root = _getRoot(); root.clear(); const tableNode = _createTableNode(); const headerRow = _createTableRowNode(); const headers = ["#", "Timestamp", "Speaker", "Text"]; for (const headerText of headers) { const cell = _createTableCellNode({ headerState: 'column' }); const paragraph = _createParagraphNode(); paragraph.append(_createTextNode(headerText)); cell.append(paragraph); headerRow.append(cell); } tableNode.append(headerRow); for (let i = 0; i < transcriptSegments.length; i++) { const segment = transcriptSegments[i]; const dataRow = _createTableRowNode(); const cellNum = _createTableCellNode(); const pNum = _createParagraphNode(); pNum.append(_createTextNode(String(i + 1))); cellNum.append(pNum); dataRow.append(cellNum); const cellTime = _createTableCellNode(); const pTime = _createParagraphNode(); const startTime = formatTimestampHtml(segment.start_time || 0); const endTime = formatTimestampHtml(segment.end_time || 0); pTime.append(_createTextNode(`${startTime} - ${endTime}`)); cellTime.append(pTime); dataRow.append(cellTime); const cellSpeaker = _createTableCellNode(); const pSpeaker = _createParagraphNode(); pSpeaker.append(_createTextNode(segment.speaker || "Unknown")); cellSpeaker.append(pSpeaker); dataRow.append(cellSpeaker); const cellText = _createTableCellNode(); if (segment.text && typeof segment.text === 'string') { let parsedSegmentState; try { parsedSegmentState = JSON.parse(segment.text); } catch (e) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Malformed cell JSON]")); cellText.append(pError); dataRow.append(cellText); tableNode.append(dataRow); continue; } function flattenNodes(nodes) { return nodes.flatMap(n => n.type === 'root' && Array.isArray(n.children) ? flattenNodes(n.children) : [n]); } const rawChildren = parsedSegmentState?.root?.children || []; const serializedChildNodes = flattenNodes(rawChildren); if (serializedChildNodes.length > 0) { serializedChildNodes.forEach(serializedNodeObject => { if (typeof serializedNodeObject !== 'object' || serializedNodeObject === null) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Invalid node object found]")); cellText.append(pError); return; } try { const liveNode = _parseSerializedNode(serializedNodeObject); if (liveNode) { if (typeof liveNode.clone === 'function') cellText.append(liveNode.clone()); else if (typeof liveNode.constructor?.clone === 'function') cellText.append(liveNode.constructor.clone(liveNode)); else { const pError = _createParagraphNode(); pError.append(_createTextNode(`[Error V6: Clone totally failed on type ${liveNode.getType()}]`)); cellText.append(pError);}} else { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: Parsed node is null before clone attempt]")); cellText.append(pError);}} catch (e) { const pError = _createParagraphNode(); pError.append(_createTextNode("[Error V6: _parseSerializedNode exception]")); cellText.append(pError);}}); } else cellText.append(_createParagraphNode()); } else cellText.append(_createParagraphNode()); dataRow.append(cellText); tableNode.append(dataRow); } root.append(tableNode); root.append(_createParagraphNode()); });
        fullLexicalTableJsonString = JSON.stringify(editorForTableAssembly.getEditorState().toJSON());
    } catch (assemblyError) {
        project.update(p => ({ ...p, error: `Save failed: Error preparing data. ${assemblyError.message}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to prepare transcript data for saving: ${assemblyError.message}`);
    }
    try {
        await invoke('save_transcript_json', { projectXmlPath: projectXmlPath, transcriptPath: transcriptPath, lexicalTableJsonString: fullLexicalTableJsonString });
        markTranscriptAsSaved(); // This now comes from transcriptStore and updates it.
    } catch (error) {
        const errorMessage = error?.message || String(error);
        project.update(p => ({ ...p, error: `Save failed: ${errorMessage}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to save transcript: ${errorMessage}`);
    }
}

export async function refreshProjectFiles(targetPathToSelect = null) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) return; project.update(p => ({ ...p, statusMessage: 'Refreshing file list...', isLoading: true })); try { await loadProjectDataAndUpdateStore(projectXmlPath, targetPathToSelect); project.update(p => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false })); } catch (error) { const errorMessage = error?.message || String(error); project.update(p => ({ ...p, error: `Refresh failed: ${errorMessage}`, statusMessage: 'Error refreshing file list.', isLoading: false })); } }
export async function renameProjectItem(itemPath, newName, itemType) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) { await message('Project data not loaded. Cannot rename.', { title: 'Rename Error', type: 'error' }); throw new Error('Project path missing.'); } if (!itemPath || !newName) { await message('Missing item path or new name.', { title: 'Rename Error', type: 'error' }); throw new Error('Missing parameters.'); } const oldFilename = await basename(itemPath); project.update(p => ({ ...p, statusMessage: `Renaming ${oldFilename} to ${newName}...`, isLoading: true })); try { await invoke('rename_project_item', { itemPath: itemPath, newName: newName, projectXmlPath: projectXmlPath }); project.update(p => ({ ...p, statusMessage: `Renamed ${oldFilename} to ${newName}. Refreshing...` })); await refreshProjectFiles(); } catch (error) { const errorMessage = error?.message || String(error); await message(`Error renaming item: ${errorMessage}`, { title: 'Rename Failed', type: 'error' }); project.update(p => ({ ...p, error: `Rename failed: ${errorMessage}`, statusMessage: `Error renaming ${oldFilename}.`, isLoading: false })); throw error; } }
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
        // These remain projectStore checks as they are not part of transcriptStore
        const wasSelectedDocument = currentProj.selectedDocumentPath === itemPath;
        const wasSelectedImportedTranscript = currentProj.currentImportedTranscriptPath === itemPath;
        const wasSelectedMediaNote = currentProj.selectedMediaNotePath === itemPath;

        if (wasSelectedMedia) selectMedia(null); // from transcriptStore
        else if (wasCurrentTranscript) clearTranscriptState(); // from transcriptStore
        else if (wasSelectedDocument) prepareDocumentView(null); // from projectStore
        else if (wasSelectedImportedTranscript) prepareImportedTranscriptView(null); // from projectStore
        else if (wasSelectedMediaNote) prepareMediaNoteView(null); // from projectStore

        await refreshProjectFiles();
        project.update(p => ({ ...p, statusMessage: `Deleted ${filename}.`})); // isLoading will be handled by refreshProjectFiles
 } catch (error) { const errorMessage = error?.message || String(error); await message(`Error deleting item: ${errorMessage}`, { title: 'Delete Failed', type: 'error' }); project.update(p => ({ ...p, error: `Delete failed: ${errorMessage}`, statusMessage: `Error deleting ${filename}.`, isLoading: false })); throw error; } }
export async function handleTrimMediaConfirm(originalMediaPath, startTime, endTime) { if (!originalMediaPath || typeof startTime !== 'number' || typeof endTime !== 'number' || startTime < 0 || endTime <= startTime) throw new Error(`Invalid trim parameters provided.`); const filename = await basename(originalMediaPath); project.update(p => ({ ...p, isImportingAsset: true, statusMessage: `Trimming ${filename}...` })); try { const updatedFiles = await invoke('trim_media', { originalMediaPath, startTime, endTime }); if (Array.isArray(updatedFiles)) { project.update(p => ({ ...p, files: updatedFiles, isImportingAsset: false, error: null, statusMessage: 'Media trimmed successfully.', isLoading: false })); let trimmedEntry = null; const originalFilename = await basename(originalMediaPath); const originalExtension = originalFilename.includes('.') ? originalFilename.substring(originalFilename.lastIndexOf('.')) : ''; function findTrimmedRecursive(nodes, stemPrefix, extension) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory && node.name.startsWith(stemPrefix) && node.name.includes('_trimmed_') && node.name.endsWith(extension)) return node; if (node.children && node.children.length > 0) { const found = findTrimmedRecursive(node.children, stemPrefix, extension); if (found) return found; } } return null; } const originalStem = originalFilename.includes('.') ? originalFilename.substring(0, originalFilename.lastIndexOf('.')) : originalFilename; trimmedEntry = findTrimmedRecursive(updatedFiles, originalStem, originalExtension); if (trimmedEntry) selectMedia(trimmedEntry); else { let firstMedia = null; function findFirstMediaRecursive(nodes) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory) return node; if (node.children && node.children.length > 0) { const found = findFirstMediaRecursive(node.children); if (found) return found; } } return null; } firstMedia = findFirstMediaRecursive(updatedFiles); if (firstMedia) selectMedia(firstMedia); } } else { await refreshProjectFiles(); throw new Error("Received invalid data from trim process."); } } catch (error) { const errorMessage = error?.message || String(error); project.update(p => ({ ...p, isImportingAsset: false, error: `Trim failed: ${errorMessage}`, statusMessage: `Error trimming media.`, isLoading: false })); throw new Error(`Trim failed: ${errorMessage}`); } }

export let transcribeModalInstance = null; export function registerTranscribeModal(instance) { transcribeModalInstance = instance; }
export async function requestTranscription() {
    const currentTs = get(transcriptStore);
    const currentProj = get(project); // For isTranscribing
    if (!currentTs.selectedMediaFile?.path) { await message('Please select a media file first.', { title: 'Transcription Request', type: 'info'}); return; }
    if (!currentTs.selectedModelName) { await message('Please select a transcription model first.', { title: 'Transcription Request', type: 'info'}); return; }
    if (currentProj.isTranscribing) { await message('A transcription job is already in progress.', { title: 'Transcription Request', type: 'info'}); return; }
    toggleTranscribeModal(true); // from transcriptStore
}
export async function handleConfirmStartTranscription() {
    const currentTs = get(transcriptStore);
    const currentProj = get(project); // For projectXmlPath
    const jobId = uuidv4();
    if (!currentTs.selectedMediaFile?.path || !currentTs.selectedModelName) {
        transcribeModalInstance?.setStatusError('Error: Missing media file or model selection.');
        clearTranscriptionStatus('Transcription failed.', 'Missing media file or model selection.'); // from transcriptStore
        toggleTranscribeModal(false); // from transcriptStore
        return;
    }
    const selectedModelIdentifier = currentTs.selectedModelName;
    const isCloudModel = selectedModelIdentifier.startsWith('google-') || selectedModelIdentifier.startsWith('gemini-');
    setTranscriptionStatus(true, jobId, isCloudModel ? 'Cloud transcription starting...' : 'Local transcription starting...'); // from transcriptStore
    try {
        let invokePromise;
        const args = { mediaPath: currentTs.selectedMediaFile.path, language: currentTs.selectedLanguage || '', numSpeakers: currentTs.speakers.count, speakerNames: currentTs.speakers.names || [], jobId: jobId };
        if (isCloudModel) {
            let cloudConfig;
            try { cloudConfig = await getCloudConfig(); } catch (e) { throw new Error(`Failed to get cloud configuration: ${e.message}`); }
            if (!cloudConfig?.consent) throw new Error("Cloud transcription consent not given.");
            if (!cloudConfig?.api_key) throw new Error("Cloud API Key is missing.");
            const cloudArgs = { ...args, cloudModelId: selectedModelIdentifier, apiKey: cloudConfig.api_key };
            invokePromise = invoke('run_cloud_transcription', cloudArgs);
        } else {
            const localArgs = { ...args, modelName: selectedModelIdentifier };
            invokePromise = invoke('run_transcription', localArgs);
        }
        const result = await invokePromise;
        if (!result || typeof result.transcript_file_path !== 'string' || !Array.isArray(result.segments)) throw new Error("Invalid transcription result structure.");
        setTranscriptData(result.transcript_file_path, result.segments, false); // from transcriptStore
        transcribeModalInstance?.setStatusDone('Transcription complete!');
        toggleTranscribeModal(true); // Ensure modal is shown for the 'Done' state
        clearTranscriptionStatus('Transcription complete.'); // from transcriptStore
        // await refreshProjectFiles(); // Removed as per request
        // setTimeout(() => { toggleTranscribeModal(false); }, 1500); // Removed as per request
    } catch (error) {
        const errorMessage = error?.message || String(error);
        if (errorMessage.toLowerCase().includes('cancelled') || errorMessage.toLowerCase().includes('canceled')) {
            transcribeModalInstance?.setStatusCancelled('Transcription cancelled.');
            clearTranscriptionStatus('Transcription cancelled.'); // from transcriptStore
            // setTimeout(() => { toggleTranscribeModal(false); }, 1500); // Removed as per request
        } else {
            transcribeModalInstance?.setStatusError(`Transcription failed: ${errorMessage}`);
            clearTranscriptionStatus('Transcription failed.', errorMessage); // from transcriptStore
            // setTimeout(() => { toggleTranscribeModal(false); }, 1500); // Removed as per request
        }
    }
}
export async function handleCancelTranscriptionRequest() {
    const currentProj = get(project); // For transcriptionJobId, isTranscribing (global states)
    const currentTs = get(transcriptStore); // For selectedModelName
    const jobId = currentProj.transcriptionJobId;
    if (!jobId || !currentProj.isTranscribing) return;
    const modelUsedForJob = currentTs.selectedModelName;
    const isCloudJob = modelUsedForJob && (modelUsedForJob.startsWith('google-') || modelUsedForJob.startsWith('gemini-'));
    const cancelCommand = isCloudJob ? 'cancel_cloud_transcription' : 'cancel_transcription';
    transcribeModalInstance?.setStatusCancelling('Requesting cancellation...');
    try {
        await invoke(cancelCommand, { jobId });
    } catch (error) {
        const errorMessage = error?.message || String(error);
        transcribeModalInstance?.setStatusError(`Failed to send cancel request: ${errorMessage}`);
        project.update(p => ({ ...p, error: `Cancellation request failed: ${errorMessage}` }));
    }
}
export let progressListenerInitialized = false; export let progressUnlistenFn = null; export async function initializeProgressListener() { if (progressListenerInitialized) return; try { progressUnlistenFn = await listen('TRANSCRIPTION_PROGRESS', (event) => { const payload = event.payload; if (!payload || typeof payload !== 'object') return; const eventJobId = payload.jobId ?? payload.job_id; const currentJobId = get(transcriptStore).transcriptionJobId; if (currentJobId && eventJobId === currentJobId) updateTranscriptionProgress({ jobId: currentJobId, percent: payload.percent ?? 0, message: payload.message ?? '' }); }); progressListenerInitialized = true; } catch (e) { project.update(p => ({ ...p, error: "Failed to initialize progress listener." })); } }
export function cleanupProgressListener() { if (progressUnlistenFn) { progressUnlistenFn(); progressUnlistenFn = null; } progressListenerInitialized = false; }

export function formatTimestampHtml(seconds) { if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00.000'; const totalMs = Math.round(seconds * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const totalS = Math.floor(totalMs / 1000); const sec = String(totalS % 60).padStart(2, '0'); const min = String(Math.floor(totalS / 60)).padStart(2, '0'); return `${min}:${sec}.${ms}`; }
export function isLexicalJson(jsonString) { if (!jsonString || typeof jsonString !== 'string') return false; try { const parsed = JSON.parse(jsonString); return parsed && typeof parsed === 'object' && parsed.root && typeof parsed.root === 'object' && Array.isArray(parsed.root.children); } catch (e) { return false; } }

export async function convertAndSaveTranscriptAsDoc() {
    const projData = get(project); // For projectXmlPath, baseDirectory
    const tsData = get(transcriptStore); // For transcriptPath, selectedMediaFile
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
        const mediaStemIdentifier = selectedMedia.media_xml_identifier || (() => { const mediaName = selectedMedia.name; return mediaName.includes('.') ? mediaName.substring(0, mediaName.lastIndexOf('.')) : mediaName; })();
        const safeStem = mediaStemIdentifier.replace(/[^a-zA-Z0-9_-]/g, '_');
        const now = new Date();
        const dateStr = now.toISOString().split('T')[0];
        const timeStr = now.toTimeString().split(' ')[0].replace(/:/g, '-');
        const docFilenameBase = `${safeStem}_transcript_as_doc_${dateStr}_${timeStr}`;
        project.update(p => ({ ...p, statusMessage: `Saving transcript document...` }));
        const targetFullPath = await invoke('get_unique_document_path', { projectBaseDirStr: projectBaseDir, baseName: docFilenameBase, extension: 'json' });
        const docFilename = await basename(targetFullPath);
        await invoke('save_document_and_update_xml', { projectXmlPath: projectXmlPath, targetPath: targetFullPath, documentName: docFilename, jsonContent: finalLexicalJsonString });
        project.update(p => ({ ...p, statusMessage: `Document file created: ${docFilename}` }));
        await refreshProjectFiles();
        return targetFullPath;
    } catch (error) {
        project.update(p => ({ ...p, statusMessage: `Error converting transcript: ${error.message || error}` }));
        throw error;
    }
}
export async function loadActiveDocumentContent() { const currentProj = get(project); const filePath = currentProj.selectedDocumentPath; if (!filePath) { project.update(p => ({...p, isDocumentLoading: false, documentError: null })); return; } const filename = await basename(filePath); project.update(p => ({ ...p, isDocumentLoading: true, documentError: null })); try { const jsonContent = await invoke('load_note_json', { filePath }); if (!jsonContent || jsonContent.trim() === '') throw new Error("Loaded document content empty/invalid."); try { JSON.parse(jsonContent); } catch (e) { throw new Error(`Loaded document content not valid JSON.`); } setLoadedDocumentData(filePath, jsonContent); } catch (error) { const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); setDocumentLoadFailed(filePath, errorMessage); await message(`Error loading document '${filename}': ${errorMessage}`, { title: 'Load Document Error', type: 'error' }); } }
export async function saveCurrentPdfAnnotations() { const projState = get(project); if (!projState.selectedDocumentPath || !projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')) return; if (!projState.isPdfAnnotationsDirty) return; const projectBaseDir = projState.baseDirectory; if (!projectBaseDir) return; let relativePdfPath = projState.selectedDocumentPath; if (relativePdfPath.startsWith(projectBaseDir + sep) || relativePdfPath.startsWith(projectBaseDir + '/')) relativePdfPath = relativePdfPath.substring(projectBaseDir.length + 1); else if (relativePdfPath.startsWith(projectBaseDir)) { relativePdfPath = relativePdfPath.substring(projectBaseDir.length); if (relativePdfPath.startsWith(sep) || relativePdfPath.startsWith('/') || relativePdfPath.startsWith('\\')) relativePdfPath = relativePdfPath.substring(1); } relativePdfPath = relativePdfPath.replace(/\\/g, '/'); try { const annList = projState.currentPdfAnnotations ?? []; await invoke('save_pdf_annotations', { originalPdfRelativePathStr: relativePdfPath, annotationsJsonContent: JSON.stringify(annList) }); markPdfAnnotationsAsSaved(); } catch (error) {} }
export async function saveDocumentContent(filePath, jsonContent) { if (filePath && filePath.toLowerCase().endsWith('.pdf')) { project.update(p => ({...p, documentError: "PDF content cannot be saved this way.", statusMessage: 'Save failed (PDF type).'})); throw new Error("PDF content saving is not handled by saveDocumentContent."); } if (!filePath || jsonContent === null || typeof jsonContent !== 'string') { const errorMsg = "Cannot save document: Missing path or invalid/missing JSON content."; await message(errorMsg, { title: 'Save Error', type: 'error' }); project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed.'})); throw new Error(errorMsg); } try { const parsed = JSON.parse(jsonContent); if (!parsed.root?.children) throw new Error("Invalid Lexical JSON structure."); } catch (e) { const errorMsg = `Cannot save document: Content not valid JSON or invalid structure. ${e.message}`; await message(errorMsg, { title: 'Save Error', type: 'error' }); project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed (invalid content).'})); throw new Error(errorMsg); } const filename = await basename(filePath); project.update(p => ({ ...p, statusMessage: `Saving document ${filename}...` })); let mainContentSaveError = null; try { await invoke('save_note_json', { targetPath: filePath, jsonContent: jsonContent }); markDocumentAsSaved(jsonContent); } catch (error) { mainContentSaveError = error; const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error'); project.update(p => ({ ...p, documentError: `Failed save document: ${errorMessage}`, statusMessage: `Error saving ${filename}.` })); } const projState = get(project); let metadataSaveError = null; if (projState.selectedDocumentPath === filePath && projState.isDocumentMetadataDirty) { try { await saveDocumentMetadata(filePath); } catch (error) { metadataSaveError = error; } } if (mainContentSaveError) { await message(`Error saving document '${filename}': ${mainContentSaveError.message || mainContentSaveError}`, { title: 'Save Document Error', type: 'error' }); throw mainContentSaveError; } if (metadataSaveError) throw metadataSaveError; }
export async function loadDocumentMetadata(originalDocumentAbsPath) { const proj = get(project); if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) return null; let relativePath = ""; const base = proj.baseDirectory; const absPath = originalDocumentAbsPath; if (absPath.startsWith(base)) { relativePath = absPath.substring(base.length); if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length); if (relativePath.startsWith('/') || relativePath.startsWith('\\')) relativePath = relativePath.substring(1); } else return null; const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/'); try { const fullMetadataJsonString = await invoke('load_document_metadata', { projectXmlPathStr: proj.xmlPath, originalDocumentRelativePathStr: originalDocumentRelativePathStr }); if (fullMetadataJsonString && typeof fullMetadataJsonString === 'string') { const parsedFullMetadata = JSON.parse(fullMetadataJsonString); if (parsedFullMetadata?.metadata && Array.isArray(parsedFullMetadata.highlights)) return parsedFullMetadata; return null; } return null; } catch (error) { return null; } }
export async function saveDocumentMetadata(originalDocumentAbsPath) { const proj = get(project); if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath ) return; if (!proj.isDocumentMetadataDirty && originalDocumentAbsPath === proj.selectedDocumentPath) return; let relativePath = ""; const base = proj.baseDirectory; const absPath = originalDocumentAbsPath; const docFilename = await basename(absPath); if (absPath.startsWith(base)) { relativePath = absPath.substring(base.length); if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length); if (relativePath.startsWith('/') || relativePath.startsWith('\\')) relativePath = relativePath.substring(1); } else { await message(`Internal error: Could not determine relative path for metadata saving.`, { title: 'Save Metadata Error', type: 'error' }); throw new Error("Failed to construct relative path."); } const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/'); const fullMetadataToSave = { metadata: { file_name: docFilename, last_modified: proj.currentDocumentFileLevelMetadata.last_modified || new Date().toISOString(), title: proj.currentDocumentFileLevelMetadata.title || "", description: proj.currentDocumentFileLevelMetadata.description || "", summary: proj.currentDocumentFileLevelMetadata.summary || "", }, highlights: proj.currentDocumentHighlights || [] }; const fullMetadataJsonContent = JSON.stringify(fullMetadataToSave, null, 2); try { await invoke('save_document_metadata', { projectXmlPathStr: proj.xmlPath, originalDocumentRelativePathStr: originalDocumentRelativePathStr, fullMetadataJsonContent: fullMetadataJsonContent }); markDocumentMetadataAsSaved(fullMetadataToSave.metadata); } catch (error) { const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown error saving metadata."); await message(`Error saving document highlights: ${errorMsg}`, { title: 'Save Metadata Error', type: 'error' }); throw new Error(errorMsg); } }

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

    const pathDescForLog = newPathToLoad ? await basename(newPathToLoad) : "NO_PATH_PROVIDED";
    const typeDescForLog = providedActionContextDescription || "unknown action";
    console.debug(`[checkUnsavedChanges] Called with newPathToLoad: '${pathDescForLog}', actionContext: '${typeDescForLog}'.`); // DEBUG

    // Check order: Media Notes -> PDF Annotations -> JSON Documents -> Imported Transcripts -> Main Transcript
    if (projState.selectedMediaNotePath && projState.isMediaNoteTranscriptDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedMediaNotePath;
        itemTypeForPrompt = 'media notes';
        if (projState.activeMediaNoteEditorRef?.ref && typeof projState.activeMediaNoteEditorRef.ref.save === 'function') {
            saveFunction = projState.activeMediaNoteEditorRef.ref.save;
            discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
            // If initial was "file not found", reset means going back to that visual state.
            initialContentForReset = projState.initialMediaNoteTranscriptJson;
            resetEditorFunction = projState.activeMediaNoteEditorRef.ref.resetEditorState;
        } else { // Fallback if ref is missing but state is dirty (should ideally not happen)
            console.warn(`[checkUnsavedChanges] Media note for ${itemPath} is dirty but editor ref missing.`); // WARN
            // Provide a way to discard at least
            discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
        }
    // PDF ANNOTATIONS CHECK - MODIFIED BLOCK
    } else if (projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') && projState.isPdfAnnotationsDirty) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        // itemName will be set later if itemIsDirty is true
        itemTypeForPrompt = 'PDF annotations'; // Used by showUnsavedChangesPrompt

        // Specific handling for PDF annotations within the main if/else for autosave
        // This section will be skipped if autosave is ON and save succeeds early.
        // For autosave OFF, or if autosave ON fails, these specific functions will be used by showUnsavedChangesPrompt
        saveFunction = async () => saveCurrentPdfAnnotations();
        discardFunction = () => markDocumentChangesDiscarded(); // This store function should also clear PDF annotation dirty state and reset them.
        initialContentForReset = projState.initialPdfAnnotations; // PDF viewer might not use this like Lexical, but good to have
        // resetEditorFunction is not typically used for PDF annotations directly via an editor ref like lexical
    } else if (projState.selectedDocumentPath && (projState.isDocumentDirty || projState.isDocumentMetadataDirty)) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'document';
        if (projState.activeDocumentEditorRef?.ref && typeof projState.activeDocumentEditorRef.ref.save === 'function') {
            saveFunction = projState.activeDocumentEditorRef.ref.save;
        } else { // Fallback
            if (projState.isDocumentDirty) saveFunction = () => saveDocumentContent(itemPath, projState.currentDocumentJson);
            else if (projState.isDocumentMetadataDirty) saveFunction = () => saveDocumentMetadata(itemPath);
        }
        discardFunction = () => markDocumentChangesDiscarded();
        initialContentForReset = projState.initialDocumentJson;
        resetEditorFunction = projState.activeDocumentEditorRef?.ref?.resetEditorState;
    } else if (projState.currentImportedTranscriptPath && projState.isImportedTranscriptDirty) {
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
    } else if (tsState.currentTranscriptPath && tsState.transcriptDirty) { // Check transcriptStore
        itemIsDirty = true;
        itemPath = tsState.currentTranscriptPath;
        itemTypeForPrompt = 'main transcript';
        saveFunction = async () => saveTranscriptData(); // Service call for main transcript
        discardFunction = () => {
            // Revert to last saved state (or initial if no undo stack)
            const undoStack = get(transcriptStore).transcriptUndoStack; // From transcriptStore
            // Update transcriptStore, not projectStore for these
            transcriptStore.update(ts => ({ ...ts, segments: undoStack.length > 0 ? undoStack[0] : ts.segments, transcriptDirty: false, transcriptUndoStack: [], transcriptRedoStack: [] }));
        };
        // No direct editor ref reset needed here typically as view re-renders from store
    }

    if (itemIsDirty && itemPath === newPathToLoad) {
        console.debug(`[checkUnsavedChanges] Attempting to load/act on the same item that is dirty ('${itemPath}'). Allowing without prompt.`); // DEBUG
        return true;
    }

    if (!itemIsDirty) {
        const actionContextForLog = newPathToLoad ? `loading item '${await basename(newPathToLoad)}'` : `performing action '${providedActionContextDescription || "unknown action"}'`;
        console.debug(`[checkUnsavedChanges] No unsaved changes for active items. Proceeding with ${actionContextForLog}.`); // DEBUG
        return true;
    }

    itemName = itemPath ? await basename(itemPath) : 'current item';
    const actionContextDisplay = newPathToLoad ? `load '${await basename(newPathToLoad)}'` : (providedActionContextDescription || "perform this action");

    console.info(`[checkUnsavedChanges] Unsaved changes detected for "${itemName}" (${itemTypeForPrompt}) while attempting to ${actionContextDisplay}. Autosave is ${projState.autosaveEnabled ? 'ON' : 'OFF'}.`); // INFO

    // If it's a media note that just means "file not found", it's not truly "dirty" in a way that blocks navigation.
    // The user hasn't made changes to an actual loaded note.
    if (itemTypeForPrompt === 'media notes' && projState.mediaNoteTranscriptError === "INFO:FILE_NOT_FOUND") {
        console.info(`[checkUnsavedChanges] Media note for "${itemName}" is in 'file not found' state. Not considered dirty for navigation blocking. Proceeding.`); // INFO
        return true;
    }


    if (projState.autosaveEnabled) {
        // PDF Annotations Autosave Handling (SPECIAL CASE WITHIN AUTOSAVE ON)
        // itemName and actionContextDisplay are already defined before this block if itemIsDirty is true.
        if (projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') && projState.isPdfAnnotationsDirty) {
            // itemName and actionContextDisplay are assumed to be set correctly by this point.
            console.info(`[checkUnsavedChanges] Autosave ON. Attempting implicit save for PDF annotations on "${itemName}".`); // INFO
            try {
                await saveCurrentPdfAnnotations();
                console.info(`[checkUnsavedChanges] Implicit save for PDF annotations successful for "${itemName}". Proceeding.`); // INFO
                return true; // PDF annotations saved, proceed with the original action
            } catch (error) {
                console.error('[checkUnsavedChanges] Implicit save for PDF annotations failed:', error); // ERROR
                const proceedAfterFail = await confirm(
                    `Failed to automatically save changes for PDF annotations on "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
                    { title: 'Autosave Failed', type: 'error', okLabel: 'Discard and Continue', cancelLabel: 'Cancel Action' }
                );
                if (proceedAfterFail) {
                    console.info('[checkUnsavedChanges] User chose to discard PDF annotations after failed autosave.'); // INFO
                    markDocumentChangesDiscarded(); // This should clear PDF dirty state and reset annotations
                    return true; // Proceed with the original action
                } else {
                    console.info('[checkUnsavedChanges] User chose to cancel action after failed PDF autosave.'); // INFO
                    return false; // Cancel the original action
                }
            }
        } // END OF PDF Annotations Autosave Handling

        // Generic Autosave for other item types
        console.info(`[checkUnsavedChanges] Autosave ON. Attempting implicit save for "${itemName}" (${itemTypeForPrompt})...`); // INFO
        if (saveFunction) {
            try {
                await saveFunction();
                console.info(`[checkUnsavedChanges] Implicit save successful for "${itemName}". Proceeding.`); // INFO
                return true;
            } catch (error) {
                console.error(`[checkUnsavedChanges] Implicit save failed for "${itemName}":`, error); // ERROR
                const proceedAfterFail = await confirm(
                    `Failed to automatically save changes for "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
                    { title: 'Autosave Failed', type: 'error', okLabel: 'Discard and Continue', cancelLabel: 'Cancel Action' }
                );
                if (proceedAfterFail) {
                    console.info(`[checkUnsavedChanges] User chose to discard after failed autosave.`); // INFO
                    if (discardFunction) discardFunction();
                    // Reset editor if applicable for non-PDF items
                    if (resetEditorFunction && typeof resetEditorFunction === 'function' && initialContentForReset !== null && itemTypeForPrompt !== 'PDF annotations') {
                         resetEditorFunction(initialContentForReset);
                    }
                    return true;
                } else {
                    console.info(`[checkUnsavedChanges] User chose to cancel action after failed autosave.`); // INFO
                    return false;
                }
            }
        } else {
            console.warn(`[checkUnsavedChanges] Autosave ON, but save method missing for dirty item "${itemName}" (${itemTypeForPrompt}). Blocking action.`); // WARN
            await message(`Cannot ${actionContextDisplay}: Unsaved changes exist for "${itemName}", but an automatic save could not be performed (missing save capability for this item type). Please save or discard changes manually.`, { title: 'Autosave Error', type: 'error'});
            return false;
        }
    } else { // Autosave is OFF
        // For PDF annotations with autosave OFF, the generic showUnsavedChangesPrompt will use
        // the saveFunction and discardFunction defined in the PDF annotations `else if` block earlier.
        console.info(`[checkUnsavedChanges] Autosave OFF. Triggering unsavedChanges modal for "${itemName}" (${itemTypeForPrompt})...`); // INFO
        return new Promise((resolve) => {
            showUnsavedChangesPrompt(itemName, itemTypeForPrompt,
                async () => { // Save action
                    console.info("[UnsavedChangesModal callback] User chose Save."); // INFO
                    hideUnsavedChangesPrompt();
                    if (saveFunction) {
                        try { await saveFunction(); console.info("[UnsavedChangesModal callback] Save successful."); resolve(true); } // INFO
                        catch (error) { console.error("[UnsavedChangesModal callback] Save failed:", error); await message(`Failed to save "${itemName}": ${error.message || error}`, {title: "Save Error", type: "error"}); resolve(false); } // ERROR
                    } else { console.error("[UnsavedChangesModal callback] Save chosen, but save function missing."); await message('Cannot save: Editor reference or save method is missing.', { title: 'Internal Error', type: 'error' }); resolve(false); } // ERROR
                },
                () => { // Discard action
                    console.info("[UnsavedChangesModal callback] User chose Don't Save (Discard)."); // INFO
                    hideUnsavedChangesPrompt();
                    if (discardFunction) discardFunction();
                    if (resetEditorFunction && typeof resetEditorFunction === 'function' && initialContentForReset !== null) resetEditorFunction(initialContentForReset);
                    resolve(true);
                },
                () => { // Cancel action
                    console.info("[UnsavedChangesModal callback] User chose Cancel."); // INFO
                    hideUnsavedChangesPrompt();
                    resolve(false);
                }
            );
        });
    }
}

export async function loadPdfAnnotationsFromFile(pdfAbsPath) {
    const currentProj = get(project); // Get current project state
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
        // If the path is already relative (e.g. from a failed load retry), this might be okay.
        // Or, this indicates an issue elsewhere in path management.
    }
    relativePdfPath = relativePdfPath.replace(/\\/g, '/'); // Normalize to forward slashes for consistency

    const filename = await basename(pdfAbsPath); // Keep for logging or UI
    project.update(p => ({ ...p, statusMessage: `Loading annotations for ${filename}...`})); // Optional: status update

    try {
        // Backend now expects original_pdf_abs_path_str to be the relative path for DB keying
        const annotationsJsonString = await invoke('load_pdf_annotations', { originalPdfAbsPathStr: relativePdfPath });

        if (annotationsJsonString && typeof annotationsJsonString === 'string') {
            try {
                const parsedAnnotations = JSON.parse(annotationsJsonString);
                setLoadedPdfAnnotations(parsedAnnotations || []);
            } catch (parseError) {
                console.error(`[ProjectService] Failed to parse annotations for ${relativePdfPath}:`, parseError);
                setPdfAnnotationsLoadFailed(pdfAbsPath, `Failed to parse loaded annotations: ${parseError.message}`);
            }
        } else if (annotationsJsonString === null) {
            setLoadedPdfAnnotations([]); // No annotations found is a valid state
        } else {
            // Should not happen if backend returns string or null.
            console.warn(`[ProjectService] Unexpected response from load_pdf_annotations for ${relativePdfPath}:`, annotationsJsonString);
            setLoadedPdfAnnotations([]);
        }
    } catch (e) {
        const errorMessage = e.message || String(e);
        console.error(`[ProjectService] Error loading annotations for ${relativePdfPath}:`, errorMessage);
        setPdfAnnotationsLoadFailed(pdfAbsPath, `Service call failed: ${errorMessage}`);
    }
}