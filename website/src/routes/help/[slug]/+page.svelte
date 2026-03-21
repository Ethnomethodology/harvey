<script>
    import { base } from '$app/paths';
    import { ArrowLeft, ArrowRight } from '@lucide/svelte';

    export let data;

    // Use passed data
    $: prevArticle = data.prev;
    $: nextArticle = data.next;

    // Find child articles for the current page (e.g., if we are on 'manage-data', show cards for 'audio', 'video', etc.)
    // Logic:
    // 1. Current page acts as a "Section Parent" if other articles share its `sidebarId` AND have a higher order (or just exist in the same group).
    // 2. However, sidebarId grouping is broad. The specific structure in HelpModal was flat but grouped visually.
    // 3. The markdown files don't strictly have a "parent" field. But we know the structure from `HelpModal.svelte` arrays.
    // 4. Heuristic: If this page matches a `sidebarId` root (e.g. `configure` matches `sidebarId: configure`), then show all OTHER articles with that `sidebarId`.

    $: childArticles = data.articles.filter(a =>
        a.sidebarId === data.meta.id && // The current page's ID matches a sidebar category ID
        a.slug !== data.meta.id // Exclude self
    );

    // Fallback: If data.meta.id is not exactly the sidebarId, we might be on a sub-page.
    // But the requirement is specifically for the "Overview" style pages which ARE the category roots.
    // So checking `a.sidebarId === data.meta.id` is correct because `configure.md` has `id: configure`.
</script>

<div class="space-y-8">
    <!-- Header Navigation (Simple) -->
    <div class="flex items-center justify-between border-b border-slate-200 pb-4">
        {#if prevArticle}
            <a href="{base}/help/{prevArticle.slug}" class="flex items-center gap-2 text-sm font-medium text-slate-500 hover:text-green-600 transition-colors">
                <ArrowLeft class="w-4 h-4" />
                <span class="hidden sm:inline">Previous</span>
            </a>
        {:else}
            <div class="text-sm font-medium text-slate-300 select-none">First Topic</div>
        {/if}

        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider hidden sm:block">
            {data.meta.sidebarId?.replace('-', ' ') || 'Overview'}
        </span>

        {#if nextArticle}
            <a href="{base}/help/{nextArticle.slug}" class="flex items-center gap-2 text-sm font-medium text-slate-500 hover:text-green-600 transition-colors">
                <span class="hidden sm:inline">Next</span>
                <ArrowRight class="w-4 h-4" />
            </a>
        {:else}
            <div class="text-sm font-medium text-slate-300 select-none">Last Topic</div>
        {/if}
    </div>

    <!-- Main Content -->
    <article class="prose prose-slate prose-lg max-w-none prose-headings:font-bold prose-headings:tracking-tight prose-a:text-green-600 hover:prose-a:text-green-700 prose-img:rounded-2xl">
        <h1 class="mb-4 text-slate-900">{data.meta.label || data.meta.title}</h1>

        {#if data.meta.description && data.meta.description !== data.meta.title && data.meta.description !== data.meta.label}
            <p class="lead text-xl text-slate-600 mt-0 mb-8">{data.meta.description}</p>
        {/if}

        <div class="mt-8">
            <svelte:component this={data.content} />
        </div>
    </article>

    <!-- Child Cards (Sub-pages) -->
    {#if childArticles.length > 0}
        <div class="not-prose grid grid-cols-1 md:grid-cols-2 gap-4 mt-12 border-t border-slate-100 pt-8">
            {#each childArticles as article}
                <a href="{base}/help/{article.slug}" class="block p-6 bg-white border border-slate-200 rounded-xl hover:border-green-500 hover:shadow-md transition-all group h-full">
                    <h3 class="text-lg font-bold text-slate-900 group-hover:text-green-600 mb-2 transition-colors">
                        {article.title}
                    </h3>
                    {#if article.description}
                        <p class="text-slate-500 text-sm line-clamp-2">
                            {article.description}
                        </p>
                    {/if}
                </a>
            {/each}
        </div>
    {/if}

    <!-- Footer Navigation (Detailed Cards) -->
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 border-t border-slate-200 pt-8 mt-12">
        {#if prevArticle}
            <a href="{base}/help/{prevArticle.slug}" class="group flex flex-col items-start justify-center gap-1 p-4 rounded-xl border border-slate-200 hover:border-green-300 hover:bg-green-50/30 transition-all text-left h-24">
                <span class="text-xs font-bold text-slate-400 uppercase tracking-wider group-hover:text-green-600 flex items-center gap-1 mb-1">
                    <ArrowLeft class="w-3 h-3" /> Previous
                </span>
                <span class="text-lg font-bold text-slate-800 group-hover:text-green-900 line-clamp-2 leading-tight w-full">
                    {prevArticle.title}
                </span>
            </a>
        {:else}
             <div class="hidden sm:block"></div>
        {/if}

        {#if nextArticle}
            <a href="{base}/help/{nextArticle.slug}" class="group flex flex-col items-end justify-center gap-1 p-4 rounded-xl border border-slate-200 hover:border-green-300 hover:bg-green-50/30 transition-all text-right h-24">
                <span class="text-xs font-bold text-slate-400 uppercase tracking-wider group-hover:text-green-600 flex items-center gap-1 mb-1">
                    Next <ArrowRight class="w-3 h-3" />
                </span>
                <span class="text-lg font-bold text-slate-800 group-hover:text-green-900 line-clamp-2 leading-tight w-full">
                    {nextArticle.title}
                </span>
            </a>
        {:else}
            <div class="hidden sm:block"></div>
        {/if}
    </div>
</div>
