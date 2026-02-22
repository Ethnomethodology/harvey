<script>
    export let data;
</script>

<article class="prose prose-slate prose-lg max-w-none prose-headings:font-bold prose-headings:tracking-tight prose-a:text-emerald-600 prose-img:rounded-2xl">
    <svelte:component this={data.content} />
</article>
