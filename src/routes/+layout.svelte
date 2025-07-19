<!-- harvey-1.0/src/routes/+layout.svelte -->
<script>
    import { onMount } from 'svelte';
    import { get } from 'svelte/store'; // Ensure 'get' is imported
    import '../app.css';
    import '$lib/styles/tabulator-tailwind-theme.scss';
    import ToastNotifications from '$lib/components/projectview/shared/ToastNotifications.svelte';
  
    // --- Import theme store loader ---
    import { loadThemePreferenceFromBackend } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { page } from '$app/stores';
    import { loadProjectDataAndUpdateStore } from '$lib/services/projectService.js';
    import { tick } from 'svelte';
  
    onMount(async () => {
      console.debug('[+layout.svelte] onMount started.'); // DEBUG
      // --- Load theme preference ---
      try {
         console.debug('[+layout.svelte] Attempting to load theme preference...'); // DEBUG
         await loadThemePreferenceFromBackend();
         console.info('[+layout.svelte] Theme preference loaded successfully (or default used).'); // INFO
      } catch (error) {
          console.error('[+layout.svelte] Failed during theme preference loading:', error); // ERROR
      }
  
      // --- Load Project Data ---
      const xmlPathFromUrl = $page.url.searchParams.get('xmlPath');
      if (xmlPathFromUrl && xmlPathFromUrl.trim() !== '') {
          try {
              console.info(`[+layout.svelte] Attempting to load project data for: ${xmlPathFromUrl}`); // INFO
              await loadProjectDataAndUpdateStore(xmlPathFromUrl);
              console.info(`[+layout.svelte] Project data loading initiated for: ${xmlPathFromUrl}`); // INFO
  
              await tick(); // Allow store updates to process
              const loadedProjectState = get(project);
              console.debug('[+layout.svelte] Store state immediately after project load attempt:', loadedProjectState); // DEBUG (large object)
  
              // --- *** REVISED CONTEXT CHECK *** ---
              if (!loadedProjectState.xmlPath) {
                   // This IS critical - project path failed to load into store
                   console.error(`[+layout.svelte] CRITICAL: xmlPath missing AFTER load attempt! Store xmlPath: ${loadedProjectState.xmlPath}`); // ERROR
                   project.update(p => ({ ...p, isLoading: false, error: 'Project context incomplete: xmlPath missing.', statusMessage: 'Error: Project load incomplete.' }));
              } else if (!loadedProjectState.selectedMediaFile) { // Check if the whole object is null/undefined
                  // Project path loaded, but no media selected (normal for new/empty projects)
                  console.info(`[+layout.svelte] Project context OK (xmlPath loaded), but no media file selected yet. Store xmlPath: ${loadedProjectState.xmlPath}`); // INFO
                  // Update status to guide user - Optional
                  project.update(p => ({ ...p, statusMessage: 'Project loaded. Import or select a media file.' }));
              } else if (!loadedProjectState.selectedMediaFile.name) {
                   // Project path loaded, media object exists, but NAME is missing (Data integrity issue!)
                   console.error(`[+layout.svelte] CRITICAL: Project context inconsistent! xmlPath is present, but selectedMediaFile is missing its 'name'. selectedMediaFile: ${JSON.stringify(loadedProjectState.selectedMediaFile)}`); // ERROR
                   project.update(p => ({ ...p, isLoading: false, error: 'Project context inconsistent: selected media file missing name.', statusMessage: 'Error: Project load inconsistent.' }));
              }
               else {
                   // Both xmlPath and selectedMediaFile.name are present and valid
                   console.info(`[+layout.svelte] Project context seems OK after load attempt (xmlPath and selectedMediaFile.name present).`); // INFO
              }
              // --- *** END REVISED CHECK *** ---
  
          } catch (e) {
              console.error('[+layout.svelte] Error calling loadProjectDataAndUpdateStore:', e); // ERROR
               project.update(p => ({ ...p, isLoading: false, error: `Project loading failed: ${e?.message || e}`, statusMessage: 'Error: Project loading failed.' }));
          }
      } else {
          // This is expected on the welcome page, don't treat as an error here
          console.info('[+layout.svelte] Project path missing in URL (likely welcome page).'); // INFO
          // Clear any potentially stale project data if landing on welcome page without path
          // Consider if this is needed:
          // project.update(p => ({ ...p, isLoading: false, error: null, xmlPath: null, selectedMediaFile: null, /* etc */ }));
      }
  
      
    });
  
  </script>
  
  <!-- This slot renders the content of the current page (+page.svelte) -->
  <slot />
  <ToastNotifications />