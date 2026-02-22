<script>
    import { base } from '$app/paths';
    export let data;

    // Filter to get only "Section Roots" to display on the main Help page
    // We want: Download, Configure, Data, Transcription, Tags, Annotate, Translate, Report Issue
    // These correspond to the sidebarTabs IDs.
    const rootIds = [
        'download', 'configure', 'manage-data', 'transcribe',
        'tags', 'annotate-page', 'translate-page', 'report-issue'
    ];

    $: articles = data.articles.filter(a => rootIds.includes(a.slug));
</script>

<div class="prose prose-slate max-w-none">
    <h1>Help Center</h1>
    <p>
        Everything you need to work with Harvey. Browse the documentation or search for specific topics.
    </p>

    <div class="not-prose grid grid-cols-1 md:grid-cols-2 gap-6 mt-8">
        {#each articles as article}
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
</div>
