<!-- src/lib/components/projectview/shared/SimpleTopBar.svelte -->
<script>
    import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
    import { project } from '$lib/stores/projectStore.js';
    import { createEventDispatcher } from 'svelte';
    import { Sun, Moon, Monitor } from '@lucide/svelte';
    import { Button } from 'flowbite-svelte';

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
  class="flex items-center justify-between px-1 h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800 relative z-30"
  data-tauri-drag-region
>
  <div class="flex items-center space-x-1.5 min-w-0">
        <div class="h-10 flex items-center justify-center flex-shrink-0">
            <button
                type="button"
                class="p-1.5 ml-1 mr-1 rounded-full border-0 bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400 hover:bg-blue-200 dark:hover:bg-blue-500/30 transition-all duration-200 active:scale-95 shadow-sm hover:shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                on:click={(e) => dispatch('requestImport', e)}
                title="Import Audio or Video"
                aria-label="Import Audio or Video"
            >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
            </button>
        </div>

      <span class="font-semibold text-sm text-gray-700 dark:text-gray-200 truncate" title={displayTitle}>{displayTitle}</span>
  </div>

  <div class="flex items-center space-x-2 flex-shrink-0">

    <div class="flex-shrink-0">
		 <button on:click="{cycleThemePreference}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-blue-500/10 dark:hover:text-blue-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors transition-transform hover:scale-105" title="{themeTitle}" aria-label="{themeTitle}">
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
</div>

<style lang="postcss">
    :global(.hover-scale-effect) {
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>
