<!-- harvey-1.0/src/routes/+layout.svelte -->
<script>
    import { onMount } from 'svelte';
    import { get } from 'svelte/store'; // Ensure 'get' is imported
    import '../app.css';
  
    // --- Import theme store loader ---
    import { loadThemePreferenceFromBackend } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { page } from '$app/stores';
    import { loadProjectDataAndUpdateStore } from '$lib/services/projectService.js';
    import { tick } from 'svelte';
  
    onMount(async () => {
      console.log('[+layout.svelte] onMount started.');
      // --- Load theme preference ---
      try {
         console.log('[+layout.svelte] Attempting to load theme preference...');
         await loadThemePreferenceFromBackend();
         console.log('[+layout.svelte] Theme preference loaded successfully (or default used).');
      } catch (error) {
          console.error('[+layout.svelte] Failed during theme preference loading:', error);
      }
  
      // --- Load Project Data ---
      const xmlPathFromUrl = $page.url.searchParams.get('xmlPath');
      if (xmlPathFromUrl && xmlPathFromUrl.trim() !== '') {
          try {
              console.log(`[+layout.svelte] Attempting to load project data for: ${xmlPathFromUrl}`);
              await loadProjectDataAndUpdateStore(xmlPathFromUrl);
              console.log(`[+layout.svelte] Project data loading initiated for: ${xmlPathFromUrl}`);
  
              await tick(); // Allow store updates to process
              const loadedProjectState = get(project);
              console.log('[+layout.svelte] Store state immediately after project load attempt:', loadedProjectState);
  
              // --- *** REVISED CONTEXT CHECK *** ---
              if (!loadedProjectState.xmlPath) {
                   // This IS critical - project path failed to load into store
                   console.error(`[+layout.svelte] CRITICAL: xmlPath missing AFTER load attempt! Store xmlPath: ${loadedProjectState.xmlPath}`);
                   project.update(p => ({ ...p, isLoading: false, error: 'Project context incomplete: xmlPath missing.', statusMessage: 'Error: Project load incomplete.' }));
              } else if (!loadedProjectState.selectedMediaFile) { // Check if the whole object is null/undefined
                  // Project path loaded, but no media selected (normal for new/empty projects)
                  console.log(`[+layout.svelte] Project context OK (xmlPath loaded), but no media file selected yet. Store xmlPath: ${loadedProjectState.xmlPath}`);
                  // Update status to guide user - Optional
                  project.update(p => ({ ...p, statusMessage: 'Project loaded. Import or select a media file.' }));
              } else if (!loadedProjectState.selectedMediaFile.name) {
                   // Project path loaded, media object exists, but NAME is missing (Data integrity issue!)
                   console.error(`[+layout.svelte] CRITICAL: Project context inconsistent! xmlPath is present, but selectedMediaFile is missing its 'name'. selectedMediaFile: ${JSON.stringify(loadedProjectState.selectedMediaFile)}`);
                   project.update(p => ({ ...p, isLoading: false, error: 'Project context inconsistent: selected media file missing name.', statusMessage: 'Error: Project load inconsistent.' }));
              }
               else {
                   // Both xmlPath and selectedMediaFile.name are present and valid
                   console.log(`[+layout.svelte] Project context seems OK after load attempt (xmlPath and selectedMediaFile.name present).`);
              }
              // --- *** END REVISED CHECK *** ---
  
          } catch (e) {
              console.error('[+layout.svelte] Error calling loadProjectDataAndUpdateStore:', e);
               project.update(p => ({ ...p, isLoading: false, error: `Project loading failed: ${e?.message || e}`, statusMessage: 'Error: Project loading failed.' }));
          }
      } else {
          // This is expected on the welcome page, don't treat as an error here
          console.log('[+layout.svelte] Project path missing in URL (likely welcome page).');
          // Clear any potentially stale project data if landing on welcome page without path
          // Consider if this is needed:
          // project.update(p => ({ ...p, isLoading: false, error: null, xmlPath: null, selectedMediaFile: null, /* etc */ }));
      }
  
      console.log('[+layout.svelte] onMount finished.');
    });
  
  </script>
  
  <!-- This slot renders the content of the current page (+page.svelte) -->
  <slot />