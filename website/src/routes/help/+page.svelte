<script>
    import { base } from '$app/paths';
    export let data;
</script>

<div class="max-w-4xl mx-auto py-12">
    <h1 class="text-4xl font-bold mb-8 text-slate-900 tracking-tight">Help Center</h1>
    <p class="text-lg text-slate-600 mb-12 max-w-2xl">
        Guides and tutorials to help you get the most out of Harvey.
    </p>

    <div class="grid gap-6">
        {#each data.articles as article}
            <a href="{base}/help/{article.slug}" class="block p-6 bg-white border border-slate-200 rounded-xl hover:border-emerald-500 hover:shadow-lg transition-all group">
                <h2 class="text-xl font-bold text-slate-900 group-hover:text-emerald-600 mb-2 transition-colors">
                    {article.title}
                </h2>
                {#if article.description}
                    <p class="text-slate-500">
                        {article.description}
                    </p>
                {/if}
            </a>
        {/each}
    </div>
</div>
