<!-- src/lib/components/projectview/shared/SimpleTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { isLexicalEditMode } from '$lib/stores/mediaEditorStore.js';
    import { createEventDispatcher } from 'svelte';
    import { Sun, Moon, Monitor, Pencil, PencilOff } from '@lucide/svelte';
    import { Button } from 'flowbite-svelte';
    import panelStateStore from '$lib/stores/panelStateStore.js';

    const dispatch = createEventDispatcher();

    // --- Theme Icons ---
	$: currentThemeName = $themePreference.charAt(0).toUpperCase() + $themePreference.slice(1);
    $: nextThemeName = $themePreference === 'light' ? 'Dark'
                     : $themePreference === 'dark' ? 'System'
                     : 'Light';
	$: themeTitle = `Current theme: ${currentThemeName}. Switch to ${nextThemeName} mode.`;

    let displayTitle = '';

    $: {
        if ($project && $project.name) {
            displayTitle = $project.name;
        } else {
            displayTitle = 'Harvey';
        }
    }
</script>

<div
  class="flex items-center h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800 relative z-30"
>
    <!-- Drag Handle Background -->
    <div class="absolute inset-0 z-0" data-tauri-drag-region></div>

    <!-- Section 1: Left Bar (w-12) — Import button -->
    <div class="w-12 flex-shrink-0 flex items-center justify-center z-10">
        <button
            type="button"
            class="p-1.5 rounded-full border-0 bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400 hover:bg-blue-200 dark:hover:bg-blue-500/30 transition-all duration-200 active:scale-95 shadow-sm hover:shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            on:click={(e) => dispatch('requestImport', e)}
            title="Import Audio or Video"
            aria-label="Import Audio or Video"
        >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
        </button>
    </div>

    <!-- Section 2: Left Panel (w-64) — Project name -->
    <div class="w-64 flex-shrink-0 flex items-center overflow-hidden z-10 transition-all duration-300 ease-in-out px-2">
        <span class="font-semibold text-sm text-gray-700 dark:text-gray-200 truncate" title={displayTitle}>{displayTitle}</span>
    </div>

    <!-- Section 3: Middle Panel (flex-grow) -->
    <div class="flex-grow flex items-center min-w-0 z-10 px-2 justify-between">
        <div class="flex items-center space-x-1.5">
            <!-- Left cluster empty for SimpleTopBar normally -->
        </div>
        <div class="flex items-center space-x-1.5">
            <button
                id="read-edit-toggle-simple"
                on:click={() => isLexicalEditMode.set(!$isLexicalEditMode)}
                class="px-2.5 py-1.5 rounded-full border-0 transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 flex items-center space-x-1.5 {$isLexicalEditMode ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400' : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10'}"
                title={$isLexicalEditMode ? "Switch to Read Mode" : "Switch to Edit Mode"}
            >
                {#if $isLexicalEditMode}
                    <Pencil class="w-3.5 h-3.5 text-blue-600 dark:text-blue-400" />
                    <span class="text-xs font-medium text-blue-600 dark:text-blue-400">Edit Mode</span>
                {:else}
                    <PencilOff class="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
                    <span class="text-xs font-medium text-gray-500 dark:text-gray-400">Read Mode</span>
                {/if}
            </button>
        </div>
    </div>

    <!-- Section 4: Right Bar (w-8) — Theme toggle -->
    <div class="w-8 flex-shrink-0 flex items-center justify-center z-10">
        <button
            on:click="{() => cycleThemePreference()}"
            class="p-1 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
            title="{themeTitle}"
            aria-label="{themeTitle}"
        >
            {#if $themePreference === 'light'}
                <Sun class="w-4 h-4" />
            {:else if $themePreference === 'dark'}
                <Moon class="w-4 h-4" />
            {:else}
                <Monitor class="w-4 h-4" />
            {/if}
        </button>
    </div>
</div>

<style lang="postcss">
    :global(.hover-scale-effect) {
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>
