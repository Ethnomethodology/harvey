<!-- harvey-1.0/src/routes/+layout.svelte -->
<script>
    import { onMount } from 'svelte';
    import { get } from 'svelte/store'; // Ensure 'get' is imported
    import '../app.css';
    import '$lib/styles/tabulator-tailwind-theme.scss';
    import ToastNotifications from '$lib/components/projectview/shared/ToastNotifications.svelte';
    import HeaderConfirmationModal from '$lib/components/modals/HeaderConfirmationModal.svelte';
  
    // --- Import theme store loader ---
    import { loadThemePreferenceFromBackend } from '$lib/stores/themeStore.js';
    import { updateConfigStatus } from '$lib/stores/configStatusStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { page } from '$app/stores';
    import { loadProjectDataAndUpdateStore, normalizePath } from '$lib/services/projectService.js';
    import { tick, onDestroy } from 'svelte';
    import { Loader } from '@lucide/svelte'; // Import Loader component
    import { open } from '@tauri-apps/plugin-shell';
  
    function handleGlobalClick(event) {
        const anchor = event.target.closest('a');
        if (!anchor) return;

        const href = anchor.getAttribute('href');
        if (!href) return;

        // Intercept external links
        if (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:')) {
            event.preventDefault();
            open(href).catch(err => {
                console.error(`[+layout.svelte] Failed to open external link ${href}:`, err);
            });
        }
    }

    onMount(async () => {
      document.addEventListener('click', handleGlobalClick, true);

      console.debug('[+layout.svelte] onMount started.'); // DEBUG
      // --- Load theme preference ---
      try {
         console.debug('[+layout.svelte] Attempting to load theme preference...'); // DEBUG
         await loadThemePreferenceFromBackend();
         await updateConfigStatus(); // Check config status on app load
         console.info('[+layout.svelte] Theme preference loaded successfully (or default used).'); // INFO
      } catch (error) {
          console.error('[+layout.svelte] Failed during theme preference loading:', error); // ERROR
      }
  
      // --- Load Project Data ---
      const xmlPathFromUrl = $page.url.searchParams.get('xmlPath');
      if (xmlPathFromUrl && xmlPathFromUrl.trim() !== '') {
          try {
              const normalizedXmlPath = normalizePath(xmlPathFromUrl);
              console.info(`[+layout.svelte] Attempting to load project data for: ${normalizedXmlPath}`); // INFO
              await loadProjectDataAndUpdateStore(normalizedXmlPath);
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
              }
               else if (!loadedProjectState.selectedMediaFile.name) {
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
          // Explicitly set isLoading to false for the welcome page
          project.update(p => ({ ...p, isLoading: false, statusMessage: 'Welcome' }));
      }
  
      
    });

    onDestroy(() => {
        if (typeof document !== 'undefined') {
            document.removeEventListener('click', handleGlobalClick, true);
        }
    });
  
</script>
  
  <!-- This slot renders the content of the current page (+page.svelte) -->
  <slot />
  <ToastNotifications />
<HeaderConfirmationModal />

{#if $project.isLoading && $page.url.pathname !== '/about' && $page.url.pathname !== '/configurations' && $page.url.pathname !== '/license' && $page.url.pathname !== '/credits' && $page.url.pathname !== '/version'}
    <div class="absolute inset-0 z-[120] flex items-center justify-center bg-black/30 backdrop-blur-sm">
        <div class="flex flex-col items-center p-6 bg-white dark:bg-gray-900 rounded-lg shadow-xl">
             <Loader class="w-12 h-12 text-blue-500 animate-spin mb-3" />
             <p class="text-sm text-gray-700 dark:text-gray-400">{$project.statusMessage || 'Loading project...'}</p>
        </div>
    </div>
{/if}
