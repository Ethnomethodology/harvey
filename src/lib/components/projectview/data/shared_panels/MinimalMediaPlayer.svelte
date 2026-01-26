<script>
    import { createEventDispatcher, onDestroy } from 'svelte';

    export let src = null;

    const dispatch = createEventDispatcher();

    const PLAY_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-play-fill" viewBox="0 0 16 16"><path d="m11.596 8.697-6.363 3.692c-.54.313-1.233-.066-1.233-.697V4.308c0-.63.692-1.01 1.233-.696l6.363 3.692a.802.802 0 0 1 0 1.393z"/></svg>`;
    const PAUSE_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pause-fill" viewBox="0 0 16 16"><path d="M5.5 3.5A1.5 1.5 0 0 1 7 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5m5 0A1.5 1.5 0 0 1 12 5v6a1.5 1.5 0 0 1-3 0V5a1.5 1.5 0 0 1 1.5-1.5"/></svg>`;
    const REWIND_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-counterclockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 1-4.546 2.914.5.5 0 0 0-.908-.417A6 6 0 1 0 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 0-.41-.192L5.23 2.308a.25.25 0 0 0 0 .384l2.36 1.966A.25.25 0 0 0 8 4.466"/></svg>`;
    const FORWARD_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" fill="currentColor" class="bi bi-arrow-clockwise" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z"/><path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466"/></svg>`;
    const VOLUME_HIGH_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-volume-up-fill" viewBox="0 0 16 16"><path d="M11.536 14.01A8.47 8.47 0 0 0 14.026 8a8.47 8.47 0 0 0-2.49-6.01l-.708.707A7.48 7.48 0 0 1 13.025 8c0 2.071-.84 3.946-2.197 5.303z"/><path d="M10.121 12.59A6.48 6.48 0 0 0 12.025 8a6.48 6.48 0 0 0-1.904-4.59l-.707.707A5.48 5.48 0 0 1 11.025 8a5.48 5.48 0 0 1-1.61 3.88z"/><path d="M8.707 11.18A4.5 4.5 0 0 0 10.025 8a4.5 4.5 0 0 0-1.318-3.18l-.707.707A3.5 3.5 0 0 1 9.025 8a3.5 3.5 0 0 1-1.025 2.47zM6.717 3.55A.5.5 0 0 1 7 4v8a.5.5 0 0 1-.812.39L3.825 10.5H1.5A.5.5 0 0 1 1 10V6a.5.5 0 0 1 .5-.5h2.325l2.393-1.85A.5.5 0 0 1 6.717 3.55"/></svg>`;
    const REPEAT_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-repeat" viewBox="0 0 16 16"><path d="M11 5.466V4H5a4 4 0 0 0-3.584 5.777.5.5 0 1 1-.896.446A5 5 0 0 1 5 3h6V1.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36 1.966a.25.25 0 0 1-.41-.192m3.81.086a.5.5 0 0 1 .67.225A5 5 0 0 1 11 13H5v1.466a.25.25 0 0 1-.41.192l-2.36-1.966a.25.25 0 0 1 0-.384l2.36-1.966a.25.25 0 0 1 .41.192V12h6a4 4 0 0 0 3.585-5.777.5.5 0 0 1 .225-.67Z"/></svg>`;
    const ICON_MINIMIZE_VIDEO = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrows-collapse" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M1 8a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 0 1h-13A.5.5 0 0 1 1 8m7-8a.5.5 0 0 1 .5.5v3.793l1.146-1.147a.5.5 0 0 1 .708.708l-2 2a.5.5 0 0 1-.708 0l-2-2a.5.5 0 1 1 .708-.708L7.5 4.293V.5A.5.5 0 0 1 8 0m-.5 11.707-1.146 1.147a.5.5 0 0 1-.708-.708l2-2a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1-.708.708L8.5 11.707V15.5a.5.5 0 0 1-1 0z"/></svg>`;
    const ICON_MAXIMIZE_VIDEO = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-arrows-expand" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M1 8a.5.5 0 0 1 .5-.5h13a.5.5 0 0 1 0 1h-13A.5.5 0 0 1 1 8M7.646.146a.5.5 0 0 1 .708 0l2 2a.5.5 0 0 1-.708.708L8.5 1.707V5.5a.5.5 0 0 1-1 0V1.707L6.354 2.854a.5.5 0 1 1-.708-.708zM8 10a.5.5 0 0 1 .5.5v3.793l1.146-1.147a.5.5 0 0 1 .708.708l-2 2a.5.5 0 0 1-.708 0l-2-2a.5.5 0 0 1 .708-.708L7.5 14.293V10.5A.5.5 0 0 1 8 10"/></svg>`;

    let mediaElement;
    let paused = true;
    let currentTime = 0;
    let duration = 0;
    let volume = 1;
    let lastLoadedSrc = null;
    let repeat = false;
    let showVideo = false;

    $: isVideo = src && /\.(mp4|mov|avi|mkv|webm)$/i.test(src);

    $: if (src && mediaElement && src !== lastLoadedSrc) {
        lastLoadedSrc = src;
        currentTime = 0;
        mediaElement.src = src;
        mediaElement.load();
        mediaElement.play().catch(e => console.error("Media play failed:", e));
    }

    function togglePlay() {
        paused = !paused;
    }

    function toggleVideo() {
        showVideo = !showVideo;
    }

    function rewind10s() {
        if (!mediaElement) return;
        currentTime = Math.max(0, currentTime - 10);
    }

    function forward10s() {
        if (!mediaElement || !duration) return;
        currentTime = Math.min(duration, currentTime + 10);
    }

    function toggleRepeat() {
        repeat = !repeat;
    }

    function handleEnded() {
        if (repeat) {
            currentTime = 0;
            mediaElement.play().catch(e => console.error("Media repeat play failed:", e));
        } else {
            dispatch('ended');
        }
    }

    // --- Progress Bar Tooltip State ---
    let progressTooltipElement;
    let progressBarElement;
    let showProgressTooltip = false;
    let progressTooltipText = '00:00:00';
    let progressTooltipLeft = '0px';

    function formatTimeWithHours(totalSeconds) {
        if (isNaN(totalSeconds) || totalSeconds < 0) return '00:00:00';
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = Math.floor(totalSeconds % 60);
        return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
    }

    function handleMouseMoveOnProgressBar(event) {
        if (!duration || !progressBarElement || !progressTooltipElement) return;

        const progressBarRect = progressBarElement.getBoundingClientRect();
        const mouseX_relative = event.clientX - progressBarRect.left;
        const percent = Math.max(0, Math.min(1, mouseX_relative / progressBarRect.width));
        const hoverTime = percent * duration;
        progressTooltipText = formatTimeWithHours(hoverTime);

        // Calculate the ideal center position for the tooltip (directly under mouse)
        let idealTooltipCenter = mouseX_relative;

        // Adjust idealTooltipCenter to prevent tooltip edges from going outside progressBarElement
        const tooltipWidth = progressTooltipElement.offsetWidth;
        const minAllowedCenter = tooltipWidth / 2;
        const maxAllowedCenter = progressBarRect.width - (tooltipWidth / 2);

        let clampedTooltipCenter;
        if (progressBarRect.width < tooltipWidth) { // Tooltip wider than bar
            clampedTooltipCenter = progressBarRect.width / 2; // Center tooltip on the bar
        } else {
            clampedTooltipCenter = Math.max(minAllowedCenter, Math.min(idealTooltipCenter, maxAllowedCenter));
        }

        progressTooltipLeft = `${clampedTooltipCenter}px`;
        showProgressTooltip = true;
    }

    function handleMouseLeaveProgressBar() {
        showProgressTooltip = false;
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
    <div class="relative w-full aspect-video bg-black mb-2 rounded-md overflow-hidden" class:hidden={!showVideo}>
        <video
            bind:this={mediaElement}
            class="w-full h-full object-contain"
            bind:paused
            bind:currentTime
            bind:duration
            bind:volume
            on:ended={handleEnded}
        ></video>
    </div>
    {#if !showVideo}
        <!-- Ensure video element is still in DOM when hidden to play audio, but we need to bind to it. 
             If showVideo is false, the div above is hidden but present. 
             Ideally we want the video element to always exist. -->
    {/if}

    <div class="flex items-center space-x-2 text-xs">
        <span class="text-gray-600 dark:text-text-secondary w-10 text-right">{formatTime(currentTime)}</span>
        <div class="relative w-full h-4 flex items-center group">
            <input 
                bind:this={progressBarElement}
                type="range" 
                class="w-full h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer absolute inset-0 m-auto" 
                min="0" 
                max={duration || 0} 
                step="0.001"
                bind:value={currentTime} 
                style="--progress: {duration > 0 ? (currentTime / duration) : 0};"
                on:mousemove={handleMouseMoveOnProgressBar}
                on:mouseleave={handleMouseLeaveProgressBar}
            >
            <span
                bind:this={progressTooltipElement}
                class="absolute bottom-full mb-1 bg-black text-white text-[10px] px-1 rounded pointer-events-none whitespace-nowrap z-50"
                style="left: {progressTooltipLeft}; transform: translateX(-50%); display: {showProgressTooltip ? 'block' : 'none'};"
            >
                {progressTooltipText}
            </span>
        </div>
        <span class="text-gray-600 dark:text-text-secondary w-10">{formatTime(duration)}</span>
    </div>

        <div class="grid grid-cols-3 items-center mt-1">
            <div class="flex justify-start">
                <!-- Left spacer -->
            </div>
            <div class="flex items-center justify-center space-x-4">
                <button on:click={rewind10s} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary" title="Rewind 10s">
                    {@html REWIND_ICON}
                </button>
                <button on:click={togglePlay} class="p-2 bg-blue-500 dark:bg-accent-primary text-white rounded-full hover:bg-blue-600 dark:hover:bg-accent-primary-hover" title={paused ? 'Play' : 'Pause'}>
                    {@html !paused ? PAUSE_ICON : PLAY_ICON}
                </button>
                <button on:click={forward10s} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary" title="Forward 10s">
                    {@html FORWARD_ICON}
                </button>
            </div>
            <div class="flex justify-end items-center space-x-1">
                {#if isVideo}
                    <button on:click={toggleVideo} class="p-1 text-gray-600 dark:text-text-secondary hover:text-black dark:hover:text-text-primary" title={showVideo ? 'Hide Video' : 'Show Video'}>
                        {@html showVideo ? ICON_MINIMIZE_VIDEO : ICON_MAXIMIZE_VIDEO}
                    </button>
                {/if}
                <button on:click={toggleRepeat} class="p-1 transition-colors" class:text-blue-500={repeat} class:text-black={!repeat} class:dark:text-text-primary={!repeat} title="Repeat">
                    {@html REPEAT_ICON}
                </button>
            </div>
        </div>
    <div class="flex items-center justify-center space-x-2 mt-2">
        <span class="text-gray-600 dark:text-text-secondary">{@html VOLUME_HIGH_ICON}</span>
        <input type="range" class="w-1/3 h-1 bg-gray-300 dark:bg-text-secondary rounded-lg appearance-none cursor-pointer" value={volume * 100} on:input={onVolumeInput}>
    </div>
</div>
{/if}


