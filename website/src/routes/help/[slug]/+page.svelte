<script>
    import { base } from '$app/paths';
    import { ArrowLeft, ArrowRight } from 'lucide-svelte';

    export let data;

    // Use passed data
    $: prevArticle = data.prev;
    $: nextArticle = data.next;

    // Fallback if data.prev/next are not passed directly (though they should be)
    $: activeTabId = data.meta.sidebarId || 'overview';
</script>

<div class="space-y-8">
    <!-- Header Navigation (Simple) -->
    <div class="flex items-center justify-between border-b border-slate-200 pb-4">
        {#if prevArticle}
            <a href="{base}/help/{prevArticle.slug}" class="flex items-center gap-2 text-sm font-medium text-slate-500 hover:text-emerald-600 transition-colors">
                <ArrowLeft class="w-4 h-4" />
                <span class="hidden sm:inline">Previous Topic</span>
            </a>
        {:else}
            <div class="text-sm font-medium text-slate-300 select-none">First Topic</div>
        {/if}

        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider hidden sm:block">
            {data.meta.sidebarId?.replace('-', ' ') || 'Overview'}
        </span>

        {#if nextArticle}
            <a href="{base}/help/{nextArticle.slug}" class="flex items-center gap-2 text-sm font-medium text-slate-500 hover:text-emerald-600 transition-colors">
                <span class="hidden sm:inline">Next Topic</span>
                <ArrowRight class="w-4 h-4" />
            </a>
        {:else}
            <div class="text-sm font-medium text-slate-300 select-none">Last Topic</div>
        {/if}
    </div>

    <!-- Main Content -->
    <article class="prose prose-slate prose-lg max-w-none prose-headings:font-bold prose-headings:tracking-tight prose-a:text-emerald-600 hover:prose-a:text-emerald-700 prose-img:rounded-2xl">
        <h1 class="mb-4 text-slate-900">{data.meta.label || data.meta.title}</h1>
        <!-- Only show description if it's not the same as the title -->
        {#if data.meta.description && data.meta.description !== data.meta.title && data.meta.description !== data.meta.label}
            <p class="lead text-xl text-slate-600 mt-0 mb-8">{data.meta.description}</p>
        {/if}

        <div class="mt-8">
            <svelte:component this={data.content} />
        </div>
    </article>

    <!-- Footer Navigation (Detailed Cards) -->
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 border-t border-slate-200 pt-8 mt-12">
        {#if prevArticle}
            <a href="{base}/help/{prevArticle.slug}" class="group flex flex-col items-start justify-center gap-1 p-4 rounded-xl border border-slate-200 hover:border-emerald-300 hover:bg-emerald-50/30 transition-all text-left h-24">
                <span class="text-xs font-bold text-slate-400 uppercase tracking-wider group-hover:text-emerald-600 flex items-center gap-1 mb-1">
                    <ArrowLeft class="w-3 h-3" /> Previous
                </span>
                <span class="text-lg font-bold text-slate-800 group-hover:text-emerald-900 line-clamp-2 leading-tight w-full">
                    {prevArticle.title}
                </span>
            </a>
        {:else}
             <div class="hidden sm:block"></div>
        {/if}

        {#if nextArticle}
            <a href="{base}/help/{nextArticle.slug}" class="group flex flex-col items-end justify-center gap-1 p-4 rounded-xl border border-slate-200 hover:border-emerald-300 hover:bg-emerald-50/30 transition-all text-right h-24">
                <span class="text-xs font-bold text-slate-400 uppercase tracking-wider group-hover:text-emerald-600 flex items-center gap-1 mb-1">
                    Next <ArrowRight class="w-3 h-3" />
                </span>
                <span class="text-lg font-bold text-slate-800 group-hover:text-emerald-900 line-clamp-2 leading-tight w-full">
                    {nextArticle.title}
                </span>
            </a>
        {:else}
            <div class="hidden sm:block"></div>
        {/if}
    </div>
</div>
