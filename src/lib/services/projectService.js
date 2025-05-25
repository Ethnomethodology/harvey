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
	setTranscriptData,
	toggleTranscribeModal,
	setTranscriptionStatus,
	updateTranscriptionProgress,
	clearTranscriptionStatus,
	selectMedia,
	clearTranscriptState,
	markTranscriptAsSaved,

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
    setPdfAnnotationsLoadFailed
} from '$lib/stores/projectStore.js';

import { getCloudConfig } from './configureActions.js';

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

export async function loadProjectDataAndUpdateStore(projectXmlPath) {
    if (!projectXmlPath || projectXmlPath.trim() === '') {
        console.error('[ProjectService] loadProjectDataAndUpdateStore called without a valid projectXmlPath');
        project.update((current) => ({ ...current, isLoading: false, error: 'Project path is missing.', statusMessage: 'Error: Project path is missing.' }));
        throw new Error('projectXmlPath is required');
    }
    console.log('[ProjectService] Calling load_project_data:', projectXmlPath);
    project.update((current) => ({ ...current, isLoading: true, error: null, statusMessage: 'Loading project data...' }));
    try {
        const loadedData = await invoke('load_project_data', { projectXmlPath });
        console.log('[ProjectService] Raw Data received from backend:', loadedData);

        const dataToSet = {
            name: loadedData.project_name,
            xmlPath: loadedData.project_xml_path,
            baseDirectory: loadedData.base_directory,
            files: loadedData.files || [],
            documentFiles: loadedData.document_files || [],
            tableFiles: loadedData.table_files || [],
            imageFiles: loadedData.image_files || [],
            importedTranscriptFiles: loadedData.imported_transcript_files || [],
            documentMetadataFiles: loadedData.document_metadata_files || [],
            pdfAnnotationFiles: loadedData.pdf_annotation_files || [],
            isLoading: false,
            error: null,
            statusMessage: `Loaded project: ${loadedData.project_name}`
        };
        project.update((current) => ({ ...current, ...dataToSet }));
        console.log('[ProjectService] Project store updated with core data.');

        await emit('project-view-ready', { projectXmlPath: projectXmlPath });
        console.log("[ProjectService] 'project-view-ready' emitted.");

        let firstMediaFileEntry = null;
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
        firstMediaFileEntry = findFirstMediaRecursive(loadedData.files || []);
        if (firstMediaFileEntry) {
            console.log(`[ProjectService] Found first media file in tree: ${firstMediaFileEntry.name}. Selecting...`);
            selectMedia(firstMediaFileEntry);
        } else {
            console.log('[ProjectService] No media files found in project tree, clearing selection via selectMedia(null).');
            selectMedia(null);
        }
    } catch (error) {
        console.error('[ProjectService] Failed to load project data:', error);
        project.update((current) => ({ ...current, isLoading: false, error: error?.message || 'Unknown error loading project.', statusMessage: `Error loading project.` }));
        throw error;
    }
}

export async function importMediaFile(importType = null) {
    console.log(`[ProjectService] Starting media import... (Type: ${importType || 'generic'})`);
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
            console.log('[ProjectService] Media import cancelled.');
            project.update(p => ({ ...p, statusMessage: 'Media import cancelled.' }));
            return;
        }

        const sourceFilePath = selected;
        const filename = await basename(sourceFilePath);

        const canProceed = await checkUnsavedChangesThenProceed(null, `importing media: ${filename}`);
        if (!canProceed) {
            console.log('[ProjectService] Media import cancelled due to unsaved changes check.');
            return;
        }

        project.update(p => ({ ...p, statusMessage: `Importing ${filename}...`, isImportingAsset: true }));

        const updatedFiles = await invoke('import_media', {
            sourceFilePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });
        console.log('[ProjectService] Import finished. Received potentially updated file list (tree):', updatedFiles);

        if (Array.isArray(updatedFiles)) {
            project.update(p => ({
                ...p,
                files: updatedFiles,
                isImportingAsset: false,
                error: null,
                statusMessage: `${filename} imported.`
            }));
            let importedEntry = null;
            const importedStem = filename.includes('.') ? filename.substring(0, filename.lastIndexOf('.')) : filename;
            function findImportedRecursive(nodes, stem, name) {
                 if (!Array.isArray(nodes)) return null;
                 for (const node of nodes) {
                     if (node.file_type === 'media' && !node.is_directory && node.name === name && node.media_xml_identifier === stem) { return node; }
                     if (node.children && node.children.length > 0) {
                         const found = findImportedRecursive(node.children, stem, name);
                         if (found) return found;
                     }
                 }
                 return null;
            }
            importedEntry = findImportedRecursive(updatedFiles, importedStem, filename);

            if (importedEntry) {
                console.log('[ProjectService] Auto-selecting imported media:', importedEntry.name);
                selectMedia(importedEntry);
            } else {
                console.warn('[ProjectService] Could not automatically find and select the newly imported media entry based on expected structure.');
                let firstMedia = null;
                function findFirstMediaRecursive(nodes) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory) return node; if (node.children && node.children.length > 0) { const found = findFirstMediaRecursive(node.children); if (found) return found; } } return null; }
                firstMedia = findFirstMediaRecursive(updatedFiles);
                if (firstMedia) { console.log('[ProjectService] Selecting first available media instead:', firstMedia.name); selectMedia(firstMedia); }
                else { console.log('[ProjectService] No media files found after import.'); selectMedia(null); }
            }
        } else {
            console.error('[ProjectService] Backend import_media returned invalid data:', updatedFiles);
            throw new Error("Received invalid data from import process.");
        }
    } catch (error) {
        console.error('[ProjectService] Failed to import media file:', error);
        const errorMessage = error.message || String(error);
        await message(`Error importing media: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        project.update(p => ({ ...p, isImportingAsset: false, error: `Import failed: ${errorMessage}`, statusMessage: `Error importing media.` }));
        throw error;
    }
}

export async function importDocumentFile() {
    console.log('[ProjectService] Starting document import...');
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;
    const projectBaseDir = currentProject.baseDirectory;

    if (!projectXmlPath || !projectBaseDir) {
        console.error('[ProjectService] Cannot import document: Project XML path or Base Directory missing.');
        await message('Project data is not fully loaded. Cannot import documents.', { title: 'Import Error', type: 'error' });
        return;
    }

     const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a document");
     if (!canProceedDialog) {
         console.log('[ProjectService] Document import cancelled due to unsaved changes check before dialog.');
         return;
     }

    let sourceFilePath = '';
    let backendResultPathAndOriginalFilename = ''; 
    let finalJsonPath = '';
    let finalJsonName = '';
    let originalSourceFilenameForMeta = ''; 

    try {
        const selected = await open({
            multiple: false,
            directory: false,
            filters: [documentFilter],
            title: 'Import Document File'
        });

        if (!selected || typeof selected !== 'string') {
            console.log('[ProjectService] Document import cancelled by user (dialog).');
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
                showConversionPrompt(sourceFilename,
                    () => { hideConversionPrompt(); resolve(true); },
                    () => { hideConversionPrompt(); resolve(false); }
                );
            });
            if (!conversionConfirmed) {
                 console.log('[ProjectService] Document import cancelled by user at conversion prompt.');
                 project.update(p => ({ ...p, statusMessage: 'Document import cancelled.' }));
                 return;
            }
        }

        setAssetImportStatus(true, `Importing ${sourceFilename}...`);

        backendResultPathAndOriginalFilename = await invoke('import_document', {
            sourcePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });
        console.log(`[ProjectService] Backend import_document finished. Result: ${backendResultPathAndOriginalFilename}`);
        
        let tempHtmlPath = backendResultPathAndOriginalFilename;
        if (backendResultPathAndOriginalFilename.includes("|original_filename:")) {
            const parts = backendResultPathAndOriginalFilename.split("|original_filename:");
            tempHtmlPath = parts[0];
        }


        if (tempHtmlPath && tempHtmlPath.toLowerCase().endsWith('.pdf')) {
            console.log('[ProjectService] PDF import successful (copied directly). Refreshing project files...');
            setAssetImportStatus(true, `Refreshing project files...`);
            await refreshProjectFiles();
            const importedPdfName = await basename(tempHtmlPath);
            setAssetImportStatus(false, `Document "${importedPdfName}" imported successfully.`);
            prepareDocumentView(tempHtmlPath, 'documents'); 
            return;
        }

        if (!tempHtmlPath || !tempHtmlPath.toLowerCase().endsWith('.html')) {
             throw new Error("Backend did not return expected temporary HTML path after conversion.");
        }
        setAssetImportStatus(true, `Reading converted HTML...`);
        const htmlContent = await invoke('read_file_content', { path: tempHtmlPath });
        console.log(`[ProjectService] Read ${htmlContent.length} bytes of HTML content.`);

        try {
            await invoke('delete_temporary_file', { path: tempHtmlPath });
            console.log(`[ProjectService] Deleted temporary HTML file: ${tempHtmlPath}`);
        } catch(delErr) {
             console.warn(`[ProjectService] Failed to delete temporary HTML file ${tempHtmlPath}:`, delErr);
        }

        setAssetImportStatus(true, `Parsing HTML and generating document structure...`);
        let lexicalJsonString = '';
        const conversionEditor = createConversionEditor('import-doc');
        try {
            const domParser = new DOMParser();
            const dom = domParser.parseFromString(htmlContent, 'text/html');

            await conversionEditor.update(() => {
                const nodes = _generateNodesFromDOM(conversionEditor, dom);
                _getRoot().clear();
                _getRoot().append(...nodes);
            });

            const editorState = conversionEditor.getEditorState();
             if (editorState.isEmpty()) {
                 console.warn("[ProjectService Import] HTML parsing resulted in an empty Lexical state. Saving basic structure.");
                 conversionEditor.update(() => {
                    _getRoot().clear();
                    const para = _createParagraphNode();
                    para.append(_createTextNode(`[Content from ${sourceFilename} could not be fully parsed] `));
                    _getRoot().append(para);
                 });
                 lexicalJsonString = JSON.stringify(conversionEditor.getEditorState().toJSON(), null, 2);

             } else {
                lexicalJsonString = JSON.stringify(editorState.toJSON(), null, 2);
                console.log(`[ProjectService] Successfully converted HTML to Lexical JSON (${lexicalJsonString.length} bytes).`);
             }
        } catch (lexicalError) {
            console.error('[ProjectService] Error during HTML to Lexical conversion:', lexicalError);
            const errorEditor = createConversionEditor('import-error');
            errorEditor.update(() => {
                _getRoot().clear();
                const p = _createParagraphNode();
                p.append(_createTextNode(`Error importing content from ${sourceFilename}: ${lexicalError.message || lexicalError}`));
                _getRoot().append(p);
            });
            lexicalJsonString = JSON.stringify(errorEditor.getEditorState().toJSON(), null, 2);
        }

        if (!lexicalJsonString) {
            throw new Error("Failed to generate Lexical JSON string from HTML.");
        }

        setAssetImportStatus(true, `Determining final file path...`);
        finalJsonPath = await invoke('get_unique_document_path', {
            projectBaseDirStr: projectBaseDir,
            baseName: sourceFilenameStem,
            extension: 'json'
        });
        finalJsonName = await basename(finalJsonPath);
        console.log(`[ProjectService] Determined final JSON path: ${finalJsonPath} (Name: ${finalJsonName})`);

        setAssetImportStatus(true, `Saving document ${finalJsonName}...`);
        await invoke('save_document_and_update_xml', {
            projectXmlPath: projectXmlPath,
            targetPath: finalJsonPath,
            documentName: finalJsonName, 
            jsonContent: lexicalJsonString
        });
        console.log(`[ProjectService] Saved final JSON document and updated XML (including metadata entry).`);

        setAssetImportStatus(true, `Refreshing project files...`);
        await refreshProjectFiles();
        setAssetImportStatus(false, `Document "${sourceFilename}" imported successfully as "${finalJsonName}".`);

        if (finalJsonPath) {
            console.log(`[ProjectService] Import successful. Auto-selecting document: ${finalJsonPath}`);
            prepareDocumentView(finalJsonPath, 'documents');
        } else {
             console.warn('[ProjectService] Import finished, but final JSON path is missing. Cannot auto-select.');
        }

    } catch (error) {
        console.error('[ProjectService] Failed during document import process:', error);
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error during import');
        await message(`Error importing document: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during import: ${errorMessage}`);

        if (backendResultPathAndOriginalFilename) {
            let pathToClean = backendResultPathAndOriginalFilename;
            if (backendResultPathAndOriginalFilename.includes("|original_filename:")) {
                pathToClean = backendResultPathAndOriginalFilename.split("|original_filename:")[0];
            }
            if (!pathToClean.toLowerCase().endsWith('.pdf') && pathToClean.toLowerCase().endsWith('.html')) {
                 try {
                     console.warn(`[ProjectService Import Error] Attempting cleanup of temp HTML file: ${pathToClean}`);
                     await invoke('delete_temporary_file', { path: pathToClean });
                 } catch(delErr) { /* Ignore cleanup errors */ }
            }
        }
    }
}

export async function importTableFile() {
    console.log('[ProjectService] Starting table import...');
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;

    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import table: Project XML path missing.');
        await message('Project data is not fully loaded. Cannot import tables.', { title: 'Import Error', type: 'error' });
        return;
    }

    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing a table");
    if (!canProceedDialog) {
        console.log('[ProjectService] Table import cancelled due to unsaved changes check.');
        return;
    }

    let sourceFilePath = '';
    try {
        const selected = await open({
            multiple: false,
            directory: false,
            filters: [tableFilter],
            title: 'Import Table File (CSV or XLSX)'
        });

        if (!selected || typeof selected !== 'string') {
            console.log('[ProjectService] Table import cancelled by user (dialog).');
            project.update(p => ({ ...p, statusMessage: 'Table import cancelled.' }));
            return;
        }
        sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);

        setAssetImportStatus(true, `Importing table ${sourceFilename}...`);

        const finalTablePath = await invoke('import_table_file', {
            sourcePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });
        console.log(`[ProjectService] Backend table import finished. Final path: ${finalTablePath}`);

        setAssetImportStatus(true, `Refreshing project files...`);
        await refreshProjectFiles();
        const importedTableName = await basename(finalTablePath);
        setAssetImportStatus(false, `Table "${importedTableName}" imported successfully.`);

        if (finalTablePath) {
            console.log(`[ProjectService] Import successful. Auto-selecting table view: ${finalTablePath}`);
             prepareDocumentView(finalTablePath, 'tables');
        } else {
            console.warn('[ProjectService] Table import finished, but final path is missing. Cannot auto-select.');
        }

    } catch (error) {
        console.error('[ProjectService] Failed during table import process:', error);
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error during table import');
        await message(`Error importing table: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during table import: ${errorMessage}`);
    }
}

export async function importImageFile() {
    console.log('[ProjectService] Starting image import...');
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;

    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import image: Project XML path missing.');
        await message('Project data is not fully loaded. Cannot import images.', { title: 'Import Error', type: 'error' });
        return;
    }

    const canProceedDialog = await checkUnsavedChangesThenProceed(null, "importing an image");
    if (!canProceedDialog) {
        console.log('[ProjectService] Image import cancelled due to unsaved changes check.');
        return;
    }

    let sourceFilePath = '';
    try {
        const selected = await open({
            multiple: false,
            directory: false,
            filters: [imageFilter],
            title: 'Import Image File'
        });

        if (!selected || typeof selected !== 'string') {
            console.log('[ProjectService] Image import cancelled by user (dialog).');
            project.update(p => ({ ...p, statusMessage: 'Image import cancelled.' }));
            return;
        }
        sourceFilePath = selected;
        const sourceFilename = await basename(sourceFilePath);

        setAssetImportStatus(true, `Importing image ${sourceFilename}...`);

        const finalImagePath = await invoke('import_image_file', {
            sourcePathStr: sourceFilePath,
            projectXmlPathStr: projectXmlPath
        });
        console.log(`[ProjectService] Backend image import finished. Final path: ${finalImagePath}`);

        setAssetImportStatus(true, `Refreshing project files...`);
        await refreshProjectFiles();
        const importedImageName = await basename(finalImagePath);
        setAssetImportStatus(false, `Image "${importedImageName}" imported successfully.`);

        if (finalImagePath) {
            console.log(`[ProjectService] Image import successful. Auto-selecting image view: ${finalImagePath}`);
            prepareDocumentView(finalImagePath, 'images');
        } else {
            console.warn('[ProjectService] Image import finished, but final path is missing. Cannot auto-select.');
        }

    } catch (error) {
        console.error('[ProjectService] Failed during image import process:', error);
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error during image import');
        await message(`Error importing image: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during image import: ${errorMessage}`);
    }
}

export async function importTranscriptFile(sourceType = 'msWord') {
    console.log(`[ProjectService] Starting transcript import (Source Type: ${sourceType})...`);
    const currentProject = get(project);
    const projectXmlPath = currentProject.xmlPath;

    if (!projectXmlPath) {
        console.error('[ProjectService] Cannot import transcript: Project XML path missing.');
        await message('Project data is not fully loaded. Cannot import transcripts.', { title: 'Import Error', type: 'error' });
        return;
    }

    const canProceedDialog = await checkUnsavedChangesThenProceed(null, `importing a ${sourceType} transcript`);
    if (!canProceedDialog) {
        console.log('[ProjectService] Transcript import cancelled due to unsaved changes check.');
        return;
    }

    let sourceDocxPath = '';
    try {
        if (sourceType === 'msWord') {
            const selected = await open({
                multiple: false,
                directory: false,
                filters: [wordDocumentFilter],
                title: 'Import MS Word Transcript (.docx)'
            });

            if (!selected || typeof selected !== 'string') {
                console.log('[ProjectService] Word transcript import cancelled by user (dialog).');
                project.update(p => ({ ...p, statusMessage: 'Transcript import cancelled.' }));
                return;
            }
            sourceDocxPath = selected;
            const sourceFilename = await basename(sourceDocxPath);

            setAssetImportStatus(true, `Importing transcript from ${sourceFilename}...`);

            const newTranscriptJsonPath = await invoke('import_word_transcript', {
                sourceDocxPathStr: sourceDocxPath,
                projectXmlPathStr: projectXmlPath
            });
            console.log(`[ProjectService] Backend transcript import finished. New JSON path: ${newTranscriptJsonPath}`);

            setAssetImportStatus(true, `Refreshing project files...`);
            await refreshProjectFiles();
            const importedTranscriptName = await basename(newTranscriptJsonPath);
            setAssetImportStatus(false, `Transcript "${importedTranscriptName}" imported successfully.`);

            if (newTranscriptJsonPath) {
                prepareImportedTranscriptView(newTranscriptJsonPath); 
            }

        } else {
            throw new Error(`Unsupported transcript source type: ${sourceType}`);
        }

    } catch (error) {
        console.error('[ProjectService] Failed during transcript import process:', error);
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error during transcript import');
        await message(`Error importing transcript: ${errorMessage}`, { title: 'Import Error', type: 'error' });
        setAssetImportStatus(false, `Error during transcript import: ${errorMessage}`);
    }
}

export async function loadTableData(tablePath) {
    if (!tablePath) {
        console.error('[ProjectService] loadTableData called without a valid tablePath');
        throw new Error('tablePath is required');
    }
    console.log(`[ProjectService] Calling load_table_data for: ${tablePath}`);
    try {
        const tableData = await invoke('load_table_data', { tablePathStr: tablePath });
        console.log(`[ProjectService] Received table data:`, Array.isArray(tableData) ? `${tableData.length} rows` : '(invalid format)');
        if (!Array.isArray(tableData)) {
            throw new Error("Backend returned invalid data format for table.");
        }
        return tableData;
    } catch (error) {
        console.error(`[ProjectService] Failed to load table data for ${tablePath}:`, error);
        const errorMessage = error.message || String(error);
        await message(`Error loading table data: ${errorMessage}`, { title: 'Load Table Error', type: 'error' });
        throw error;
    }
}

// Helper function to parse HH:MM:SS.mmm, MM:SS.mmm, or SS.mmm into seconds
function parseTimestampStringToSeconds(timestampStr) {
    if (!timestampStr || typeof timestampStr !== 'string') return 0;
    const cleanedStr = timestampStr.trim();
    const parts = cleanedStr.split(':');
    let seconds = 0;
    try {
        if (parts.length === 3) { // HH:MM:SS.mmm
            seconds = parseInt(parts[0], 10) * 3600 + parseInt(parts[1], 10) * 60 + parseFloat(parts[2]);
        } else if (parts.length === 2) { // MM:SS.mmm
            seconds = parseInt(parts[0], 10) * 60 + parseFloat(parts[1]);
        } else if (parts.length === 1) { // SS.mmm or S.mmm
            seconds = parseFloat(parts[0]);
        } else {
            console.warn(`[parseTimestampStringToSeconds] Unexpected timestamp format: ${timestampStr}`);
            return 0;
        }
    } catch (e) {
        console.error(`[parseTimestampStringToSeconds] Error parsing timestamp: ${timestampStr}`, e);
        return 0;
    }
    // Ensure result is a number and fix to 3 decimal places for milliseconds
    return isNaN(seconds) ? 0 : parseFloat(seconds.toFixed(3));
}

export function parseLexicalTableToSegments(lexicalTableJsonString) {
    console.log("[ProjectService] Attempting parseLexicalTableToSegments...");
    let parsedFullEditorState;
    try {
        parsedFullEditorState = JSON.parse(lexicalTableJsonString);
        if (!parsedFullEditorState || !parsedFullEditorState.root || !Array.isArray(parsedFullEditorState.root.children)) {
            console.error("[ProjectService] Invalid Lexical JSON structure: missing root or root.children. Content snapshot:", lexicalTableJsonString.substring(0, 500));
            return [];
        }
    } catch (error) {
        console.error("[ProjectService] Failed to parse lexicalTableJsonString:", error, "Content snapshot:", lexicalTableJsonString.substring(0, 500));
        return [];
    }

    const cellTextEditor = createHeadlessEditor({
        nodes: [RootNode, ParagraphNode, TextNode, ExtendedTextNode, LineBreakNode], // Minimal set for cell text processing
        namespace: `cell-parser-editor-${Date.now()}`,
        onError: (e) => console.error("[CellParserEditor] Error:", e),
    });

    const segmentsArray = [];
    try {
        const rootChildren = parsedFullEditorState.root.children;
        const tableNode = rootChildren.find(node => node.type === 'table');

        if (!tableNode || !tableNode.children || !Array.isArray(tableNode.children)) {
            console.error("[ProjectService] No 'table' node found or table has no children in parsed JSON.");
            return [];
        }

        // Iterate over table rows, skipping the header row (index 0)
        for (let i = 1; i < tableNode.children.length; i++) {
            const rowNode = tableNode.children[i];
            if (rowNode.type !== 'tablerow' || !rowNode.children || !Array.isArray(rowNode.children) || rowNode.children.length < 4) {
                console.warn(`[ProjectService] Skipping row at index ${i}: not a valid 'tablerow' or has insufficient cells (<4). Row:`, JSON.stringify(rowNode).substring(0,300));
                continue;
            }

            try {
                // --- Timestamp Cell (index 1 of rowNode.children) ---
                const timestampCellNode = rowNode.children[1]; // This is the TableCellNode
                if (timestampCellNode.type !== 'tablecell') {
                     console.warn(`[ProjectService] Row ${i}, Expected Cell 1 (Timestamp) to be 'tablecell', got '${timestampCellNode.type}'. Skipping row.`); continue;
                }
                // Construct a valid Lexical JSON string for this cell's content by wrapping its children in a root
                const timestampCellContentForEditor = {
                    root: { type: 'root', children: timestampCellNode.children || [], direction: null, format: '', indent: 0, version: 1 }
                };
                cellTextEditor.setEditorState(cellTextEditor.parseEditorState(JSON.stringify(timestampCellContentForEditor)));
                const timestampText = cellTextEditor.getEditorState().read(() => _getRoot().getTextContent());
                const timeParts = timestampText.split(' - ');
                const startTime = parseTimestampStringToSeconds(timeParts[0]);
                const endTime = timeParts.length > 1 ? parseTimestampStringToSeconds(timeParts[1]) : startTime;

                // --- Speaker Cell (index 2 of rowNode.children) ---
                const speakerCellNode = rowNode.children[2]; // TableCellNode
                if (speakerCellNode.type !== 'tablecell') {
                     console.warn(`[ProjectService] Row ${i}, Expected Cell 2 (Speaker) to be 'tablecell', got '${speakerCellNode.type}'. Skipping row.`); continue;
                }
                const speakerCellContentForEditor = {
                    root: { type: 'root', children: speakerCellNode.children || [], direction: null, format: '', indent: 0, version: 1 }
                };
                cellTextEditor.setEditorState(cellTextEditor.parseEditorState(JSON.stringify(speakerCellContentForEditor)));
                let speakerName = cellTextEditor.getEditorState().read(() => _getRoot().getTextContent()).trim();
                if (!speakerName) speakerName = "Unknown";

                // --- Text Content Cell (index 3 of rowNode.children) ---
                const textContentCellNode = rowNode.children[3]; // TableCellNode
                if (textContentCellNode.type !== 'tablecell') {
                     console.warn(`[ProjectService] Row ${i}, Expected Cell 3 (Text) to be 'tablecell', got '${textContentCellNode.type}'. Skipping row.`); continue;
                }
                // The segment's 'text' field requires a stringified Lexical JSON representing this cell's content.
                // So, we wrap the cell's children in a root structure.
                const segmentLexicalContentJson = {
                    root: {
                        type: 'root',
                        children: textContentCellNode.children || [], // These are the actual content nodes (e.g., ParagraphNode)
                        direction: null,
                        format: '',
                        indent: 0,
                        version: 1
                    }
                };
                const segmentTextJsonString = JSON.stringify(segmentLexicalContentJson);

                segmentsArray.push({
                    start_time: startTime,
                    end_time: endTime,
                    speaker: speakerName,
                    text: segmentTextJsonString
                });

            } catch (cellProcessingError) {
                console.error(`[ProjectService] Error processing cells in row ${i}:`, cellProcessingError, "RowNode snapshot:", JSON.stringify(rowNode).substring(0,300));
                // Add a placeholder segment to indicate an error for this specific row
                segmentsArray.push({
                    start_time: 0,
                    end_time: 0,
                    speaker: "Error Processing Row",
                    text: JSON.stringify({ root: { type: 'root', children:[], direction:null, format:'', indent:0, version:1 } }) // Empty valid Lexical JSON
                });
            }
        }
    } catch (tableProcessingError) {
        console.error("[ProjectService] Error processing table structure:", tableProcessingError, "Parsed Full Editor State snapshot:", JSON.stringify(parsedFullEditorState).substring(0,500));
        return []; // Return empty if there's a major error in table processing
    }
    console.log(`[ProjectService] parseLexicalTableToSegments finished. Successfully parsed ${segmentsArray.length} segments.`);
    return segmentsArray;
}


export async function loadTranscriptFile(transcriptFilePath) { 
    if (!transcriptFilePath) { 
        project.update(p => ({ ...p, isTranscriptLoading: false, error: "Transcript file path is missing." })); 
        throw new Error("Transcript file path is required."); 
    } 
    if (!transcriptFilePath.toLowerCase().endsWith('.json')) { 
        console.warn(`[ProjectService] Attempting to load non-JSON file as transcript: ${transcriptFilePath}. Proceeding, but might fail.`); 
    } 
    const filename = transcriptFilePath.split(/[\\/]/).pop(); 
    console.log(`[ProjectService] Loading transcript: ${filename}`); 
    project.update(p => ({ ...p, isTranscriptLoading: true, error: null, statusMessage: `Loading transcript ${filename}...` })); 
    try { 
        const fullLexicalJsonString = await invoke('load_transcript_json', { transcriptPath: transcriptFilePath });
        const segmentsArray = parseLexicalTableToSegments(fullLexicalJsonString);
        setTranscriptData(transcriptFilePath, segmentsArray, false); 
        console.log(`[ProjectService] Transcript ${filename} loaded successfully. Parsed ${segmentsArray.length} segments.`); 
    } catch (error) { 
        const errorMessage = error?.message || String(error); 
        console.error(`[ProjectService] Failed to load transcript file ${filename}:`, errorMessage); 
        project.update(p => ({ ...p, segments: [], currentTranscriptPath: null, transcriptDirty: false, isTranscriptLoading: false, error: `Transcript load failed: ${errorMessage}`, statusMessage: `Error loading transcript ${filename}.` })); 
        throw new Error(`Failed to load transcript: ${errorMessage}`); 
    } 
}

export async function saveTranscriptData() {
    const projData = get(project);
    const transcriptPath = projData.currentTranscriptPath;
    const transcriptSegments = projData.segments; // These segments now have Lexical JSON in their .text field for each cell
    const projectXmlPath = projData.xmlPath;

    if (!transcriptPath) {
        console.error("[ProjectService] Save failed: No transcript path is currently set in the store.");
        project.update(p => ({ ...p, statusMessage: 'Error: Cannot save, no transcript loaded.' }));
        throw new Error("Cannot save, no transcript loaded.");
    }
    if (!projectXmlPath) {
        console.error("[ProjectService] Save failed: Project XML path is missing in the store.");
        project.update(p => ({ ...p, statusMessage: 'Error: Cannot save, project path unknown.' }));
        throw new Error("Cannot save, project path unknown.");
    }
    if (!transcriptPath.toLowerCase().endsWith('.json')) {
        console.error(`[ProjectService] Attempting to save transcript to non-JSON file: ${transcriptPath}. Aborting.`);
        project.update(p => ({ ...p, statusMessage: 'Error: Transcript must be saved as .json.'}));
        throw new Error("Transcript must be saved as .json.");
    }
    const filename = transcriptPath.split(/[\\/]/).pop();
    console.log(`[ProjectService] Saving transcript: ${filename}`);
    project.update(p => ({ ...p, statusMessage: `Saving transcript ${filename}...` }));

    // --- MODIFICATION: Construct the full Lexical Table JSON from store segments ---
    let fullLexicalTableJsonString = "";
    try {
        const editorForTableAssembly = createHeadlessEditor({
            nodes: ALL_EDITOR_NODES,
            namespace: `table-assembly-editor-${Date.now()}`,
            onError: (e) => console.error("[TableAssemblyEditor] Error:", e),
        });

        await editorForTableAssembly.update(() => {
            const root = _getRoot();
            root.clear();
            const tableNode = _createTableNode();
            // Assuming colWidths were set when the table was first created/loaded,
            // or handle default widths here if needed.
            // const colWidths = [50, 140, 120, 450]; // Example
            // tableNode.setColWidths?.(colWidths);


            // Header Row (assuming it's static and not stored in `transcriptSegments`)
            const headerRow = _createTableRowNode();
            const headers = ["#", "Timestamp", "Speaker", "Text"];
            for (const headerText of headers) {
                const cell = _createTableCellNode({ headerState: 'column' });
                const paragraph = _createParagraphNode();
                paragraph.append(_createTextNode(headerText));
                cell.append(paragraph);
                headerRow.append(cell);
            }
            tableNode.append(headerRow);

            // Data Rows from store's segments
            for (let i = 0; i < transcriptSegments.length; i++) {
                const segment = transcriptSegments[i];
                const dataRow = _createTableRowNode();

                // Segment Number Cell
                const cellNum = _createTableCellNode();
                const pNum = _createParagraphNode();
                pNum.append(_createTextNode(String(i + 1)));
                cellNum.append(pNum);
                dataRow.append(cellNum);

                // Timestamp Cell
                const cellTime = _createTableCellNode();
                const pTime = _createParagraphNode();
                const startTime = formatTimestampHtml(segment.start_time || 0);
                const endTime = formatTimestampHtml(segment.end_time || 0);
                pTime.append(_createTextNode(`${startTime} - ${endTime}`));
                cellTime.append(pTime);
                dataRow.append(cellTime);

                // Speaker Cell
                const cellSpeaker = _createTableCellNode();
                const pSpeaker = _createParagraphNode();
                pSpeaker.append(_createTextNode(segment.speaker || "Unknown"));
                cellSpeaker.append(pSpeaker);
                dataRow.append(cellSpeaker);

                // Text Content Cell (segment.text is already Lexical JSON for the cell's content)
                const cellText = _createTableCellNode();
                if (segment.text && typeof segment.text === 'string') {
                    try {
                        // The segment.text is already a stringified Lexical JSON for the cell's content.
                        // We need to parse it and append its root's children to the new cellText.
                        const cellEditorState = editorForTableAssembly.parseEditorState(segment.text);
                        
                        // Create a temporary root in the assembly editor to access children
                        const tempRoot = cellEditorState.read(() => _getRoot());
                        const cellChildren = tempRoot.getChildren();
                        
                        // Append clones of these children to the actual table cell node
                        cellChildren.forEach(node => cellText.append(node.clone()));

                    } catch (parseError) {
                        console.error(`[ProjectService Save] Error parsing cell content for segment ${i}:`, parseError, segment.text.substring(0,100));
                        const pError = _createParagraphNode();
                        pError.append(_createTextNode("[Error rendering cell content]"));
                        cellText.append(pError);
                    }
                } else {
                    cellText.append(_createParagraphNode()); // Empty if no text
                }
                dataRow.append(cellText);
                tableNode.append(dataRow);
            }
            root.append(tableNode);
            root.append(_createParagraphNode()); // Trailing paragraph
        });
        fullLexicalTableJsonString = JSON.stringify(editorForTableAssembly.getEditorState().toJSON());
        console.log("[ProjectService] Successfully assembled full Lexical Table JSON for saving.");
    } catch (assemblyError) {
        console.error("[ProjectService] Error assembling full Lexical Table JSON:", assemblyError);
        project.update(p => ({ ...p, error: `Save failed: Error preparing data. ${assemblyError.message}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to prepare transcript data for saving: ${assemblyError.message}`);
    }
    // --- END MODIFICATION ---

    try {
        // Corrected invoke: arguments should be in an object
        await invoke('save_transcript_json', {
            projectXmlPath: projectXmlPath,
            transcriptPath: transcriptPath,
            lexicalTableJsonString: fullLexicalTableJsonString // Pass the assembled string
        });
        console.log("[ProjectService] Transcript save invoke successful.");
        markTranscriptAsSaved(); 
    } catch (error) {
        const errorMessage = error?.message || String(error);
        console.error("[ProjectService] Failed to save transcript:", errorMessage);
        project.update(p => ({ ...p, error: `Save failed: ${errorMessage}`, statusMessage: `Error saving transcript.` }));
        throw new Error(`Failed to save transcript: ${errorMessage}`);
    }
}


export async function refreshProjectFiles() { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) { console.warn('[ProjectService] Cannot refresh: No project path loaded.'); return; } console.log('[ProjectService] Refreshing file list (via load_project_data) for project:', projectXmlPath); project.update(p => ({ ...p, statusMessage: 'Refreshing file list...' })); try { await loadProjectDataAndUpdateStore(projectXmlPath); console.log('[ProjectService] File list refreshed successfully via reload.'); project.update(p => ({ ...p, statusMessage: 'Project refreshed.' })); } catch (error) { const errorMessage = error?.message || String(error); console.error('[ProjectService] Failed to refresh project files:', error); project.update(p => ({ ...p, error: `Refresh failed: ${errorMessage}`, statusMessage: 'Error refreshing file list.' })); } }
export async function renameProjectItem(itemPath, newName, itemType) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) { await message('Project data not loaded. Cannot rename.', { title: 'Rename Error', type: 'error' }); throw new Error('Project path missing.'); } if (!itemPath || !newName) { await message('Missing item path or new name.', { title: 'Rename Error', type: 'error' }); throw new Error('Missing parameters.'); } const oldFilename = await basename(itemPath); project.update(p => ({ ...p, statusMessage: `Renaming ${oldFilename} to ${newName}...` })); try { console.log(`[ProjectService Rename] Calling backend: rename_project_item`, { itemPath, newName, projectXmlPath }); await invoke('rename_project_item', { itemPath: itemPath, newName: newName, projectXmlPath: projectXmlPath }); console.log(`[ProjectService Rename] Item renamed successfully. Refreshing file list.`); project.update(p => ({ ...p, statusMessage: `Renamed ${oldFilename} to ${newName}. Refreshing...` })); await refreshProjectFiles(); } catch (error) { const errorMessage = error?.message || String(error); console.error(`[ProjectService Rename] Failed to rename item ${oldFilename}:`, error); await message(`Error renaming item: ${errorMessage}`, { title: 'Rename Failed', type: 'error' }); project.update(p => ({ ...p, error: `Rename failed: ${errorMessage}`, statusMessage: `Error renaming ${oldFilename}.` })); throw error; } }
export async function deleteProjectItem(itemPath) { const currentProj = get(project); const projectXmlPath = currentProj.xmlPath; if (!projectXmlPath) { await message('Project data not loaded. Cannot delete.', { title: 'Delete Error', type: 'error' }); throw new Error('Project path missing.'); } if (!itemPath) { await message('Missing item path.', { title: 'Delete Error', type: 'error' }); throw new Error('Missing parameters.'); } const filename = await basename(itemPath); project.update(p => ({ ...p, statusMessage: `Deleting ${filename}...` })); try { console.log(`[ProjectService Delete] Calling backend: delete_project_item`, { itemPath, projectXmlPath }); await invoke('delete_project_item', { itemPath: itemPath, projectXmlPath: projectXmlPath }); console.log(`[ProjectService Delete] Item deleted successfully. Refreshing file list.`); project.update(p => ({ ...p, statusMessage: `Deleted ${filename}. Refreshing...` })); const projState = get(project); const wasSelectedMedia = projState.selectedMediaFile?.path === itemPath; const wasCurrentTranscript = projState.currentTranscriptPath === itemPath; const wasSelectedDocument = projState.selectedDocumentPath === itemPath; const wasSelectedImportedTranscript = projState.currentImportedTranscriptPath === itemPath; if (wasSelectedMedia) { console.log("[ProjectService Delete] Deleted item was selected media. Clearing selection."); selectMedia(null); } else if (wasCurrentTranscript) { console.log("[ProjectService Delete] Deleted item was loaded media transcript. Clearing transcript state."); clearTranscriptState(); } else if (wasSelectedDocument) { console.log("[ProjectService Delete] Deleted item was selected document/table/image. Clearing view state."); prepareDocumentView(null); } else if (wasSelectedImportedTranscript) { console.log("[ProjectService Delete] Deleted item was selected imported transcript. Clearing view state."); prepareImportedTranscriptView(null); } await refreshProjectFiles(); } catch (error) { const errorMessage = error?.message || String(error); console.error(`[ProjectService Delete] Failed to delete item ${filename}:`, error); await message(`Error deleting item: ${errorMessage}`, { title: 'Delete Failed', type: 'error' }); project.update(p => ({ ...p, error: `Delete failed: ${errorMessage}`, statusMessage: `Error deleting ${filename}.` })); throw error; } }
export async function handleTrimMediaConfirm(originalMediaPath, startTime, endTime) { if (!originalMediaPath || typeof startTime !== 'number' || typeof endTime !== 'number' || startTime < 0 || endTime <= startTime) { throw new Error(`Invalid trim parameters provided.`); } const filename = await basename(originalMediaPath); project.update(p => ({ ...p, isImportingAsset: true, statusMessage: `Trimming ${filename}...` })); try { const updatedFiles = await invoke('trim_media', { originalMediaPath, startTime, endTime }); console.log('[ProjectService] Trim finished. Received potentially updated file list (tree):', updatedFiles); if (Array.isArray(updatedFiles)) { project.update(p => ({ ...p, files: updatedFiles, isImportingAsset: false, error: null, statusMessage: 'Media trimmed successfully.' })); let trimmedEntry = null; const originalFilename = await basename(originalMediaPath); const originalExtension = originalFilename.includes('.') ? originalFilename.substring(originalFilename.lastIndexOf('.')) : ''; function findTrimmedRecursive(nodes, stemPrefix, extension) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory && node.name.startsWith(stemPrefix) && node.name.includes('_trimmed_') && node.name.endsWith(extension)) { return node; } if (node.children && node.children.length > 0) { const found = findTrimmedRecursive(node.children, stemPrefix, extension); if (found) return found; } } return null; } const originalStem = originalFilename.includes('.') ? originalFilename.substring(0, originalFilename.lastIndexOf('.')) : originalFilename; trimmedEntry = findTrimmedRecursive(updatedFiles, originalStem, originalExtension); if (trimmedEntry) { console.log('[ProjectService] Auto-selecting trimmed media:', trimmedEntry.name); selectMedia(trimmedEntry); } else { console.warn('[ProjectService] Could not automatically find and select the newly trimmed media entry.'); let firstMedia = null; function findFirstMediaRecursive(nodes) { if (!Array.isArray(nodes)) return null; for (const node of nodes) { if (node.file_type === 'media' && !node.is_directory) return node; if (node.children && node.children.length > 0) { const found = findFirstMediaRecursive(node.children); if (found) return found; } } return null; } firstMedia = findFirstMediaRecursive(updatedFiles); if (firstMedia) { console.log('[ProjectService] Selecting first available media instead:', firstMedia.name); selectMedia(firstMedia); } } } else { console.error('[ProjectService] Backend trim_media returned invalid data structure.'); await refreshProjectFiles(); throw new Error("Received invalid data from trim process. File list may be outdated."); } } catch (error) { const errorMessage = error?.message || String(error); console.error('[ProjectService] Backend trim_media command failed:', error); project.update(p => ({ ...p, isImportingAsset: false, error: `Trim failed: ${errorMessage}`, statusMessage: `Error trimming media.` })); throw new Error(`Trim failed: ${errorMessage}`); } }

export let transcribeModalInstance = null; export function registerTranscribeModal(instance) { transcribeModalInstance = instance; console.log('[ProjectService] TranscribeConfirmModal instance registered.'); }
export async function requestTranscription() { const currentProj = get(project); if (!currentProj.selectedMediaFile?.path) { await message('Please select a media file first.', { title: 'Transcription Request', type: 'info'}); return; } if (!currentProj.selectedModelName) { await message('Please select a transcription model first.', { title: 'Transcription Request', type: 'info'}); return; } if (currentProj.isTranscribing) { await message('A transcription job is already in progress.', { title: 'Transcription Request', type: 'info'}); return; } toggleTranscribeModal(true); }
export async function handleConfirmStartTranscription() { console.log('[ProjectService] User confirmed, attempting to start transcription...'); const currentProj = get(project); const jobId = uuidv4(); if (!currentProj.selectedMediaFile?.path || !currentProj.selectedModelName) { console.error('[ProjectService] Start transcription aborted: Missing selected media or model in store state.'); transcribeModalInstance?.setStatusError('Error: Missing media file or model selection.'); clearTranscriptionStatus('Transcription failed.', 'Missing media file or model selection.'); toggleTranscribeModal(false); return; } const selectedModelIdentifier = currentProj.selectedModelName; const isCloudModel = selectedModelIdentifier.startsWith('google-') || selectedModelIdentifier.startsWith('gemini-'); setTranscriptionStatus(true, jobId, `Preparing ${isCloudModel ? 'cloud' : 'local'} transcription...`); try { let invokePromise; const args = { mediaPath: currentProj.selectedMediaFile.path, language: currentProj.selectedLanguage || '', numSpeakers: currentProj.speakers.count, speakerNames: currentProj.speakers.names || [], jobId: jobId }; if (isCloudModel) { console.log(`[ProjectService] Invoking CLOUD transcription backend command...`); let cloudConfig; try { cloudConfig = await getCloudConfig(); } catch (e) { throw new Error(`Failed to get cloud configuration: ${e.message}`); } if (!cloudConfig?.consent) { throw new Error("Cloud transcription consent not given in configuration."); } if (!cloudConfig?.api_key) { throw new Error("Cloud API Key is missing in configuration."); } const cloudArgs = { ...args, cloudModelId: selectedModelIdentifier, apiKey: cloudConfig.api_key }; invokePromise = invoke('run_cloud_transcription', cloudArgs); } else { console.log(`[ProjectService] Invoking LOCAL transcription backend command...`); const localArgs = { ...args, modelName: selectedModelIdentifier }; invokePromise = invoke('run_transcription', localArgs); } const result = await invokePromise; if (!result || typeof result.transcript_file_path !== 'string' || !Array.isArray(result.segments)) { throw new Error("Received invalid transcription result structure from backend."); } console.log(`[ProjectService] Transcription job ${jobId} successful. Received ${result.segments.length} segments. Path: ${result.transcript_file_path}`); setTranscriptData(result.transcript_file_path, result.segments, false); transcribeModalInstance?.setStatusDone('Transcription complete!'); clearTranscriptionStatus('Transcription complete.'); await refreshProjectFiles(); setTimeout(() => { toggleTranscribeModal(false); }, 1500); } catch (error) { const errorMessage = error?.message || String(error); console.error(`[ProjectService] Transcription job ${jobId} failed:`, error); if (errorMessage.toLowerCase().includes('cancelled') || errorMessage.toLowerCase().includes('canceled')) { transcribeModalInstance?.setStatusCancelled('Transcription cancelled.'); clearTranscriptionStatus('Transcription cancelled.'); setTimeout(() => { toggleTranscribeModal(false); }, 1500); } else { transcribeModalInstance?.setStatusError(`Transcription failed: ${errorMessage}`); clearTranscriptionStatus('Transcription failed.', errorMessage); } } }
export async function handleCancelTranscriptionRequest() { const currentProj = get(project); const jobId = currentProj.transcriptionJobId; if (!jobId || !currentProj.isTranscribing) { console.warn('[ProjectService] No active transcription job found to cancel.'); return; } const modelUsedForJob = currentProj.selectedModelName; const isCloudJob = modelUsedForJob && (modelUsedForJob.startsWith('google-') || modelUsedForJob.startsWith('gemini-')); const cancelCommand = isCloudJob ? 'cancel_cloud_transcription' : 'cancel_transcription'; console.log(`[ProjectService] Requesting cancellation for job ${jobId} (Type: ${isCloudJob ? 'Cloud' : 'Local'}) using command: ${cancelCommand}`); transcribeModalInstance?.setStatusCancelling('Requesting cancellation...'); try { await invoke(cancelCommand, { jobId }); console.log(`[ProjectService] Cancellation request sent successfully for job ${jobId}.`); } catch (error) { const errorMessage = error?.message || String(error); console.error(`[ProjectService] Failed to send cancel request for job ${jobId}:`, error); transcribeModalInstance?.setStatusError(`Failed to send cancel request: ${errorMessage}`); project.update(p => ({ ...p, error: `Cancellation request failed: ${errorMessage}` })); } }
export let progressListenerInitialized = false; export let progressUnlistenFn = null; export async function initializeProgressListener() { if (progressListenerInitialized) { return; } console.log('[ProjectService] Initializing progress listener...'); try { progressUnlistenFn = await listen('TRANSCRIPTION_PROGRESS', (event) => { const payload = event.payload; if (!payload || typeof payload !== 'object') { console.warn('[ProjectService] Invalid progress payload:', payload); return; } const eventJobId = payload.jobId ?? payload.job_id; const currentJobId = get(project).transcriptionJobId; if (currentJobId && eventJobId === currentJobId) { updateTranscriptionProgress({ jobId: currentJobId, percent: payload.percent ?? 0, message: payload.message ?? '' }); } }); progressListenerInitialized = true; console.log('[ProjectService] Progress listener attached successfully.'); } catch (e) { console.error('[ProjectService] Failed to initialize progress listener:', e); project.update(p => ({ ...p, error: "Failed to initialize progress listener." })); } }
export function cleanupProgressListener() { if (progressUnlistenFn) { console.log('[ProjectService] Cleaning up progress listener.'); progressUnlistenFn(); progressUnlistenFn = null; } progressListenerInitialized = false; }

export function formatTimestampHtml(seconds) { if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00.000'; const totalMs = Math.round(seconds * 1000); const ms = String(totalMs % 1000).padStart(3, '0'); const totalS = Math.floor(totalMs / 1000); const sec = String(totalS % 60).padStart(2, '0'); const min = String(Math.floor(totalS / 60)).padStart(2, '0'); return `${min}:${sec}.${ms}`; }
export function isLexicalJson(jsonString) { if (!jsonString || typeof jsonString !== 'string') return false; try { const parsed = JSON.parse(jsonString); return parsed && typeof parsed === 'object' && parsed.root && typeof parsed.root === 'object' && Array.isArray(parsed.root.children); } catch (e) { return false; } }

export async function convertAndSaveTranscriptAsDoc() {
    console.log("[ProjectService] convertAndSaveTranscriptAsDoc initiated (TABLE EXPORT v2).");
    const projData = get(project);
    const transcriptPath = projData.currentTranscriptPath;
    const selectedMedia = projData.selectedMediaFile;
    const projectXmlPath = projData.xmlPath;
    const projectBaseDir = projData.baseDirectory;

    if (!transcriptPath) { throw new Error("No transcript file loaded to convert."); }
    if (!selectedMedia || !selectedMedia.path) { throw new Error("No media file selected to base document name on."); }
    if (!projectBaseDir) { throw new Error("Project base directory not found."); }
    if (!projectXmlPath) { throw new Error("Project XML path not found."); }

    project.update(p => ({ ...p, statusMessage: `Converting transcript to table document...` }));

    const finalTableEditor = createHeadlessEditor({
        nodes: ALL_EDITOR_NODES,
        namespace: `doc-table-finalizer-${Date.now()}`,
        onError: (error) => console.error("[DocTableFinalizerEditor] Error:", error),
    });

    let finalLexicalJsonString = "";

    try {
        console.log(`[ProjectService] Reading content from transcript file: ${transcriptPath}`);
        // Assuming loadTranscriptFile now loads the *full lexical table string* from the JSON file
        const fullLexicalTableString = await invoke('load_transcript_json', { transcriptPath: transcriptPath });
        
        if (!fullLexicalTableString) {
            throw new Error("Transcript file content is empty or could not be read as Lexical Table JSON.");
        }

        // At this point, fullLexicalTableString IS the content we want to save as a new document.
        // No further complex parsing or segment-by-segment reconstruction is needed here,
        // as the source file already contains the desired Lexical Table.
        finalLexicalJsonString = fullLexicalTableString;
        
        console.log(`[ProjectService] Using existing Lexical Table JSON for new document (${finalLexicalJsonString.length} bytes).`);
        
        const mediaStemIdentifier = selectedMedia.media_xml_identifier || (() => {
            const mediaName = selectedMedia.name;
            return mediaName.includes('.') 
                ? mediaName.substring(0, mediaName.lastIndexOf('.')) 
                : mediaName;
        })();
        const safeStem = mediaStemIdentifier.replace(/[^a-zA-Z0-9_-]/g, '_');
        const now = new Date();
        const dateStr = now.toISOString().split('T')[0];
        const timeStr = now.toTimeString().split(' ')[0].replace(/:/g, '-');
        const docFilenameBase = `${safeStem}_transcript_as_doc_${dateStr}_${timeStr}`; // Changed name slightly

        project.update(p => ({ ...p, statusMessage: `Saving transcript document...` }));
        const targetFullPath = await invoke('get_unique_document_path', {
            projectBaseDirStr: projectBaseDir,
            baseName: docFilenameBase,
            extension: 'json'
        });
        const docFilename = await basename(targetFullPath);

        await invoke('save_document_and_update_xml', {
            projectXmlPath: projectXmlPath,
            targetPath: targetFullPath,
            documentName: docFilename,
            jsonContent: finalLexicalJsonString // Save the full table JSON
        });

        project.update(p => ({ ...p, statusMessage: `Document file created: ${docFilename}` }));
        await refreshProjectFiles();
        return targetFullPath;

    } catch (error) {
        console.error("[ProjectService] Error in convertAndSaveTranscriptAsDoc (Table Export):", error);
        project.update(p => ({ ...p, statusMessage: `Error converting transcript to document: ${error.message || error}` }));
        throw error; 
    }
}

export async function loadActiveDocumentContent() {
    const currentProj = get(project);
    const filePath = currentProj.selectedDocumentPath;
    if (!filePath) { console.log("[ProjectService] loadActiveDocumentContent called but no document path is selected."); project.update(p => ({...p, isDocumentLoading: false, documentError: null })); return; }
    const filename = await basename(filePath);
    console.log(`[ProjectService] Loading active document: ${filename} (${filePath})`);
    project.update(p => ({ ...p, isDocumentLoading: true, documentError: null }));
    try {
        const jsonContent = await invoke('load_note_json', { filePath });
        if (!jsonContent || jsonContent.trim() === '') { throw new Error("Loaded document content empty/invalid."); }
        try { JSON.parse(jsonContent); } catch (e) { throw new Error(`Loaded document content not valid JSON.`); }
        setLoadedDocumentData(filePath, jsonContent);
        console.log(`[ProjectService] Loaded document content (JSON) for ${filename}.`);
    } catch (error) {
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        console.error(`[ProjectService] Failed load document ${filePath}:`, errorMessage);
        setDocumentLoadFailed(filePath, errorMessage);
        await message(`Error loading document '${filename}': ${errorMessage}`, { title: 'Load Document Error', type: 'error' });
    }
}

export async function saveCurrentPdfAnnotations() {
    const projState = get(project);
    if (!projState.selectedDocumentPath ||
        !projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
        console.warn('[ProjectService] saveCurrentPdfAnnotations called but no PDF is selected.');
        return;
    }
    if (!projState.isPdfAnnotationsDirty) {
        console.log('[ProjectService] PDF annotations are not dirty – nothing to save.');
        return;
    }

    const projectXmlPath = projState.xmlPath;
    const projectBaseDir = projState.baseDirectory;
    if (!projectXmlPath || !projectBaseDir) {
        console.error('[ProjectService] Cannot save PDF annotations: missing project paths.');
        return;
    }
    
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
            projectXmlPathStr: projectXmlPath,
            originalPdfRelativePathStr: relativePdfPath,
            annotationsJsonContent: JSON.stringify(annList)
        });
        markPdfAnnotationsAsSaved();
        console.log('[ProjectService] PDF annotations saved immediately.');
    } catch (error) {
        console.error('[ProjectService] Failed to save PDF annotations immediately:', error);
    }
}

export async function saveDocumentContent(filePath, jsonContent) {
    if (filePath && filePath.toLowerCase().endsWith('.pdf')) {
        console.warn(`[ProjectService saveDocumentContent] Attempted to save PDF content for ${filePath}. This should be handled by PDF annotation saving logic. Aborting.`);
        project.update(p => ({...p, documentError: "PDF content cannot be saved this way.", statusMessage: 'Save failed (PDF type).'}));
        throw new Error("PDF content saving is not handled by saveDocumentContent. Use PDF annotation saving.");
    }

    if (!filePath || jsonContent === null || typeof jsonContent !== 'string') { const errorMsg = "Cannot save document: Missing path or invalid/missing JSON content."; console.error(`[ProjectService] ${errorMsg}`); await message(errorMsg, { title: 'Save Error', type: 'error' }); project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed.'})); throw new Error(errorMsg); }
    try { const parsed = JSON.parse(jsonContent); if (!parsed.root || !parsed.root.children || !Array.isArray(parsed.root.children)) { throw new Error("Invalid Lexical JSON structure."); } } catch (e) { const errorMsg = `Cannot save document: Content not valid JSON or invalid structure. ${e.message}`; console.error(`[ProjectService] ${errorMsg}`); console.error(`[ProjectService] Invalid JSON save attempt:`, jsonContent.substring(0, 500) + "..."); await message(errorMsg, { title: 'Save Error', type: 'error' }); project.update(p => ({...p, documentError: errorMsg, statusMessage: 'Save failed (invalid content).'})); throw new Error(errorMsg); }
    const filename = await basename(filePath);
    console.log(`[ProjectService] Saving document (JSON) to: ${filename} (${filePath})`);
    project.update(p => ({ ...p, statusMessage: `Saving document ${filename}...` }));

    let mainContentSaveError = null;
    try {
        await invoke('save_note_json', { targetPath: filePath, jsonContent: jsonContent });
        markDocumentAsSaved(jsonContent);
        console.log(`[ProjectService] Saved document (JSON): ${filename}`);
    } catch (error) {
        mainContentSaveError = error;
        const errorMessage = typeof error === 'string' ? error : (error?.message || 'Unknown error');
        console.error(`[ProjectService] Failed save document ${filePath}:`, error);
        project.update(p => ({ ...p, documentError: `Failed save document: ${errorMessage}`, statusMessage: `Error saving ${filename}.` }));
    }

    const projState = get(project);
    let metadataSaveError = null;
    if (projState.selectedDocumentPath === filePath && projState.isDocumentMetadataDirty) {
        console.log(`[ProjectService] Document metadata is dirty for ${filename}. Saving metadata...`);
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
        throw metadataSaveError;
    }
}


export async function loadDocumentMetadata(originalDocumentAbsPath) {
    const proj = get(project);
    if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) {
        console.error("[ProjectService loadDocumentMetadata] Missing project XML path, base directory, or original document path.");
        return null; 
    }

    let relativePath = "";
    const base = proj.baseDirectory;
    const absPath = originalDocumentAbsPath;

    if (absPath.startsWith(base)) {
        relativePath = absPath.substring(base.length);
        if (relativePath.startsWith(sep)) { 
            relativePath = relativePath.substring(sep.length);
        }
        if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
             relativePath = relativePath.substring(1);
        }
    } else {
        console.error(`[ProjectService loadDocumentMetadata] Cannot make path relative: absPath "${absPath}" does not start with base "${base}"`);
        return null; 
    }
    const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/');

    console.log(`[ProjectService] Loading metadata for original document rel path: ${originalDocumentRelativePathStr}`);
    try {
        const fullMetadataJsonString = await invoke('load_document_metadata', {
            projectXmlPathStr: proj.xmlPath,
            originalDocumentRelativePathStr: originalDocumentRelativePathStr
        });
        if (fullMetadataJsonString && typeof fullMetadataJsonString === 'string') {
            const parsedFullMetadata = JSON.parse(fullMetadataJsonString);
            if (parsedFullMetadata && typeof parsedFullMetadata.metadata === 'object' && Array.isArray(parsedFullMetadata.highlights)) {
                return parsedFullMetadata; 
            } else {
                console.warn("[ProjectService] Loaded metadata is not in expected full structure. Returning null.", parsedFullMetadata);
                return null;
            }
        }
        return null; 
    } catch (error) {
        console.error(`[ProjectService] Error loading document metadata for ${originalDocumentRelativePathStr}:`, error);
        return null; 
    }
}

export async function saveDocumentMetadata(originalDocumentAbsPath) {
    const proj = get(project);
    if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath ) {
        console.log("[ProjectService saveDocumentMetadata] Conditions not met for saving metadata (missing path, baseDir, or xmlPath).");
        return;
    }
    if (!proj.isDocumentMetadataDirty && originalDocumentAbsPath === proj.selectedDocumentPath) {
        console.log("[ProjectService saveDocumentMetadata] Metadata not dirty, skipping save for:", originalDocumentAbsPath);
        return;
    }

    let relativePath = "";
    const base = proj.baseDirectory;
    const absPath = originalDocumentAbsPath;
     const docFilename = await basename(absPath);


    if (absPath.startsWith(base)) {
        relativePath = absPath.substring(base.length);
         if (relativePath.startsWith(sep)) { 
            relativePath = relativePath.substring(sep.length);
        }
        if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
             relativePath = relativePath.substring(1);
        }
    } else {
        console.error(`[ProjectService saveDocumentMetadata] Cannot make path relative: absPath "${absPath}" does not start with base "${base}"`);
        await message(`Internal error: Could not determine relative path for metadata saving. Absolute: ${absPath}, Base: ${base}`, { title: 'Save Metadata Error', type: 'error' });
        throw new Error("Failed to construct relative path for metadata operation.");
    }
    const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/');

    const fullMetadataToSave = {
        metadata: { 
            file_name: docFilename, 
            last_modified: proj.currentDocumentFileLevelMetadata.last_modified || new Date().toISOString(), 
            title: proj.currentDocumentFileLevelMetadata.title || "",
            description: proj.currentDocumentFileLevelMetadata.description || "",
            summary: proj.currentDocumentFileLevelMetadata.summary || "",
        },
        highlights: proj.currentDocumentHighlights || [] 
    };

    const fullMetadataJsonContent = JSON.stringify(fullMetadataToSave, null, 2);

    console.log(`[ProjectService] Saving full metadata for original document rel path: ${originalDocumentRelativePathStr}`);
    try {
        await invoke('save_document_metadata', {
            projectXmlPathStr: proj.xmlPath,
            originalDocumentRelativePathStr: originalDocumentRelativePathStr,
            fullMetadataJsonContent: fullMetadataJsonContent 
        });
        markDocumentMetadataAsSaved(fullMetadataToSave.metadata); 
        console.log("[ProjectService] Document metadata (full structure) saved successfully.");
    } catch (error) {
        const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown error saving metadata.");
        console.error(`[ProjectService] Error saving document metadata for ${originalDocumentRelativePathStr}:`, errorMsg);
        await message(`Error saving document highlights: ${errorMsg}`, { title: 'Save Metadata Error', type: 'error' });
        throw new Error(errorMsg); 
    }
}

export async function checkUnsavedChangesThenProceed(newPathToLoad, providedActionContextDescription) {
    const projState = get(project);
    let itemIsDirty = false;
    let itemPath = null;
    let itemName = '';
    let itemTypeForPrompt = '';
    let saveFunction = null;
    let discardFunction = null;
    let resetEditorFunction = null;
    let initialContentForReset = null;

    const pathDescForLog = newPathToLoad ? await basename(newPathToLoad) : "NO_PATH_PROVIDED_TO_CHECK_UNSAVED";
    const typeDescForLog = providedActionContextDescription || "NO_CONTEXT_DESC_PROVIDED_TO_CHECK_UNSAVED";
    console.log(`[checkUnsavedChanges] Called with newPathToLoad: '${pathDescForLog}', actionContextDescription: '${typeDescForLog}'.`);

    if (projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') &&
        (projState.isPdfAnnotationsDirty )) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'PDF annotations'; 
        const projectXmlPath = projState.xmlPath;
        const projectBaseDir = projState.baseDirectory;

        saveFunction = async () => {
            if (!projectXmlPath || !projectBaseDir) {
                throw new Error("Project XML path or base directory is missing for saving PDF annotations.");
            }
            let relativePdfPath = itemPath;
            if (itemPath.startsWith(projectBaseDir + sep) || itemPath.startsWith(projectBaseDir + '/')) { 
                relativePdfPath = itemPath.substring(projectBaseDir.length + 1);
            } else if (itemPath.startsWith(projectBaseDir)) {
                 relativePdfPath = itemPath.substring(projectBaseDir.length);
                 if (relativePdfPath.startsWith(sep) || relativePdfPath.startsWith('/') || relativePdfPath.startsWith('\\')) {
                    relativePdfPath = relativePdfPath.substring(1);
                }
            }
            
            relativePdfPath = relativePdfPath.replace(/\\/g, '/'); 
            
            const annotationsToSave = get(project).currentPdfAnnotations;
            await invoke('save_pdf_annotations', {
                projectXmlPathStr: projectXmlPath,
                originalPdfRelativePathStr: relativePdfPath,
                annotationsJsonContent: JSON.stringify(annotationsToSave || [])
            });
            markPdfAnnotationsAsSaved(); 
        };
        discardFunction = () => {
            markDocumentChangesDiscarded(); 
        };
        resetEditorFunction = null; 
        initialContentForReset = projState.initialPdfAnnotations;
    }
    else if (projState.selectedDocumentPath && (projState.isDocumentDirty || projState.isDocumentMetadataDirty)) {
        itemIsDirty = true;
        itemPath = projState.selectedDocumentPath;
        itemTypeForPrompt = 'document';
        if (projState.activeDocumentEditorRef && typeof projState.activeDocumentEditorRef.save === 'function') {
            saveFunction = projState.activeDocumentEditorRef.save;
        } else {
             if (projState.isDocumentDirty) saveFunction = () => saveDocumentContent(itemPath, projState.currentDocumentJson);
             else if (projState.isDocumentMetadataDirty) saveFunction = () => saveDocumentMetadata(itemPath);
        }
        discardFunction = () => {
            markDocumentChangesDiscarded();
        };
        resetEditorFunction = projState.activeDocumentEditorRef?.resetEditorState;
        initialContentForReset = projState.initialDocumentJson;
    }
    else if (projState.currentImportedTranscriptPath && projState.isImportedTranscriptDirty) {
        itemIsDirty = true;
        itemPath = projState.currentImportedTranscriptPath;
        itemTypeForPrompt = 'imported transcript';
        if (projState.activeImportedTranscriptEditorRef && typeof projState.activeImportedTranscriptEditorRef.save === 'function') {
            saveFunction = projState.activeImportedTranscriptEditorRef.save;
            discardFunction = () => markImportedTranscriptChangesDiscarded(itemPath);
            resetEditorFunction = projState.activeImportedTranscriptEditorRef.resetEditorState;
            initialContentForReset = projState.initialImportedTranscriptLexicalJson;
        }
    }


    if (itemIsDirty && itemPath === newPathToLoad) {
        console.log(`[checkUnsavedChanges] Attempting to load/act on the same item that is dirty ('${itemPath}'). Allowing without prompt.`);
        return true;
    }

    if (!itemIsDirty) {
        const actionContextForLog = newPathToLoad
            ? `loading item '${await basename(newPathToLoad)}'`
            : `performing action '${providedActionContextDescription || "unknown action"}'`;
        console.log(`[checkUnsavedChanges] No unsaved changes for active fieldnote items. Proceeding with ${actionContextForLog}.`);
        return true;
    }

    itemName = await basename(itemPath);
    const actionContextDisplay = newPathToLoad
        ? `load '${await basename(newPathToLoad)}'`
        : (providedActionContextDescription || "perform this action");

    console.log(`[checkUnsavedChanges] Unsaved changes detected for "${itemName}" (${itemTypeForPrompt}) while attempting to ${actionContextDisplay}. Autosave is ${projState.autosaveEnabled ? 'ON' : 'OFF'}.`);

    if (projState.autosaveEnabled) {
        console.log(`[checkUnsavedChanges] Autosave ON. Attempting implicit save for "${itemName}"...`);
        if (saveFunction) {
            try {
                await saveFunction();
                console.log(`[checkUnsavedChanges] Implicit save successful for "${itemName}". Proceeding.`);
                return true;
            } catch (error) {
                console.error(`[checkUnsavedChanges] Implicit save failed for "${itemName}":`, error);
                const proceedAfterFail = await confirm(
                    `Failed to automatically save changes for "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
                    { title: 'Autosave Failed', type: 'error', okLabel: 'Discard and Continue', cancelLabel: 'Cancel Action' }
                );
                if (proceedAfterFail) {
                    console.log(`[checkUnsavedChanges] User chose to discard after failed autosave.`);
                    if (discardFunction) discardFunction();
                    if(resetEditorFunction && initialContentForReset !== null && typeof resetEditorFunction === 'function') resetEditorFunction(initialContentForReset);
                    return true;
                } else {
                    console.log(`[checkUnsavedChanges] User chose to cancel action after failed autosave.`);
                    return false;
                }
            }
        } else {
            console.warn(`[checkUnsavedChanges] Autosave ON, but editor ref or save method missing for "${itemName}". Blocking action.`);
            await message(`Cannot ${actionContextDisplay}: Failed to find the editor to perform automatic save. Please save manually first.`, { title: 'Internal Error', type: 'error'});
            return false;
        }
    } else {
        console.log(`[checkUnsavedChanges] Autosave OFF. Triggering unsaved changes modal for "${itemName}"...`);
        return new Promise((resolve) => {
            showUnsavedChangesPrompt(itemName, itemTypeForPrompt,
                async () => {
                    console.log("[UnsavedChangesModal callback] User chose Save.");
                    hideUnsavedChangesPrompt();
                    if (saveFunction) {
                        try { await saveFunction(); console.log("[UnsavedChangesModal callback] Save successful."); resolve(true); }
                        catch (error) { console.error("[UnsavedChangesModal callback] Save failed:", error); resolve(false); }
                    } else { console.error("[UnsavedChangesModal callback] Save chosen, but save function missing."); await message('Cannot save: Editor reference or save method is missing.', { title: 'Internal Error', type: 'error' }); resolve(false); }
                },
                () => {
                    console.log("[UnsavedChangesModal callback] User chose Don't Save (Discard).");
                    hideUnsavedChangesPrompt();
                    if (discardFunction) discardFunction();
                    if (resetEditorFunction && initialContentForReset !== null && typeof resetEditorFunction === 'function') resetEditorFunction(initialContentForReset);
                    resolve(true);
                },
                () => {
                    console.log("[UnsavedChangesModal callback] User chose Cancel.");
                    hideUnsavedChangesPrompt();
                    resolve(false);
                }
            );
        });
    }
}

export async function loadPdfAnnotationsFromFile(pdfAbsPath) {
    if (!pdfAbsPath) {
        console.warn("[ProjectService loadPdfAnnotationsFromFile] pdfAbsPath is missing.");
        setLoadedPdfAnnotations([]); 
        project.update(p => {
            if(p.selectedDocumentPath === pdfAbsPath && p.isDocumentLoading) return {...p, isDocumentLoading: false };
            return p;
        });
        return;
    }
    const filename = await basename(pdfAbsPath);
    console.log(`[ProjectService] Loading PDF annotations for: ${filename} (Path: ${pdfAbsPath})`);

    try {
        const annotationsJsonString = await invoke('load_pdf_annotations', {
            originalPdfAbsPathStr: pdfAbsPath
        });

        console.log(`[ProjectService] Received from backend 'load_pdf_annotations' for ${filename}:`, annotationsJsonString);

        if (annotationsJsonString && typeof annotationsJsonString === 'string') {
            try {
                const parsedAnnotations = JSON.parse(annotationsJsonString);
                console.log(`[ProjectService] Parsed annotations for ${filename}:`, parsedAnnotations);
                setLoadedPdfAnnotations(parsedAnnotations || []); 
            } catch (parseError) {
                console.error(`[ProjectService] Failed to parse annotations JSON string for ${filename}:`, parseError, "\nString was:", annotationsJsonString);
                setPdfAnnotationsLoadFailed(pdfAbsPath, `Failed to parse loaded annotations: ${parseError.message}`);
            }
        } else if (annotationsJsonString === null) {
            console.log(`[ProjectService] No annotation file found or it was explicitly null for ${filename}. Setting empty.`);
            setLoadedPdfAnnotations([]);
        } else {
            console.log(`[ProjectService] Annotation file likely empty or backend returned unexpected non-string for ${filename}. Setting empty.`);
            setLoadedPdfAnnotations([]);
        }
    } catch (e) {
        const errorMessage = e.message || String(e);
        console.error(`[ProjectService] Error invoking 'load_pdf_annotations' for ${filename}:`, errorMessage);
        setPdfAnnotationsLoadFailed(pdfAbsPath, `Service call failed: ${errorMessage}`);
    }
}