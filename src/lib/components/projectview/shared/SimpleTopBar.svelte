<!-- src/lib/components/projectview/shared/SimpleTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { derived } from 'svelte/store';
    import { createEventDispatcher } from 'svelte';
    import { Sun, Moon, Monitor } from 'lucide-svelte';

    const dispatch = createEventDispatcher();

    // --- Theme Icons ---
    $: nextThemeName = $themePreference === 'light' ? 'Dark'
                     : $themePreference === 'dark' ? 'System'
                     : 'Light';
    $: themeTitle = `Switch to ${nextThemeName} Mode`;

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
  class="flex items-center justify-between px-1 h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800"
  data-tauri-drag-region
>
  <div class="flex items-center space-x-1.5 min-w-0">
        <div class="h-10 flex items-center justify-center flex-shrink-0">
            <button title="Import" aria-label="Import" class="ui-button-import hover-scale-effect ml-1 mr-1" on:click={(e) => dispatch('requestImport', e)}>
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
            </button>
        </div>

      <span class="font-semibold text-lg text-gray-700 dark:text-gray-200 truncate" title={displayTitle}>{displayTitle}</span>
  </div>

  <div class="flex items-center space-x-2 flex-shrink-0">
    <div class="w-px h-4 bg-gray-300 dark:bg-gray-700 mx-2"></div>
    <div class="flex-shrink-0">
		 <button on:click="{cycleThemePreference}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-blue-500/10 dark:hover:text-blue-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors transition-transform hover:scale-105" title="{themeTitle}">
            {#if $themePreference === 'light'}
                <Moon class="w-4 h-4" />
            {:else if $themePreference === 'dark'}
                <Monitor class="w-4 h-4" />
            {:else}
                <Sun class="w-4 h-4" />
            {/if}
		 </button>
	</div>
  </div>
</div>

<style lang="postcss">
    .ui-button-icon-no-border {
		@apply inline-flex items-center justify-center p-1.5 text-sm font-medium rounded-md text-gray-700 dark:text-white bg-transparent hover:bg-blue-100 dark:hover:bg-blue-700 disabled:hover:bg-transparent dark:disabled:hover:!bg-transparent focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors;
	}
    .ui-button-import {
        @apply w-8 h-8 rounded-full flex items-center justify-center transition-colors;
        @apply bg-transparent;
        @apply text-gray-700 dark:text-white;
        @apply border border-gray-300 dark:border-gray-600;
        @apply hover:bg-blue-100 dark:hover:bg-blue-700;
        @apply hover:text-blue-500 dark:hover:text-blue-400;
        @apply hover:border-blue-500 dark:hover:border-blue-500;
        @apply focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500;
        @apply disabled:hover:bg-transparent disabled:hover:border-gray-300 dark:disabled:hover:border-gray-600 dark:disabled:hover:!bg-transparent;
    }
    .ui-button-icon {
        @apply inline-flex items-center justify-center p-1.5 border border-transparent text-sm font-medium rounded-md text-gray-700 dark:text-white bg-transparent hover:bg-blue-100 dark:hover:bg-blue-700 disabled:hover:bg-transparent dark:disabled:hover:!bg-transparent focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors;
    }
    .ui-button-icon:disabled {
        @apply opacity-50 cursor-not-allowed;
    }
    .ui-button-icon svg {
        @apply w-4 h-4 flex-shrink-0;
    }
    .hover-scale-effect {
        @apply transition-transform hover:scale-105 disabled:hover:scale-100;
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>
