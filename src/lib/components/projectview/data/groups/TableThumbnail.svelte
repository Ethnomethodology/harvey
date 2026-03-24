<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { Sheet, Database } from '@lucide/svelte';

    export let file;

    let headers = [];
    let rows = [];
    let isLoading = true;
    let error = false;

    async function loadPreview() {
        if (!file || !file.full_path) {
            isLoading = false;
            error = true;
            return;
        }

        try {
            const tableData = await invoke('load_table_data', {
                tablePathStr: file.full_path,
                hasHeaders: true
            });

            if (tableData && Array.isArray(tableData.headers) && Array.isArray(tableData.data)) {
                // Slice data to only show a small thumbnail preview (e.g. 4x5)
                headers = tableData.headers.slice(0, 4).map(h => h.value || h.name || h.field || String(h));
                rows = tableData.data.slice(0, 5).map(rowObj => {
                    return headers.map(header => {
                        let val = rowObj[header];
                        if (val === null || val === undefined) return '';
                        if (typeof val === 'string') return val.replace(/\r/g, '').trim();
                        return String(val);
                    });
                });
            } else {
                error = true;
            }
        } catch (e) {
            console.error("Failed to load table thumbnail:", e);
            error = true;
        } finally {
            isLoading = false;
        }
    }

    onMount(() => {
        loadPreview();
    });
</script>

<div class="w-full h-full relative overflow-hidden bg-white dark:bg-gray-900 border border-gray-100 dark:border-gray-800 rounded-[10px] flex items-center justify-center p-2 group-hover:bg-blue-50/30 dark:group-hover:bg-blue-900/10 transition-colors">
    {#if isLoading}
        <div class="animate-pulse flex flex-col w-full h-full space-y-1">
            <div class="h-4 bg-gray-200 dark:bg-gray-800 rounded w-full"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-800 rounded w-[90%]"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-800 rounded w-full"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-800 rounded w-[80%]"></div>
        </div>
    {:else if error || headers.length === 0}
        <div class="flex items-center justify-center opacity-40 transition-transform duration-300 group-hover:scale-110">
            <Sheet class="w-12 h-12 text-gray-400 dark:text-gray-500" />
        </div>
    {:else}
        <div class="absolute inset-0 flex items-start justify-start p-1.5 opacity-90 transition-transform duration-300 origin-top-left group-hover:scale-[1.03] w-full">
            <table class="w-full text-left table-fixed border-collapse">
                <thead>
                    <tr>
                        <th class="w-4 h-4 border border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800/50"></th>
                        {#each headers as header}
                            <th class="border border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800/50 px-1 py-0.5 text-[8px] font-medium text-gray-600 dark:text-gray-400 truncate max-w-[40px]">
                                {header}
                            </th>
                        {/each}
                    </tr>
                </thead>
                <tbody>
                    {#each rows as row, i}
                        <tr>
                            <td class="border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/20 text-center text-[7px] text-gray-400 dark:text-gray-500 font-medium">
                                {i + 1}
                            </td>
                            {#each row as cell}
                                <td class="border border-gray-100 dark:border-gray-800 px-1 py-0.5 text-[8px] text-gray-700 dark:text-gray-300 truncate max-w-[40px]">
                                    {cell}
                                </td>
                            {/each}
                        </tr>
                    {/each}
                </tbody>
            </table>
            <!-- Fade overlay for bottom edge -->
            <div class="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-white dark:from-gray-900 to-transparent pointer-events-none"></div>
        </div>
    {/if}
</div>
