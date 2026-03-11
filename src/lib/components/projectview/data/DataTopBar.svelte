<!-- src/lib/components/projectview/data/DataTopBar.svelte -->
<script>
    import { Button, Select } from 'flowbite-svelte';
    import { MessageSquareText, Share, Languages, ImageDown, Mic, LayoutDashboard, SquareSplitHorizontal, SquareSplitVertical, Sun, Moon, Monitor } from 'lucide-svelte';
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { message } from '@tauri-apps/plugin-dialog';
    import { invoke } from '@tauri-apps/api/core';
    import { project, switchTranscriptInDataTab } from '$lib/stores/projectStore.js';
    import { isMediaEditorOpen } from '$lib/stores/mediaEditorStore.js';
    import LayoutSettingsModal from '../modals/LayoutSettingsModal.svelte';
    import ExportModal from '../modals/ExportModal.svelte';
    import { transcriptStore, toggleTranslateModal } from "$lib/stores/transcriptStore.js";
    import { configStatus } from '$lib/stores/configStatusStore.js';
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
    export let dataViewRef = null;

    // Determine platform-specific modifier key name
    const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    const modKeyName = isMac ? 'Cmd' : 'Ctrl';

    let isLiveTranscriptionActive = false;
    let liveTranscriptionError = null;
    let showLiveTranscribeModal = false;
    let isAddingTimestamps = false;
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

        let targetCode = langCode;
        if (langCode.includes('-')) {
            targetCode = langCode.split('-').pop(); // e.g., 'en-hi' -> 'hi'
        }

		const option = languageOptions.find(opt => opt.value === targetCode);
		return option ? option.label : targetCode; // Fallback to code if not found
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
    $: nextThemeName = $themePreference === 'light' ? 'Dark'
                     : $themePreference === 'dark' ? 'System'
                     : 'Light';
    $: themeTitle = `Switch to ${nextThemeName} Mode`;

    let isDocumentDirty = false;
    let isImportedTranscriptDirty = false;
    let isMediaNoteTranscriptDirty = false; // New state for media note
    let isPdfAnnotationsDirty = false;
    let activeDocumentEditorRef = null;
    let activeImportedTranscriptEditorRef = null;
    let activeMediaNoteEditorRef = null; // New ref
    let isAnythingDirty = false;
    let showDirtyIndicator = false;
    let isLayoutSettingsModalOpen = false;
    let isExportModalOpen = false;
    let currentActivePath;

    let displayTitle = '';
  
    $: { // This is the existing reactive block for autosave related logic
        const p = $project;
        isDocumentDirty = p.isDocumentDirty || p.isDocumentMetadataDirty; // Combine content and metadata dirty for documents
        isImportedTranscriptDirty = p.isImportedTranscriptDirty;
        isMediaNoteTranscriptDirty = p.isMediaNoteTranscriptDirty; // Read from store
        isPdfAnnotationsDirty = p.isPdfAnnotationsDirty;

        activeDocumentEditorRef = p.activeDocumentEditorRef;
        activeImportedTranscriptEditorRef = p.activeImportedTranscriptEditorRef;
        activeMediaNoteEditorRef = p.activeMediaNoteEditorRef; // Read from store

        isAnythingDirty = isDocumentDirty || isImportedTranscriptDirty || isMediaNoteTranscriptDirty || isPdfAnnotationsDirty;
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
        // Autosave handles everything.
    }
  

  
    let autosaveTimeout;
    $: {
        const p = $project;
        let shouldAutosave = false;
        let activeEditorRefToSave = null;
        let saveAction = null;

        if (true) {
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
                editorRef.ref.updateLiveTranscriptionText(text, is_final, start_time, end_time, isAddingTimestamps);
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
        const { model, language, saveAudio, family, addTimestamps } = event.detail;
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
            isAddingTimestamps = addTimestamps;
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
        <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => dispatch('requestTranscriptionTabWithMediaAndDialog', { mediaPath: $activeMediaFile.path })} title="Transcribe">
            <MessageSquareText class="w-3.5 h-3.5" />
            <span>Transcribe</span>
        </Button>

        <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => dispatch('requestTranslationTabWithMediaAndDialog', { mediaPath: $activeMediaFile.path, transcriptPath: $project.activeTranscriptPathInDataTab })} title="Translate Transcript">
            <Languages class="w-3.5 h-3.5" />
            <span>Translate</span>
        </Button>
        {/if}
        {#if $project.activeDocumentEditorRef}
        <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={toggleLiveTranscription} title="Live Transcribe">
            <Mic class="w-3.5 h-3.5 {isLiveTranscriptionActive ? 'text-red-500 animate-pulse' : ''}" />
            <span>Live Transcribe</span>
        </Button>
        {/if}
        {#if isLexicalDocument}
            <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => toggleTranslateModal(true)} title="Translate Document">
                <Languages class="w-3.5 h-3.5" />
                <span>Translate</span>
            </Button>
        {/if}
    </div>

    <div class="flex justify-center min-w-0 z-10"> <!-- Middle Column -->
    </div>

    <div class="flex items-center justify-end space-x-2 flex-shrink-0 z-10"> <!-- Right Column -->
        <!-- Transcript Dropdown -->
        {#if $activeMediaFile}
            <Dropdown
                containerClasses="w-72"
                options={$displayedTranscripts.map(t => ({ value: t.path, label: t.displayLabel }))}
                value={$project.activeTranscriptPathInDataTab}
                on:change={(e) => switchTranscriptInDataTab(e.detail)}
                placeholder="Select Transcript"
            />
            <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => { pathForExportModal = $project.activeTranscriptPathInDataTab; isExportModalOpen = true; }} title="Export Transcript">
                <Share class="w-3.5 h-3.5" />
                <span>Export</span>
            </Button>
        {/if}
        {#if isLexicalDocument}
            <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => {
                    if (isImportedTranscript) {
                        pathForExportModal = $project.currentImportedTranscriptPath;
                        isExportModalOpen = true;
                    } else {
                        showDocumentExportModal = true;
                    }
                }} title={isImportedTranscript ? "Export Transcript" : "Export Document"}>
                <Share class="w-3.5 h-3.5" />
                <span>Export</span>
            </Button>
        {/if}
        {#if isTable}
            <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => showTableExportModal = true} title="Export Table">
                <Share class="w-3.5 h-3.5" />
                <span>Export</span>
            </Button>
        {/if}
        {#if isImage}
            <Button size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1" on:click={() => dispatch('requestImageExport')} title="Export Image">
                <ImageDown class="w-3.5 h-3.5" />
                <span>Export</span>
            </Button>
        {/if}
        {#if isImportedTranscript || ($activeMediaFile && $displayedTranscripts.length > 1)}
            <Button size="xs" color="alternative" class="ml-2 px-2 !py-1" on:click={() => project.update(p => ({ ...p, showSplitTranscriptModal: true, pendingSplitOrientation: 'horizontal' }))} title="Split Transcript (Horizontal)">
                <SquareSplitHorizontal class="w-3.5 h-3.5" />
            </Button>

            <Button size="xs" color="alternative" class="ml-2 px-2 !py-1" on:click={() => project.update(p => ({ ...p, showSplitTranscriptModal: true, pendingSplitOrientation: 'vertical' }))} title="Split Transcript (Vertical)">
                <SquareSplitVertical class="w-3.5 h-3.5" />
            </Button>
        {/if}

  
         <!-- <button
            id="theme-toggle-button"
            on:click={cycleThemePreference}
            class="ui-button-icon h-8 w-8 flex items-center justify-center p-1"
            title={themeTitle}
        >
            {@html themeIconHtml}
         </button> -->
    <div class="flex-shrink-0">
        {#if $isMediaEditorOpen || isImportedTranscript || $activeMediaFile}
        <button
            on:click="{() => openLayoutSettingsModal()}"
            class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
            title="Change Transcript View Layout"
        >
            <LayoutDashboard class="w-4 h-4" />
        </button>
        {/if}
				 <button on:click="{() => cycleThemePreference()}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors" title="{themeTitle}">
            {#if $themePreference === 'light'}
                <Sun class="w-4 h-4" />
            {:else if $themePreference === 'dark'}
                <Moon class="w-4 h-4" />
            {:else}
                <Monitor class="w-4 h-4" />
            {/if}
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
                hideDualModeOptions={true}
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
    on:openConfig={() => dispatch("openConfig")}
    on:closeAndReset={() => toggleTranslateModal(false)}
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
    getExportData={dataViewRef?.getExportData}
    on:confirm={() => message('Table exported successfully.', { title: 'Success', type: 'info' })}
    on:close={() => showTableExportModal = false}
/>

<SplitTranscriptModal />
