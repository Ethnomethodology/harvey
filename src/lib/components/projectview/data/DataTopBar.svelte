<!-- src/lib/components/projectview/data/DataTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { message } from '@tauri-apps/plugin-dialog';
    import { invoke } from '@tauri-apps/api/core';
    import { project, toggleAutosave, switchTranscriptInDataTab } from '$lib/stores/projectStore.js';
    import { isMediaEditorOpen } from '$lib/stores/mediaEditorStore.js';
    import LayoutSettingsModal from '../modals/LayoutSettingsModal.svelte';
    import ExportModal from '../modals/ExportModal.svelte';
    import { transcriptStore } from '$lib/stores/transcriptStore.js';
    import { exportTranscript } from '$lib/services/configureActions.js';
    import { activeLayout } from '$lib/stores/layoutStore.js';
    import { get, derived } from 'svelte/store';
    import { basename } from '@tauri-apps/api/path';
    import { languageOptions } from '$lib/constants/transcriptionOptions.js';
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { listen } from '@tauri-apps/api/event';
    import LiveTranscribeModelModal from '../modals/LiveTranscribeModelModal.svelte';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import TranslateDocumentModal from '../modals/TranslateDocumentModal.svelte';
    import DocumentExportModal from '../modals/DocumentExportModal.svelte';
    import TableExportModal from '../modals/TableExportModal.svelte';
    import SplitTranscriptModal from '../modals/SplitTranscriptModal.svelte';
    import { requestDocumentTranslation, requestImportedTranscriptTranslation } from '$lib/services/projectService.js';

    const dispatch = createEventDispatcher();
    export let tableViewRef = null;

    // Determine platform-specific modifier key name
    const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const modKeyName = isMac ? 'Cmd' : 'Ctrl';

    let isLiveTranscriptionActive = false;
    let liveTranscriptionError = null;
    let showLiveTranscribeModal = false;
    let showTranslateDocumentModal = false;
    let showDocumentExportModal = false;
    let showTableExportModal = false;
    let isLexicalDocument = false;
    let isImportedTranscript = false;
    let isImage = false;
    let isTable = false;
    let pathForExportModal = '';

    $: {
        const p = $project;
        // console.log("[DataTopBar] Path:", p.selectedDocumentPath, "Type:", p.selectedDocumentType);
        if (p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.json')) {
             isLexicalDocument = true;
             isImportedTranscript = !!p.currentImportedTranscriptPath;
             isImage = false;
             isTable = false;
        } else if (p.currentImportedTranscriptPath) {
             isLexicalDocument = true;
             isImportedTranscript = true;
             isImage = false;
             isTable = false;
        } else if (p.selectedDocumentType === 'images' || p.selectedDocumentType === 'image' || (p.selectedDocumentPath && /\.(jpg|jpeg|png|gif|webp|bmp|tiff)$/i.test(p.selectedDocumentPath))) {
             isImage = true;
             isLexicalDocument = false;
             isImportedTranscript = false;
             isTable = false;
        } else if (p.selectedDocumentType === 'tables' || p.selectedDocumentType === 'table' || (p.selectedDocumentPath && /\.(csv|xlsx)$/i.test(p.selectedDocumentPath))) {
             isTable = true;
             isImage = false;
             isLexicalDocument = false;
             isImportedTranscript = false;
        } else {
            isLexicalDocument = false;
            isImportedTranscript = false;
            isImage = false;
            isTable = false;
        }
        // console.log("[DataTopBar] isTable:", isTable, "isImage:", isImage, "isLexicalDocument:", isLexicalDocument);
    }

    function handleDocumentTranslateConfirm(event) {
        const { documentPath, model, targetLanguage, sourceLanguage } = event.detail;
        if (isImportedTranscript) {
            requestImportedTranscriptTranslation(documentPath, model, targetLanguage, sourceLanguage);
        } else {
            requestDocumentTranslation(documentPath, model, targetLanguage, sourceLanguage);
        }
    }

    $: showTranslateDocumentModal = $transcriptStore.showTranslateModal;

    function getLanguageLabel(langCode) {
		if (!langCode || langCode === 'original') return 'Original';
		const option = languageOptions.find(opt => opt.value === langCode);
		return option ? option.label : langCode; // Fallback to code if not found
	}

    // --- Transcript Dropdown Logic ---
    const activeMediaFile = derived(project, ($project) => {
        if (!$project.selectedMediaNotePath || !$project.files) return null;

        // Helper to search the file tree
        function findFileInTree(nodes, path) {
            for (const node of nodes) {
                if (node.path === path) return node;
                if (node.children) {
                    const found = findFileInTree(node.children, path);
                    if (found) return found;
                }
            }
            return null;
        }
        return findFileInTree($project.files, $project.selectedMediaNotePath);
    });

    const displayedTranscripts = derived(activeMediaFile, ($activeMediaFile) => {
        const transcripts = $activeMediaFile?.associated_transcripts;
        if (!transcripts || transcripts.length === 0) return [];

        const withLabels = transcripts.map(t => {
            const langLabel = getLanguageLabel(t.language_code || 'original');            let fileName = t.name;            if (!fileName && t.path) {
                try {
                    const pathParts = t.path.split(/[\\/]/);
                    fileName = pathParts[pathParts.length - 1];
                    if (fileName.toLowerCase().endsWith('.json')) {
                        fileName = fileName.substring(0, fileName.length - 5);
                    }
                } catch (e) {
                    console.error("Error extracting filename from path:", e);
                    fileName = '';
                }
            }
            const fileNamePart = fileName ? ` (${fileName})` : '';
            const displayLabel = `${langLabel}${fileNamePart}`;
            return { ...t, displayLabel };
        });

        return withLabels.sort((a, b) => a.displayLabel.localeCompare(b.displayLabel));
    });

    // --- Theme Icons ---
	const SUN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" /></svg>`;
	const MOON_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" /></svg>`;
	const SYSTEM_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25m18 0A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25m18 0V12a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 12V5.25" /></svg>`;
	$: themeIconHtml = $themePreference === 'light' ? SUN_ICON
					 : $themePreference === 'dark' ? MOON_ICON
					 : SYSTEM_ICON;
    $: nextThemeName = $themePreference === 'light' ? 'Dark'
                     : $themePreference === 'dark' ? 'System'
                     : 'Light';
    $: themeTitle = `Switch to ${nextThemeName} Mode`;

    const SAVE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M10.125 2.25h-4.5c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125v-9M10.125 2.25h.375a9 9 0 0 1 9 9v.375M10.125 2.25A3.375 3.375 0 0 1 13.5 5.625v1.5c0 .621.504 1.125 1.125 1.125h1.5a3.375 3.375 0 0 1 3.375 3.375M9 15l2.25 2.25L15 12" /></svg>`;

    let autosaveEnabled = true;
    let isDocumentDirty = false;
    let isImportedTranscriptDirty = false;
    let isMediaNoteTranscriptDirty = false; // New state for media note
    let isPdfAnnotationsDirty = false;
    let activeDocumentEditorRef = null;
    let activeImportedTranscriptEditorRef = null;
    let activeMediaNoteEditorRef = null; // New ref
    let isAnythingDirty = false;
    let canSave = false;
    let showDirtyIndicator = false;
    let isLayoutSettingsModalOpen = false;
    let isExportModalOpen = false;
    let currentActivePath;

    let displayTitle = '';
  
    $: { // This is the existing reactive block for autosave related logic
        const p = $project;
        autosaveEnabled = p.autosaveEnabled;
        isDocumentDirty = p.isDocumentDirty || p.isDocumentMetadataDirty; // Combine content and metadata dirty for documents
        isImportedTranscriptDirty = p.isImportedTranscriptDirty;
        isMediaNoteTranscriptDirty = p.isMediaNoteTranscriptDirty; // Read from store
        isPdfAnnotationsDirty = p.isPdfAnnotationsDirty;

        activeDocumentEditorRef = p.activeDocumentEditorRef;
        activeImportedTranscriptEditorRef = p.activeImportedTranscriptEditorRef;
        activeMediaNoteEditorRef = p.activeMediaNoteEditorRef; // Read from store

        isAnythingDirty = isDocumentDirty || isImportedTranscriptDirty || isMediaNoteTranscriptDirty || isPdfAnnotationsDirty;
        canSave = !autosaveEnabled && isAnythingDirty;
        showDirtyIndicator = isAnythingDirty;
    }

    // New reactive block for displayTitle
    $: {
        if ($project && $project.name) {
            let currentFileName = null;
            let activePath = $project.selectedDocumentPath ||
                             $project.selectedMediaNotePath ||
                             $project.currentImportedTranscriptPath ||
                             $project.selectedTablePath ||
                             $project.selectedImagePath;

            if (activePath) {
                basename(activePath).then(name => {
                    currentFileName = name;
                    if (currentFileName) {
                        displayTitle = `${$project.name} : ${currentFileName}`;
                    } else {
                        displayTitle = $project.name;
                    }
                }).catch(err => {
                    console.error("Error getting basename for top bar:", err);
                    displayTitle = $project.name; // Fallback
                });
            } else {
                displayTitle = $project.name;
            }
        } else {
            displayTitle = 'Harvey'; // Default if project or name is not available
        }
    }

    // Stop live transcription if the user switches to another file
    $: {
        const newActivePath = $project.selectedDocumentPath ||
                             $project.selectedMediaNotePath ||
                             $project.currentImportedTranscriptPath ||
                             $project.selectedTablePath ||
                             $project.selectedImagePath;

        if (newActivePath !== currentActivePath) {
            if (isLiveTranscriptionActive) {
                invoke('stop_live_transcription').then(stopped => {
                    if (stopped) {
                        isLiveTranscriptionActive = false;
                    }
                }).catch(err => {
                    console.error("Failed to stop live transcription on file switch:", err);
                });
            }
            currentActivePath = newActivePath;
        }
    }
  
    function openLayoutSettingsModal() {
		isLayoutSettingsModalOpen = true;
	}

	function handleLayoutSelected(event) {
		const newLayoutKey = event.detail;
		activeLayout.setLayout(newLayoutKey);
		// Modal closes itself on selection
	}

    const LAYOUT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-wtf" viewBox="0 0 16 16"><path d="M5 1v8H1V1zM1 0a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1V1a1 1 0 0 0-1-1zm13 2v5H9V2zM9 1a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1zM5 13v2H3v-2zm-2-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1zm12-1v2H9v-2zm-6-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1z"/></svg>`;

    async function handleManualSave() {
        const projState = get(project);
        const currentCanSave = !projState.autosaveEnabled && 
                               (projState.isDocumentDirty || projState.isDocumentMetadataDirty || projState.isImportedTranscriptDirty || projState.isImportedTranscriptMetadataDirty || projState.isMediaNoteTranscriptDirty || projState.isPdfAnnotationsDirty);
        
        if (!currentCanSave) { 
            console.warn("[DataTopBar] Manual save clicked but conditions not met."); 
            return; 
        }
        console.log("[DataTopBar] Manual save proceeding...");

        if ((projState.isDocumentDirty || projState.isDocumentMetadataDirty) && projState.selectedDocumentPath && projState.activeDocumentEditorRef?.ref && typeof projState.activeDocumentEditorRef.ref.save === 'function') {
            console.log("[DataTopBar] Manual save triggered for DOCUMENT via editor ref:", projState.selectedDocumentPath);
            try { await projState.activeDocumentEditorRef.ref.save(); console.log("[DataTopBar] Document manual save successful."); } 
            catch (error) { console.error("[DataTopBar] Document manual save via editor ref failed:", error); }
        } else if (projState.isPdfAnnotationsDirty && projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            console.log("[DataTopBar] Manual save triggered for PDF ANNOTATIONS:", projState.selectedDocumentPath);
            try { 
                // PDF Annotations save might be handled differently, e.g. a direct service call if no 'ref.save'
                // Assuming there's a service for this like `saveCurrentPdfAnnotations`
                const { saveCurrentPdfAnnotations } = await import('$lib/services/projectService.js');
                await saveCurrentPdfAnnotations();
                console.log("[DataTopBar] PDF Annotations manual save successful."); 
            } catch (error) { console.error("[DataTopBar] PDF Annotations manual save failed:", error); }
        } else if ((projState.isImportedTranscriptDirty || projState.isImportedTranscriptMetadataDirty) && projState.currentImportedTranscriptPath && projState.activeImportedTranscriptEditorRef?.ref && typeof projState.activeImportedTranscriptEditorRef.ref.save === 'function') {
             console.log("[DataTopBar] Manual save triggered for IMPORTED TRANSCRIPT via editor ref:", projState.currentImportedTranscriptPath);
            try { await projState.activeImportedTranscriptEditorRef.ref.save(); console.log("[DataTopBar] Imported Transcript manual save successful."); } 
            catch (error) { console.error("[DataTopBar] Imported Transcript manual save via editor ref failed:", error); }
        } else if (projState.isMediaNoteTranscriptDirty && projState.selectedMediaNotePath && projState.activeMediaNoteEditorRef?.ref && typeof projState.activeMediaNoteEditorRef.ref.save === 'function') {
            console.log("[DataTopBar] Manual save triggered for MEDIA NOTE TRANSCRIPT via editor ref:", projState.selectedMediaNotePath);
            try { await projState.activeMediaNoteEditorRef.ref.save(); console.log("[DataTopBar] Media Note Transcript manual save successful."); }
            catch (error) { console.error("[DataTopBar] Media Note Transcript manual save via editor ref failed:", error); }
        } else if (projState.isDocumentDirty && projState.selectedTablePath && tableViewRef) {
            console.log("[DataTopBar] Manual save triggered for TABLE via editor ref:", projState.selectedTablePath);
            try { await tableViewRef.save(); console.log("[DataTopBar] Table manual save successful."); }
            catch (error) { console.error("[DataTopBar] Table manual save via editor ref failed:", error); }
        } else { 
            console.warn("[DataTopBar] Manual save triggered but no specific dirty item found with an active editor ref capable of saving, or PDF annotations were not handled by a direct save call."); 
        }
    }
  
    function handleToggleChange() {
        toggleAutosave();
    }
  
    let autosaveTimeout;
    $: {
        const p = $project;
        let shouldAutosave = false;
        let activeEditorRefToSave = null;
        let saveAction = null;

        if (p.autosaveEnabled) {
            if ((p.isDocumentDirty || p.isDocumentMetadataDirty) && p.selectedDocumentPath && p.activeDocumentEditorRef?.ref) {
                shouldAutosave = true;
                activeEditorRefToSave = p.activeDocumentEditorRef.ref;
                saveAction = 'document';
                console.log(`[DataTopBar Autosave Watch] Document ${p.selectedDocumentPath} is dirty.`);
            } else if (p.isPdfAnnotationsDirty && p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
                shouldAutosave = true;
                // No direct editorRef.save() for PDF annotations usually, service call is direct
                saveAction = 'pdfAnnotations';
                console.log(`[DataTopBar Autosave Watch] PDF Annotations for ${p.selectedDocumentPath} are dirty.`);
            } else if ((p.isImportedTranscriptDirty || p.isImportedTranscriptMetadataDirty) && p.currentImportedTranscriptPath && p.activeImportedTranscriptEditorRef?.ref) {
                shouldAutosave = true;
                activeEditorRefToSave = p.activeImportedTranscriptEditorRef.ref;
                saveAction = 'importedTranscript';
                 console.log(`[DataTopBar Autosave Watch] Imported Transcript ${p.currentImportedTranscriptPath} is dirty.`);
            } else if (p.isMediaNoteTranscriptDirty && p.selectedMediaNotePath && p.activeMediaNoteEditorRef?.ref) {
                shouldAutosave = true;
                activeEditorRefToSave = p.activeMediaNoteEditorRef.ref;
                saveAction = 'mediaNoteTranscript';
                console.log(`[DataTopBar Autosave Watch] Media Note Transcript for ${p.selectedMediaNotePath} is dirty.`);
            } else if (p.isDocumentDirty && p.selectedTablePath && tableViewRef) {
                shouldAutosave = true;
                activeEditorRefToSave = tableViewRef;
                saveAction = 'table';
                console.log(`[DataTopBar Autosave Watch] Table for ${p.selectedTablePath} is dirty.`);
            } else {
                // console.log(`[DataTopBar Autosave Watch] Conditions not met.`);
            }
        }


        clearTimeout(autosaveTimeout);
        if (shouldAutosave) {
            autosaveTimeout = setTimeout(async () => {
                console.log("[DataTopBar] Autosave timer fired. Attempting save...");
                const currentProjState = get(project); // Re-fetch current state
                let editorStillActiveAndDirty = false;

                if (saveAction === 'document' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeDocumentEditorRef?.ref === activeEditorRefToSave && (currentProjState.isDocumentDirty || currentProjState.isDocumentMetadataDirty);
                } else if (saveAction === 'pdfAnnotations') {
                    editorStillActiveAndDirty = currentProjState.selectedDocumentPath?.toLowerCase().endsWith('.pdf') && currentProjState.isPdfAnnotationsDirty;
                } else if (saveAction === 'importedTranscript' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeImportedTranscriptEditorRef?.ref === activeEditorRefToSave && (currentProjState.isImportedTranscriptDirty || currentProjState.isImportedTranscriptMetadataDirty);
                } else if (saveAction === 'mediaNoteTranscript' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeMediaNoteEditorRef?.ref === activeEditorRefToSave && currentProjState.isMediaNoteTranscriptDirty;
                } else if (saveAction === 'table' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = tableViewRef === activeEditorRefToSave && currentProjState.isDocumentDirty;
                }

                if (editorStillActiveAndDirty) {
                     console.log(`[DataTopBar] Autosaving (Action: ${saveAction})...`);
                     try { 
                        if (saveAction === 'pdfAnnotations') {
                            const { saveCurrentPdfAnnotations } = await import('$lib/services/projectService.js');
                            await saveCurrentPdfAnnotations();
                        } else if (activeEditorRefToSave && typeof activeEditorRefToSave.save === 'function') {
                            await activeEditorRefToSave.save(); 
                        } else {
                            console.warn(`[DataTopBar Autosave] No valid save method for action ${saveAction}`);
                        }
                        console.log(`[DataTopBar] Autosave successful for ${saveAction}.`); 
                    }
                     catch (error) { console.error(`[DataTopBar] Autosave failed for ${saveAction}:`, error); }
                } else { 
                    console.log(`[DataTopBar] Autosave timer fired, but conditions no longer met (Action: ${saveAction}, StillDirty: ${editorStillActiveAndDirty}). Save skipped.`); 
                }
            }, 3000); 
        }
    }
    async function handleExportConfirm(event) {
        const { filePath, format, layoutChoice, excludeSpeakerNames } = event.detail;
        const activeTranscriptPath = pathForExportModal;

        if (!activeTranscriptPath) {
            message("No active transcript selected to export.", { title: "Export Failed", type: "error" });
            return;
        }

        try {
            const jsonContent = await invoke('load_transcript_json', { transcriptPath: activeTranscriptPath });
            if (!jsonContent) {
                throw new Error("Transcript file is empty or could not be read.");
            }

            const transcriptData = JSON.parse(jsonContent);
            const segmentsToExport = [];

            const getTextFromLexicalNode = (node) => {
                if (!node) return '';
                if (node.type === 'text' || node.type === 'extended-text') {
                    return node.text || '';
                }
                let text = '';
                if (node.children && Array.isArray(node.children)) {
                    for (const child of node.children) {
                        text += getTextFromLexicalNode(child);
                    }
                }
                return text;
            };

            const parseTimestamp = (tsStr) => {
                if (!tsStr) return 0;
                // Handles HH:MM:SS.mmm, MM:SS.mmm, SS.mmm
                const parts = tsStr.split(':').reverse(); // [SS.mmm, MM, HH]
                let seconds = 0;
                if (parts[0]) seconds += parseFloat(parts[0]);
                if (parts[1]) seconds += parseInt(parts[1], 10) * 60;
                if (parts[2]) seconds += parseInt(parts[2], 10) * 3600;
                return isNaN(seconds) ? 0 : seconds;
            };

            const tableNode = transcriptData?.root?.children?.find(c => c.type === 'table');

            if (tableNode && tableNode.children) {
                // Skip header row (i=0)
                for (let i = 1; i < tableNode.children.length; i++) {
                    const rowNode = tableNode.children[i];
                    if (rowNode.type !== 'tablerow' || !rowNode.children || rowNode.children.length < 4) continue;

                    const cells = rowNode.children;
                    const timestampText = getTextFromLexicalNode(cells[1]);
                    const [startStr, endStr] = timestampText.split(' - ').map(s => s.trim());
                    
                    const startTime = parseTimestamp(startStr);
                    const endTime = parseTimestamp(endStr);
                    const speaker = getTextFromLexicalNode(cells[2]);
                    const cellContentNode = cells[3];

                    // Re-wrap the cell's content into a valid Lexical root structure for export
                    const textJson = JSON.stringify({
                        root: {
                            children: cellContentNode.children || [],
                            direction: 'ltr',
                            format: '',
                            indent: 0,
                            type: 'root',
                            version: 1
                        }
                    });

                    segmentsToExport.push({
                        start_time: startTime,
                        end_time: endTime,
                        speaker: speaker,
                        text: textJson,
                    });
                }
            }

            if (segmentsToExport.length === 0) {
                message("No transcript data available to export.", { title: "Export Failed", type: "error" });
                return;
            }

            await exportTranscript(filePath, format, segmentsToExport, activeTranscriptPath, layoutChoice, excludeSpeakerNames);
            message(`Transcript successfully exported to ${filePath}`, { title: "Export Successful", type: "info" });

        } catch (error) {
            console.error("Export failed:", error);
            message(`Failed to export transcript: ${error?.message || error}`, { title: "Export Failed", type: "error" });
        }
    }

    let unlisten = null;

    onMount(async () => {
        unlisten = await listen('live_transcription_result', (event) => {
            const { text, is_final, start_time, end_time } = event.payload;
            const p = get(project);
            let editorRef = null;

            // Find the correct active editor
            if (p.activeDocumentEditorRef) {
                editorRef = p.activeDocumentEditorRef;
            } else if (p.activeImportedTranscriptEditorRef) {
                editorRef = p.activeImportedTranscriptEditorRef;
            }
            // Removed activeMediaNoteEditorRef from live transcription updates

            if (editorRef?.ref?.updateLiveTranscriptionText) {
                editorRef.ref.updateLiveTranscriptionText(text, is_final, start_time, end_time);
            }
        });
    });

    onDestroy(async () => {
        if (isLiveTranscriptionActive) {
            await invoke('stop_live_transcription');
        }
        if (unlisten) {
            unlisten();
        }
    });

    async function toggleLiveTranscription() {
        if (isLiveTranscriptionActive) {
            try {
                const stopped = await invoke('stop_live_transcription');
                if (stopped) {
                    isLiveTranscriptionActive = false;
                }
            } catch (error) {
                message(`Failed to stop live transcription: ${error}`, { title: 'Error', type: 'error' });
            }
        } else {
            showLiveTranscribeModal = true;
        }
    }

    async function handleLiveTranscribe(event) {
        const { model, language, saveAudio, family } = event.detail;
        try {
            const projectState = get(project);
            const activePath = projectState.selectedDocumentPath || projectState.selectedMediaNotePath;
            if (!activePath) {
                message('No active document to transcribe into.', { title: 'Error', type: 'error' });
                return;
            }
            if (!projectState.id || !projectState.baseDirectory) {
                message('Project details not available. Cannot start transcription.', { title: 'Error', type: 'error' });
                return;
            }

            liveTranscriptionError = null;
            const started = await invoke('start_live_transcription', {
                modelName: model,
                language: language,
                saveAudio: saveAudio,
                activeDocumentPath: activePath,
                projectUuid: projectState.id,
                projectBaseDir: projectState.baseDirectory,
                engine: family
            });
            if (started) {
                isLiveTranscriptionActive = true;
            }
        } catch (error) {
            isLiveTranscriptionActive = false;
            liveTranscriptionError = error;
            message(`Failed to start live transcription: ${error}`, { title: 'Error', type: 'error' });
        }
    }

    $: liveTranscriptionStatus = isLiveTranscriptionActive
        ? 'active'
        : liveTranscriptionError
        ? 'error'
        : 'default';
  </script>
  
  <div
    class="grid grid-cols-3 items-center px-1 h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800 relative z-50"
    on:requestTranscriptionTabWithMediaAndDialog
  >
    <!-- Drag Handle Background -->
    <div class="absolute inset-0 z-0" data-tauri-drag-region></div>

    <div class="flex items-center space-x-1.5 min-w-0 z-10"> <!-- Left Column -->
        <div class="h-10 flex items-center justify-center flex-shrink-0">
            <button title="Import" aria-label="Import" class="ui-button-import hover-scale-effect ml-1 mr-1" on:click={(e) => dispatch('requestImport', e)}>
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
            </button>
        </div>
        
        <span class="font-semibold text-lg text-gray-700 dark:text-gray-200 truncate" title={displayTitle}>{displayTitle}</span>
        {#if $activeMediaFile}
        <button class="ui-button-icon flex items-center ml-2 space-x-0.5 hover-scale-effect"
            on:click={() => dispatch('requestTranscriptionTabWithMediaAndDialog', { mediaPath: $activeMediaFile.path })}
            title="Transcribe"
        >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 8.25h9m-9 3H12m-9.75 1.51c0 1.6 1.123 2.994 2.707 3.227 1.129.166 2.27.293 3.423.379.35.026.67.21.865.501L12 21l2.755-4.133a1.14 1.14 0 0 1 .865-.501 48.172 48.172 0 0 0 3.423-.379c1.584-.233 2.707-1.626 2.707-3.228V6.741c0-1.602-1.123-2.995-2.707-3.228A48.394 48.394 0 0 0 12 3c-2.392 0-4.744.175-7.043.513C3.373 3.746 2.25 5.14 2.25 6.741v6.018Z" />
            </svg>
            <span class="text-xs">Transcribe</span>
        </button>
        {/if}
        {#if $project.activeDocumentEditorRef}
        <button class="ui-button-icon flex items-center ml-2 space-x-0.5 hover-scale-effect" on:click={toggleLiveTranscription} title="Live Transcribe">
            {#if isLiveTranscriptionActive}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-mic-fill" class:blinking-red-text={isLiveTranscriptionActive} viewBox="0 0 16 16">
                <path d="M5 3a3 3 0 0 1 6 0v5a3 3 0 0 1-6 0z"/>
                <path d="M3.5 6.5A.5.5 0 0 1 4 7v1a4 4 0 0 0 8 0V7a.5.5 0 0 1 1 0v1a5 5 0 0 1-4.5 4.975V15h3a.5.5 0 0 1 0 1h-7a.5.5 0 0 1 0-1h3v-2.025A5 5 0 0 1 3 8V7a.5.5 0 0 1 .5-.5"/>
            </svg>
            {:else}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-mic" viewBox="0 0 16 16">
                <path d="M3.5 6.5A.5.5 0 0 1 4 7v1a4 4 0 0 0 8 0V7a.5.5 0 0 1 1 0v1a5 5 0 0 1-4.5 4.975V15h3a.5.5 0 0 1 0 1h-7a.5.5 0 0 1 0-1h3v-2.025A5 5 0 0 1 3 8V7a.5.5 0 0 1 .5-.5"/>
                <path d="M10 8a2 2 0 1 1-4 0V3a2 2 0 1 1 4 0zM8 0a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V3a3 3 0 0 0-3-3"/>
            </svg>
            {/if}
            <span class="text-xs">Live Transcribe</span>
        </button>
        {/if}
    </div>

    <div class="flex justify-center min-w-0 z-10"> <!-- Middle Column -->
    </div>
  
    <div class="flex items-center justify-end space-x-2 flex-shrink-0 z-10"> <!-- Right Column -->
        <!-- Transcript Dropdown -->
        {#if $activeMediaFile}
            <Dropdown
                containerClasses="w-48"
                options={$displayedTranscripts.map(t => ({ value: t.path, label: t.displayLabel }))}
                value={$project.activeTranscriptPathInDataTab}
                on:change={(e) => switchTranscriptInDataTab(e.detail)}
                placeholder="Select Transcript"
            />
            <button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{() => { pathForExportModal = $project.activeTranscriptPathInDataTab; isExportModalOpen = true; }}" title="Export Transcript" >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"> <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" /> </svg>
                <span class="text-xs">Export</span>
            </button>
        {/if}
        {#if isLexicalDocument}
            <button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{() => showTranslateDocumentModal = true}" title="Translate Document" >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 21l5.25-11.25L21 21m-9-3h7.5M3 5.621a48.474 48.474 0 016-.371m0 0c1.12 0 2.233.038 3.334.114M9 5.25V3m3.334 2.364C11.176 10.658 7.69 15.08 3 17.502m9.334-12.138c.896.061 1.785.147 2.666.257m-4.589 8.495a18.023 18.023 0 01-3.827-5.802" />
                </svg>
                <span class="text-xs">Translate</span>
            </button>
            <button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" 
                on:click="{() => {
                    if (isImportedTranscript) {
                        pathForExportModal = $project.currentImportedTranscriptPath;
                        isExportModalOpen = true;
                    } else {
                        showDocumentExportModal = true;
                    }
                }}" 
                title={isImportedTranscript ? "Export Transcript" : "Export Document"} 
            >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"> <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" /> </svg>
                <span class="text-xs">Export</span>
            </button>
        {/if}
        {#if isTable}
            <button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{() => showTableExportModal = true}" title="Export Table" >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"> <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" /> </svg>
                <span class="text-xs">Export</span>
            </button>
        {/if}
        {#if isImage}
            <button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{() => dispatch('requestImageExport')}" title="Export Image" >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"> <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" /> </svg>
                <span class="text-xs">Export</span>
            </button>
        {/if}
        {#if isImportedTranscript || ($activeMediaFile && $displayedTranscripts.length > 1)}
            <button 
                class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" 
                on:click="{() => project.update(p => ({ ...p, showSplitTranscriptModal: true, pendingSplitOrientation: 'horizontal' }))}" 
                title="Split Transcript (Horizontal)" 
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-split" viewBox="0 0 16 16">
                    <path d="M0 3a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm8.5-1v12H14a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1zm-1 0H2a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h5.5z"/>
                </svg>
            </button>
            <button 
                class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" 
                on:click="{() => project.update(p => ({ ...p, showSplitTranscriptModal: true, pendingSplitOrientation: 'vertical' }))}" 
                title="Split Transcript (Vertical)" 
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-split" viewBox="0 0 16 16" style="transform: rotate(90deg);">
                    <path d="M0 3a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm8.5-1v12H14a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1zm-1 0H2a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h5.5z"/>
                </svg>
            </button>
        {/if}
        <button
            class="ui-button-icon flex items-center h-7 px-2 py-0.5 rounded text-xs hover-scale-effect"
            title={canSave ? `Save Changes (${modKeyName}+S)` : (autosaveEnabled ? "Autosave is ON" : "No changes to save")}
            disabled={!canSave}
            on:click={handleManualSave}
        >
            {@html SAVE_ICON}
            <span class="ml-1 hidden sm:inline">Save</span>
            {#if showDirtyIndicator}<span class="text-orange-500 ml-0.5">*</span>{/if}
        </button>
  
        <div class="flex items-center space-x-1.5" title={autosaveEnabled ? 'Autosave is ON' : 'Autosave is OFF'}> <!-- Reduced space-x-2 to space-x-1.5 -->
          <span class="text-xs font-medium text-gray-700 dark:text-gray-300">
            Autosave
          </span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer" 
                bind:checked={autosaveEnabled}
                on:change={handleToggleChange}
              >
              <div class="w-11 h-5 bg-gray-200 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-[24px] peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all dark:border-gray-700 peer-checked:bg-gray-400 dark:peer-checked:bg-gray-500"></div>
              <span
                class="absolute top-0 bottom-0 flex items-center text-xs font-medium text-gray-700 dark:text-gray-300 pointer-events-none"
                class:left-1={autosaveEnabled}
                class:right-1={!autosaveEnabled}
              >
                {autosaveEnabled ? 'On' : 'Off'}
              </span>
            </label>
        </div>
  
         <!-- <button
            id="theme-toggle-button"
            on:click={cycleThemePreference}
            class="ui-button-icon h-8 w-8 flex items-center justify-center p-1"
            title={themeTitle}
        >
            {@html themeIconHtml}
         </button> -->
    <div class="flex-shrink-0">
        {#if $isMediaEditorOpen || isLexicalDocument || $activeMediaFile}
        <button
            on:click="{() => openLayoutSettingsModal()}"
            class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
            title="Change Transcript View Layout"
        >
            {@html LAYOUT_ICON_SVG}
        </button>
        {/if}
				 <button on:click="{() => cycleThemePreference()}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors" title="{themeTitle}"> <!-- Adjusted padding --> <!-- Adjusted padding -->
			{@html themeIconHtml}
		 </button>
	</div>
    </div>
  </div>
  
  <style lang="postcss">
    .ui-button-icon-no-border {
		@apply inline-flex items-center justify-center p-1.5 text-sm font-medium rounded-md text-gray-700 dark:text-white bg-transparent hover:bg-blue-100 dark:hover:bg-blue-700 disabled:hover:bg-transparent dark:disabled:hover:!bg-transparent focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors;
	}

    .blinking-red-text {
        animation: blink-text 1s infinite;
    }

    @keyframes blink-text {
        0% { color: #f87171; }
        50% { color: #ef4444; }
        100% { color: #f87171; }
    }



    .ui-button-import {
        @apply w-8 h-8 rounded-full flex items-center justify-center transition-colors;
        @apply bg-transparent;
        @apply text-gray-700 dark:text-white;
        @apply border border-gray-300 dark:border-gray-700;
        @apply hover:bg-blue-100 dark:hover:bg-blue-700;
        @apply hover:text-blue-500 dark:hover:text-blue-400;
        @apply hover:border-blue-500 dark:hover:border-blue-500;
        @apply focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500;
        @apply disabled:hover:bg-transparent disabled:hover:border-gray-300 dark:disabled:hover:border-gray-700 dark:disabled:hover:!bg-transparent;
    }
  
    .hover-scale-effect {
        /* @apply transition-transform hover:scale-105 disabled:hover:scale-100; */
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>


			<LayoutSettingsModal
				bind:showModal="{isLayoutSettingsModalOpen}"
				currentLayoutKey="{$activeLayout}"
				on:selectLayout="{handleLayoutSelected}"
				on:close={() => isLayoutSettingsModalOpen = false}
				hideWaveformOptions={true}
			/>
<ExportModal
    bind:showModal={isExportModalOpen}
    transcriptPath={pathForExportModal}
    on:confirm={handleExportConfirm}
    on:close={() => isExportModalOpen = false}
/>

<LiveTranscribeModelModal
    bind:showModal={showLiveTranscribeModal}
    on:confirm={handleLiveTranscribe}
    on:close={() => showLiveTranscribeModal = false}
/>

<TranslateDocumentModal
    bind:showModal={showTranslateDocumentModal}
    activeDocumentPath={isImportedTranscript ? $project.currentImportedTranscriptPath : $project.selectedDocumentPath}
    on:confirm={handleDocumentTranslateConfirm}
    on:closeAndReset={() => showTranslateDocumentModal = false}
/>

<DocumentExportModal 
    bind:showModal={showDocumentExportModal} 
    documentPath={isImportedTranscript ? $project.currentImportedTranscriptPath : $project.selectedDocumentPath}
    on:confirm={() => message('Document exported successfully.', { title: 'Success', type: 'info' })}
    on:close={() => showDocumentExportModal = false}
/>

<TableExportModal
    bind:showModal={showTableExportModal}
    tablePath={isTable ? $project.selectedDocumentPath : null}
    on:confirm={() => message('Table exported successfully.', { title: 'Success', type: 'info' })}
    on:close={() => showTableExportModal = false}
/>

<SplitTranscriptModal />
