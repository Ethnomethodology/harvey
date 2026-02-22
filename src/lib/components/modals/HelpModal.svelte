<!-- src/lib/components/modals/HelpModal.svelte -->
<script>
  import { createEventDispatcher, onMount } from 'svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  // Platform detection for modifiers (optional, but good for dynamic text if needed)
  const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
  const modKeyName = isMac ? 'Cmd' : 'Ctrl';

  // --- Dynamic Content Loading ---
  // Load all markdown files from the synced directory
  const modules = import.meta.glob('/src/content/help/*.md', { eager: true });

  let allPages = [];
  let sidebarTabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'configure', label: 'Configure' },
    { id: 'manage-data', label: 'Data' },
    { id: 'transcribe', label: 'Transcription' },
    { id: 'tags', label: 'Tags' },
    { id: 'annotate-page', label: 'Annotate' },
    { id: 'translate-page', label: 'Translate' },
    { id: 'report-issue', label: 'Report Issue' }
  ];

  // Process loaded modules into the allPages array
  for (const path in modules) {
      const mod = modules[path];
      const metadata = mod.metadata || {};

      if (metadata.id) {
          allPages.push({
              id: metadata.id,
              label: metadata.label || metadata.title || metadata.id,
              sidebarId: metadata.sidebarId || 'overview',
              order: metadata.order || 999,
              component: mod.default // The Svelte component for the markdown content
          });
      }
  }

  // Sort pages by order
  allPages.sort((a, b) => a.order - b.order);

  // --- Navigation Logic ---
  let currentPageId = 'overview';
  let searchQuery = '';
  let showSearchResults = false;

  $: filteredResults = searchQuery.trim() === '' 
    ? [] 
    : allPages.filter(p => p.label.toLowerCase().includes(searchQuery.toLowerCase()));

  $: currentIndex = allPages.findIndex(p => p.id === currentPageId);
  $: currentPage = allPages[currentIndex] || allPages[0]; // Fallback to first page
  $: prevPage = currentIndex > 0 ? allPages[currentIndex - 1] : null;
  $: nextPage = currentIndex < allPages.length - 1 ? allPages[currentIndex + 1] : null;

  function close() {
    dispatch('close');
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      if (showSearchResults) {
        showSearchResults = false;
      } else {
          // Optional: Close modal on Esc if not searching
          // close();
      }
    }
  }

  function navigateTo(pageId) {
    currentPageId = pageId;
    searchQuery = '';
    showSearchResults = false;
  }

  function goToPrev() {
    if (prevPage) navigateTo(prevPage.id);
  }

  function goToNext() {
    if (nextPage) navigateTo(nextPage.id);
  }

  // Helper to intercept link clicks in markdown content
  function handleContentClick(event) {
    const anchor = event.target.closest('a');
    if (anchor) {
      const href = anchor.getAttribute('href');
      if (href && href.startsWith('#')) {
        event.preventDefault();
        const targetId = href.substring(1);
        // Find page with this ID
        const targetPage = allPages.find(p => p.id === targetId);
        if (targetPage) {
            navigateTo(targetId);
        }
      }
    }
  }

  const ICON_PREV = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-compact-left" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M9.224 1.553a.5.5 0 0 1 .223.67L6.56 8l2.888 5.776a.5.5 0 1 1-.894.448l-3-6a.5.5 0 0 1 0-.448l3-6a.5.5 0 0 1 .67-.223"/></svg>`;
  const ICON_NEXT = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-compact-right" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M6.776 1.553a.5.5 0 0 1 .671.223l3 6a.5.5 0 0 1 0 .448l-3 6a.5.5 0 1 1-.894-.448L9.44 8 6.553 2.224a.5.5 0 0 1 .223-.671"/></svg>`;

  onMount(() => {
    const listener = (e) => {
        if (!e.target.closest('.help-search-container')) {
            showSearchResults = false;
        }
    };
    document.addEventListener('click', listener);
    return () => document.removeEventListener('click', listener);
  });
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[150] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="help-modal-title"
    tabindex="-1"
  >
    <div class="bg-white dark:bg-gray-900 rounded-lg shadow-xl w-[80vw] h-[80vh] flex flex-col relative overflow-hidden" role="document">
      
      <!-- Main Modal Header -->
      <div class="flex-shrink-0 px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center bg-gray-50 dark:bg-gray-900">
        <div class="flex items-center space-x-3">
            <img src="/logo.png" alt="Harvey Logo" class="w-8 h-8 rounded-lg" />
            <h2 id="help-modal-title" class="text-xl font-bold text-gray-800 dark:text-white">Help Center</h2>
        </div>
        
        <div class="flex items-center space-x-4 flex-grow justify-end">
            <!-- Search Bar -->
            <div class="relative w-64 help-search-container">
                <div class="relative">
                    <input 
                        type="text" 
                        bind:value={searchQuery}
                        on:focus={() => showSearchResults = true}
                        placeholder="Search help..." 
                        autocomplete="off"
                        autocorrect="off"
                        autocapitalize="off"
                        spellcheck="false"
                        class="w-full pl-9 pr-4 py-1.5 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 dark:text-white transition-all"
                    />
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                </div>

                <!-- Search Results Dropdown -->
                {#if showSearchResults && filteredResults.length > 0}
                    <div class="absolute top-full mt-1 left-0 right-0 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl z-[160] py-1 max-h-64 overflow-y-auto">
                        {#each filteredResults as result}
                            <button 
                                class="w-full text-left px-4 py-2 text-sm hover:bg-blue-50 dark:hover:bg-blue-900/30 dark:text-gray-200 transition-colors flex items-center justify-between group"
                                on:click={() => navigateTo(result.id)}
                            >
                                <span>{result.label}</span>
                                <span class="text-[10px] text-gray-400 group-hover:text-blue-500 uppercase tracking-wider">{result.sidebarId}</span>
                            </button>
                        {/each}
                    </div>
                {:else if showSearchResults && searchQuery.trim() !== ''}
                    <div class="absolute top-full mt-1 left-0 right-0 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl z-[160] p-4 text-center">
                        <p class="text-sm text-gray-500">No results found for "{searchQuery}"</p>
                    </div>
                {/if}
            </div>

            <button 
                on:click={close} 
                aria-label="Close" 
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-400 rounded-full p-1 transition-colors"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>
      </div>

      <!-- Main Body -->
      <div class="flex flex-grow overflow-hidden">
        <!-- Sidebar -->
        <div class="w-64 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 p-4 flex flex-col flex-shrink-0 overflow-y-auto font-sans">
          <nav class="flex flex-col space-y-1">
            {#each sidebarTabs as tab}
              <button
                class="px-4 py-2 rounded-md text-left transition-all duration-200 {currentPage.sidebarId === tab.id 
                  ? 'bg-blue-600 text-white font-medium shadow-sm' 
                  : 'text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-gray-100'} text-sm"
                on:click={() => navigateTo(tab.id)}
              >
                {tab.label}
              </button>
            {/each}
          </nav>
        </div>

        <!-- Content Area Container -->
        <div class="flex-grow flex flex-col overflow-hidden bg-white dark:bg-gray-900 font-sans">
          
          <!-- Inner Page Header (Navigation) -->
          <div class="flex-shrink-0 px-8 py-3 border-b border-gray-100 dark:border-gray-800 flex justify-between items-center bg-gray-50 dark:bg-gray-900 h-12">
            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToPrev}
                disabled={!prevPage}
            >
                <span class="flex-shrink-0">{@html ICON_PREV}</span>
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <span class="flex-shrink-0">{@html ICON_NEXT}</span>
            </button>
          </div>

          <!-- Scrollable Page Content -->
          <!-- We use a generic container for the dynamic markdown component -->
          <!-- We apply 'prose' (tailwindcss-typography) to style the markdown content -->
          <div
            class="flex-grow overflow-y-auto p-8 text-gray-700 dark:text-gray-300 prose dark:prose-invert prose-slate max-w-none prose-headings:font-bold prose-headings:tracking-tight prose-a:text-blue-600 hover:prose-a:text-blue-700 prose-img:rounded-xl"
            on:click={handleContentClick}
          >
            {#if currentPage && currentPage.component}
                <svelte:component this={currentPage.component} />
            {:else}
                <div class="flex items-center justify-center h-full text-gray-500">
                    <p>Content not found.</p>
                </div>
            {/if}
          </div>

          <!-- Inner Page Footer (Navigation) -->
          <div class="flex-shrink-0 px-8 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-between items-center bg-gray-50 dark:bg-gray-900 h-14">
            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToPrev}
                disabled={!prevPage}
            >
                <span class="flex-shrink-0">{@html ICON_PREV}</span>
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <span class="flex-shrink-0">{@html ICON_NEXT}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Custom scrollbar for Webkit browsers */
  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }
  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: rgba(156, 163, 175, 0.3);
    border-radius: 10px;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background-color: rgba(156, 163, 175, 0.5);
  }

  /* Kbd styling match (Tailwind prose might handle this but good to have) */
  :global(kbd) {
    display: inline-block;
    padding: 0.125rem 0.375rem;
    font-size: 0.75rem;
    line-height: 1;
    color: #4b5563;
    vertical-align: middle;
    background-color: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 0.25rem;
    box-shadow: 0 1px 0 rgba(0,0,0,0.2), inset 0 0 0 1px #fff;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  }

  :global(.dark kbd) {
    color: #e5e7eb;
    background-color: #374151;
    border-color: #4b5563;
    box-shadow: 0 1px 0 rgba(0,0,0,0.5), inset 0 0 0 1px #444;
  }

  /* Additional override for grid layouts in markdown (e.g. Overview cards) */
  /* We might need to write custom CSS or component logic if we want those grid cards back exactly as they were. */
  /* For now, markdown will render them as standard headings/lists/links. */
  /* To support the grid layout again, we would need custom mdsvex components or a specific layout component. */
</style>
