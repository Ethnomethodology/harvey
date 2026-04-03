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
  ParagraphNode,
  RootNode,
  TextNode,
  LineBreakNode,
  ElementNode
} from 'lexical';
import {
  $createTableNode as _createTableNode,
  $createTableRowNode as _createTableRowNode,
  $createTableCellNode as _createTableCellNode,
  $isTableNode as _isTableNode,
  $isTableRowNode as _isTableRowNode,
  $isTableCellNode as _isTableCellNode,
  TableNode,
  TableRowNode,
  TableCellNode
} from '@lexical/table';
import {
  $createHeadingNode as _createHeadingNode,
  HeadingNode,
  QuoteNode,
  $isHeadingNode as _isHeadingNode
} from '@lexical/rich-text';
import {
  $isListNode as _isListNode,
  ListNode,
  ListItemNode,
  $isListItemNode as _isListItemNode
} from '@lexical/list';
import { createHeadlessEditor } from '@lexical/headless';
import {
  $generateHtmlFromNodes as _generateHtmlFromNodes,
  $generateNodesFromDOM as _generateNodesFromDOM
} from '@lexical/html';

import { LinkNode, $isLinkNode as _isLinkNode } from '@lexical/link';
import { SHARED_NODES } from '$lib/nodes/LexicalConfig.js';

import { dirname, basename, sep, join } from '@tauri-apps/api/path';

import { activeLayout } from '$lib/stores/layoutStore.js';
import { DOCX_LAYOUT_COLUMN_CONFIGS } from '$lib/constants/exportLayouts.js';

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
  setDocumentHighlights,
  prepareStandaloneTranscriptView,
  markStandaloneTranscriptChangesDiscarded,
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
  saveManualSettingsForTranscript // Added import
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
 * Inlines CSS rules from <style> tags into elements' style attributes.
 * This is necessary because Lexical's _generateNodesFromDOM only respects inline styles,
 * while Pandoc's --standalone output uses CSS classes for colors and other formatting.
 * @param {Document} dom - The DOM document parsed from HTML.
 */
function inlineCssRules(dom) {
  const styleMap = {};
  const styleTags = dom.querySelectorAll('style');
  styleTags.forEach((style) => {
    const css = style.textContent;
    // Simple regex to match .class { properties }
    // Pandoc usually uses .c1 { color: #123456; }
    const regex = /\.([a-zA-Z0-9_\-]+)\s*\{\s*([^}]+)\}/g;
    let match;
    while ((match = regex.exec(css)) !== null) {
      const className = match[1];
      const rules = match[2].trim();
      styleMap[className] = rules;
    }
  });

  if (Object.keys(styleMap).length === 0) return;

  const elementsWithClass = dom.querySelectorAll('[class]');
  elementsWithClass.forEach((el) => {
    const classes = el.getAttribute('class').split(/\s+/);
    let inlinedStyles = '';
    classes.forEach((cls) => {
      if (styleMap[cls]) {
        inlinedStyles += (inlinedStyles ? ';' : '') + styleMap[cls];
      }
    });
    if (inlinedStyles) {
      const existingStyle = el.getAttribute('style') || '';
      // Append inlined styles, ensuring we don't double-semicolon
      const separator = existingStyle && !existingStyle.trim().endsWith(';') ? ';' : '';
      el.setAttribute('style', existingStyle + separator + inlinedStyles);
    }
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
    console.error(
      `[ProjectService] Error saving table layout preferences for ${tablePath}:`,
      error
    );
    throw error;
  }
}

export async function loadHighlightsForFile(filePath, itemType) {
  if (!filePath || !itemType) {
    console.warn(
      '[ProjectService] loadHighlightsForFile called with missing filePath or itemType.'
    );
    return;
  }

  const lowerPath = filePath.toLowerCase();

  // Determine the correct loading function based on itemType
  if (itemType === 'doc' && lowerPath.endsWith('.pdf')) {
    await loadPdfAnnotationsFromFile(filePath);
  } else if (itemType === 'images') {
    await loadImageAnnotations(filePath);
  } else if (
    itemType === 'tables' ||
    itemType === 'table' ||
    lowerPath.endsWith('.csv') ||
    lowerPath.endsWith('.xlsx')
  ) {
    await loadTableHighlights(filePath);
  } else if (itemType === 'standalone_transcript') {
    // Assuming there's a function to load highlights for imported transcripts
    // If not, this part needs to be implemented. For now, let's log it.
    console.log(
      `[ProjectService] Highlight loading for 'standalone_transcript' is not yet implemented.`
    );
  } else {
    // 'doc' (non-PDF), etc.
    const metadata = await loadDocumentMetadata(filePath);
    if (metadata && metadata.highlights) {
      const { setDocumentHighlights } = await import('$lib/stores/projectStore.js');
      setDocumentHighlights(metadata.highlights);
    } else {
      console.log(`[ProjectService] No highlights found for document type '${itemType}'.`);
    }
  }
}

export async function saveTableSchema(tablePath, schema) {
  if (!tablePath || !schema) return;
  const normalizedPath = normalizePath(tablePath);
  const { project } = await import('$lib/stores/projectStore.js');
  const projectId = get(project).id;
  try {
    await invoke('save_table_schema', { projectId, tablePath: normalizedPath, schema });
  } catch (error) {
    console.error(`[ProjectService] Error saving table schema for ${tablePath}:`, error);
    throw error;
  }
}

export async function loadTableSchema(tablePath) {
  if (!tablePath) return null;
  const normalizedPath = normalizePath(tablePath);
  const { project } = await import('$lib/stores/projectStore.js');
  const projectId = get(project).id;
  try {
    const schema = await invoke('load_table_schema', { projectId, tablePath: normalizedPath });
    return schema || {};
  } catch (error) {
    console.error(`[ProjectService] Error loading table schema for ${tablePath}:`, error);
    return null;
  }
}

export async function deleteTableColumn(tablePath, columnName) {
  if (!tablePath || !columnName) {
    throw new Error('Missing required parameters for deleting table column.');
  }

  try {
    await invoke('delete_table_column', {
      tablePathStr: tablePath,
      columnNameToDelete: columnName
    });
  } catch (error) {
    const errorMessage = error.message || String(error);
    await message(`Error deleting column: ${errorMessage}`, {
      title: 'Delete Column Error',
      type: 'error'
    });
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
    await invoke('save_table_styles', { filePath: tablePath, styles: styles });
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

import {
  setLoadedTableHighlights,
  setTableHighlightsLoadFailed,
  markTableHighlightsAsSaved
} from '$lib/stores/projectStore.js';

export async function loadTableHighlights(filePath) {
  if (!filePath) {
    setLoadedTableHighlights([]);
    return;
  }
  try {
    const highlights = await loadTableStyles(filePath);
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
      styles: highlights
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
    await message('Project data is not fully loaded. Cannot create documents.', {
      title: 'Create Error',
      type: 'error'
    });
    return;
  }

  try {
    const newDocument = await invoke('create_new_document', {
      projectXmlPath: projectXmlPath,
      documentName: 'Untitled.json'
    });

    await refreshProjectFiles();

    prepareDocumentView(newDocument, 'documents');
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    await message(`Error creating document: ${errorMessage}`, {
      title: 'Create Error',
      type: 'error'
    });
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
    console.error(
      `[ProjectService] Error loading table layout preferences for ${tablePath}:`,
      error
    );
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
const AUDIOS_DIR_NAME = 'Audios';
const VIDEOS_DIR_NAME = 'Videos';
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

const ALL_EDITOR_NODES = SHARED_NODES;

function createConversionEditor(instanceId) {
  return createHeadlessEditor({
    nodes: ALL_EDITOR_NODES,
    namespace: `html-converter-${instanceId}-${Math.random()}`,
    onError: (e) => console.error(`[Lexical HTML Converter ${instanceId}] Error:`, e)
  });
}

export async function loadProjectDataAndUpdateStore(
  projectXmlPath,
  targetPathToSelect = null,
  targetTranscriptPathToSelect = null
) {
  if (!projectXmlPath || projectXmlPath.trim() === '') {
    console.error(
      '[ProjectService] loadProjectDataAndUpdateStore called without a valid projectXmlPath'
    );
    project.update((current) => ({
      ...current,
      isLoading: false,
      error: 'Project path is missing.',
      statusMessage: 'Error: Project path is missing.'
    }));
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
            node.associated_transcripts = Array.isArray(node.associated_transcripts)
              ? node.associated_transcripts
              : [];
            node.associated_transcripts = node.associated_transcripts.map((t) => {
              let absolutePath = null;
              let name = t.name; // Preserve existing name if available
              if (
                normalizedBaseDirectory &&
                typeof normalizedBaseDirectory === 'string' &&
                t.relativePath &&
                typeof t.relativePath === 'string'
              ) {
                // Ensure no double slashes if base_directory ends with one and relativePath starts with one (though unlikely for relativePath)
                const base =
                  normalizedBaseDirectory.endsWith('/') || normalizedBaseDirectory.endsWith('\\')
                    ? normalizedBaseDirectory.slice(0, -1)
                    : normalizedBaseDirectory;
                const rel =
                  t.relativePath.startsWith('/') || t.relativePath.startsWith('\\')
                    ? t.relativePath.substring(1)
                    : t.relativePath;
                absolutePath = normalizePath(`${base}/${rel}`);
                if (!name) {
                  // If name is not provided by backend, derive from relativePath
                  name = t.relativePath.split(/[\\/]/).pop();
                }
              } else {
                // If base_directory or relativePath is missing, we can't form a full path.
                // Log this, as it indicates an issue with the data from the backend or project structure.
                console.warn(
                  `[ProjectService] Cannot construct absolute path for transcript. Base dir: ${normalizedBaseDirectory}, Relative path: ${t.relativePath}`
                );
                if (!name) {
                  // If name is not provided and path construction failed, use relativePath as fallback
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
      standaloneTranscriptFiles: loadedData.standalone_transcript_files || [],
      documentMetadataFiles: loadedData.document_metadata_files || [],
      isLoading: false,
      error: null,
      statusMessage: `Loaded project: ${loadedData.project_name}`
    };
    project.update((current) => ({
      ...current,
      ...dataToSet
    }));

    // Load autosave preference for this project (defaults to true)
    if (loadedData.project_uuid) {
    }

    // Update project groups list
    try {
      const { updateProjectGroupsList } = await import('$lib/stores/projectStore.js');
      if (loadedData.project_uuid) {
        // Ensure project_uuid (as id) is available
        await updateProjectGroupsList(loadedData.project_uuid);
      } else {
        console.warn(
          '[ProjectService] Project UUID not available after loading, cannot update groups list.'
        );
      }
    } catch (e) {
      console.error('[ProjectService] Error importing or calling updateProjectGroupsList:', e);
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
        if (node.file_type === 'media' && !node.is_directory) {
          return node;
        }
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
        console.warn(
          `[ProjectService] Target media path ${targetPathToSelect} provided but not found. Falling back to first media.`
        );
        mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
      }
    } else {
      mediaFileToSelect = findFirstMediaRecursive(loadedData.files || []);
    }

    if (mediaFileToSelect) {
      selectMedia(mediaFileToSelect, targetTranscriptPathToSelect);
    }
  } catch (error) {
    console.error('[ProjectService] Failed to load project data:', error);
    project.update((current) => ({
      ...current,
      isLoading: false,
      error: error?.message || 'Unknown error loading project.',
      statusMessage: `Error loading project.`
    }));
    throw error;
  }
}

export async function silentlyRefreshProjectData(projectXmlPath) {
  if (!projectXmlPath || projectXmlPath.trim() === '') {
    console.error(
      '[ProjectService] silentlyRefreshProjectData called without a valid projectXmlPath'
    );
    project.update((current) => ({
      ...current,
      isLoading: false,
      error: 'Project path is missing for silent refresh.',
      statusMessage: 'Error: Project path missing.'
    }));
    return;
  }
  project.update((current) => ({
    ...current,
    isLoading: true,
    error: null,
    statusMessage: 'Refreshing project data silently...'
  }));
  try {
    const loadedData = await invoke('load_project_data', { projectXmlPath });

    if (Array.isArray(loadedData.files)) {
      const attachTranscripts = (nodes) => {
        for (const node of nodes) {
          if (node.file_type === 'media') {
            node.associated_transcripts = Array.isArray(node.associated_transcripts)
              ? node.associated_transcripts
              : [];
            node.associated_transcripts = node.associated_transcripts.map((t) => {
              let absolutePath = null;
              let name = t.name; // Preserve existing name if available
              if (
                loadedData.base_directory &&
                typeof loadedData.base_directory === 'string' &&
                t.relativePath &&
                typeof t.relativePath === 'string'
              ) {
                // Ensure no double slashes if base_directory ends with one and relativePath starts with one (though unlikely for relativePath)
                const base =
                  loadedData.base_directory.endsWith('/') ||
                  loadedData.base_directory.endsWith('\\')
                    ? loadedData.base_directory.slice(0, -1)
                    : loadedData.base_directory;
                const rel =
                  t.relativePath.startsWith('/') || t.relativePath.startsWith('\\')
                    ? t.relativePath.substring(1)
                    : t.relativePath;
                absolutePath = normalizePath(`${base}/${rel}`);
                if (!name) {
                  // If name is not provided by backend, derive from relativePath
                  name = t.relativePath.split(/[\\/]/).pop();
                }
              } else {
                // If base_directory or relativePath is missing, we can't form a full path.
                // Log this, as it indicates an issue with the data from the backend or project structure.
                console.warn(
                  `[ProjectService] Cannot construct absolute path for transcript. Base dir: ${loadedData.base_directory}, Relative path: ${t.relativePath}`
                );
                if (!name) {
                  // If name is not provided and path construction failed, use relativePath as fallback
                  name = t.relativePath;
                }
              }
              return {
                path: absolutePath ? normalizePath(absolutePath) : null,
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
      foundMediaFileObjectFromNewList = findMediaByPathRecursive(
        loadedData.files || [],
        preRefreshSelectedPath
      );
    }

    if (foundMediaFileObjectFromNewList) {
      transcriptStore.update((ts) => {
        if (
          get(transcriptStore).selectedMediaFile?.path === preRefreshSelectedPath ||
          !get(transcriptStore).selectedMediaFile
        ) {
          return { ...ts, selectedMediaFile: foundMediaFileObjectFromNewList };
        }
        return ts;
      });
    } else if (preRefreshSelectedPath) {
      transcriptStore.update((ts) => {
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
      standaloneTranscriptFiles: loadedData.standalone_transcript_files || [],
      documentMetadataFiles: loadedData.document_metadata_files || [],
      isLoading: false,
      error: null,
      statusMessage: 'File list updated.'
    };
    project.update((current) => ({
      ...current,
      ...dataToSet
    }));
  } catch (error) {
    console.error('[ProjectService] Failed to silently refresh project data:', error);
    project.update((current) => ({
      ...current,
      isLoading: false,
      error: error?.message || 'Unknown error refreshing project data.',
      statusMessage: 'Error refreshing project data.'
    }));
    throw error;
  }
}

export async function importMediaFile(importType = null) {
  const currentProject = get(project);
  const projectXmlPath = currentProject.xmlPath;
  if (!projectXmlPath) {
    console.error('[ProjectService] Cannot import media: Project XML path missing.');
    await message('Project data is not fully loaded. Cannot import media.', {
      title: 'Import Error',
      type: 'error'
    });
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
      project.update((p) => ({ ...p, statusMessage: 'Media import cancelled.' }));
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

    const newlyImportedFileEntry = await invoke('import_media', {
      sourceFilePathStr: sourceFilePath,
      projectXmlPathStr: projectXmlPath,
      importType: importType // Added parameter
    });

    if (
      !newlyImportedFileEntry ||
      typeof newlyImportedFileEntry !== 'object' ||
      !newlyImportedFileEntry.path
    ) {
      console.error(
        '[ProjectService] import_media returned invalid FileEntry:',
        newlyImportedFileEntry
      );
      setAssetImportStatus(false, `Error importing ${filename}: Invalid data from backend.`);
      await message(`Error importing ${filename}: Backend returned invalid data.`, {
        title: 'Import Error',
        type: 'error'
      });
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
    console.log(
      `[ProjectService] Media imported. The new file has been selected. Path: ${newlyImportedFileEntry.path}`
    );

    setAssetImportStatus(false, `${filename} imported successfully.`);
    return newlyImportedFileEntry.path;
  } catch (error) {
    console.error('[ProjectService] Failed to import media file:', error);
    const errorMessage = getErrorMessage(error);
    await message(`Error importing media: ${errorMessage}`, {
      title: 'Import Error',
      type: 'error'
    });
    setAssetImportStatus(false, `Error importing media: ${errorMessage}`);
    // Ensure loading states are reset on error
    project.update((p) => ({
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
    await message('Project data is not fully loaded. Cannot import documents.', {
      title: 'Import Error',
      type: 'error'
    });
    return;
  }

  const canProceedDialog = await checkUnsavedChangesThenProceed(null, 'importing a document');
  if (!canProceedDialog) {
    setAssetImportStatus(false, 'Document import cancelled by user.');
    return;
  }

  let sourceFilePath = '';
  let backendResultPathAndOriginalFilename = '';
  let finalJsonPath = '';
  let finalJsonName = '';

  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [documentFilter],
      title: 'Import Document File'
    });
    if (!selected || typeof selected !== 'string') {
      project.update((p) => ({ ...p, statusMessage: 'Document import cancelled.' }));
      return;
    }
    sourceFilePath = selected;
    const sourceFilename = await basename(sourceFilePath);
    const sourceFilenameStem = sourceFilename.includes('.')
      ? sourceFilename.substring(0, sourceFilename.lastIndexOf('.'))
      : sourceFilename;
    const sourceExtension = (
      sourceFilename.includes('.')
        ? sourceFilename.substring(sourceFilename.lastIndexOf('.') + 1)
        : ''
    ).toLowerCase();

    const needsConversionPrompt = ['docx', 'rtf'].includes(sourceExtension);
    if (needsConversionPrompt) {
      const conversionConfirmed = await new Promise((resolve) => {
        showConversionPrompt(
          sourceFilename,
          () => {
            hideConversionPrompt();
            resolve(true);
          },
          () => {
            hideConversionPrompt();
            resolve(false);
          }
        );
      });
      if (!conversionConfirmed) {
        project.update((p) => ({ ...p, statusMessage: 'Document import cancelled.' }));
        return;
      }
    }

    setAssetImportStatus(true, `Importing ${sourceFilename}...`);

    backendResultPathAndOriginalFilename = await invoke('import_document', {
      sourcePathStr: sourceFilePath,
      projectXmlPathStr: projectXmlPath
    });
    let tempHtmlPath = backendResultPathAndOriginalFilename;
    let uniqueDocFilenameWithExt = sourceFilename;

    if (backendResultPathAndOriginalFilename.includes('|original_filename:')) {
      const parts = backendResultPathAndOriginalFilename.split('|original_filename:');
      tempHtmlPath = parts[0];
      uniqueDocFilenameWithExt = parts[1];
    }

    const uniqueDocStem = uniqueDocFilenameWithExt.includes('.')
      ? uniqueDocFilenameWithExt.substring(0, uniqueDocFilenameWithExt.lastIndexOf('.'))
      : uniqueDocFilenameWithExt;

    console.log(
      `[importDocumentFile] Backend returned tempHtmlPath: ${tempHtmlPath}, uniqueDocFilenameWithExt: ${uniqueDocFilenameWithExt}, uniqueDocStem: ${uniqueDocStem}`
    );

    if (tempHtmlPath && tempHtmlPath.toLowerCase().endsWith('.pdf')) {
      await refreshProjectFiles();
      const importedPdfName = await basename(tempHtmlPath);
      setAssetImportStatus(false, `Document "${importedPdfName}" imported successfully.`);
      prepareDocumentView(tempHtmlPath, 'documents');
      return;
    }
    if (!tempHtmlPath || !tempHtmlPath.toLowerCase().endsWith('.html'))
      throw new Error('Backend did not return expected temporary HTML path.');

    const htmlContent = await invoke('read_file_content', { path: tempHtmlPath });
    try {
      await invoke('delete_temporary_file', { path: tempHtmlPath });
    } catch (delErr) {
      console.warn(`[ProjectService] Failed to delete temp HTML: ${tempHtmlPath}`);
    }

    let lexicalJsonString = '';
    const conversionEditor = createConversionEditor('import-doc');
    try {
      const domParser = new DOMParser();
      const dom = domParser.parseFromString(htmlContent, 'text/html');

      // Inline CSS styles so Lexical can see colors and other formatting
      inlineCssRules(dom);

      await conversionEditor.update(() => {
        const nodes = _generateNodesFromDOM(conversionEditor, dom);
        _getRoot().clear();
        _getRoot().append(...nodes);
      });
      const editorState = conversionEditor.getEditorState();
      if (editorState.isEmpty()) {
        conversionEditor.update(() => {
          _getRoot().clear();
          const para = _createParagraphNode();
          para.append(
            _createTextNode(`[Content from ${sourceFilename} could not be fully parsed] `)
          );
          _getRoot().append(para);
        });
      }
      lexicalJsonString = JSON.stringify(conversionEditor.getEditorState().toJSON(), null, 2);
    } catch (lexicalError) {
      const errorEditor = createConversionEditor('import-error');
      errorEditor.update(() => {
        _getRoot().clear();
        const p = _createParagraphNode();
        p.append(
          _createTextNode(
            `Error importing content from ${sourceFilename}: ${lexicalError.message || lexicalError}`
          )
        );
        _getRoot().append(p);
      });
      lexicalJsonString = JSON.stringify(errorEditor.getEditorState().toJSON(), null, 2);
    }
    if (!lexicalJsonString) throw new Error('Failed to generate Lexical JSON from HTML.');

    const docsFolderPath = `${projectBaseDir}/${HARVEY_FILES_DIR}/${DOCS_DIR_NAME}/${uniqueDocStem}`;
    finalJsonPath = `${docsFolderPath}/${uniqueDocStem}.json`;
    finalJsonName = await basename(finalJsonPath);
    await invoke('save_document_and_update_xml', {
      projectXmlPath: projectXmlPath,
      targetPath: finalJsonPath,
      documentName: finalJsonName,
      jsonContent: lexicalJsonString
    });
    await refreshProjectFiles();
    setAssetImportStatus(false, `Document "${uniqueDocFilenameWithExt}" imported successfully.`);
    prepareDocumentView(finalJsonPath, 'documents');
    return finalJsonPath;
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    await message(`Error importing document: ${errorMessage}`, {
      title: 'Import Error',
      type: 'error'
    });
    setAssetImportStatus(false, `Error importing: ${errorMessage}`);
    if (
      backendResultPathAndOriginalFilename &&
      !backendResultPathAndOriginalFilename.toLowerCase().endsWith('.pdf') &&
      backendResultPathAndOriginalFilename.includes('.html')
    ) {
      let pathToClean = backendResultPathAndOriginalFilename.split('|original_filename:')[0];
      try {
        await invoke('delete_temporary_file', { path: pathToClean });
      } catch (delErr) {}
    }
  }
}

export async function importImageFile() {
  const currentProject = get(project);
  const projectXmlPath = currentProject.xmlPath;
  if (!projectXmlPath) {
    console.error('[ProjectService] Cannot import image: Project data not fully loaded.');
    await message('Project data is not fully loaded. Cannot import images.', {
      title: 'Import Error',
      type: 'error'
    });
    return;
  }
  const canProceedDialog = await checkUnsavedChangesThenProceed(null, 'importing an image');
  if (!canProceedDialog) {
    setAssetImportStatus(false, 'Image import cancelled by user.');
    return;
  }
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [imageFilter],
      title: 'Import Image File'
    });
    if (!selected || typeof selected !== 'string') {
      project.update((p) => ({ ...p, statusMessage: 'Image import cancelled.' }));
      return;
    }
    const sourceFilePath = selected;
    const sourceFilename = await basename(sourceFilePath);
    setAssetImportStatus(true, `Importing image ${sourceFilename}...`);
    const finalImagePath = await invoke('import_image_file', {
      sourcePathStr: sourceFilePath,
      projectXmlPathStr: projectXmlPath
    });
    await refreshProjectFiles();
    const importedImageName = await basename(finalImagePath);
    setAssetImportStatus(false, `Image "${importedImageName}" imported successfully.`);
    prepareDocumentView(finalImagePath, 'images');
    return finalImagePath;
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    await message(`Error importing image: ${errorMessage}`, {
      title: 'Import Error',
      type: 'error'
    });
    setAssetImportStatus(false, `Error during image import: ${errorMessage}`);
  }
}

export async function importTranscriptFile(sourceType = 'msWord') {
  const currentProject = get(project);
  const projectXmlPath = currentProject.xmlPath;
  if (!projectXmlPath) {
    console.error('[ProjectService] Cannot import transcript: Project data not fully loaded.');
    await message('Project data is not fully loaded. Cannot import transcripts.', {
      title: 'Import Error',
      type: 'error'
    });
    return;
  }
  const canProceedDialog = await checkUnsavedChangesThenProceed(
    null,
    `importing a ${sourceType} transcript`
  );
  if (!canProceedDialog) {
    setAssetImportStatus(false, 'Transcript import cancelled by user.');
    return;
  }
  try {
    if (sourceType === 'msWord') {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [wordDocumentFilter],
        title: 'Import MS Word Transcript (.docx)'
      });
      if (!selected || typeof selected !== 'string') {
        project.update((p) => ({ ...p, statusMessage: 'Transcript import cancelled.' }));
        return;
      }
      const sourceDocxPath = selected;
      const sourceFilename = await basename(sourceDocxPath);
      setAssetImportStatus(true, `Importing transcript from ${sourceFilename}...`);
      const newTranscriptJsonPath = await invoke('import_word_transcript', {
        sourceDocxPathStr: sourceDocxPath,
        projectXmlPathStr: projectXmlPath
      });
      await refreshProjectFiles();
      const standaloneTranscriptName = await basename(newTranscriptJsonPath);
      setAssetImportStatus(
        false,
        `Transcript "${standaloneTranscriptName}" imported successfully.`
      );
      prepareStandaloneTranscriptView(newTranscriptJsonPath);
      return newTranscriptJsonPath;
    } else {
      throw new Error(`Unsupported transcript source type: ${sourceType}`);
    }
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    await message(`Error importing transcript: ${errorMessage}`, {
      title: 'Import Error',
      type: 'error'
    });
    setAssetImportStatus(false, `Error during transcript import: ${errorMessage}`);
  }
}

export async function deleteStandaloneTranscript(transcriptAbsolutePath) {
  return deleteProjectItem(transcriptAbsolutePath);
}

export async function importTableSheet(sourceFilePath, projectXmlPath, sheetName, filename) {
  // Only invoke the backend and return the promise so the UI orchestrator
  // has control over the loading state, preventing loading spinner glitches.
  const result = await invoke('import_table_file', {
    sourcePathStr: sourceFilePath,
    projectXmlPathStr: projectXmlPath,
    sheetNameOpt: sheetName,
    appendSheetName: true
  });
  if (result && result.table_path && result.preview_data) {
    return { ...result, filename: `${filename} (${sheetName})` };
  } else {
    throw new Error('Invalid response from backend during table sheet import.');
  }
}

export async function importTableFile(hasHeaders) {
  const currentProject = get(project);
  const projectXmlPath = currentProject.xmlPath;
  console.log(`[ProjectService] importTableFile: projectXmlPath = ${projectXmlPath}`);

  if (!projectXmlPath) {
    console.error('[ProjectService] Cannot import table: Project data not fully loaded.');
    await message('Project data is not fully loaded. Cannot import tables.', {
      title: 'Import Error',
      type: 'error'
    });
    return null;
  }

  const canProceedDialog = await checkUnsavedChangesThenProceed(null, 'importing a table');
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
      project.update((p) => ({ ...p, statusMessage: 'Table import cancelled.' }));
      return null;
    }

    const sourceFilePath = selected;
    console.log(`[ProjectService] importTableFile: sourceFilePath = ${sourceFilePath}`);
    const sourceFilename = await basename(sourceFilePath);
    setAssetImportStatus(true, `Inspecting table ${sourceFilename}...`);

    let selectedSheets = null;
    if (sourceFilePath.toLowerCase().endsWith('.xlsx')) {
      console.log(`[ProjectService] Invoking 'get_xlsx_sheets' for ${sourceFilePath}`);
      const sheets = await invoke('get_xlsx_sheets', { sourcePathStr: sourceFilePath });
      if (sheets && sheets.length > 1) {
        // If there are multiple sheets, we stop here and return them to the UI
        // so the UI can prompt the user to select which ones to import.
        setAssetImportStatus(false, `Select sheets to import from ${sourceFilename}`);
        return { sheets, sourceFilePath, filename: sourceFilename, projectXmlPath };
      } else if (sheets && sheets.length === 1) {
        // Single sheet, proceed directly
        selectedSheets = [sheets[0]];
      }
    }

    // If it's a CSV or an XLSX with a single sheet, we can import it right away
    setAssetImportStatus(true, `Importing table ${sourceFilename}...`);

    console.log(
      `[ProjectService] Invoking 'import_table_file' with sourcePathStr: ${sourceFilePath}, projectXmlPathStr: ${projectXmlPath}`
    );
    const result = await invoke('import_table_file', {
      sourcePathStr: sourceFilePath,
      projectXmlPathStr: projectXmlPath,
      sheetNameOpt: selectedSheets ? selectedSheets[0] : null,
      appendSheetName: false // For single-sheet imports directly from here, don't append the sheet name
    });
    console.log(`[ProjectService] Result from 'import_table_file':`, result);

    if (result && result.table_path && result.preview_data) {
      setAssetImportStatus(false, `${sourceFilename} imported successfully.`);
      return [{ ...result, filename: sourceFilename }];
    } else {
      throw new Error('Invalid response from backend during table import.');
    }
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    await message(`Error importing table: ${errorMessage}`, {
      title: 'Import Error',
      type: 'error'
    });
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
    if (
      typeof tableData !== 'object' ||
      tableData === null ||
      !Array.isArray(tableData.headers) ||
      !Array.isArray(tableData.data)
    ) {
      throw new Error('Backend returned invalid data format for table.');
    }

    // Sanitize data: remove carriage returns from all cell values
    const sanitizedData = tableData.data.map((row) => {
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
    await message(`Error loading table data: ${errorMessage}`, {
      title: 'Load Table Error',
      type: 'error'
    });
    throw error;
  }
}
function parseTimestampStringToSeconds(timestampStr) {
  if (!timestampStr || typeof timestampStr !== 'string') return 0;
  const cleanedStr = timestampStr.trim();
  const parts = cleanedStr.split(':');
  let seconds = 0;
  try {
    if (parts.length === 3) {
      seconds = parseInt(parts[0], 10) * 3600 + parseInt(parts[1], 10) * 60 + parseFloat(parts[2]);
    } else if (parts.length === 2) {
      seconds = parseInt(parts[0], 10) * 60 + parseFloat(parts[1]);
    } else if (parts.length === 1) {
      seconds = parseFloat(parts[0]);
    } else {
      return 0;
    }
  } catch (e) {
    return 0;
  }
  return isNaN(seconds) ? 0 : parseFloat(seconds.toFixed(3));
}
function extractPlainTextFromLexicalNode(node) {
  if (!node) return '';
  if (node.type === 'text' || node.type === 'extended-text') return node.text || '';
  let text = '';
  if (node.children && Array.isArray(node.children)) {
    for (const child of node.children) text += extractPlainTextFromLexicalNode(child);
  }
  if (node.type === 'linebreak') return '\n';
  return text;
}
export function parseLexicalTableToSegments(lexicalTableJsonString) {
  let parsedFullEditorState;
  try {
    parsedFullEditorState = JSON.parse(lexicalTableJsonString);
    if (!parsedFullEditorState?.root?.children) return [];
  } catch (error) {
    return [];
  }
  const segmentsArray = [];
  try {
    const tableNode = parsedFullEditorState.root.children.find((node) => node.type === 'table');
    if (!tableNode?.children) return [];
    for (let i = 1; i < tableNode.children.length; i++) {
      const rowNode = tableNode.children[i];
      if (
        rowNode.type !== 'tablerow' ||
        !rowNode.children ||
        !rowNode.children.length ||
        rowNode.children.length < 4
      )
        continue;
      try {
        let startTime = 0,
          endTime = 0,
          speakerName = 'Unknown',
          segmentTextJsonString = '{}',
          indexJsonString = '{}',
          timestampJsonString = '{}',
          speakerJsonString = '{}';

        const indexCellNode = rowNode.children[0];
        if (indexCellNode.type === 'tablecell') {
          indexJsonString = JSON.stringify({
            root: {
              type: 'root',
              children: JSON.parse(JSON.stringify(indexCellNode.children || [])),
              direction: null,
              format: '',
              indent: 0,
              version: 1
            }
          });
        }

        const timestampCellNode = rowNode.children[1];
        if (timestampCellNode.type !== 'tablecell') continue;
        let timestampFullText = '';
        if (timestampCellNode.children)
          timestampCellNode.children.forEach(
            (child) => (timestampFullText += extractPlainTextFromLexicalNode(child))
          );
        const timeParts = timestampFullText.split(' - ');
        startTime = parseTimestampStringToSeconds(timeParts[0]);
        endTime = timeParts.length > 1 ? parseTimestampStringToSeconds(timeParts[1]) : startTime;
        timestampJsonString = JSON.stringify({
          root: {
            type: 'root',
            children: JSON.parse(JSON.stringify(timestampCellNode.children || [])),
            direction: null,
            format: '',
            indent: 0,
            version: 1
          }
        });

        const speakerCellNode = rowNode.children[2];
        if (speakerCellNode.type !== 'tablecell') continue;
        let tempSpeakerName = '';
        if (speakerCellNode.children)
          speakerCellNode.children.forEach(
            (child) => (tempSpeakerName += extractPlainTextFromLexicalNode(child))
          );
        speakerName = tempSpeakerName.trim() || 'Unknown';
        if (speakerName.endsWith(':')) {
          speakerName = speakerName.slice(0, -1).trim();
        }
        speakerJsonString = JSON.stringify({
          root: {
            type: 'root',
            children: JSON.parse(JSON.stringify(speakerCellNode.children || [])),
            direction: null,
            format: '',
            indent: 0,
            version: 1
          }
        });

        const textContentCellNode = rowNode.children[3];
        if (textContentCellNode.type !== 'tablecell') continue;
        const deepClonedCellChildren = JSON.parse(
          JSON.stringify(textContentCellNode.children || [])
        );
        segmentTextJsonString = JSON.stringify({
          root: {
            type: 'root',
            children: deepClonedCellChildren,
            direction: null,
            format: '',
            indent: 0,
            version: 1
          }
        });

        segmentsArray.push({
          start_time: startTime,
          end_time: endTime,
          speaker: speakerName,
          text: segmentTextJsonString,
          index_json: indexJsonString,
          timestamp_json: timestampJsonString,
          speaker_json: speakerJsonString
        });
      } catch (cellProcessingError) {
        segmentsArray.push({
          start_time: 0,
          end_time: 0,
          speaker: 'Error Processing Row',
          text: JSON.stringify({
            root: {
              type: 'root',
              children: [],
              direction: null,
              format: '',
              indent: 0,
              version: 1
            }
          })
        });
      }
    }
  } catch (tableProcessingError) {
    return [];
  }
  return segmentsArray;
}

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
    project.update((p) => ({ ...p, error: 'Transcript file path is missing.' }));
    throw new Error('Transcript file path is required.');
  }
  if (!transcriptFilePath.toLowerCase().endsWith('.json')) {
  }
  const filename = transcriptFilePath.split(/[\\/]/).pop();
  project.update((p) => ({ ...p, statusMessage: `Loading transcript ${filename}...` }));
  try {
    const normalizedPath = normalizePath(transcriptFilePath);
    const fullLexicalJsonString = await invoke('load_transcript_json', {
      transcriptPath: normalizedPath
    });
    const segmentsArray = parseLexicalTableToSegments(fullLexicalJsonString);
    const currentProject = get(project);
    const projectBaseDir = currentProject.baseDirectory;
    let relativeTranscriptPath = transcriptFilePath;
    if (projectBaseDir && transcriptFilePath.startsWith(projectBaseDir)) {
      relativeTranscriptPath = transcriptFilePath.substring(projectBaseDir.length);
      if (
        relativeTranscriptPath.startsWith(sep) ||
        relativeTranscriptPath.startsWith('/') ||
        relativeTranscriptPath.startsWith('\\')
      ) {
        relativeTranscriptPath = relativeTranscriptPath.substring(1);
      }
    }
    setTranscriptData(relativeTranscriptPath, segmentsArray, false);
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    console.error(`[ProjectService] loadTranscriptFile failed for "${transcriptFilePath}":`, error);
    project.update((p) => ({
      ...p,
      error: `Transcript load failed: ${errorMessage}`,
      statusMessage: `Error loading transcript ${filename}.`
    }));
    throw new Error(`Failed to load transcript: ${errorMessage}`);
  }
}
export function extractHighlightsFromLexicalJson(lexicalJsonString, existingHighlights = []) {
  let finalHighlights = [];
  try {
    const parsed =
      typeof lexicalJsonString === 'string' ? JSON.parse(lexicalJsonString) : lexicalJsonString;

    let allTextNodes = [];
    function walk(node) {
      if (!node) return;
      if (node.highlightId) {
        allTextNodes.push(node);
      } else if (Array.isArray(node.children)) {
        node.children.forEach(walk);
      }
    }
    walk(parsed?.root);

    if (allTextNodes.length === 0) return [];

    const existingHighlightsMap = new Map((existingHighlights || []).map((h) => [h.id, h]));

    const blocks = {};
    for (const node of allTextNodes) {
      const id = node.highlightId;
      if (!blocks[id]) {
        blocks[id] = [];
      }
      blocks[id].push(node);
    }

    let orderIndex = 0;
    for (const [highlightId, block] of Object.entries(blocks)) {
      const firstNode = block[0];
      const style = typeof firstNode.style === 'string' ? firstNode.style : '';
      const colorMatch = style.match(/background-color:\s*([^;]+)/);
      const color = colorMatch ? colorMatch[1].trim() : 'transparent';

      const metadata = existingHighlightsMap.get(highlightId);

      let extractedText = '';
      for (const n of block) {
        if (typeof n.text === 'string') {
          extractedText += n.text;
        }
      }

      finalHighlights.push({
        id: highlightId,
        text: extractedText,
        nodeKey: firstNode.key || null,
        color: color,
        tags: metadata ? [...(metadata.tags || [])] : [],
        comments: metadata ? [...(metadata.comments || [])] : [],
        documentOrder: orderIndex++
      });
    }
    return finalHighlights;
  } catch (e) {
    console.error('[extractHighlightsFromLexicalJson] error:', e);
    return [];
  }
}
export async function saveTranscriptData() {
  const projData = get(project);
  const tsData = get(transcriptStore);
  const transcriptPath = tsData.currentTranscriptPath;
  const transcriptSegments = tsData.segments;
  const projectXmlPath = projData.xmlPath;

  if (!transcriptPath) throw new Error('Cannot save, no transcript loaded.');
  if (!projectXmlPath) throw new Error('Cannot save, project path unknown.');
  if (!transcriptPath.toLowerCase().endsWith('.json'))
    throw new Error('Transcript must be saved as .json.');
  const filename = transcriptPath.split(/[\\/]/).pop();
  project.update((p) => ({ ...p, statusMessage: `Saving transcript ${filename}...` }));
  let fullLexicalTableJsonString = '';
  try {
    const editorForTableAssembly = createHeadlessEditor({
      nodes: ALL_EDITOR_NODES,
      namespace: `table-assembly-editor-${Date.now()}`,
      onError: (e) => console.error('[TableAssemblyEditor] Error:', e)
    });
    await editorForTableAssembly.update(() => {
      const root = _getRoot();
      root.clear();
      const tableNode = _createTableNode();
      const headerRow = _createTableRowNode();
      const headers = ['#', 'Timestamp', 'Speaker', 'Text'];
      for (const headerText of headers) {
        const cell = _createTableCellNode({ headerState: 'column' });
        const paragraph = _createParagraphNode();
        paragraph.append(_createTextNode(headerText));
        cell.append(paragraph);
        headerRow.append(cell);
      }
      tableNode.append(headerRow);

      function appendNodesToCell(cell, serializedChildren) {
        if (!Array.isArray(serializedChildren) || serializedChildren.length === 0) {
          cell.append(_createParagraphNode());
          return;
        }
        serializedChildren.forEach((serializedNode) => {
          try {
            const liveNode = _parseSerializedNode(serializedNode);
            if (liveNode) {
              if (typeof liveNode.clone === 'function') cell.append(liveNode.clone());
              else if (typeof liveNode.constructor?.clone === 'function')
                cell.append(liveNode.constructor.clone(liveNode));
            }
          } catch (e) {
            console.error('[TableAssembly] Error parsing node for rich cell:', e);
          }
        });
      }

      for (let i = 0; i < transcriptSegments.length; i++) {
        const segment = transcriptSegments[i];
        const dataRow = _createTableRowNode();

        // 1. Index Column
        const cellNum = _createTableCellNode();
        const expectedIdxText = String(i + 1);
        let idxRichNodesUsed = false;
        if (segment.index_json) {
          try {
            const parsed = JSON.parse(segment.index_json);
            const plainText = parsed.root.children.map(extractPlainTextFromLexicalNode).join('');
            if (plainText.trim() === expectedIdxText) {
              appendNodesToCell(cellNum, parsed.root.children);
              idxRichNodesUsed = true;
            }
          } catch (e) {}
        }
        if (!idxRichNodesUsed) {
          const pNum = _createParagraphNode();
          pNum.append(_createTextNode(expectedIdxText));
          cellNum.append(pNum);
        }
        dataRow.append(cellNum);

        // 2. Timestamp Column
        const cellTime = _createTableCellNode();
        const startTimeStr = formatTimestampHtml(segment.start_time || 0);
        const endTimeStr = formatTimestampHtml(segment.end_time || 0);
        const expectedTimeText = `${startTimeStr} - ${endTimeStr}`;
        let timeRichNodesUsed = false;
        if (segment.timestamp_json) {
          try {
            const parsed = JSON.parse(segment.timestamp_json);
            const plainText = parsed.root.children.map(extractPlainTextFromLexicalNode).join('');
            if (plainText.trim() === expectedTimeText) {
              appendNodesToCell(cellTime, parsed.root.children);
              timeRichNodesUsed = true;
            }
          } catch (e) {}
        }
        if (!timeRichNodesUsed) {
          const pTime = _createParagraphNode();
          pTime.append(_createTextNode(expectedTimeText));
          cellTime.append(pTime);
        }
        dataRow.append(cellTime);

        // 3. Speaker Column
        const cellSpeaker = _createTableCellNode();
        let speakerName = segment.speaker || 'Unknown';
        if (speakerName !== 'Unknown' && !speakerName.endsWith(':')) {
          speakerName += ':';
        }
        let speakerRichNodesUsed = false;
        if (segment.speaker_json) {
          try {
            const parsed = JSON.parse(segment.speaker_json);
            const plainText = parsed.root.children.map(extractPlainTextFromLexicalNode).join('');
            if (plainText.trim() === speakerName) {
              appendNodesToCell(cellSpeaker, parsed.root.children);
              speakerRichNodesUsed = true;
            }
          } catch (e) {}
        }
        if (!speakerRichNodesUsed) {
          const pSpeaker = _createParagraphNode();
          pSpeaker.append(_createTextNode(speakerName));
          cellSpeaker.append(pSpeaker);
        }
        dataRow.append(cellSpeaker);

        // 4. Text Column (Existing Logic)
        const cellText = _createTableCellNode();
        if (segment.text && typeof segment.text === 'string') {
          let parsedSegmentState;
          try {
            parsedSegmentState = JSON.parse(segment.text);
          } catch (e) {
            const pError = _createParagraphNode();
            pError.append(_createTextNode('[Error V6: Malformed cell JSON]'));
            cellText.append(pError);
            dataRow.append(cellText);
            tableNode.append(dataRow);
            continue;
          }
          function flattenNodes(nodes) {
            return nodes.flatMap((n) =>
              n.type === 'root' && Array.isArray(n.children) ? flattenNodes(n.children) : [n]
            );
          }
          const rawChildren = parsedSegmentState?.root?.children || [];
          const serializedChildNodes = flattenNodes(rawChildren);
          if (serializedChildNodes.length > 0) {
            serializedChildNodes.forEach((serializedNodeObject) => {
              if (typeof serializedNodeObject !== 'object' || serializedNodeObject === null) return;
              try {
                const liveNode = _parseSerializedNode(serializedNodeObject);
                if (liveNode) {
                  if (typeof liveNode.clone === 'function') cellText.append(liveNode.clone());
                  else if (typeof liveNode.constructor?.clone === 'function')
                    cellText.append(liveNode.constructor.clone(liveNode));
                }
              } catch (e) {}
            });
          } else cellText.append(_createParagraphNode());
        } else cellText.append(_createParagraphNode());
        dataRow.append(cellText);

        tableNode.append(dataRow);
      }
      root.append(tableNode);
      root.append(_createParagraphNode());
    });
    fullLexicalTableJsonString = JSON.stringify(editorForTableAssembly.getEditorState().toJSON());

    // Add validation here
    const parsedJson = JSON.parse(fullLexicalTableJsonString);
    if (!parsedJson || !parsedJson.root || !Array.isArray(parsedJson.root.children)) {
      throw new Error('Generated Lexical JSON is invalid: missing root or children.');
    }

    // Auto-extract highlights from the constructed JSON and sync with the database
    const currentHighlights = get(project).currentDocumentHighlights || [];
    const extractedHighlights = extractHighlightsFromLexicalJson(parsedJson, currentHighlights);

    try {
      await invoke('save_lexical_highlights', {
        args: {
          projectId: projData.id,
          documentPath: transcriptPath,
          highlightsJson: JSON.stringify(extractedHighlights)
        }
      });
      setDocumentHighlights(extractedHighlights);
    } catch (hlError) {
      console.error('[ProjectService] Failed to sync extracted highlights to DB:', hlError);
    }
  } catch (assemblyError) {
    project.update((p) => ({
      ...p,
      error: `Save failed: Error preparing data. ${assemblyError.message}`,
      statusMessage: `Error saving transcript.`
    }));
    throw new Error(`Failed to prepare transcript data for saving: ${assemblyError.message}`);
  }
  const languageCode = tsData.activeTranscript?.language_code ?? null;
  try {
    await invoke('save_transcript_json', {
      projectXmlPath: projectXmlPath,
      transcriptPath: transcriptPath,
      lexicalTableJsonString: fullLexicalTableJsonString,
      language_code: languageCode
    });
    markTranscriptAsSaved();
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    project.update((p) => ({
      ...p,
      error: `Save failed: ${errorMessage}`,
      statusMessage: `Error saving transcript.`
    }));
    throw new Error(`Failed to save transcript: ${errorMessage}`);
  }
}

export async function replaceTranscriptText(
  segmentIndex,
  isPrimary,
  find,
  replace,
  offset,
  length
) {
  const tsStore = get(transcriptStore);
  const segments = isPrimary ? tsStore.segments : tsStore.secondaryTranscriptSegments;
  if (!segments[segmentIndex]) return;

  const segment = segments[segmentIndex];
  const updatedJson = await performReplaceInLexicalJson(
    segment.text,
    find,
    replace,
    offset,
    length
  );

  const { updateSegment, updateSecondarySegment } = await import('$lib/stores/transcriptStore.js');
  if (isPrimary) {
    updateSegment(segmentIndex, { text: updatedJson });
  } else {
    updateSecondarySegment(segmentIndex, { text: updatedJson });
  }
}

export async function replaceAllTranscriptText(find, replace, options = {}) {
  const tsStore = get(transcriptStore);
  const { isCaseSensitive = false, isRegex = false, isWholeWord = false } = options;

  project.update((p) => ({ ...p, isLoading: true, statusMessage: 'Replacing all occurrences...' }));

  const processSegments = async (segs, updateFn) => {
    const newSegs = [];
    for (let i = 0; i < segs.length; i++) {
      const updatedJson = await performReplaceAllInLexicalJson(
        segs[i].text,
        find,
        replace,
        options
      );
      if (updatedJson !== segs[i].text) {
        updateFn(i, { text: updatedJson }, true); // silent update
      }
    }
  };

  const { updateSegment, updateSecondarySegment, pushToUndoStack } =
    await import('$lib/stores/transcriptStore.js');

  pushToUndoStack();
  await processSegments(tsStore.segments, updateSegment);
  if (tsStore.isDualModeActive) {
    await processSegments(tsStore.secondaryTranscriptSegments, updateSecondarySegment);
  }

  project.update((p) => ({ ...p, isLoading: false, statusMessage: 'Replace all complete.' }));
}

async function performReplaceInLexicalJson(json, find, replace, offset, length) {
  const editor = createHeadlessEditor({ nodes: ALL_EDITOR_NODES });
  try {
    editor.setEditorState(editor.parseEditorState(json));
  } catch (e) {
    return json;
  }

  await editor.update(() => {
    const root = _getRoot();
    const textNodes = [];
    const visit = (node) => {
      if (_isTextNode(node)) textNodes.push(node);
      else if (_isElementNode(node)) node.getChildren().forEach(visit);
    };
    visit(root);

    // 1. Map global offset to nodes
    let currentOffset = 0;
    let startNode = null;
    let startOffsetInNode = 0;
    let nodesToRemove = [];

    const matchEnd = offset + length;

    for (const node of textNodes) {
      const nodeLength = node.getTextContentSize();
      const nodeEnd = currentOffset + nodeLength;

      if (nodeEnd > offset && currentOffset < matchEnd) {
        // This node is part of match
        if (!startNode) {
          startNode = node;
          startOffsetInNode = offset - currentOffset;
        }
        nodesToRemove.push(node);
      }
      currentOffset += nodeLength;
      if (currentOffset >= matchEnd) break;
    }

    // 2. Perform cross-node replacement
    if (startNode && nodesToRemove.length > 0) {
      // We'll keep the first node, update its text, and remove the others
      const firstNode = nodesToRemove[0];
      const lastNode = nodesToRemove[nodesToRemove.length - 1];

      const firstNodeText = firstNode.getTextContent();
      const lastNodeText = lastNode.getTextContent();

      const endOffsetInLastNode = matchEnd - (currentOffset - lastNode.getTextContentSize());

      if (nodesToRemove.length === 1) {
        // Single node replacement
        const newText =
          firstNodeText.slice(0, startOffsetInNode) +
          replace +
          firstNodeText.slice(startOffsetInNode + length);
        firstNode.setTextContent(newText);
      } else {
        // Multi-node replacement
        // Keep prefix of first node, add replacement, add suffix of last node
        const prefix = firstNodeText.slice(0, startOffsetInNode);
        const suffix = lastNodeText.slice(endOffsetInLastNode);
        firstNode.setTextContent(prefix + replace + suffix);

        // Remove intermediate nodes
        for (let i = 1; i < nodesToRemove.length; i++) {
          nodesToRemove[i].remove();
        }
      }
    }
  });

  return JSON.stringify(editor.getEditorState().toJSON());
}

async function performReplaceAllInLexicalJson(json, find, replace, options) {
  const { isCaseSensitive = false, isRegex = false, isWholeWord = false } = options;
  const editor = createHeadlessEditor({ nodes: ALL_EDITOR_NODES });
  try {
    editor.setEditorState(editor.parseEditorState(json));
  } catch (e) {
    return json;
  }

  let pattern = isRegex ? find : find.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (isWholeWord) pattern = `\\b${pattern}\\b`;
  const regex = new RegExp(pattern, isCaseSensitive ? 'g' : 'gi');

  await editor.update(() => {
    const root = _getRoot();

    // We must re-visit and re-flatten after each match replacement
    // OR work backwards. Reverse order is safer for offsets.

    const textNodes = [];
    const visit = (node) => {
      if (_isTextNode(node)) textNodes.push(node);
      else if (_isElementNode(node)) node.getChildren().forEach(visit);
    };
    visit(root);

    let fullText = '';
    const nodeRanges = [];
    for (const node of textNodes) {
      const start = fullText.length;
      const text = node.getTextContent();
      fullText += text;
      nodeRanges.push({ node, start, end: fullText.length });
    }

    const matches = [];
    let m;
    while ((m = regex.exec(fullText)) !== null) {
      matches.push(m);
      if (m.index === regex.lastIndex) regex.lastIndex++;
    }

    // Process matches in reverse order
    for (let i = matches.length - 1; i >= 0; i--) {
      const match = matches[i];
      const matchStart = match.index;
      const matchEnd = match.index + match[0].length;

      const nodesInMatch = [];
      for (const range of nodeRanges) {
        if (range.end > matchStart && range.start < matchEnd) {
          nodesInMatch.push({
            node: range.node,
            startInNode: Math.max(0, matchStart - range.start),
            endInNode: Math.min(range.node.getTextContentSize(), matchEnd - range.start)
          });
        }
      }

      if (nodesInMatch.length > 0) {
        const first = nodesInMatch[0];
        const last = nodesInMatch[nodesInMatch.length - 1];

        const prefix = first.node.getTextContent().slice(0, first.startInNode);
        const suffix = last.node.getTextContent().slice(last.endInNode);

        first.node.setTextContent(prefix + replace + suffix);

        // Remove intermediate and last nodes if multiple nodes involved
        for (let j = 1; j < nodesInMatch.length; j++) {
          nodesInMatch[j].node.remove();
        }
      }
    }
  });

  return JSON.stringify(editor.getEditorState().toJSON());
}

export async function refreshProjectFiles(
  targetPathToSelect = null,
  targetTranscriptPathToSelect = null
) {
  const currentProj = get(project);
  const projectXmlPath = currentProj.xmlPath;
  if (!projectXmlPath) return;
  project.update((p) => ({ ...p, statusMessage: 'Refreshing file list...', isLoading: true }));
  try {
    await loadProjectDataAndUpdateStore(
      projectXmlPath,
      targetPathToSelect,
      targetTranscriptPathToSelect
    );
    project.update((p) => ({ ...p, statusMessage: 'Project refreshed.', isLoading: false }));
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    project.update((p) => ({
      ...p,
      error: `Refresh failed: ${errorMessage}`,
      statusMessage: 'Error refreshing file list.',
      isLoading: false
    }));
  }
}
export async function renameProjectItem(itemPath, newName, itemType) {
  const currentProj = get(project);
  const projectXmlPath = currentProj.xmlPath;
  if (!projectXmlPath) {
    await message('Project data not loaded. Cannot rename.', {
      title: 'Rename Error',
      type: 'error'
    });
    throw new Error('Project path missing.');
  }
  if (!itemPath || !newName) {
    await message('Missing item path or new name.', { title: 'Rename Error', type: 'error' });
    throw new Error('Missing parameters.');
  }
  const oldFilename = await basename(itemPath);
  project.update((p) => ({
    ...p,
    statusMessage: `Renaming ${oldFilename} to ${newName}...`,
    isLoading: true
  }));
  try {
    const newPath = await invoke('rename_project_item', {
      itemPath: itemPath,
      newName: newName,
      itemType: itemType,
      projectXmlPath: projectXmlPath
    });
    await refreshProjectFiles(); // Refresh the file list after rename
    project.update((p) => ({
      ...p,
      statusMessage: `Renamed ${oldFilename} to ${newName}.`,
      fileRenamed: { oldPath: itemPath, newPath: newPath }
    }));
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    await message(`Error renaming item: ${errorMessage}`, {
      title: 'Rename Failed',
      type: 'error'
    });
    project.update((p) => ({
      ...p,
      error: `Rename failed: ${errorMessage}`,
      statusMessage: `Error renaming ${oldFilename}.`,
      isLoading: false
    }));
    throw error;
  }
}

function getErrorMessage(error) {
  if (!error) return 'Unknown error';
  if (typeof error === 'string') return error;
  if (typeof error.payload === 'string') return error.payload;
  if (typeof error.message === 'string') return error.message;
  if (typeof error.error === 'string') return error.error;
  if (typeof error.err === 'string') return error.err;

  // Recursive check for nested payloads (Tauri sometimes nests them)
  if (error.payload && typeof error.payload === 'object') {
    return getErrorMessage(error.payload);
  }

  try {
    const stringified = JSON.stringify(error);
    return stringified === '{}' ? String(error) : stringified;
  } catch (e) {
    return String(error);
  }
}
export async function deleteProjectItem(itemPath) {
  const currentProj = get(project);
  const currentTs = get(transcriptStore);
  const projectXmlPath = currentProj.xmlPath;
  if (!projectXmlPath) {
    await message('Project data not loaded. Cannot delete.', {
      title: 'Delete Error',
      type: 'error'
    });
    throw new Error('Project path missing.');
  }
  if (!itemPath) {
    await message('Missing item path.', { title: 'Delete Error', type: 'error' });
    throw new Error('Missing parameters.');
  }
  const filename = await basename(itemPath);
  project.update((p) => ({ ...p, statusMessage: `Deleting ${filename}...`, isLoading: true }));
  try {
    await invoke('delete_project_item', { itemPath: itemPath, projectXmlPath: projectXmlPath });

    const wasSelectedMedia = currentTs.selectedMediaFile?.path === itemPath;
    const wasCurrentTranscript = currentTs.currentTranscriptPath === itemPath;
    const wasSelectedDocument = currentProj.selectedDocumentPath === itemPath;
    const wasSelectedStandaloneTranscript =
      currentProj.currentStandaloneTranscriptPath === itemPath;
    const wasSelectedMediaNote = currentProj.selectedMediaNotePath === itemPath;
    const wasActiveTranscriptInDataTab = currentProj.activeTranscriptPathInDataTab === itemPath;

    if (wasSelectedMedia) selectMedia(null);
    else if (wasCurrentTranscript) clearTranscriptState();
    else if (wasSelectedDocument) prepareDocumentView(null);
    else if (wasSelectedStandaloneTranscript) prepareStandaloneTranscriptView(null);
    else if (wasSelectedMediaNote) prepareMediaNoteView(null);

    // Clear the data tab's active transcript if it was deleted, so it can correctly fall back or show "No Transcription Yet"
    if (wasActiveTranscriptInDataTab) {
      project.update((p) => ({
        ...p,
        activeTranscriptPathInDataTab: null,
        mediaNoteTranscriptError: 'INFO:FILE_NOT_FOUND'
      }));
    }

    await refreshProjectFiles();

    // Re-prepare the media note view to auto-select any remaining transcript if the active one was deleted
    if (wasActiveTranscriptInDataTab && currentProj.selectedMediaNotePath) {
      // Instead of calling prepareMediaNoteView directly (which forces the app into a global loading state
      // that might get stuck if the user is in the Transcription tab), we just resolve the fallback manually.
      const currentProjectState = get(project);
      function findMediaFileInTree(nodes, path) {
        if (!Array.isArray(nodes)) return null;
        for (const node of nodes) {
          if (node.path === path && node.file_type === 'media') return node;
          if (node.children) {
            const found = findMediaFileInTree(node.children, path);
            if (found) return found;
          }
        }
        return null;
      }
      const mediaFileNode = findMediaFileInTree(
        currentProjectState.files,
        currentProj.selectedMediaNotePath
      );
      const fallbackPath = mediaFileNode?.associated_transcripts?.[0]?.path || null;

      project.update((p) => ({
        ...p,
        activeTranscriptPathInDataTab: fallbackPath,
        mediaNoteTranscriptError: fallbackPath ? null : 'INFO:FILE_NOT_FOUND'
      }));
    }

    project.update((p) => ({ ...p, statusMessage: `Deleted ${filename}.`, isLoading: false }));
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    await message(`Error deleting item: ${errorMessage}`, {
      title: 'Delete Failed',
      type: 'error'
    });
    project.update((p) => ({
      ...p,
      error: `Delete failed: ${errorMessage}`,
      statusMessage: `Error deleting ${filename}.`,
      isLoading: false
    }));
    throw error;
  }
}
export async function handleTrimMediaConfirm(originalMediaPath, startTime, endTime) {
  if (
    !originalMediaPath ||
    typeof startTime !== 'number' ||
    typeof endTime !== 'number' ||
    startTime < 0 ||
    endTime <= startTime
  )
    throw new Error(`Invalid trim parameters provided.`);
  const filename = await basename(originalMediaPath);
  project.update((p) => ({
    ...p,
    isImportingAsset: true,
    statusMessage: `Trimming ${filename}...`
  }));
  try {
    const updatedFiles = await invoke('trim_media', { originalMediaPath, startTime, endTime });
    if (Array.isArray(updatedFiles)) {
      project.update((p) => ({
        ...p,
        files: updatedFiles,
        isImportingAsset: false,
        error: null,
        statusMessage: 'Media trimmed successfully.',
        isLoading: false
      }));
      let trimmedEntry = null;
      const originalFilename = await basename(originalMediaPath);
      const originalExtension = originalFilename.includes('.')
        ? originalFilename.substring(originalFilename.lastIndexOf('.'))
        : '';
      function findTrimmedRecursive(nodes, stemPrefix, extension) {
        if (!Array.isArray(nodes)) return null;
        for (const node of nodes) {
          if (
            node.file_type === 'media' &&
            !node.is_directory &&
            node.name.startsWith(stemPrefix) &&
            node.name.includes('_trimmed_') &&
            node.name.endsWith(extension)
          )
            return node;
          if (node.children && node.children.length > 0) {
            const found = findTrimmedRecursive(node.children, stemPrefix, extension);
            if (found) return found;
          }
        }
        return null;
      }
      const originalStem = originalFilename.includes('.')
        ? originalFilename.substring(0, originalFilename.lastIndexOf('.'))
        : originalFilename;
      trimmedEntry = findTrimmedRecursive(updatedFiles, originalStem, originalExtension);
      if (trimmedEntry) await selectMedia(trimmedEntry);
      else {
        let firstMedia = null;
        function findFirstMediaRecursive(nodes) {
          if (!Array.isArray(nodes)) return null;
          for (const node of nodes) {
            if (node.file_type === 'media' && !node.is_directory) return node;
            if (node.children && node.children.length > 0) {
              const found = findFirstMediaRecursive(node.children);
              if (found) return found;
            }
          }
          return null;
        }
        firstMedia = findFirstMediaRecursive(updatedFiles);
        if (firstMedia) await selectMedia(firstMedia);
      }
    } else {
      await refreshProjectFiles();
      throw new Error('Received invalid data from trim process.');
    }
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    project.update((p) => ({
      ...p,
      isImportingAsset: false,
      error: `Trim failed: ${errorMessage}`,
      statusMessage: `Error trimming media.`,
      isLoading: false
    }));
    throw new Error(`Trim failed: ${errorMessage}`);
  }
}

export let transcribeModalInstance = null;
export function registerTranscribeModal(instance) {
  transcribeModalInstance = instance;
}
export async function requestTranscription() {
  const storeState = get(transcriptStore);
  console.log(
    `[DEBUG PS requestTranscription] Called. Current store state: isTranscribing=${storeState.isTranscribing}, showModal=${storeState.showTranscribeModal}, jobStatus=${storeState.transcriptionJobStatus}`
  );
  const currentTs = get(transcriptStore);
  const currentProj = get(project);
  if (!currentTs.selectedMediaFile?.path) {
    await message('Please select a media file first.', {
      title: 'Transcription Request',
      type: 'info'
    });
    return;
  }
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

  console.log(
    `[DEBUG] projectService.handleConfirmStartTranscription: modelNameForJob = ${modelNameForJob}`
  );

  if (!mediaPathForJob || !modelNameForJob) {
    // Use notification store for error
    notificationStore.add('Error: Missing media file or model selection.', 'error', 0);
    // Call setTranscriptionStatus to reflect the error state in the modal and keep it open
    setTranscriptionStatus(false, null, {
      // isTranscribing is false
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
    language_code:
      currentTs.selectedLanguage === 'auto' || !currentTs.selectedLanguage
        ? null
        : currentTs.selectedLanguage,
    model_name: modelNameForJob,
    translate_to_english: translateToEnglish, // Use variable defined above
    speaker_names: currentTs.speakers.names || [],
    translated_speaker_names: translateToEnglish ? currentTs.speakers.translatedNames || [] : [],
    transcription_mode: transcriptionMode,
    transcription_engine: currentTs.selectedTranscriptionEngine,
    initial_prompt: currentTs.initialPrompt || null,
    hotwords: currentTs.hotwords || null
  };

  // Step 1: Set status to 'initiating'. JobId is null at this point.
  // This makes isTranscribing=true, and the modal should show an "Initiating..." state.
  setTranscriptionStatus(true, null, {
    // jobIdToSet is null
    status: 'initiating',
    initialProgressMessage: `Initiating with ${modelNameForJob}...`,
    mediaPath: mediaPathForJob
  });

  try {
    // Always call the unified command
    const initiatedPayload = await invoke('transcribe_media_command', { payload: payload });

    if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
      throw new Error('Backend did not return a valid job_id.');
    }
    const backendJobId = initiatedPayload.job_id;

    // Immediately set the job ID in the store
    transcriptStore.update((ts) => ({ ...ts, transcriptionJobId: backendJobId }));

    // Step 2: Update status to 'running' with the actual job ID from the backend.
    setTranscriptionStatus(true, backendJobId, {
      // Pass the backendJobId
      status: 'running',
      // The progress message might be quickly updated by the first actual progress event.
      initialProgressMessage: `Transcription started (Job: ${backendJobId.substring(0, 8)})...`,
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
        if (stringifiedError !== '{}') {
          // Avoid empty object stringification
          displayMessage = `Operation failed: ${stringifiedError}`;
        }
      } catch (stringifyError) {
        // Ignore if stringify fails
      }
    }

    // Check if the error message indicates cancellation
    const lowerCaseMessage = displayMessage.toLowerCase();
    if (
      lowerCaseMessage.includes('cancel') ||
      lowerCaseMessage.includes('cancelled') ||
      lowerCaseMessage.includes('canceled')
    ) {
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
    console.warn(
      '[ProjectService handleCancel] No active job ID or not transcribing. JobID:',
      jobId,
      'IsTranscribing:',
      currentTs.isTranscribing
    );
    return;
  }

  // Update UI to "cancelling" state immediately
  transcriptStore.update((ts) => ({ ...ts, transcriptionJobStatus: 'cancelling' }));

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
    transcriptStore.update((ts) => ({
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
  // console.log('[DEBUG] initializeProgressListener called');
  if (progressListenerInitialized) return;
  try {
    progressUnlistenFn = await listen('TRANSCRIPTION_PROGRESS', (event) => {
      // console.log('[DEBUG] projectService: TRANSCRIPTION_PROGRESS event received:', event);
      const payload = event.payload;
      if (!payload || typeof payload !== 'object') {
        // console.log('[DEBUG] projectService: Payload empty or not an object');
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
    console.error('[ProjectService] Failed to initialize progress listener:', e);
    project.update((p) => ({ ...p, error: 'Failed to initialize progress listener.' }));
  }
}
export function cleanupProgressListener() {
  if (progressUnlistenFn) {
    progressUnlistenFn();
    progressUnlistenFn = null;
  }
  progressListenerInitialized = false;
}

export function formatTimestampHtml(seconds) {
  if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00.000';
  const totalMs = Math.round(seconds * 1000);
  const ms = String(totalMs % 1000).padStart(3, '0');
  const totalS = Math.floor(totalMs / 1000);
  const sec = String(totalS % 60).padStart(2, '0');
  const min = String(Math.floor(totalS / 60)).padStart(2, '0');
  return `${min}:${sec}.${ms}`;
}
export function isLexicalJson(jsonString) {
  if (!jsonString || typeof jsonString !== 'string') return false;
  try {
    const parsed = JSON.parse(jsonString);
    return (
      parsed &&
      typeof parsed === 'object' &&
      parsed.root &&
      typeof parsed.root === 'object' &&
      Array.isArray(parsed.root.children)
    );
  } catch (e) {
    return false;
  }
}

async function processJsonToRemoveHighlights(jsonString, isDocument = false) {
  if (!jsonString) return jsonString;
  const editor = createHeadlessEditor({
    nodes: ALL_EDITOR_NODES,
    namespace: `highlight-remover-${Date.now()}`,
    onError: (e) => console.error('[HighlightRemover] Error:', e)
  });

  let parsedState;
  try {
    parsedState = editor.parseEditorState(jsonString);
  } catch (e) {
    console.warn('Failed to parse JSON for highlight removal, returning original.', e);
    return jsonString;
  }

  editor.setEditorState(parsedState);

  const currentLayoutKey = get(activeLayout) || 'Layout1';
  const layoutConfig = DOCX_LAYOUT_COLUMN_CONFIGS[currentLayoutKey];

  await editor.update(() => {
    const root = _getRoot();
    const nodes = [];

    const traverse = (node) => {
      nodes.push(node);
      if (node.getChildren) {
        Array.from(node.getChildren()).forEach(traverse);
      }
    };
    traverse(root);

    nodes.forEach((node) => {
      if (
        (node.getType() === 'extended-text' || node instanceof ExtendedTextNode) &&
        node.setHighlightId
      ) {
        node.setHighlightId(null);
      }
      if (
        node instanceof TextNode ||
        node.getType() === 'text' ||
        node.getType() === 'extended-text'
      ) {
        const style = node.getStyle();
        if (style && typeof style === 'string' && style.includes('background-color')) {
          const newStyle = style.replace(/background-color\s*:\s*[^;]+;?/gi, '');
          node.setStyle(newStyle);
        }
      }
      if (node.getType() === 'table' && typeof node.setColWidths === 'function') {
        const firstRow = node.getFirstChild();
        const numCols = firstRow ? firstRow.getChildrenSize() : 0;
        if (numCols > 0) {
          let newWidths;

          // Use percentage-based widths from layout configuration if available to maintain
          // responsiveness across both the editor and exported documents.
          if (
            layoutConfig &&
            layoutConfig.colgroup &&
            Array.isArray(layoutConfig.colgroup) &&
            layoutConfig.colgroup.length === numCols
          ) {
            newWidths = [...layoutConfig.colgroup];
          } else {
            const defaultPct = Math.floor(100 / numCols);
            newWidths = Array(numCols).fill(`${defaultPct}%`);
          }

          // Final safety check to ensure no null/undefined values slip in
          for (let i = 0; i < newWidths.length; i++) {
            if (newWidths[i] == null) {
              newWidths[i] = `${Math.floor(100 / numCols)}%`;
            }
          }

          // For documents, convert any percentage strings to absolute pixel numbers
          // relative to a standard 1000px width to match the behavior of working tables.
          if (isDocument) {
            const REFERENCE_WIDTH = 1000;
            newWidths = newWidths.map((w) => {
              if (typeof w === 'string' && w.endsWith('%')) {
                const pct = parseFloat(w);
                return (REFERENCE_WIDTH * pct) / 100;
              }
              return typeof w === 'string' ? parseFloat(w) : w;
            });
          }

          node.setColWidths(newWidths);
        }
      }
    });
  });

  return JSON.stringify(editor.getEditorState().toJSON());
}

export async function convertAndSaveTranscriptAsDoc() {
  const projData = get(project);
  const tsData = get(transcriptStore);
  const transcriptPath = tsData.currentTranscriptPath;
  const selectedMedia = tsData.selectedMediaFile;
  const projectXmlPath = projData.xmlPath;
  const projectBaseDir = projData.baseDirectory;
  if (!transcriptPath) throw new Error('No transcript file loaded.');
  if (!selectedMedia?.path) throw new Error('No media file selected.');
  if (!projectBaseDir) throw new Error('Project base directory not found.');
  if (!projectXmlPath) throw new Error('Project XML path not found.');
  project.update((p) => ({ ...p, statusMessage: `Converting transcript to table document...` }));
  const finalTableEditor = createHeadlessEditor({
    nodes: ALL_EDITOR_NODES,
    namespace: `doc-table-finalizer-${Date.now()}`,
    onError: (error) => console.error(error)
  });
  let finalLexicalJsonString = '';
  try {
    const fullLexicalTableString = await invoke('load_transcript_json', {
      transcriptPath: transcriptPath
    });
    if (!fullLexicalTableString) throw new Error('Transcript file content is empty.');
    finalLexicalJsonString = await processJsonToRemoveHighlights(fullLexicalTableString, true);

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

    project.update((p) => ({ ...p, statusMessage: `Saving transcript document...` }));

    const targetFullPath = await invoke('get_unique_document_path', {
      targetDirStr: targetDocumentDir, // Pass the correctly constructed target directory
      baseName: docFilenameBase,
      extension: 'json'
    });
    console.debug(
      `[ProjectService] targetFullPath from get_unique_document_path: ${targetFullPath}`
    );
    const docFilename = await basename(targetFullPath);
    await invoke('save_document_and_update_xml', {
      projectXmlPath: projectXmlPath,
      targetPath: targetFullPath,
      documentName: docFilename,
      jsonContent: finalLexicalJsonString
    });

    const relativePath = targetFullPath.substring(projectBaseDir.length + 1).replace(/\\/g, '/');
    const fileMetadata = {
      file_name: docFilename,
      file_path: targetFullPath,
      last_modified: new Date().toISOString(),
      title: '',
      description: '',
      summary: '',
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
      file_type: 'document'
    };

    await invoke('update_asset_metadata_command', {
      projectXmlPathStr: projectXmlPath,
      assetRelativePath: relativePath,
      metadataPayload: fileMetadata,
      customFieldsPayload: null,
      assetType: 'document'
    });

    project.update((p) => ({ ...p, statusMessage: `Document file created: ${docFilename}` }));
    await refreshProjectFiles();
    return targetFullPath;
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    project.update((p) => ({
      ...p,
      statusMessage: `Error converting transcript: ${errorMessage}`
    }));
    throw error;
  }
}

export async function convertAndSaveTranscriptAsTranscript() {
  const projData = get(project);
  const tsData = get(transcriptStore);
  const transcriptPath = tsData.currentTranscriptPath;
  const selectedMedia = tsData.selectedMediaFile;
  const projectXmlPath = projData.xmlPath;
  const projectBaseDir = projData.baseDirectory;

  if (!transcriptPath) throw new Error('No transcript file loaded.');
  if (!selectedMedia?.path) throw new Error('No media file selected.');
  if (!projectBaseDir) throw new Error('Project base directory not found.');
  if (!projectXmlPath) throw new Error('Project XML path not found.');

  project.update((p) => ({ ...p, statusMessage: `Saving as imported transcript...` }));

  try {
    const rawLexicalTableString = await invoke('load_transcript_json', {
      transcriptPath: transcriptPath
    });
    if (!rawLexicalTableString) throw new Error('Transcript file content is empty.');
    const fullLexicalTableString = await processJsonToRemoveHighlights(
      rawLexicalTableString,
      false
    );

    const originalTranscriptFilename = await basename(transcriptPath);
    const originalTranscriptStem = originalTranscriptFilename.includes('.')
      ? originalTranscriptFilename.substring(0, originalTranscriptFilename.lastIndexOf('.'))
      : originalTranscriptFilename;

    const transcriptFilenameBase = originalTranscriptStem;

    // Target is Transcripts folder
    // Note: sep() returns a promise in Tauri v2, so await it if needed, or use the imported symbol if it's a string.
    // In this file 'sep' is imported from @tauri-apps/api/path which is a Promise in v2 usually?
    // Checking imports: import { dirname, basename, sep, join } from '@tauri-apps/api/path';
    // Wait, `sep` is a property in v1 but might be a Promise in v2.
    // In `convertAndSaveTranscriptAsDoc`, it uses `sep()`. Let's check.
    // `const targetDocumentDir = ${projectBaseDir}${sep()}${HARVEY_FILES_DIR}${sep()}${DOCS_DIR_NAME}${sep()}${originalTranscriptStem};`
    // So yes, it is called as a function.

    const targetTranscriptsDir = `${projectBaseDir}${sep()}${HARVEY_FILES_DIR}${sep()}${TRANSCRIPTS_DIR_IMPORTED}${sep()}${originalTranscriptStem}`;

    // Use get_unique_document_path to ensure uniqueness (reuse checking logic)
    const targetFullPath = await invoke('get_unique_document_path', {
      targetDirStr: targetTranscriptsDir,
      baseName: transcriptFilenameBase,
      extension: 'json'
    });

    const transcriptFilename = await basename(targetFullPath);

    await invoke('save_standalone_transcript_and_update_xml', {
      projectXmlPath: projectXmlPath,
      targetPath: targetFullPath,
      transcriptName: transcriptFilename,
      jsonContent: fullLexicalTableString
    });

    // Save metadata
    // Construct relative path manually to ensure it's correct for DB key
    // Normalize targetFullPath to ensure consistent separators and prefix handling
    const normalizedTargetFullPath = normalizePath(targetFullPath);
    let relativePath = normalizedTargetFullPath.substring(projectBaseDir.length);
    if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
      relativePath = relativePath.substring(1);
    }
    relativePath = relativePath.replace(/\\/g, '/');

    const fileMetadata = {
      file_name: transcriptFilename,
      file_path: targetFullPath,
      last_modified: new Date().toISOString(),
      title: '',
      description: '',
      summary: '',
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
      file_type: 'transcript'
    };

    await invoke('update_asset_metadata_command', {
      projectXmlPathStr: projectXmlPath,
      assetRelativePath: relativePath,
      metadataPayload: fileMetadata,
      customFieldsPayload: null,
      assetType: 'standalone_transcript'
    });

    // Attach the original media file to the new transcript
    if (selectedMedia?.path) {
      try {
        project.update((p) => ({ ...p, statusMessage: `Attaching media to transcript...` }));
        await invoke('upload_attachment', {
          projectXmlPathStr: projectXmlPath,
          assetRelativePath: relativePath,
          sourceFilePathStr: selectedMedia.path
        });
        console.log(`[ProjectService] Media attached to transcript: ${relativePath}`);
      } catch (attachErr) {
        console.error(`[ProjectService] Failed to attach media:`, attachErr);
        // Don't fail the whole operation if attachment fails, just log it.
        await message(`Transcript saved, but failed to attach media: ${attachErr}`, {
          title: 'Attachment Warning',
          type: 'warning'
        });
      }
    }

    project.update((p) => ({
      ...p,
      statusMessage: `Transcript saved as imported: ${transcriptFilename}`
    }));
    await refreshProjectFiles();
    return targetFullPath;
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    project.update((p) => ({
      ...p,
      statusMessage: `Error saving transcript as imported: ${errorMessage}`
    }));
    throw error;
  }
}

export async function loadActiveDocumentContent() {
  const currentProj = get(project);
  const filePath = currentProj.selectedDocumentPath;
  if (!filePath) {
    project.update((p) => ({ ...p, isDocumentLoading: false, documentError: null }));
    return;
  }
  const filename = await basename(filePath);
  project.update((p) => ({ ...p, isDocumentLoading: true, documentError: null }));
  try {
    const jsonContent = await invoke('load_note_json', { filePath });
    if (!jsonContent || jsonContent.trim() === '')
      throw new Error('Loaded document content empty/invalid.');
    try {
      JSON.parse(jsonContent);
    } catch (e) {
      throw new Error(`Loaded document content not valid JSON.`);
    }
    setLoadedDocumentData(filePath, jsonContent);
  } catch (error) {
    const errorMessage = getErrorMessage(error);
    setDocumentLoadFailed(filePath, errorMessage);
    await message(`Error loading document '${filename}': ${errorMessage}`, {
      title: 'Load Document Error',
      type: 'error'
    });
  }
}
export async function saveCurrentPdfAnnotations() {
  const projState = get(project);
  if (
    !projState.selectedDocumentPath ||
    !projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')
  )
    return;
  if (!projState.isPdfAnnotationsDirty) return;

  const projectBaseDir = projState.baseDirectory;
  if (!projectBaseDir) {
    console.error('[ProjectService] saveCurrentPdfAnnotations: Project base directory is missing.');
    notificationStore.add(
      'Error: Project base directory is missing. Cannot save PDF annotations.',
      'error'
    );
    return;
  }
  if (!projState.id || typeof projState.id !== 'string' || projState.id.trim() === '') {
    // project_uuid is stored as 'id' in projectStore
    console.error(
      '[ProjectService] saveCurrentPdfAnnotations: project_uuid (project.id) is missing or invalid.',
      projState
    );
    await message(
      'Cannot save annotations: Project identifier is missing or invalid. Please ensure the project is fully loaded.',
      { title: 'Save Error', type: 'error' }
    );
    return;
  }
  const projectId = projState.id;

  let relativePdfPath = projState.selectedDocumentPath;
  if (
    relativePdfPath.startsWith(projectBaseDir + sep) ||
    relativePdfPath.startsWith(projectBaseDir + '/')
  ) {
    relativePdfPath = relativePdfPath.substring(projectBaseDir.length + 1);
  } else if (relativePdfPath.startsWith(projectBaseDir)) {
    relativePdfPath = relativePdfPath.substring(projectBaseDir.length);
    if (
      relativePdfPath.startsWith(sep) ||
      relativePdfPath.startsWith('/') ||
      relativePdfPath.startsWith('\\')
    ) {
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
    console.log(
      `[ProjectService] PDF annotations saved for ${relativePdfPath} in project ${projectId}`
    );
  } catch (error) {
    console.error(
      `[ProjectService] Error saving PDF annotations for ${relativePdfPath} in project ${projectId}:`,
      error
    );
    const errorMessage = getErrorMessage(error);
    notificationStore.add(`Error saving PDF annotations: ${errorMessage}`, 'error');
    // Do not throw here to avoid unhandled promise rejections if the caller doesn't catch.
  }
}
export async function saveTableData(tablePath, tableData, orderedHeaders) {
  if (!tablePath) {
    throw new Error('Cannot save, no table path specified.');
  }
  if (!tableData) {
    throw new Error('Cannot save, no table data provided.');
  }

  const filename = await basename(tablePath);
  project.update((p) => ({ ...p, statusMessage: `Saving table ${filename}...` }));

  try {
    await invoke('save_table_data', {
      tablePathStr: tablePath,
      tableData: tableData,
      headers: orderedHeaders
    });
    project.update((p) => ({
      ...p,
      isDocumentDirty: false,
      statusMessage: `Table saved: ${filename}`
    }));
  } catch (error) {
    const errorMessage = error?.message || String(error);
    project.update((p) => ({
      ...p,
      documentError: `Failed to save table: ${errorMessage}`,
      statusMessage: `Error saving ${filename}.`
    }));
    await message(`Error saving table: ${errorMessage}`, {
      title: 'Save Table Error',
      type: 'error'
    });
    throw error;
  }
}
export async function saveDocumentContent(filePath, jsonContent) {
  if (filePath && filePath.toLowerCase().endsWith('.pdf')) {
    project.update((p) => ({
      ...p,
      documentError: 'PDF content cannot be saved this way.',
      statusMessage: 'Save failed (PDF type).'
    }));
    throw new Error('PDF content saving is not handled by saveDocumentContent.');
  }
  if (!filePath || jsonContent === null || typeof jsonContent !== 'string') {
    const errorMsg = 'Cannot save document: Missing path or invalid/missing JSON content.';
    await message(errorMsg, { title: 'Save Error', type: 'error' });
    project.update((p) => ({ ...p, documentError: errorMsg, statusMessage: 'Save failed.' }));
    throw new Error(errorMsg);
  }
  try {
    const parsed = JSON.parse(jsonContent);
    if (!parsed.root?.children) throw new Error('Invalid Lexical JSON structure.');
  } catch (e) {
    const errorMsg = `Cannot save document: Content not valid JSON or invalid structure. ${e.message}`;
    await message(errorMsg, { title: 'Save Error', type: 'error' });
    project.update((p) => ({
      ...p,
      documentError: errorMsg,
      statusMessage: 'Save failed (invalid content).'
    }));
    throw new Error(errorMsg);
  }

  const projState = get(project);
  const filename = await basename(filePath);
  project.update((p) => ({ ...p, statusMessage: `Saving document ${filename}...` }));

  let mainContentSaveError = null;
  try {
    const highlights_json = projState.isDocumentMetadataDirty
      ? JSON.stringify(projState.currentDocumentHighlights || [])
      : null;

    await invoke('save_note_json', {
      targetPath: filePath,
      jsonContent: jsonContent,
      highlightsJson: highlights_json
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
    if (highlights_json !== null) {
      markDocumentMetadataAsSaved(projState.currentDocumentFileLevelMetadata);
    }
  } catch (error) {
    mainContentSaveError = error;
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    project.update((p) => ({
      ...p,
      documentError: `Failed save document: ${errorMessage}`,
      statusMessage: `Error saving ${filename}.`
    }));
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
    await message(
      `Error saving document '${filename}': ${mainContentSaveError.message || mainContentSaveError}`,
      { title: 'Save Document Error', type: 'error' }
    );
    throw mainContentSaveError;
  }
  if (metadataSaveError) {
    // We don't throw here because the main content saved successfully.
    // The error will be handled by saveDocumentMetadata itself.
  }
}

export async function saveHighlightChanges(highlight) {
  if (!highlight || !highlight.id || !highlight.source || !highlight.source.file_path) {
    console.error(
      '[ProjectService] saveHighlightChanges: Invalid highlight object provided.',
      highlight
    );
    throw new Error('Invalid highlight object provided for saving.');
  }

  const { source, ...highlightData } = highlight;
  const filePath = source.file_path;
  const docType = source.file_type;
  const proj = get(project);

  if (!proj.id) {
    console.error('[ProjectService] saveHighlightChanges: Project ID is missing.');
    throw new Error('Project ID is missing.');
  }

  try {
    await invoke('save_highlight_changes', {
      projectId: proj.id,
      filePath: filePath,
      docType: docType,
      highlight: highlightData
    });
    console.log(`[ProjectService] Highlight changes saved for ${filePath}`);
  } catch (error) {
    console.error(`[ProjectService] Error saving highlight changes for ${filePath}:`, error);
    notificationStore.add(`Error saving highlight changes: ${error.message || error}`, 'error');
    throw error;
  }
}

export async function saveStandaloneTranscriptContent(
  filePath,
  jsonContent,
  highlightsJson = null
) {
  if (!filePath || jsonContent === null || typeof jsonContent !== 'string') {
    const errorMsg = 'Cannot save transcript: Missing path or invalid/missing JSON content.';
    await message(errorMsg, { title: 'Save Error', type: 'error' });
    project.update((p) => ({
      ...p,
      standaloneTranscriptError: errorMsg,
      statusMessage: 'Save failed.'
    }));
    throw new Error(errorMsg);
  }
  try {
    const parsed = JSON.parse(jsonContent);
    if (!parsed.root?.children) throw new Error('Invalid Lexical JSON structure.');
  } catch (e) {
    const errorMsg = `Cannot save transcript: Content not valid JSON or invalid structure. ${e.message}`;
    await message(errorMsg, { title: 'Save Error', type: 'error' });
    project.update((p) => ({
      ...p,
      standaloneTranscriptError: errorMsg,
      statusMessage: 'Save failed (invalid content).'
    }));
    throw new Error(errorMsg);
  }

  const projState = get(project);
  const filename = await basename(filePath);
  project.update((p) => ({ ...p, statusMessage: `Saving transcript ${filename}...` }));

  try {
    let finalHighlightsJson = highlightsJson;
    if (finalHighlightsJson === null) {
      finalHighlightsJson = projState.isStandaloneTranscriptMetadataDirty
        ? JSON.stringify(projState.currentStandaloneTranscriptHighlights || [])
        : null;
    }

    await invoke('save_note_json', {
      targetPath: filePath,
      jsonContent: jsonContent,
      highlightsJson: finalHighlightsJson
    });

    const { markStandaloneTranscriptAsSaved } = await import('$lib/stores/projectStore.js');
    markStandaloneTranscriptAsSaved(filePath, jsonContent);
  } catch (error) {
    const errorMessage = typeof error === 'string' ? error : error?.message || 'Unknown error';
    project.update((p) => ({
      ...p,
      standaloneTranscriptError: `Failed save transcript: ${errorMessage}`,
      statusMessage: `Error saving ${filename}.`
    }));
    await message(`Error saving transcript '${filename}': ${errorMessage}`, {
      title: 'Save Transcript Error',
      type: 'error'
    });
    throw error;
  }
}
export async function loadDocumentMetadata(originalDocumentAbsPath) {
  const proj = get(project);
  if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) return null;
  let relativePath = '';
  const base = proj.baseDirectory;
  const absPath = originalDocumentAbsPath;
  if (absPath.startsWith(base)) {
    relativePath = absPath.substring(base.length);
    if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length);
    if (relativePath.startsWith('/') || relativePath.startsWith('\\'))
      relativePath = relativePath.substring(1);
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
          console.error('Failed to parse highlights JSON from backend:', e);
          result.highlights = [];
        }
      } else {
        result.highlights = [];
      }
      return result;
    }
    return null;
  } catch (error) {
    console.error('Error loading document metadata:', error);
    return null;
  }
}
export async function saveDocumentMetadata(originalDocumentAbsPath) {
  const proj = get(project);
  if (!proj.xmlPath || !proj.baseDirectory || !originalDocumentAbsPath) {
    console.error(
      '[ProjectService saveDocMeta] Pre-condition failed: Missing project data or path.'
    );
    return;
  }
  // If not dirty and it's the currently selected document, no need to save.
  if (!proj.isDocumentMetadataDirty && originalDocumentAbsPath === proj.selectedDocumentPath) {
    console.log('[ProjectService saveDocMeta] No metadata changes to save for current document.');
    return;
  }

  let relativePath = '';
  const base = proj.baseDirectory;
  const absPath = originalDocumentAbsPath;
  const docFilename = await basename(absPath);

  if (absPath.startsWith(base)) {
    relativePath = absPath.substring(base.length);
    if (relativePath.startsWith(sep)) relativePath = relativePath.substring(sep.length);
    // Normalize path separators for consistency, though backend might do this too
    if (relativePath.startsWith('/') || relativePath.startsWith('\\'))
      relativePath = relativePath.substring(1);
  } else {
    await message(
      `Internal error: Could not determine relative path for metadata saving. Path ${absPath} not in base ${base}`,
      { title: 'Save Metadata Error', type: 'error' }
    );
    throw new Error('Failed to construct relative path for metadata saving.');
  }
  const originalDocumentRelativePathStr = relativePath.replace(/\\/g, '/');

  // Prepare the metadata fields from the store for the payload
  // This fullMetadataToSave structure is slightly different from what's directly passed.
  // We'll use its components to build the metadataPayload.
  const fullMetadataToSave = {
    metadata: {
      file_name: docFilename,
      last_modified:
        proj.currentDocumentFileLevelMetadata.last_modified || new Date().toISOString(),
      title: proj.currentDocumentFileLevelMetadata.title || '',
      description: proj.currentDocumentFileLevelMetadata.description || '',
      summary: proj.currentDocumentFileLevelMetadata.summary || ''
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
      summary: fullMetadataToSave.metadata.summary
      // Optional fields from Rust's FileMetadata struct (duration_seconds, width, height, etc.)
      // are intentionally omitted. The backend should treat missing fields as None
      // and not update them, preserving existing technical metadata. `created_at` also not sent.
    };

    await invoke('update_asset_metadata_command', {
      projectXmlPathStr: proj.xmlPath,
      assetRelativePath: originalDocumentRelativePathStr, // Key for DB lookup
      metadataPayload: metadataPayload,
      customFieldsPayload: null, // Ensure this is null
      assetType: 'doc' // Explicitly set asset type
    });

    markDocumentMetadataAsSaved(fullMetadataToSave.metadata); // Update UI state
    console.log(
      `[ProjectService saveDocMeta] Document metadata saved for: ${originalDocumentRelativePathStr}`
    );
  } catch (error) {
    const errorMsg =
      error.message || (typeof error === 'string' ? error : 'Unknown error saving metadata.');
    console.error(
      `[ProjectService saveDocMeta] Error for ${originalDocumentRelativePathStr}:`,
      errorMsg
    );
    await message(`Error saving document metadata: ${errorMsg}`, {
      title: 'Save Metadata Error',
      type: 'error'
    });
    throw new Error(errorMsg); // Re-throw to indicate failure
  }
}

export async function loadImageAnnotations(imageAbsPath) {
  const { setLoadedImageAnnotations, setImageAnnotationsLoadFailed } =
    await import('$lib/stores/projectStore.js');

  const currentProj = get(project);
  const projectBaseDir = currentProj.baseDirectory;
  const projectId = currentProj.id;

  if (!imageAbsPath) {
    setLoadedImageAnnotations([]);
    return;
  }

  if (!projectBaseDir || !projectId) {
    const errorMsg = 'Project data not fully loaded.';
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
    console.error('[ProjectService] saveImageAnnotations: Project data not fully loaded.');
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
    console.error(
      `[ProjectService] Error saving image annotations for ${relativeImagePath}:`,
      error
    );
    notificationStore.add(`Error saving image annotations: ${error.message || error}`, 'error');
  }
}

export async function checkUnsavedChangesThenProceed(
  newPathToLoad,
  providedActionContextDescription
) {
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
    if (
      projState.activeMediaNoteEditorRef?.ref &&
      typeof projState.activeMediaNoteEditorRef.ref.save === 'function'
    ) {
      saveFunction = projState.activeMediaNoteEditorRef.ref.save;
      discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
      initialContentForReset = projState.initialMediaNoteTranscriptJson;
      resetEditorFunction = projState.activeMediaNoteEditorRef.ref.resetEditorState;
    } else {
      console.warn(
        `[checkUnsavedChanges] Media note for ${itemPath} is dirty but editor ref missing.`
      );
      discardFunction = () => markMediaNoteTranscriptChangesDiscarded(itemPath);
    }
  } else if (
    projState.selectedDocumentPath &&
    projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') &&
    projState.isPdfAnnotationsDirty
  ) {
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
  } else if (projState.selectedDocumentType == 'tables' && projState.isDocumentDirty) {
    itemIsDirty = true;
    itemPath = projState.selectedDocumentPath;
    itemTypeForPrompt = 'table';
    saveFunction = async () => saveTableData(itemPath, projState.tableData);
    discardFunction = () => {};
  } else if (
    projState.selectedDocumentPath &&
    (projState.isDocumentDirty || projState.isDocumentMetadataDirty)
  ) {
    itemIsDirty = true;
    itemPath = projState.selectedDocumentPath;
    itemTypeForPrompt = 'document';
    if (
      projState.activeDocumentEditorRef?.ref &&
      typeof projState.activeDocumentEditorRef.ref.save === 'function'
    ) {
      saveFunction = projState.activeDocumentEditorRef.ref.save;
    } else {
      if (projState.isDocumentDirty || projState.isDocumentMetadataDirty) {
        saveFunction = () => saveDocumentContent(itemPath, projState.currentDocumentJson);
      }
    }
    discardFunction = () => markDocumentChangesDiscarded();
    initialContentForReset = projState.initialDocumentJson;
    resetEditorFunction = projState.activeDocumentEditorRef?.ref?.resetEditorState;
  } else if (
    projState.currentStandaloneTranscriptPath &&
    (projState.isStandaloneTranscriptDirty || projState.isStandaloneTranscriptMetadataDirty)
  ) {
    itemIsDirty = true;
    itemPath = projState.currentStandaloneTranscriptPath;
    itemTypeForPrompt = 'imported transcript';
    if (
      projState.activeStandaloneTranscriptEditorRef?.ref &&
      typeof projState.activeStandaloneTranscriptEditorRef.ref.save === 'function'
    ) {
      saveFunction = projState.activeStandaloneTranscriptEditorRef.ref.save;
      discardFunction = () => markStandaloneTranscriptChangesDiscarded(itemPath);
      initialContentForReset = projState.initialStandaloneTranscriptLexicalJson;
      resetEditorFunction = projState.activeStandaloneTranscriptEditorRef.ref.resetEditorState;
    } else {
      discardFunction = () => markStandaloneTranscriptChangesDiscarded(itemPath);
    }
  } else if (tsState.currentTranscriptPath && tsState.transcriptDirty) {
    itemIsDirty = true;
    itemPath = tsState.currentTranscriptPath;
    itemTypeForPrompt = 'main transcript';
    saveFunction = async () => saveTranscriptData();
    discardFunction = () => {
      const undoStack = get(transcriptStore).transcriptUndoStack;
      transcriptStore.update((ts) => ({
        ...ts,
        segments: undoStack.length > 0 ? undoStack[0] : ts.segments,
        transcriptDirty: false,
        transcriptUndoStack: [],
        transcriptRedoStack: []
      }));
    };
  }

  if (itemIsDirty && itemPath === newPathToLoad) {
    return true;
  }

  if (!itemIsDirty) {
    return true;
  }

  itemName = itemPath ? await basename(itemPath) : 'current item';
  const actionContextDisplay = newPathToLoad
    ? `load '${await basename(newPathToLoad)}'`
    : providedActionContextDescription || 'perform this action';

  if (
    itemTypeForPrompt === 'media notes' &&
    projState.mediaNoteTranscriptError === 'INFO:FILE_NOT_FOUND'
  ) {
    return true;
  }

  if (true) {
    if (
      projState.selectedDocumentPath &&
      projState.selectedDocumentPath.toLowerCase().endsWith('.pdf') &&
      projState.isPdfAnnotationsDirty
    ) {
      try {
        await saveCurrentPdfAnnotations();
        return true;
      } catch (error) {
        console.error('[checkUnsavedChanges] Implicit save for PDF annotations failed:', error);
        const proceedAfterFail = await confirm(
          `Failed to automatically save changes for PDF annotations on "${itemName}".\nError: ${error.message || error}\n\nDiscard unsaved changes and continue to ${actionContextDisplay}?`,
          {
            title: 'Autosave Failed',
            type: 'error',
            okLabel: 'Discard and Continue',
            cancelLabel: 'Cancel Action'
          }
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
          {
            title: 'Autosave Failed',
            type: 'error',
            okLabel: 'Discard and Continue',
            cancelLabel: 'Cancel Action'
          }
        );
        if (proceedAfterFail) {
          if (discardFunction) discardFunction();
          if (
            resetEditorFunction &&
            typeof resetEditorFunction === 'function' &&
            initialContentForReset !== null &&
            itemTypeForPrompt !== 'PDF annotations'
          ) {
            resetEditorFunction(initialContentForReset);
          }
          return true;
        } else {
          return false;
        }
      }
    } else {
      console.warn(
        `[checkUnsavedChanges] Autosave ON, but save method missing for dirty item "${itemName}" (${itemTypeForPrompt}). Blocking action.`
      );
      await message(
        `Cannot ${actionContextDisplay}: Unsaved changes exist for "${itemName}", but an automatic save could not be performed (missing save capability for this item type). Please save or discard changes manually.`,
        { title: 'Autosave Error', type: 'error' }
      );
      return false;
    }
  }
}

export async function loadPdfAnnotationsFromFile(pdfAbsPath) {
  const currentProj = get(project);
  const projectBaseDir = currentProj.baseDirectory;

  if (!pdfAbsPath) {
    setLoadedPdfAnnotations([]);
    project.update((p) => {
      if (p.selectedDocumentPath === pdfAbsPath && p.isDocumentLoading) {
        return { ...p, isDocumentLoading: false, isLoading: false };
      }
      return p;
    });
    return;
  }

  if (!projectBaseDir) {
    console.error(
      '[ProjectService] Cannot load PDF annotations: Project base directory is missing.'
    );
    setPdfAnnotationsLoadFailed(pdfAbsPath, 'Project base directory not found.');
    return;
  }

  let relativePdfPath = pdfAbsPath;
  if (pdfAbsPath.startsWith(projectBaseDir)) {
    relativePdfPath = pdfAbsPath.substring(projectBaseDir.length);
    if (
      relativePdfPath.startsWith(sep) ||
      relativePdfPath.startsWith('/') ||
      relativePdfPath.startsWith('\\')
    ) {
      relativePdfPath = relativePdfPath.substring(1);
    }
  } else {
    console.warn(
      `[ProjectService] pdfAbsPath "${pdfAbsPath}" does not seem to be within projectBaseDir "${projectBaseDir}". Using it as is, but this might be an issue for DB lookup.`
    );
  }
  relativePdfPath = relativePdfPath.replace(/\\/g, '/');

  const filename = await basename(pdfAbsPath);
  project.update((p) => ({ ...p, statusMessage: `Loading annotations for ${filename}...` }));

  try {
    if (
      !currentProj ||
      !currentProj.id ||
      typeof currentProj.id !== 'string' ||
      currentProj.id.trim() === ''
    ) {
      console.error(
        '[ProjectService] loadPdfAnnotationsFromFile: project ID (from $project.id) is missing or invalid.',
        currentProj
      );
      setPdfAnnotationsLoadFailed(pdfAbsPath, 'Project identifier is missing or invalid.'); // Assuming pdfAbsPath is available
      return; // Or throw error
    }
    const projectId = currentProj.id;
    const annotationsJsonString = await invoke('load_pdf_annotations', {
      projectId: projectId,
      originalPdfRelativePathStr: relativePdfPath
    });

    if (annotationsJsonString && typeof annotationsJsonString === 'string') {
      try {
        const parsedAnnotations = JSON.parse(annotationsJsonString);
        setLoadedPdfAnnotations(parsedAnnotations || []);
      } catch (parseError) {
        console.error(
          `[ProjectService] Failed to parse annotations for ${relativePdfPath}:`,
          parseError
        );
        setPdfAnnotationsLoadFailed(
          pdfAbsPath,
          `Failed to parse loaded annotations: ${parseError.message}`
        );
      }
    } else if (annotationsJsonString === null) {
      setLoadedPdfAnnotations([]);
    } else {
      console.warn(
        `[ProjectService] Unexpected response from load_pdf_annotations for ${relativePdfPath}:`,
        annotationsJsonString
      );
      setLoadedPdfAnnotations([]);
    }
  } catch (e) {
    const errorMessage = e.message || String(e);
    console.error(
      `[ProjectService] Error loading annotations for ${relativePdfPath}:`,
      errorMessage
    );
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
  const { tagStore } = await import('$lib/stores/tagStore.svelte.js');

  projectStoreModule.project.set({ ...projectStoreModule.initialState, isLoading: false });
  projectStoreModule.currentProjectGroupsList.set([]);

  transcriptStoreModule.clearTranscriptState();

  tagStore.selectedTag = null;
  tagStore.tagInfo = null;
  tagStore.tagSearchQuery = '';

  // Optionally, inform other parts of the app that the project has been cleared
  // await emit('project-cleared');
  console.log('[ProjectService] Project data store cleared.');
}

export async function renameTableHeader(tablePath, oldHeader, newHeader) {
  if (!tablePath || !oldHeader || !newHeader) {
    throw new Error('Missing required parameters for renaming table header.');
  }

  try {
    await invoke('rename_table_header', {
      tablePathStr: tablePath,
      oldHeader: oldHeader,
      newHeader: newHeader
    });
  } catch (error) {
    const errorMessage = error.message || String(error);
    await message(`Error renaming header: ${errorMessage}`, {
      title: 'Rename Header Error',
      type: 'error'
    });
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
    console.error('[ProjectService] Failed to initialize translation progress listener:', e);
  }
}

export async function requestTranslation(
  transcriptPath,
  modelName,
  targetLanguage,
  sourceLanguage
) {
  const currentProject = get(project);
  const ts = get(transcriptStore);

  if (ts.isTranslating) {
    toggleTranslateModal(true);
    return;
  }

  if (!currentProject.xmlPath) {
    await message('Cannot translate: Project path is not set.', {
      title: 'Translation Error',
      type: 'error'
    });
    return;
  }

  setTranslationStatus(true, null, { status: 'initiating', sourcePath: transcriptPath });

  try {
    const initiatedPayload = await invoke('translate_transcript_command', {
      projectXmlPath: currentProject.xmlPath,
      transcriptPath,
      modelName: modelName,
      targetLanguage: targetLanguage || ts.selectedLanguage || 'en',
      sourceLanguage: sourceLanguage || null
    });

    if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
      throw new Error('Backend did not return a valid job_id for translation.');
    }

    setTranslationStatus(true, initiatedPayload.job_id, { status: 'running' });
  } catch (error) {
    const errorMessage = error.message || String(error);
    setTranslationStatus(false, null, { status: 'error', errorMessage });
    console.error(`[ProjectService] Error during translate_transcript_command invocation:`, error);
  }
}

export async function requestDocumentTranslation(
  documentPath,
  modelName,
  targetLanguage,
  sourceLanguage
) {
  const currentProject = get(project);
  const ts = get(transcriptStore);

  if (ts.isTranslating) {
    toggleTranslateModal(true);
    return;
  }

  if (!currentProject.xmlPath) {
    await message('Cannot translate: Project path is not set.', {
      title: 'Translation Error',
      type: 'error'
    });
    return;
  }

  setTranslationStatus(true, null, { status: 'initiating', sourcePath: documentPath });

  try {
    const initiatedPayload = await invoke('translate_document_command', {
      projectXmlPath: currentProject.xmlPath,
      documentPath: documentPath,
      modelName: modelName,
      targetLanguage: targetLanguage || 'en',
      sourceLanguage: sourceLanguage || null
    });

    if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
      throw new Error('Backend did not return a valid job_id for translation.');
    }

    setTranslationStatus(true, initiatedPayload.job_id, { status: 'running' });
  } catch (error) {
    const errorMessage = error.message || String(error);
    setTranslationStatus(false, null, { status: 'error', errorMessage });
    console.error(`[ProjectService] Error during translate_document_command invocation:`, error);
  }
}

export async function requestStandaloneTranscriptTranslation(
  transcriptPath,
  modelName,
  targetLanguage,
  sourceLanguage
) {
  const currentProject = get(project);
  const ts = get(transcriptStore);

  if (ts.isTranslating) {
    toggleTranslateModal(true);
    return;
  }

  if (!currentProject.xmlPath) {
    await message('Cannot translate: Project path is not set.', {
      title: 'Translation Error',
      type: 'error'
    });
    return;
  }

  setTranslationStatus(true, null, { status: 'initiating', sourcePath: transcriptPath });

  try {
    const initiatedPayload = await invoke('translate_standalone_transcript_command', {
      projectXmlPath: currentProject.xmlPath,
      transcriptPath,
      modelName: modelName,
      targetLanguage: targetLanguage || 'en',
      sourceLanguage: sourceLanguage || null
    });

    if (!initiatedPayload || typeof initiatedPayload.job_id !== 'string') {
      throw new Error('Backend did not return a valid job_id for translation.');
    }

    setTranslationStatus(true, initiatedPayload.job_id, { status: 'running' });
  } catch (error) {
    const errorMessage = error.message || String(error);
    setTranslationStatus(false, null, { status: 'error', errorMessage });
    console.error(
      `[ProjectService] Error during translate_standalone_transcript_command invocation:`,
      error
    );
  }
}

export async function handleCancelTranslationRequest() {
  const ts = get(transcriptStore);
  const jobId = ts.translationJobId;

  if (!jobId || !ts.isTranslating) {
    console.warn('[ProjectService] No active translation job to cancel.');
    return;
  }

  transcriptStore.update((s) => ({ ...s, translationJobStatus: 'cancelling' }));

  try {
    await invoke('cancel_translation_command', { jobId });
  } catch (error) {
    const errorMessage = error.message || String(error);
    transcriptStore.update((s) => ({
      ...s,
      translationJobStatus: 'error',
      translationErrorMessage: `Failed to send cancel request: ${errorMessage}`
    }));
    notificationStore.add(`Cancellation request failed: ${errorMessage}`, 'error');
  }
}

export async function createManualTranscript(mediaPath, segments, settings = null) {
  const currentProj = get(project);
  const projectXmlPath = currentProj.xmlPath;
  if (!projectXmlPath) throw new Error('Project XML path missing.');

  // 1. Calculate transcripts directory
  const mediaDir = await dirname(mediaPath);
  const stemDir = await dirname(mediaDir);
  const transcriptsDir = await join(stemDir, 'transcripts');

  // 2. Determine unique filename
  const mediaFilename = await basename(mediaPath);
  const mediaStem =
    mediaFilename.lastIndexOf('.') > -1
      ? mediaFilename.substring(0, mediaFilename.lastIndexOf('.'))
      : mediaFilename;

  const store = get(transcriptStore);
  // Use existing transcripts from store to avoid many FS calls, assuming store is up to date
  const existingTranscripts = store.selectedMediaFile?.associated_transcripts || [];
  const existingNames = existingTranscripts.map((t) => t.name || t.path.split(/[\\/]/).pop());

  let counter = 1;
  let newFilename = `${mediaStem}_${counter}.json`;
  while (existingNames.includes(newFilename)) {
    counter++;
    newFilename = `${mediaStem}_${counter}.json`;
  }
  const newTranscriptPath = await join(transcriptsDir, newFilename);

  // Save manual settings if provided
  if (settings) {
    saveManualSettingsForTranscript(newTranscriptPath, {
      duration: settings.segmentDuration,
      speakerMode: settings.speakerMode,
      lastUsedSpeakerIndex: -1
    });
  }

  // 3. Generate Lexical JSON
  let fullLexicalTableJsonString = '';
  try {
    const editorForTableAssembly = createHeadlessEditor({
      nodes: ALL_EDITOR_NODES,
      namespace: `manual-table-assembly-${Date.now()}`,
      onError: (e) => console.error('[ManualTableAssembly] Error:', e)
    });

    await editorForTableAssembly.update(() => {
      const root = _getRoot();
      root.clear();
      const tableNode = _createTableNode();

      // Headers
      const headerRow = _createTableRowNode();
      const headers = ['#', 'Timestamp', 'Speaker', 'Text'];
      for (const headerText of headers) {
        const cell = _createTableCellNode({ headerState: 'column' });
        const paragraph = _createParagraphNode();
        paragraph.append(_createTextNode(headerText));
        cell.append(paragraph);
        headerRow.append(cell);
      }
      tableNode.append(headerRow);

      // Data Rows
      for (let i = 0; i < segments.length; i++) {
        const segment = segments[i];
        const dataRow = _createTableRowNode();

        // #
        const cellNum = _createTableCellNode();
        const pNum = _createParagraphNode();
        pNum.append(_createTextNode(String(i + 1)));
        cellNum.append(pNum);
        dataRow.append(cellNum);

        // Timestamp
        const cellTime = _createTableCellNode();
        const pTime = _createParagraphNode();
        const startTime = formatTimestampHtml(segment.start_time || 0);
        const endTime = formatTimestampHtml(segment.end_time || 0);
        pTime.append(_createTextNode(`${startTime} - ${endTime}`));
        cellTime.append(pTime);
        dataRow.append(cellTime);

        // Speaker
        const cellSpeaker = _createTableCellNode();
        const pSpeaker = _createParagraphNode();
        let speakerName = segment.speaker || 'Unknown';
        if (speakerName !== 'Unknown' && !speakerName.endsWith(':')) {
          speakerName += ':';
        }
        pSpeaker.append(_createTextNode(speakerName));
        cellSpeaker.append(pSpeaker);
        dataRow.append(cellSpeaker);

        // Text
        const cellText = _createTableCellNode();
        if (segment.text && typeof segment.text === 'string') {
          let parsedSegmentState;
          try {
            parsedSegmentState = JSON.parse(segment.text);
          } catch (e) {
            const pError = _createParagraphNode();
            pError.append(_createTextNode('[Error: Malformed segment JSON]'));
            cellText.append(pError);
            dataRow.append(cellText);
            tableNode.append(dataRow);
            continue;
          }

          function flattenNodes(nodes) {
            return nodes.flatMap((n) =>
              n.type === 'root' && Array.isArray(n.children) ? flattenNodes(n.children) : [n]
            );
          }

          const rawChildren = parsedSegmentState?.root?.children || [];
          const serializedChildNodes = flattenNodes(rawChildren);

          if (serializedChildNodes.length > 0) {
            const nodesToAppend = [];
            try {
              for (const serializedNode of serializedChildNodes) {
                if (serializedNode.type === 'root') continue;
                const node = _parseSerializedNode(serializedNode);
                if (node) nodesToAppend.push(node);
              }
              if (nodesToAppend.length > 0) {
                cellText.append(...nodesToAppend);
              } else {
                cellText.append(_createParagraphNode());
              }
            } catch (parseErr) {
              console.error('Error parsing nodes for manual segment:', parseErr);
              cellText.append(_createParagraphNode());
            }
          } else {
            cellText.append(_createParagraphNode());
          }
        } else {
          cellText.append(_createParagraphNode());
        }
        dataRow.append(cellText);
        tableNode.append(dataRow);
      }
      root.append(tableNode);
      root.append(_createParagraphNode());
    });

    fullLexicalTableJsonString = JSON.stringify(editorForTableAssembly.getEditorState().toJSON());
  } catch (e) {
    throw new Error(`Failed to generate manual transcript content: ${e.message}`);
  }

  // 4. Save
  await invoke('save_transcript_json', {
    projectXmlPath: projectXmlPath,
    transcriptPath: newTranscriptPath,
    lexicalTableJsonString: fullLexicalTableJsonString,
    language_code: 'original'
  });

  // 5. Refresh and Load
  await refreshProjectFiles(mediaPath);
  await loadTranscriptFile(newTranscriptPath);
}

/**
 * Fetches and groups all project assets for link fields, using backend file_type categorization.
 * Excludes attachments.
 * @param {string} projectId
 * @returns {Promise<Array>}
 */
export async function getProjectAssetsForLink(projectId) {
  try {
    const rawAssets = await invoke('get_project_assets_for_link_command', { projectId });

    const categoryMap = {
      audio: 'Audios',
      video: 'Videos',
      'audio-transcript': 'Audio Transcripts',
      'video-transcript': 'Video Transcripts',
      transcript: 'Transcripts',
      standalone_transcript: 'Transcripts',
      document: 'Documents',
      doc: 'Documents',
      pdf: 'Documents',
      table: 'Tables',
      image: 'Images'
    };

    const assets = rawAssets.map((node) => {
      let category = categoryMap[node.file_type] || 'Other';
      return {
        label: `${category} - ${node.name}`,
        value: node.path,
        category: category
      };
    });

    return assets.sort((a, b) => {
      if (a.category !== b.category) {
        return a.category.localeCompare(b.category);
      }
      return a.label.localeCompare(b.label);
    });
  } catch (e) {
    console.error('Failed to fetch project assets for link:', e);
    return [];
  }
}
