<!-- src/lib/components/projectview/data/DataTopBar.svelte -->
<script>
  import { Button, Select, Dropdown, DropdownItem } from 'flowbite-svelte';
  import {
    MessageSquareText,
    Share,
    Languages,
    ImageDown,
    Mic,
    LayoutDashboard,
    SquareSplitHorizontal,
    SquareSplitVertical,
    Sun,
    Moon,
    Monitor,
    LayoutGrid,
    List,
    ChevronDown,
    Pencil,
    PencilOff
  } from '@lucide/svelte';
  import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
  import panelStateStore from '$lib/stores/panelStateStore.svelte.js';
  import { message } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import {
    project,
    switchTranscriptInDataTab,
    clearStandaloneTranscriptSplit
  } from '$lib/stores/projectStore.js';
  import { mediaEditorStore } from '$lib/stores/mediaEditorStore.svelte.js';
  import ExportModal from '../modals/ExportModal.svelte';
  import { transcriptStore, toggleTranslateModal } from '$lib/stores/transcriptStore.js';
  import { configStatus } from '$lib/stores/configStatusStore.js';
  import { exportTranscript } from '$lib/services/configureActions.js';
  import { activeLayout } from '$lib/stores/layoutStore.js';
  import { get, derived } from 'svelte/store';
  import { basename } from '@tauri-apps/api/path';
  import { languageOptions } from '$lib/constants/transcriptionOptions.js';
  import { DOCX_LAYOUT_OPTIONS } from '$lib/constants/exportLayouts.js';
  import { createEventDispatcher, onMount, onDestroy, untrack } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import LiveTranscribeModelModal from '../modals/LiveTranscribeModelModal.svelte';
  // import Dropdown from '$lib/components/shared/Dropdown.svelte'; // Removed custom dropdown
  import TranslateDocumentModal from '../modals/TranslateDocumentModal.svelte';
  import DocumentExportModal from '../modals/DocumentExportModal.svelte';
  import TableExportModal from '../modals/TableExportModal.svelte';
  import SplitTranscriptModal from '../modals/SplitTranscriptModal.svelte';
  import TopBarTableViewsDropdown from './TopBarTableViewsDropdown.svelte';
  import {
    requestDocumentTranslation,
    requestStandaloneTranscriptTranslation
  } from '$lib/services/projectService.js';

  const dispatch = createEventDispatcher();
  let {
    dataViewRef = null,
    getExportData = null,
    activeSubItemPath = null,
    activeSubItemType = null
  } = $props();

  // Determine platform-specific modifier key name
  const isMac =
    typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
  const modKeyName = isMac ? 'Cmd' : 'Ctrl';

  let isLiveTranscriptionActive = $state(false);
  let isLiveTranscriptionReady = $state(false);
  let liveTranscriptionError = $state(null);
  let showLiveTranscribeModal = $state(false);
  let isAddingTimestamps = $state(false);

  let dotCount = $state(1);
  let dotInterval = $state();

  function startDotAnimation() {
    dotInterval = setInterval(() => {
      dotCount = (dotCount % 3) + 1;
    }, 500);
  }

  function stopDotAnimation() {
    clearInterval(dotInterval);
  }

  let dots = $derived('.'.repeat(dotCount));
  let showTranslateDocumentModal = $state(false);
  let showDocumentExportModal = $state(false);
  let showTableExportModal = $state(false);
  const isStandaloneTranscript = $derived(!!$project.currentStandaloneTranscriptPath);
  const isLexicalDocument = $derived(
    isStandaloneTranscript ||
    ($project.selectedDocumentPath &&
      $project.selectedDocumentPath.toLowerCase().endsWith('.json'))
  );
  const isImage = $derived(
    $project.selectedDocumentPath &&
    ($project.selectedDocumentType === 'images' ||
      $project.selectedDocumentType === 'image' ||
      /\.(jpg|jpeg|png|gif|webp|bmp|tiff)$/i.test($project.selectedDocumentPath))
  );
  const isTable = $derived(
    $project.selectedDocumentPath &&
    ($project.selectedDocumentType === 'tables' ||
      $project.selectedDocumentType === 'table' ||
      /\.(csv|xlsx)$/i.test($project.selectedDocumentPath))
  );
  const isGroup = $derived(!!$project.selectedGroupId);
  let pathForExportModal = $state('');

  let currentTranscriptPathForSplit = $derived(
    isStandaloneTranscript
      ? $project.currentStandaloneTranscriptPath
      : $project.activeTranscriptPathInDataTab
  );
  let splitState = $derived(
    currentTranscriptPathForSplit && $project.standaloneTranscriptSplits
      ? $project.standaloneTranscriptSplits[currentTranscriptPathForSplit]
      : null
  );
  let isHorizontalSplitActive = $derived(splitState?.orientation === 'horizontal');
  let isVerticalSplitActive = $derived(splitState?.orientation === 'vertical');

  function handleSplitToggle(orientation) {
    if (orientation === 'horizontal' && isHorizontalSplitActive) {
      clearStandaloneTranscriptSplit(currentTranscriptPathForSplit);
    } else if (orientation === 'vertical' && isVerticalSplitActive) {
      clearStandaloneTranscriptSplit(currentTranscriptPathForSplit);
    } else {
      project.update((p) => ({
        ...p,
        showSplitTranscriptModal: true,
        pendingSplitOrientation: orientation
      }));
    }
  }


  function handleDocumentTranslateConfirm(event) {
    const { documentPath, model, targetLanguage, sourceLanguage } = event.detail;
    if (isStandaloneTranscript) {
      requestStandaloneTranscriptTranslation(documentPath, model, targetLanguage, sourceLanguage);
    } else {
      requestDocumentTranslation(documentPath, model, targetLanguage, sourceLanguage);
    }
  }

  $effect(() => {
    showTranslateDocumentModal = $transcriptStore.showTranslateModal;
  });

  function getLanguageLabel(langCode) {
    if (!langCode || langCode === 'original') return 'Original';

    let targetCode = langCode;
    if (langCode.includes('-')) {
      targetCode = langCode.split('-').pop(); // e.g., 'en-hi' -> 'hi'
    }

    const option = languageOptions.find((opt) => opt.value === targetCode);
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

    const withLabels = transcripts.map((t) => {
      const langLabel = getLanguageLabel(t.language_code || 'original');
      let fileName = t.name;
      if (!fileName && t.path) {
        try {
          const pathParts = t.path.split(/[\\/]/);
          fileName = pathParts[pathParts.length - 1];
          if (fileName.toLowerCase().endsWith('.json')) {
            fileName = fileName.substring(0, fileName.length - 5);
          }
        } catch (e) {
          console.error('Error extracting filename from path:', e);
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
  let currentThemeName = $derived(
    $themePreference.charAt(0).toUpperCase() + $themePreference.slice(1)
  );
  let nextThemeName = $derived(
    $themePreference === 'light' ? 'Dark' : $themePreference === 'dark' ? 'System' : 'Light'
  );
  let themeTitle = $derived(`Current theme: ${currentThemeName}. Switch to ${nextThemeName} mode.`);

  const isDocumentDirty = $derived($project.isDocumentDirty || $project.isDocumentMetadataDirty);
  const isStandaloneTranscriptDirty = $derived($project.isStandaloneTranscriptDirty);
  const isMediaNoteTranscriptDirty = $derived($project.isMediaNoteTranscriptDirty);
  const isPdfAnnotationsDirty = $derived($project.isPdfAnnotationsDirty);
  const activeDocumentEditorRef = $derived($project.activeDocumentEditorRef);
  const activeStandaloneTranscriptEditorRef = $derived($project.activeStandaloneTranscriptEditorRef);
  const activeMediaNoteEditorRef = $derived($project.activeMediaNoteEditorRef);

  const isAnythingDirty = $derived(
    isDocumentDirty ||
    isStandaloneTranscriptDirty ||
    isMediaNoteTranscriptDirty ||
    isPdfAnnotationsDirty
  );
  const showDirtyIndicator = $derived(isAnythingDirty);
  let isExportModalOpen = $state(false);
  let isLayoutDropdownOpen = $state(false);
  let currentActivePath = $state();

  let displayTitle = $state('');


  // New reactive block for displayTitle
  $effect(() => {
    if ($project && $project.name) {
      let currentFileName = null;
      let activePath =
        $project.selectedDocumentPath ||
        $project.selectedMediaNotePath ||
        $project.currentStandaloneTranscriptPath ||
        $project.selectedTablePath ||
        $project.selectedImagePath;

      if (activePath) {
        basename(activePath)
          .then((name) => {
            currentFileName = name;
            if (currentFileName) {
              displayTitle = `${$project.name} : ${currentFileName}`;
            } else {
              displayTitle = $project.name;
            }
          })
          .catch((err) => {
            console.error('Error getting basename for top bar:', err);
            displayTitle = $project.name; // Fallback
          });
      } else {
        displayTitle = $project.name;
      }
    } else {
      displayTitle = 'Harvey'; // Default if project or name is not available
    }
  });

  // Stop live transcription if the user switches to another file
  $effect(() => {
    const newActivePath =
      $project.selectedDocumentPath ||
      $project.selectedMediaNotePath ||
      $project.currentStandaloneTranscriptPath ||
      $project.selectedTablePath ||
      $project.selectedImagePath;

    if (newActivePath !== untrack(() => currentActivePath)) {
      if (isLiveTranscriptionActive) {
        invoke('stop_live_transcription')
          .then((stopped) => {
            if (stopped) {
              isLiveTranscriptionActive = false;
              isLiveTranscriptionReady = false;
              stopDotAnimation();
            }
          })
          .catch((err) => {
            console.error('Failed to stop live transcription on file switch:', err);
          });
      }
      currentActivePath = newActivePath;
    }
  });

  const LAYOUT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-wtf" viewBox="0 0 16 16"><path d="M5 1v8H1V1zM1 0a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1V1a1 1 0 0 0-1-1zm13 2v5H9V2zM9 1a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1zM5 13v2H3v-2zm-2-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1zm12-1v2H9v-2zm-6-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1z"/></svg>`;

  async function handleManualSave() {
    // Autosave handles everything.
  }

  let autosaveTimeout = $state();
  $effect(() => {
    const p = $project;
    let shouldAutosave = false;
    let activeEditorRefToSave = null;
    let saveAction = null;

    if (true) {
      if (
        (p.isDocumentDirty || p.isDocumentMetadataDirty) &&
        p.selectedDocumentPath &&
        p.activeDocumentEditorRef?.ref
      ) {
        shouldAutosave = true;
        activeEditorRefToSave = p.activeDocumentEditorRef.ref;
        saveAction = 'document';
        console.log(`[DataTopBar Autosave Watch] Document ${p.selectedDocumentPath} is dirty.`);
      } else if (
        p.isPdfAnnotationsDirty &&
        p.selectedDocumentPath &&
        p.selectedDocumentPath.toLowerCase().endsWith('.pdf')
      ) {
        shouldAutosave = true;
        // No direct editorRef.save() for PDF annotations usually, service call is direct
        saveAction = 'pdfAnnotations';
        console.log(
          `[DataTopBar Autosave Watch] PDF Annotations for ${p.selectedDocumentPath} are dirty.`
        );
      } else if (
        (p.isStandaloneTranscriptDirty || p.isStandaloneTranscriptMetadataDirty) &&
        p.currentStandaloneTranscriptPath &&
        p.activeStandaloneTranscriptEditorRef?.ref
      ) {
        shouldAutosave = true;
        activeEditorRefToSave = p.activeStandaloneTranscriptEditorRef.ref;
        saveAction = 'standaloneTranscript';
        console.log(
          `[DataTopBar Autosave Watch] Imported Transcript ${p.currentStandaloneTranscriptPath} is dirty.`
        );
      } else if (
        p.isMediaNoteTranscriptDirty &&
        p.selectedMediaNotePath &&
        p.activeMediaNoteEditorRef?.ref
      ) {
        shouldAutosave = true;
        activeEditorRefToSave = p.activeMediaNoteEditorRef.ref;
        saveAction = 'mediaNoteTranscript';
        console.log(
          `[DataTopBar Autosave Watch] Media Note Transcript for ${p.selectedMediaNotePath} is dirty.`
        );
      } else if (p.isDocumentDirty && p.selectedTablePath && dataViewRef) {
        shouldAutosave = true;
        activeEditorRefToSave = dataViewRef;
        saveAction = 'table';
        console.log(`[DataTopBar Autosave Watch] Table for ${p.selectedTablePath} is dirty.`);
      } else {
        // console.log(`[DataTopBar Autosave Watch] Conditions not met.`);
      }
    }

    untrack(() => clearTimeout(autosaveTimeout));
    if (shouldAutosave) {
      autosaveTimeout = setTimeout(async () => {
        console.log('[DataTopBar] Autosave timer fired. Attempting save...');
        const currentProjState = get(project); // Re-fetch current state
        let editorStillActiveAndDirty = false;

        if (saveAction === 'document' && activeEditorRefToSave) {
          editorStillActiveAndDirty =
            currentProjState.activeDocumentEditorRef?.ref === activeEditorRefToSave &&
            (currentProjState.isDocumentDirty || currentProjState.isDocumentMetadataDirty);
        } else if (saveAction === 'pdfAnnotations') {
          editorStillActiveAndDirty =
            currentProjState.selectedDocumentPath?.toLowerCase().endsWith('.pdf') &&
            currentProjState.isPdfAnnotationsDirty;
        } else if (saveAction === 'standaloneTranscript' && activeEditorRefToSave) {
          editorStillActiveAndDirty =
            currentProjState.activeStandaloneTranscriptEditorRef?.ref === activeEditorRefToSave &&
            (currentProjState.isStandaloneTranscriptDirty ||
              currentProjState.isStandaloneTranscriptMetadataDirty);
        } else if (saveAction === 'mediaNoteTranscript' && activeEditorRefToSave) {
          editorStillActiveAndDirty =
            currentProjState.activeMediaNoteEditorRef?.ref === activeEditorRefToSave &&
            currentProjState.isMediaNoteTranscriptDirty;
        } else if (saveAction === 'table' && activeEditorRefToSave) {
          editorStillActiveAndDirty =
            dataViewRef === activeEditorRefToSave && currentProjState.isDocumentDirty;
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
          } catch (error) {
            console.error(`[DataTopBar] Autosave failed for ${saveAction}:`, error);
          }
        } else {
          console.log(
            `[DataTopBar] Autosave timer fired, but conditions no longer met (Action: ${saveAction}, StillDirty: ${editorStillActiveAndDirty}). Save skipped.`
          );
        }
      }, 3000);
    }
  });
  async function handleExportConfirm(event) {
    const { filePath, format, layoutChoice, excludeSpeakerNames } = event.detail;
    const activeTranscriptPath = pathForExportModal;

    if (!activeTranscriptPath) {
      message('No active transcript selected to export.', {
        title: 'Export Failed',
        type: 'error'
      });
      return;
    }

    try {
      const jsonContent = await invoke('load_transcript_json', {
        transcriptPath: activeTranscriptPath
      });
      if (!jsonContent) {
        throw new Error('Transcript file is empty or could not be read.');
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

      const tableNode = transcriptData?.root?.children?.find((c) => c.type === 'table');

      if (tableNode && tableNode.children) {
        // Skip header row (i=0)
        for (let i = 1; i < tableNode.children.length; i++) {
          const rowNode = tableNode.children[i];
          if (rowNode.type !== 'tablerow' || !rowNode.children || rowNode.children.length < 4)
            continue;

          const cells = rowNode.children;
          const timestampText = getTextFromLexicalNode(cells[1]);
          const [startStr, endStr] = timestampText.split(' - ').map((s) => s.trim());

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
            text: textJson
          });
        }
      }

      if (segmentsToExport.length === 0) {
        message('No transcript data available to export.', {
          title: 'Export Failed',
          type: 'error'
        });
        return;
      }

      await exportTranscript(
        filePath,
        format,
        segmentsToExport,
        activeTranscriptPath,
        layoutChoice,
        excludeSpeakerNames
      );
      message(`Transcript successfully exported to ${filePath}`, {
        title: 'Export Successful',
        type: 'info'
      });
    } catch (error) {
      console.error('Export failed:', error);
      message(`Failed to export transcript: ${error?.message || error}`, {
        title: 'Export Failed',
        type: 'error'
      });
    }
  }

  let unlisten = null;
  let unlistenReady = null;

  onMount(async () => {
    unlistenReady = await listen('live_transcription_ready', () => {
      isLiveTranscriptionReady = true;
      startDotAnimation();
    });

    unlisten = await listen('live_transcription_result', (event) => {
      const { text, is_final, start_time, end_time } = event.payload;
      const p = get(project);
      let editorRef = null;

      // Find the correct active editor
      if (p.activeDocumentEditorRef) {
        editorRef = p.activeDocumentEditorRef;
      } else if (p.activeStandaloneTranscriptEditorRef) {
        editorRef = p.activeStandaloneTranscriptEditorRef;
      }
      // Removed activeMediaNoteEditorRef from live transcription updates

      if (editorRef?.ref?.updateLiveTranscriptionText) {
        editorRef.ref.updateLiveTranscriptionText(
          text,
          is_final,
          start_time,
          end_time,
          isAddingTimestamps
        );
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
    if (unlistenReady) {
      unlistenReady();
    }
    stopDotAnimation();
  });

  async function toggleLiveTranscription() {
    if (isLiveTranscriptionActive) {
      try {
        const stopped = await invoke('stop_live_transcription');
        if (stopped) {
          isLiveTranscriptionActive = false;
          isLiveTranscriptionReady = false;
          stopDotAnimation();
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
        message('Project details not available. Cannot start transcription.', {
          title: 'Error',
          type: 'error'
        });
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
      isLiveTranscriptionReady = false;
      liveTranscriptionError = error;
      message(`Failed to start live transcription: ${error}`, { title: 'Error', type: 'error' });
    }
  }

  let liveTranscriptionStatus = $derived(
    isLiveTranscriptionActive ? 'active' : liveTranscriptionError ? 'error' : 'default'
  );
</script>

<div
  class="flex items-center h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800 relative z-30"
  on:requestTranscriptionTabWithMediaAndDialog
>
  <!-- Drag Handle Background -->
  <div class="absolute inset-0 z-0" data-tauri-drag-region></div>

  <!-- Section 1: Left Bar equivalent (w-12) — Import button -->
  <div class="w-12 flex-shrink-0 flex items-center justify-center z-10">
    <button
      type="button"
      class="p-1.5 rounded-full border-0 bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400 hover:bg-blue-200 dark:hover:bg-blue-500/30 transition-all duration-200 active:scale-95 shadow-sm hover:shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
      on:click={(e) => dispatch('requestImport', e)}
      title="Import Audio or Video"
      aria-label="Import Audio or Video"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        stroke-width="2"
        stroke="currentColor"
        class="w-5 h-5"
      >
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
      </svg>
    </button>
  </div>

  <!-- Section 2: Left Panel equivalent (w-64) — Project name + file name -->
  <div
    class="w-64 flex-shrink-0 flex items-center overflow-hidden z-10 px-2 transition-all duration-300 ease-in-out"
  >
    <span
      class="font-semibold text-sm text-gray-700 dark:text-gray-200 truncate"
      title={displayTitle}>{displayTitle}</span
    >
  </div>

  <!-- Combined Section 3 & 4: Middle + Right Panel -->
  <div class="flex-grow flex items-center min-w-0 z-10 px-2 justify-between">
    <!-- Left side: Actions -->
    <div class="flex items-center space-x-1.5">
      {#if $activeMediaFile}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() =>
            dispatch('requestTranscriptionTabWithMediaAndDialog', {
              mediaPath: $activeMediaFile.path
            })}
          title="Transcribe"
        >
          <MessageSquareText class="w-3.5 h-3.5" />
          <span>Transcribe</span>
        </Button>
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() =>
            dispatch('requestTranslationTabWithMediaAndDialog', {
              mediaPath: $activeMediaFile.path,
              transcriptPath: $project.activeTranscriptPathInDataTab
            })}
          title={$transcriptStore.isTranslating
            ? 'View Translation Status'
            : 'Translate Transcript'}
        >
          {#if $transcriptStore.isTranslating}
            <Languages class="w-3.5 h-3.5 animate-spin" />
            <span>Translating...</span>
          {:else}
            <Languages class="w-3.5 h-3.5" />
            <span>Translate</span>
          {/if}
        </Button>
      {/if}
      {#if $project.activeDocumentEditorRef}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={toggleLiveTranscription}
          title="Live Transcribe"
        >
          <Mic
            class="w-3.5 h-3.5 {isLiveTranscriptionActive ? 'text-red-500 animate-pulse' : ''}"
          />
          <span class="whitespace-nowrap w-24 text-left">
            {#if isLiveTranscriptionActive && !isLiveTranscriptionReady}
              Initializing...
            {:else if isLiveTranscriptionActive && isLiveTranscriptionReady}
              Listening{dots}
            {:else}
              Live Transcribe
            {/if}
          </span>
        </Button>
      {/if}
      {#if isLexicalDocument}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() => toggleTranslateModal(true)}
          title={$transcriptStore.isTranslating ? 'View Translation Status' : 'Translate Document'}
        >
          {#if $transcriptStore.isTranslating}
            <Languages class="w-3.5 h-3.5 animate-spin" />
            <span>Translating...</span>
          {:else}
            <Languages class="w-3.5 h-3.5" />
            <span>Translate</span>
          {/if}
        </Button>
      {/if}
    </div>

    <!-- Right side: Controls -->
    <div class="flex items-center space-x-1.5">
      {#if $activeMediaFile}
        <div class="relative">
          <Button
            id="transcript-selection-btn"
            size="xs"
            color="alternative"
            class="w-60 justify-between px-3 !py-1.5 focus:ring-0"
            title="Select Transcript"
          >
            <span class="truncate"
              >{$displayedTranscripts.find((t) => t.path === $project.activeTranscriptPathInDataTab)
                ?.displayLabel || 'Select Transcript'}</span
            >
            <ChevronDown class="w-3.5 h-3.5 ml-2 text-gray-500 shrink-0" />
          </Button>
          <Dropdown
            triggeredBy="#transcript-selection-btn"
            class="w-60 z-[1001] max-h-96 overflow-y-auto"
          >
            {#each $displayedTranscripts as t (t.path)}
              <DropdownItem
                class="text-xs flex items-center {$project.activeTranscriptPathInDataTab === t.path
                  ? 'font-bold bg-blue-50 dark:bg-gray-700'
                  : ''}"
                on:click={() => switchTranscriptInDataTab(t.path)}
              >
                <span class="truncate">{t.displayLabel}</span>
              </DropdownItem>
            {/each}
          </Dropdown>
        </div>
      {/if}
      {#if isTable}
        <TopBarTableViewsDropdown
          tablePath={$project.selectedDocumentPath}
          {activeSubItemPath}
          {activeSubItemType}
          on:requestOpenView
          on:requestOpenLexicalDocument
          on:requestClearSubItem
        />
      {/if}
      {#if isGroup}
        <div class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-900 rounded-lg p-0.5">
          <button
            on:click={() => panelStateStore.setGroupDetailViewMode('list')}
            class="p-1 rounded-md border-0 transition-colors {panelStateStore.groupDetailViewMode ===
            'list'
              ? 'bg-white dark:bg-gray-700 shadow-sm text-blue-600 dark:text-blue-400'
              : 'text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'}"
            title="List View"
          >
            <List class="w-4 h-4" />
          </button>
          <button
            on:click={() => panelStateStore.setGroupDetailViewMode('grid')}
            class="p-1 rounded-md border-0 transition-colors {panelStateStore.groupDetailViewMode ===
            'grid'
              ? 'bg-white dark:bg-gray-700 shadow-sm text-blue-600 dark:text-blue-400'
              : 'text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'}"
            title="Grid View"
          >
            <LayoutGrid class="w-4 h-4" />
          </button>
        </div>
      {/if}
      <!-- Export buttons: right-aligned, after dropdowns -->
      {#if $activeMediaFile}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() => {
            pathForExportModal = $project.activeTranscriptPathInDataTab;
            isExportModalOpen = true;
          }}
          title="Export Transcript"
        >
          <Share class="w-3.5 h-3.5" />
          <span class="hidden xl:inline">Export</span>
        </Button>
      {/if}
      {#if isLexicalDocument}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() => {
            if (isStandaloneTranscript) {
              pathForExportModal = $project.currentStandaloneTranscriptPath;
              isExportModalOpen = true;
            } else {
              showDocumentExportModal = true;
            }
          }}
          title={isStandaloneTranscript ? 'Export Transcript' : 'Export Document'}
        >
          <Share class="w-3.5 h-3.5" />
          <span class="hidden xl:inline">Export</span>
        </Button>
      {/if}
      {#if isTable}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() => (showTableExportModal = true)}
          title="Export Table"
        >
          <Share class="w-3.5 h-3.5" />
          <span class="hidden xl:inline">Export</span>
        </Button>
      {/if}
      {#if isImage}
        <Button
          size="xs"
          color="alternative"
          class="space-x-1 px-2 !py-1"
          on:click={() => dispatch('requestImageExport')}
          title="Export Image"
        >
          <Share class="w-3.5 h-3.5" />
          <span class="hidden xl:inline">Export</span>
        </Button>
      {/if}
      {#if isStandaloneTranscript || ($activeMediaFile && $displayedTranscripts.length > 1)}
        <button
          on:click={() => handleSplitToggle('horizontal')}
          class="p-1.5 rounded-sm border-0 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors {isHorizontalSplitActive
            ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400'
            : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10'}"
          title="Split Transcript (Horizontal)"
        >
          <SquareSplitHorizontal class="w-4 h-4" />
        </button>
        <button
          on:click={() => handleSplitToggle('vertical')}
          class="p-1.5 rounded-sm border-0 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors {isVerticalSplitActive
            ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400'
            : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10'}"
          title="Split Transcript (Vertical)"
        >
          <SquareSplitVertical class="w-4 h-4" />
        </button>
      {/if}
      {#if mediaEditorStore.isMediaEditorOpen || isStandaloneTranscript || $activeMediaFile}
        <button
          id="layout-settings-btn-data"
          class="p-1.5 rounded-full border-0 focus:outline-none focus:ring-2 focus:ring-offset-2 transition-colors {isLayoutDropdownOpen
            ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400 focus:ring-blue-500'
            : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:ring-indigo-500'}"
          title="Change Transcript View Layout"
        >
          <LayoutDashboard class="w-4 h-4" />
        </button>
        <Dropdown
          bind:open={isLayoutDropdownOpen}
          triggeredBy="#layout-settings-btn-data"
          class="w-72 z-[1001] p-3 shadow-xl border border-gray-200 dark:border-gray-700"
        >
          <div class="mb-3 px-1">
            <h3 class="text-sm font-semibold text-gray-900 dark:text-white">Transcript Layout</h3>
            <p class="text-xxs text-gray-500 dark:text-gray-400">
              Select how the transcript appears on screen.
            </p>
          </div>
          <div class="grid grid-cols-1 gap-2">
            {#each DOCX_LAYOUT_OPTIONS as layout (layout.id)}
              <button
                type="button"
                class="text-left p-3 border rounded-xl transition-all duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 group relative {$activeLayout ===
                layout.rustLayoutKey
                  ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-400'
                  : 'border-gray-200 dark:border-gray-700 hover:border-blue-300'}"
                on:click={() => activeLayout.setLayout(layout.rustLayoutKey)}
                title="Select {layout.name} layout"
              >
                <div
                  class="font-bold mb-1.5 text-xs {$activeLayout === layout.rustLayoutKey
                    ? 'text-blue-700 dark:text-blue-300'
                    : 'text-gray-700 dark:text-gray-300'}"
                >
                  {layout.name}
                </div>
                <div
                  class="{layout.previewClasses} min-h-[22px] opacity-80 rounded shadow-sm overflow-hidden border border-gray-100 dark:border-gray-800 bg-white dark:bg-gray-800"
                >
                  {#each layout.columnStyles as style (style.class)}
                    <div
                      class="{style.class} !p-0.5 !text-[8px] leading-tight flex items-center justify-center"
                    >
                      {style.content}
                    </div>
                  {/each}
                </div>
                {#if $activeLayout === layout.rustLayoutKey}
                  <div class="absolute top-2 right-2 w-1.5 h-1.5 bg-blue-500 rounded-full"></div>
                {/if}
              </button>
            {/each}
          </div>
        </Dropdown>
      {/if}
      <button
        id="read-edit-toggle-data"
        on:click={() => (mediaEditorStore.isLexicalEditMode = !mediaEditorStore.isLexicalEditMode)}
        class="px-2.5 py-1.5 rounded-full border-0 transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 flex items-center space-x-1.5 {mediaEditorStore.isLexicalEditMode
          ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400'
          : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10'}"
        title={mediaEditorStore.isLexicalEditMode ? 'Switch to Read Mode' : 'Switch to Edit Mode'}
      >
        {#if mediaEditorStore.isLexicalEditMode}
          <Pencil class="w-3.5 h-3.5 text-blue-600 dark:text-blue-400" />
          <span class="hidden xl:inline text-xs font-medium text-blue-600 dark:text-blue-400"
            >Edit Mode</span
          >
        {:else}
          <PencilOff class="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
          <span class="hidden xl:inline text-xs font-medium text-gray-500 dark:text-gray-400"
            >Read Mode</span
          >
        {/if}
      </button>
    </div>
  </div>

  <!-- Section 5: Right Bar equivalent (w-8) — Theme button -->
  <div class="w-8 flex-shrink-0 flex items-center justify-center z-10">
    <button
      on:click={() => cycleThemePreference()}
      class="p-1 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
      title={themeTitle}
      aria-label={themeTitle}
    >
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

<ExportModal
  bind:showModal={isExportModalOpen}
  transcriptPath={pathForExportModal}
  on:confirm={handleExportConfirm}
  on:close={() => (isExportModalOpen = false)}
/>

<LiveTranscribeModelModal
  bind:showModal={showLiveTranscribeModal}
  on:confirm={handleLiveTranscribe}
  on:close={() => (showLiveTranscribeModal = false)}
/>

<TranslateDocumentModal
  bind:showModal={showTranslateDocumentModal}
  activeDocumentPath={isStandaloneTranscript
    ? $project.currentStandaloneTranscriptPath
    : $project.selectedDocumentPath}
  on:confirm={handleDocumentTranslateConfirm}
  on:openConfig={() => dispatch('openConfig')}
  on:runInBackgroundAndClose={() => toggleTranslateModal(false)}
  on:closeAndReset={() => toggleTranslateModal(false)}
/>

<DocumentExportModal
  bind:showModal={showDocumentExportModal}
  documentPath={isStandaloneTranscript
    ? $project.currentStandaloneTranscriptPath
    : $project.selectedDocumentPath}
  on:confirm={() => message('Document exported successfully.', { title: 'Success', type: 'info' })}
  on:close={() => (showDocumentExportModal = false)}
/>

<TableExportModal
  bind:showModal={showTableExportModal}
  tablePath={isTable ? $project.selectedDocumentPath : null}
  getExportData={getExportData || dataViewRef?.getExportData}
  on:confirm={() => message('Table exported successfully.', { title: 'Success', type: 'info' })}
  on:close={() => (showTableExportModal = false)}
/>

<SplitTranscriptModal />

<style lang="postcss">
  :global(.hover-scale-effect) {
    will-change: transform;
    backface-visibility: hidden;
    transform: translateZ(0);
  }
</style>
