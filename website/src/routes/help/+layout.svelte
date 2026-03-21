<script>
    import { page } from '$app/stores';
    import { base } from '$app/paths';
    import { onMount } from 'svelte';
    import { Book, Cog, Database, Mic, Tag, Edit, Languages, MessageSquare, Search, X } from '@lucide/svelte';

    // Sidebar navigation structure (must match desktop app order)
    const sidebarTabs = [
        { id: 'overview', label: 'Overview', icon: Book },
        { id: 'configure', label: 'Configure', icon: Cog },
        { id: 'manage-data', label: 'Data', icon: Database },
        { id: 'transcribe', label: 'Transcription', icon: Mic },
        { id: 'tags', label: 'Tags', icon: Tag },
        { id: 'annotate-page', label: 'Annotate', icon: Edit },
        { id: 'translate-page', label: 'Translate', icon: Languages },
        { id: 'report-issue', label: 'Report Issue', icon: MessageSquare }
    ];

    export let data;
    $: ({ articles } = data);

    // Active tab logic
    $: activeTabId = $page.data.meta?.sidebarId || 'overview';

    // Search logic
    let searchQuery = '';
    let isSearchOpen = false;
    let searchInput;

    $: filteredArticles = searchQuery.trim() === ''
        ? []
        : articles.filter(a => 
            a.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
            a.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
            a.slug.toLowerCase().includes(searchQuery.toLowerCase())
        ).slice(0, 8);

    function closeSearch() {
        isSearchOpen = false;
        searchQuery = '';
    }

    onMount(() => {
        const handleKeydown = (e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                searchInput?.focus();
                isSearchOpen = true;
            }
            if (e.key === 'Escape') {
                closeSearch();
            }
        };

        const handleClickOutside = (e) => {
            if (isSearchOpen && !e.target.closest('.search-container')) {
                closeSearch();
            }
        };

        window.addEventListener('keydown', handleKeydown);
        window.addEventListener('click', handleClickOutside);
        return () => {
            window.removeEventListener('keydown', handleKeydown);
            window.removeEventListener('click', handleClickOutside);
        };
    });
</script>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col md:flex-row gap-8">
    <!-- Sidebar -->
    <aside class="w-full md:w-64 flex-shrink-0">
        <div class="sticky top-24 space-y-6">
            <!-- Search Bar -->
            <div class="relative search-container">
                <div class="relative group">
                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                        <Search class="h-4 w-4 text-slate-400 group-focus-within:text-green-500 transition-colors" />
                    </div>
                    <input
                        bind:this={searchInput}
                        type="text"
                        bind:value={searchQuery}
                        on:focus={() => isSearchOpen = true}
                        placeholder="Search documentation..."
                        class="block w-full pl-10 pr-12 py-2 text-sm bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-green-500/20 focus:border-green-500 transition-all"
                    />
                    <div class="absolute inset-y-0 right-0 flex items-center pr-3 pointer-events-none">
                        <kbd class="hidden sm:inline-flex items-center px-2 py-0.5 rounded border border-slate-200 bg-slate-50 text-[10px] font-medium text-slate-400">
                            ⌘K
                        </kbd>
                    </div>
                </div>

                <!-- Search Results -->
                {#if isSearchOpen && searchQuery.trim() !== ''}
                    <div class="absolute top-full mt-2 w-full md:w-80 bg-white border border-slate-200 rounded-xl shadow-xl z-50 overflow-hidden py-2 animate-in fade-in slide-in-from-top-1 duration-200">
                        {#if filteredArticles.length > 0}
                            <div class="px-3 py-1 text-[10px] font-bold text-slate-400 uppercase tracking-wider">
                                Articles
                            </div>
                            {#each filteredArticles as article}
                                <a
                                    href="{base}/help/{article.slug}"
                                    class="flex flex-col px-4 py-2 hover:bg-green-50 transition-colors group"
                                    on:click={closeSearch}
                                >
                                    <span class="text-sm font-semibold text-slate-700 group-hover:text-green-700">
                                        {article.title}
                                    </span>
                                    {#if article.description}
                                        <span class="text-xs text-slate-400 line-clamp-1">
                                            {article.description}
                                        </span>
                                    {/if}
                                </a>
                            {/each}
                        {:else}
                            <div class="px-4 py-6 text-center">
                                <p class="text-sm text-slate-500">No results for "{searchQuery}"</p>
                            </div>
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- Navigation -->
            <nav class="space-y-1">
                {#each sidebarTabs as tab}
                    <a
                        href="{base}/help/{tab.id}"
                        class="group flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors {activeTabId === tab.id ? 'bg-green-50 text-green-700' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                    >
                        <svelte:component
                            this={tab.icon}
                            class="flex-shrink-0 -ml-1 mr-3 h-5 w-5 {activeTabId === tab.id ? 'text-green-500' : 'text-slate-400 group-hover:text-slate-500'}"
                        />
                        <span class="truncate">{tab.label}</span>
                    </a>
                {/each}
            </nav>
        </div>
    </aside>

    <!-- Content -->
    <div class="flex-1 min-w-0">
        <slot />
    </div>
</div>
