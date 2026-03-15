<!-- src/lib/components/modals/HelpModal.svelte -->
<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { Search, X, ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { Modal } from 'flowbite-svelte';

  export let showModal = false;
  export let isCompact = false; // New prop to control sidebar width

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
              description: metadata.description || '',
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

  // --- Logic for Child Cards in App ---
  // If current page is a Category Root (like 'configure'), find all its children to display as cards
  // A Category Root usually shares the ID with the sidebarId (e.g. id='configure', sidebarId='configure')
  $: childPages = allPages.filter(p =>
      p.sidebarId === currentPage?.id && // Child belongs to this sidebar section
      p.id !== currentPage?.id // Not the page itself
  );

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

      // Handle hash links (legacy support or internal page anchors)
      if (href && href.startsWith('#')) {
        event.preventDefault();
        const targetId = href.substring(1);
        const targetPage = allPages.find(p => p.id === targetId);
        if (targetPage) {
            navigateTo(targetId);
        }
      }
      // Handle absolute paths used in website (e.g., /help/config-app)
      else if (href && href.startsWith('/help/')) {
        event.preventDefault();
        const slug = href.replace('/help/', '');
        // Find page by ID (which usually matches slug)
        const targetPage = allPages.find(p => p.id === slug);
        if (targetPage) {
            navigateTo(slug);
        } else {
            console.warn(`[HelpModal] Target page not found for slug: ${slug}`);
        }
      }
      // Handle relative paths (e.g., [Configure](configure)) for compatibility
      else if (href && !href.startsWith('http') && !href.startsWith('/')) {
         event.preventDefault();
         // Basic relative link handling: assume it's just the slug
         const slug = href;
         const targetPage = allPages.find(p => p.id === slug);
         if (targetPage) {
             navigateTo(slug);
         } else {
             console.warn(`[HelpModal] Target page not found for relative slug: ${slug}`);
         }
      }
    }
  }


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

<svelte:window on:keydown={handleKeydown} />

<Modal
  bind:open={showModal}
  outsideclose
  placement="center"
  size="xl"
  on:close={close}
  backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
  dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
  class="h-[80vh] flex flex-col p-0 overflow-hidden mx-auto"
  bodyClass="flex-grow overflow-y-auto bg-white dark:bg-gray-900 p-0"
  headerClass="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
>
      <!-- Main Modal Header -->
      <svelte:fragment slot="header">
        <div class="flex items-center justify-between flex-grow mr-8">
            <div class="flex items-center space-x-3">
                <img src="/logo.png" alt="Harvey Logo" class="w-8 h-8 rounded-lg" />
                <h3 id="help-modal-title" class="text-lg font-bold text-gray-900 dark:text-white truncate">Help Center</h3>
            </div>

            <div class="flex items-center space-x-4">
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
                        <Search class="h-4 w-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
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
            </div>
        </div>
      </svelte:fragment>

      <!-- Main Body -->
      <div class="flex flex-grow overflow-hidden h-full">
        <!-- Sidebar -->
        <div class="{isCompact ? 'w-32' : 'w-64'} bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 {isCompact ? 'p-2' : 'p-4'} flex flex-col flex-shrink-0 overflow-y-auto font-sans transition-all duration-300">
          <nav class="flex flex-col space-y-1">
            {#each sidebarTabs as tab}
              <button
                class="{isCompact ? 'px-2 py-1.5 text-xs truncate' : 'px-4 py-2 text-sm'} rounded-md text-left transition-all duration-200 {currentPage.sidebarId === tab.id
                  ? 'bg-blue-600 text-white font-medium shadow-sm' 
                  : 'text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-gray-100'}"
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
                <ChevronLeft class="w-4 h-4 flex-shrink-0" />
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <ChevronRight class="w-4 h-4 flex-shrink-0" />
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
                <!-- Render title manually here since we removed it from MD files -->
                <h1 class="mb-4">{currentPage.label}</h1>
                <svelte:component this={currentPage.component} />

                <!-- Child Pages Grid (Sub-categories) -->
                {#if childPages && childPages.length > 0}
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-8 not-prose">
                        {#each childPages as child}
                            <button
                                on:click={() => navigateTo(child.id)}
                                class="flex flex-col p-6 text-left border border-gray-200 dark:border-gray-700 rounded-xl hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group bg-gray-50/50 dark:bg-gray-800/30"
                            >
                                <span class="text-xl font-bold mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 text-gray-800 dark:text-gray-100">{child.label}</span>
                                {#if child.description}
                                    <p class="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">{child.description}</p>
                                {/if}
                            </button>
                        {/each}
                    </div>
                {/if}

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
                <ChevronLeft class="w-4 h-4 flex-shrink-0" />
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <ChevronRight class="w-4 h-4 flex-shrink-0" />
            </button>
          </div>
        </div>
      </div>
</Modal>

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
</style>
