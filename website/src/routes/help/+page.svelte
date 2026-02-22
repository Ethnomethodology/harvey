<script>
    import { base } from '$app/paths';
    export let data;

    // Group articles by sidebarId
    const sidebarTabs = [
        { id: 'overview', label: 'Overview' },
        { id: 'configure', label: 'Configure' },
        { id: 'manage-data', label: 'Data' },
        { id: 'transcribe', label: 'Transcription' },
        { id: 'tags', label: 'Tags' },
        { id: 'annotate-page', label: 'Annotate' },
        { id: 'translate-page', label: 'Translate' },
        { id: 'report-issue', label: 'Report Issue' }
    ];

    $: groupedArticles = sidebarTabs.map(tab => ({
        ...tab,
        articles: data.articles.filter(a => a.sidebarId === tab.id)
    })).filter(group => group.articles.length > 0);

</script>

<div class="max-w-7xl mx-auto py-12">
    <div class="text-center mb-16">
        <h1 class="text-4xl font-bold mb-4 text-slate-900 tracking-tight">Help Center</h1>
        <p class="text-lg text-slate-600 max-w-2xl mx-auto">
            Everything you need to work with Harvey. Browse by category below.
        </p>
    </div>

    <div class="space-y-16">
        {#each groupedArticles as group}
            <section id="{group.id}" class="scroll-mt-24">
                <h2 class="text-2xl font-bold text-slate-900 mb-6 border-b border-slate-200 pb-2">{group.label}</h2>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {#each group.articles as article}
                        <a href="{base}/help/{article.slug}" class="block p-6 bg-white border border-slate-200 rounded-xl hover:border-emerald-500 hover:shadow-lg transition-all group h-full">
                            <h3 class="text-lg font-bold text-slate-900 group-hover:text-emerald-600 mb-2 transition-colors">
                                {article.title}
                            </h3>
                            {#if article.description}
                                <p class="text-slate-500 text-sm line-clamp-3">
                                    {article.description}
                                </p>
                            {/if}
                        </a>
                    {/each}
                </div>
            </section>
        {/each}
    </div>
</div>
