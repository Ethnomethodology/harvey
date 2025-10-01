<!-- src/lib/components/projectview/shared/SimpleTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { derived } from 'svelte/store';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    // --- Theme Icons ---
	const SUN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" /></svg>`;
	const MOON_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" /></svg>`;
	const SYSTEM_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25m18 0A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25m18 0V12a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 12V5.25" /></svg>`;
	$: themeIconHtml = $themePreference === 'light' ? SUN_ICON
					 : $themePreference === 'dark' ? MOON_ICON
					 : SYSTEM_ICON;
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
  class="flex items-center justify-between px-1 h-10 flex-shrink-0 bg-white dark:bg-surface-1 border-b border-gray-200 dark:border-dark-bg-tertiary"
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
    <div class="flex-shrink-0">
		 <button on:click="{cycleThemePreference}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-surface-2 dark:text-gray-300 hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-accent-background-hover dark:hover:text-blue-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors transition-transform hover:scale-105" title="{themeTitle}">
			{@html themeIconHtml}
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
