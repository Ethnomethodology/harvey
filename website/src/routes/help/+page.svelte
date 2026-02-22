<script>
    import { base } from '$app/paths';
    export let data;

    $: ({ overviewContent, overviewMeta, articles } = data);

    // This page acts as the "Overview" page by default
    const sidebarId = 'overview';
    // Filter out 'overview' from the list of cards since we are displaying it here
    $: displayedArticles = articles.filter(a => a.sidebarId === sidebarId && a.slug !== 'overview');
</script>

<div class="prose prose-slate max-w-none">
    <h1>{overviewMeta?.label || 'Help Center'}</h1>
    
    {#if overviewContent}
        <svelte:component this={overviewContent} />
    {:else}
        <p>
            Everything you need to work with Harvey. Browse the documentation or search for specific topics.
        </p>
    {/if}

    <div class="not-prose grid grid-cols-1 md:grid-cols-2 gap-6 mt-8">
        {#each displayedArticles as article}
            <a href="{base}/help/{article.slug}" class="block p-6 bg-white border border-slate-200 rounded-xl hover:border-green-500 hover:shadow-lg transition-all group h-full">
                <h3 class="text-lg font-bold text-slate-900 group-hover:text-green-600 mb-2 transition-colors">
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
