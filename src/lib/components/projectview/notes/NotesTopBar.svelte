<!-- src/lib/components/projectview/notes/NotesTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { message } from '@tauri-apps/plugin-dialog';
    import { project, toggleAutosave } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';
    import { basename } from '@tauri-apps/api/path';
  
    const SUN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" /></svg>`;
    const MOON_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" /></svg>`;
    const SYSTEM_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25m18 0A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25m18 0V12a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 12V5.25" /></svg>`;
  
    $: themeIconHtml = $themePreference === 'light' ? MOON_ICON
                     : $themePreference === 'dark' ? SYSTEM_ICON
                     : SUN_ICON;
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
  
    async function handleManualSave() {
        const projState = get(project);
        const currentCanSave = !projState.autosaveEnabled && 
                               (projState.isDocumentDirty || projState.isDocumentMetadataDirty || projState.isImportedTranscriptDirty || projState.isMediaNoteTranscriptDirty || projState.isPdfAnnotationsDirty);
        
        if (!currentCanSave) { 
            console.warn("[NotesTopBar] Manual save clicked but conditions not met."); 
            return; 
        }
        console.log("[NotesTopBar] Manual save proceeding...");

        if ((projState.isDocumentDirty || projState.isDocumentMetadataDirty) && projState.selectedDocumentPath && projState.activeDocumentEditorRef?.ref && typeof projState.activeDocumentEditorRef.ref.save === 'function') {
            console.log("[NotesTopBar] Manual save triggered for DOCUMENT via editor ref:", projState.selectedDocumentPath);
            try { await projState.activeDocumentEditorRef.ref.save(); console.log("[NotesTopBar] Document manual save successful."); } 
            catch (error) { console.error("[NotesTopBar] Document manual save via editor ref failed:", error); }
        } else if (projState.isPdfAnnotationsDirty && projState.selectedDocumentPath && projState.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
            console.log("[NotesTopBar] Manual save triggered for PDF ANNOTATIONS:", projState.selectedDocumentPath);
            try { 
                // PDF Annotations save might be handled differently, e.g. a direct service call if no 'ref.save'
                // Assuming there's a service for this like `saveCurrentPdfAnnotations`
                const { saveCurrentPdfAnnotations } = await import('$lib/services/projectService.js');
                await saveCurrentPdfAnnotations();
                console.log("[NotesTopBar] PDF Annotations manual save successful."); 
            } catch (error) { console.error("[NotesTopBar] PDF Annotations manual save failed:", error); }
        } else if (projState.isImportedTranscriptDirty && projState.currentImportedTranscriptPath && projState.activeImportedTranscriptEditorRef?.ref && typeof projState.activeImportedTranscriptEditorRef.ref.save === 'function') {
             console.log("[NotesTopBar] Manual save triggered for IMPORTED TRANSCRIPT via editor ref:", projState.currentImportedTranscriptPath);
            try { await projState.activeImportedTranscriptEditorRef.ref.save(); console.log("[NotesTopBar] Imported Transcript manual save successful."); } 
            catch (error) { console.error("[NotesTopBar] Imported Transcript manual save via editor ref failed:", error); }
        } else if (projState.isMediaNoteTranscriptDirty && projState.selectedMediaNotePath && projState.activeMediaNoteEditorRef?.ref && typeof projState.activeMediaNoteEditorRef.ref.save === 'function') {
            console.log("[NotesTopBar] Manual save triggered for MEDIA NOTE TRANSCRIPT via editor ref:", projState.selectedMediaNotePath);
            try { await projState.activeMediaNoteEditorRef.ref.save(); console.log("[NotesTopBar] Media Note Transcript manual save successful."); }
            catch (error) { console.error("[NotesTopBar] Media Note Transcript manual save via editor ref failed:", error); }
        } else { 
            console.warn("[NotesTopBar] Manual save triggered but no specific dirty item found with an active editor ref capable of saving, or PDF annotations were not handled by a direct save call."); 
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
                console.log(`[NotesTopBar Autosave Watch] Document ${p.selectedDocumentPath} is dirty.`);
            } else if (p.isPdfAnnotationsDirty && p.selectedDocumentPath && p.selectedDocumentPath.toLowerCase().endsWith('.pdf')) {
                shouldAutosave = true;
                // No direct editorRef.save() for PDF annotations usually, service call is direct
                saveAction = 'pdfAnnotations';
                console.log(`[NotesTopBar Autosave Watch] PDF Annotations for ${p.selectedDocumentPath} are dirty.`);
            } else if (p.isImportedTranscriptDirty && p.currentImportedTranscriptPath && p.activeImportedTranscriptEditorRef?.ref) {
                shouldAutosave = true;
                activeEditorRefToSave = p.activeImportedTranscriptEditorRef.ref;
                saveAction = 'importedTranscript';
                 console.log(`[NotesTopBar Autosave Watch] Imported Transcript ${p.currentImportedTranscriptPath} is dirty.`);
            } else if (p.isMediaNoteTranscriptDirty && p.selectedMediaNotePath && p.activeMediaNoteEditorRef?.ref) {
                shouldAutosave = true;
                activeEditorRefToSave = p.activeMediaNoteEditorRef.ref;
                saveAction = 'mediaNoteTranscript';
                console.log(`[NotesTopBar Autosave Watch] Media Note Transcript for ${p.selectedMediaNotePath} is dirty.`);
            } else {
                // console.log(`[NotesTopBar Autosave Watch] Conditions not met.`);
            }
        }


        clearTimeout(autosaveTimeout);
        if (shouldAutosave) {
            autosaveTimeout = setTimeout(async () => {
                console.log("[NotesTopBar] Autosave timer fired. Attempting save...");
                const currentProjState = get(project); // Re-fetch current state
                let editorStillActiveAndDirty = false;

                if (saveAction === 'document' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeDocumentEditorRef?.ref === activeEditorRefToSave && (currentProjState.isDocumentDirty || currentProjState.isDocumentMetadataDirty);
                } else if (saveAction === 'pdfAnnotations') {
                    editorStillActiveAndDirty = currentProjState.selectedDocumentPath?.toLowerCase().endsWith('.pdf') && currentProjState.isPdfAnnotationsDirty;
                } else if (saveAction === 'importedTranscript' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeImportedTranscriptEditorRef?.ref === activeEditorRefToSave && currentProjState.isImportedTranscriptDirty;
                } else if (saveAction === 'mediaNoteTranscript' && activeEditorRefToSave) {
                    editorStillActiveAndDirty = currentProjState.activeMediaNoteEditorRef?.ref === activeEditorRefToSave && currentProjState.isMediaNoteTranscriptDirty;
                }

                if (editorStillActiveAndDirty) {
                     console.log(`[NotesTopBar] Autosaving (Action: ${saveAction})...`);
                     try { 
                        if (saveAction === 'pdfAnnotations') {
                            const { saveCurrentPdfAnnotations } = await import('$lib/services/projectService.js');
                            await saveCurrentPdfAnnotations();
                        } else if (activeEditorRefToSave && typeof activeEditorRefToSave.save === 'function') {
                            await activeEditorRefToSave.save(); 
                        } else {
                            console.warn(`[NotesTopBar Autosave] No valid save method for action ${saveAction}`);
                        }
                        console.log(`[NotesTopBar] Autosave successful for ${saveAction}.`); 
                    }
                     catch (error) { console.error(`[NotesTopBar] Autosave failed for ${saveAction}:`, error); }
                } else { 
                    console.log(`[NotesTopBar] Autosave timer fired, but conditions no longer met (Action: ${saveAction}, StillDirty: ${editorStillActiveAndDirty}). Save skipped.`); 
                }
            }, 3000); 
        }
    }
  
  </script>
  
  <div
    class="flex items-center justify-between px-3 h-10 flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700"
    data-tauri-drag-region
  >
    <div class="flex items-center min-w-0"> <!-- Added min-w-0 for truncate to work -->
        <span class="font-semibold text-lg text-gray-700 dark:text-gray-200 pl-1 truncate" title={displayTitle}>{displayTitle}</span>
    </div>
  
    <div class="flex items-center space-x-2 flex-shrink-0">
        <button
            class="ui-button-icon flex items-center h-7 px-2 py-0.5 rounded text-xs"
            title={canSave ? "Save Changes (Ctrl+S)" : (autosaveEnabled ? "Autosave is ON" : "No changes to save")}
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
              <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all dark:border-gray-600 peer-checked:bg-gray-400 dark:peer-checked:bg-gray-500"></div>
              <span
                class="absolute top-0 bottom-0 flex items-center text-xs font-medium text-gray-700 dark:text-gray-300 pointer-events-none"
                class:left-1={autosaveEnabled}
                class:right-1={!autosaveEnabled}
              >
                {autosaveEnabled ? 'On' : 'Off'}
              </span>
            </label>
        </div>
  
         <button
            on:click={cycleThemePreference}
            class="ui-button-icon h-7 w-7 flex items-center justify-center p-1"
            title={themeTitle}
        >
            {@html themeIconHtml}
         </button>
    </div>
  </div>
  
  <style lang="postcss">
    .ui-button-icon {
        @apply inline-flex items-center justify-center p-2 border border-transparent text-sm font-medium rounded-md text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors;
        &.px-2 { padding-left: 0.5rem; padding-right: 0.5rem; }
        &.py-1 { padding-top: 0.25rem; padding-bottom: 0.25rem; }
        &.text-xs { font-size: 0.75rem; line-height: 1rem; }
    }
    .ui-button-icon:disabled {
        @apply opacity-50 cursor-not-allowed;
    }
    .ui-button-icon svg {
        @apply w-5 h-5 flex-shrink-0;
    }
    .w-5 { width: 1.25rem; }
    .h-5 { height: 1.25rem; }
  
    :global(html.dark) .dark\:bg-gray-800 {
         background-color: #1f2937 !important;
    }
    :global(html.dark) .dark\:border-gray-700 {
         border-color: #374151 !important;
    }
     :global(html.dark) .dark\:bg-gray-700 {
         background-color: #374151 !important;
     }
     :global(html.dark) .dark\:hover\:bg-gray-600 {
          background-color: #4b5563 !important;
     }
     :global(html.dark) .dark\:text-gray-300 {
         color: #d1d5db !important;
     }
     :global(html.dark) .dark\:text-gray-200 {
          color: #e5e7eb !important;
     }
     :global(html.dark) .dark\:border-gray-600 {
          border-color: #4b5563 !important;
     }
     :global(html.dark) .dark\:bg-green-700 {
         background-color: #047857 !important;
     }
     :global(html.dark) .dark\:text-green-100 {
         color: #d1fae5 !important;
     }
  
  </style>