<script>
    import { createEventDispatcher, onDestroy } from 'svelte';

    export let src = null;

    const dispatch = createEventDispatcher();

    const PLAY_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-play-fill" viewBox="0 0 16 16"><path d="m11.596 8.697-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393z"/></svg>`;
    const PAUSE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pause-fill" viewBox="0 0 16 16"><path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5m5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5"/></svg>`;
    const PREVIOUS_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-skip-start-fill" viewBox="0 0 16 16"><path d="M4 4a.5.5 0 0 1 1 0v3.248l6.267-3.636c.54-.313 1.232.066 1.232.696v7.384c0 .63-.692 1.01-1.232.697L5 8.752V12a.5.5 0 0 1-1 0z"/></svg>`;
    const NEXT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-skip-end-fill" viewBox="0 0 16 16"><path d="M12.5 4a.5.5 0 0 0-1 0v3.248L5.233 3.612C4.693 3.3 4 3.682 4 4.308v7.384c0 .626.693 1.01 1.233.697L11.5 8.752V12a.5.5 0 0 0 1 0z"/></svg>`;
    const VOLUME_HIGH_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-volume-up-fill" viewBox="0 0 16 16"><path d="M11.536 14.01A8.47 8.47 0 0 0 14.026 8a8.47 8.47 0 0 0-2.49-6.01l-.708.707A7.48 7.48 0 0 1 13.025 8c0 2.071-.84 3.946-2.197 5.303z"/><path d="M10.121 12.59A6.48 6.48 0 0 0 12.025 8a6.48 6.48 0 0 0-1.904-4.59l-.707.707A5.48 5.48 0 0 1 11.025 8a5.48 5.48 0 0 1-1.61 3.88z"/><path d="M8.707 11.18A4.5 4.5 0 0 0 10.025 8a4.5 4.5 0 0 0-1.318-3.18l-.707.707A3.5 3.5 0 0 1 9.025 8a3.5 3.5 0 0 1-1.025 2.47zM6.717 3.55A.5.5 0 0 1 7 4v8a.5.5 0 0 1-.812.39L3.825 10.5H1.5A.5.5 0 0 1 1 10V6a.5.5 0 0 1 .5-.5h2.325l2.393-1.85A.5.5 0 0 1 6.717 3.55"/></svg>`;

    let audio;
    let paused = true;
    let currentTime = 0;
    let duration = 0;
    let volume = 1;
    let lastLoadedSrc = null;

    $: if (src && audio && src !== lastLoadedSrc) {
        lastLoadedSrc = src;
        audio.src = src;
        audio.load();
        audio.play().catch(e => console.error("Audio play failed:", e));
    }

    function togglePlay() {
        paused = !paused;
    }

    function onVolumeInput(e) {
        volume = parseFloat(e.target.value) / 100;
    }

    function formatTime(seconds) {
        if (isNaN(seconds) || !isFinite(seconds)) return "0:00";
        const minutes = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${minutes}:${secs < 10 ? '0' : ''}${secs}`;
    }
</script>

{#if src}
<div class="p-2 border-t border-gray-200 dark:border-border bg-gray-50 dark:bg-surface-3">
    <audio
        bind:this={audio}
        bind:paused
        bind:currentTime
        bind:duration
        bind:volume
        on:ended={() => dispatch('ended')}
    ></audio>

    <div class="flex items-center space-x-2 text-xs">
        <span class="text-gray-600 dark:text-text-secondary w-10 text-right">{formatTime(currentTime)}</span>
        <input 
            type="range" 
            class="w-full h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer" 
            min="0" 
            max={duration || 0} 
            step="0.001"
            bind:value={currentTime} 
            style="--progress: {duration > 0 ? (currentTime / duration) : 0};"
        >
        <span class="text-gray-600 dark:text-text-secondary w-10">{formatTime(duration)}</span>
    </div>

    <div class="flex items-center justify-center space-x-4 mt-1">
        <button on:click={() => dispatch('previous')} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary" title="Previous">
            {@html PREVIOUS_ICON}
        </button>
        <button on:click={togglePlay} class="p-2 bg-blue-500 dark:bg-accent-primary text-white rounded-full hover:bg-blue-600 dark:hover:bg-accent-primary-hover" title={paused ? 'Play' : 'Pause'}>
            {@html !paused ? PAUSE_ICON : PLAY_ICON}
        </button>
        <button on:click={() => dispatch('next')} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary" title="Next">
            {@html NEXT_ICON}
        </button>
    </div>

    <div class="flex items-center justify-center space-x-2 mt-2">
        <span class="text-gray-600 dark:text-text-secondary">{@html VOLUME_HIGH_ICON}</span>
        <input type="range" class="w-1/3 h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer" value={volume * 100} on:input={onVolumeInput}>
    </div>
</div>
{/if}


