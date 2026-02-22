<script>
    import { page } from '$app/stores';
    import { base } from '$app/paths';
    import { Download, Cog, Database, Mic, Tag, Edit, Languages, MessageSquare } from 'lucide-svelte';

    // Sidebar navigation structure (must match desktop app order)
    const sidebarTabs = [
        { id: 'download', label: 'Download', icon: Download },
        { id: 'configure', label: 'Configure', icon: Cog },
        { id: 'manage-data', label: 'Data', icon: Database },
        { id: 'transcribe', label: 'Transcription', icon: Mic },
        { id: 'tags', label: 'Tags', icon: Tag },
        { id: 'annotate-page', label: 'Annotate', icon: Edit },
        { id: 'translate-page', label: 'Translate', icon: Languages },
        { id: 'report-issue', label: 'Report Issue', icon: MessageSquare }
    ];

    export let data;

    // Active tab logic
    $: activeTabId = $page.data.meta?.sidebarId || 'download';
</script>

<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col md:flex-row gap-8">
    <!-- Sidebar -->
    <aside class="w-full md:w-64 flex-shrink-0">
        <nav class="sticky top-24 space-y-1">
            {#each sidebarTabs as tab}
                <a
                    href="{base}/help/{tab.id}"
                    class="group flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors {activeTabId === tab.id ? 'bg-emerald-50 text-emerald-700' : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                >
                    <svelte:component
                        this={tab.icon}
                        class="flex-shrink-0 -ml-1 mr-3 h-5 w-5 {activeTabId === tab.id ? 'text-emerald-500' : 'text-slate-400 group-hover:text-slate-500'}"
                    />
                    <span class="truncate">{tab.label}</span>
                </a>
            {/each}
        </nav>
    </aside>

    <!-- Content -->
    <div class="flex-1 min-w-0">
        <slot />
    </div>
</div>
