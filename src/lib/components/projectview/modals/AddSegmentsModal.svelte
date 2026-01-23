<script>
    import { createEventDispatcher } from 'svelte';
    import { fade, scale } from 'svelte/transition';
    import { message } from '@tauri-apps/plugin-dialog';

    export let showModal = false;
    export let mediaName = '';
    export let mediaDuration = 0;
    export let existingSegments = [];
    export let speakerList = []; // Array of speaker names

    const dispatch = createEventDispatcher();

    // Configuration
    let segmentCount = 1;
    let segmentDuration = 60; // in seconds
    let speakerMode = 'unselected'; // 'unselected' | 'alternate'

    // Derived state
    $: totalDurationNeeded = segmentCount * segmentDuration;
    $: isDurationValid = totalDurationNeeded <= mediaDuration; // Basic check, refining below

    function formatDuration(seconds) {
        const m = Math.floor(seconds / 60);
        const s = Math.floor(seconds % 60);
        return `${m}m ${s}s`;
    }

    function closeModal() {
        dispatch('close');
    }

    function calculateInsertionPoints() {
        // Find the last segment's end time
        const lastEndTime = existingSegments.length > 0 
            ? existingSegments[existingSegments.length - 1].end_time 
            : 0;
        
        const availableSpace = mediaDuration - lastEndTime;
        
        if (availableSpace < totalDurationNeeded) {
            return {
                valid: false,
                reason: `Not enough space at the end of the media. Required: ${formatDuration(totalDurationNeeded)}, Available: ${formatDuration(availableSpace)}`
            };
        }

        return { valid: true, startTime: lastEndTime };
    }

    async function handleConfirm() {
        const check = calculateInsertionPoints();
        if (!check.valid) {
            await message(check.reason, { title: 'Invalid Configuration', type: 'warning' });
            return;
        }

        dispatch('confirm', {
            segmentCount,
            segmentDuration,
            speakerMode,
            startTime: check.startTime
        });
        closeModal();
    }
</script>

{#if showModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50" transition:fade={{ duration: 200 }}>
        <div class="bg-white dark:bg-surface-2 rounded-lg shadow-xl w-full max-w-md p-6" transition:scale={{ duration: 200, start: 0.95 }}>
            <h2 class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Add Manual Segments</h2>
            
            <div class="mb-4 text-sm text-gray-600 dark:text-gray-400">
                <p>Media: <span class="font-medium text-gray-800 dark:text-gray-200">{mediaName}</span></p>
                <p>Total Duration: <span class="font-medium text-gray-800 dark:text-gray-200">{formatDuration(mediaDuration)}</span></p>
            </div>

            <!-- Settings Form -->
            <div class="space-y-4">
                
                <!-- Segment Count -->
                <div>
                    <label for="seg-count" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Number of Segments</label>
                    <input 
                        id="seg-count" 
                        type="number" 
                        min="1" 
                        max="100" 
                        bind:value={segmentCount}
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-blue-500 focus:border-blue-500 dark:bg-surface-1 dark:text-white"
                    />
                </div>

                <!-- Duration Per Segment -->
                <div>
                    <label for="seg-duration" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Duration per Segment (seconds)</label>
                    <div class="flex items-center gap-2">
                        <input 
                            id="seg-duration" 
                            type="number" 
                            min="1" 
                            bind:value={segmentDuration}
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:ring-blue-500 focus:border-blue-500 dark:bg-surface-1 dark:text-white"
                        />
                        <span class="text-xs text-gray-500 whitespace-nowrap">({formatDuration(segmentDuration)})</span>
                    </div>
                </div>

                <!-- Speaker Mode -->
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Speaker Assignment</label>
                    <div class="flex gap-4">
                        <label class="inline-flex items-center">
                            <input type="radio" group={speakerMode} value="unselected" class="form-radio text-blue-600 dark:bg-surface-1 dark:border-gray-600">
                            <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">Unselected</span>
                        </label>
                        <label class="inline-flex items-center">
                            <input type="radio" group={speakerMode} value="alternate" class="form-radio text-blue-600 dark:bg-surface-1 dark:border-gray-600" disabled={speakerList.length < 2}>
                            <span class="ml-2 text-sm text-gray-700 dark:text-gray-300" class:opacity-50={speakerList.length < 2}>Alternate Speakers</span>
                        </label>
                    </div>
                    {#if speakerMode === 'alternate' && speakerList.length < 2}
                        <p class="text-xs text-red-500 mt-1">Need at least 2 speakers configured to alternate.</p>
                    {/if}
                </div>

                <!-- Total Impact Preview -->
                <div class="p-3 bg-gray-50 dark:bg-surface-3 rounded border border-gray-200 dark:border-gray-700 text-sm">
                    <div class="flex justify-between mb-1">
                        <span class="text-gray-600 dark:text-gray-400">Total New Time:</span>
                        <span class="font-medium text-gray-900 dark:text-white">{formatDuration(totalDurationNeeded)}</span>
                    </div>
                    {#if !calculateInsertionPoints().valid}
                        <p class="text-red-500 text-xs mt-2">{calculateInsertionPoints().reason}</p>
                    {/if}
                </div>

            </div>

            <!-- Actions -->
            <div class="mt-6 flex justify-end space-x-3">
                <button 
                    on:click={closeModal}
                    class="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-surface-1 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-surface-3"
                >
                    Cancel
                </button>
                <button 
                    on:click={handleConfirm}
                    disabled={!calculateInsertionPoints().valid}
                    class="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                    Add Segments
                </button>
            </div>
        </div>
    </div>
{/if}
