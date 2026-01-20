<script>
    import { createEventDispatcher, onMount } from 'svelte';

    export let value = 0; // in seconds
    export let disabled = false;

    const dispatch = createEventDispatcher();

    let displayValue = '';

    function formatTime(sec) {
        if (typeof sec !== 'number' || isNaN(sec) || sec < 0) return '00:00.000';
        const totalMs = Math.round(sec * 1000);
        const ms = String(totalMs % 1000).padStart(3, '0');
        const totalSeconds = Math.floor(sec);
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, '0');
        const seconds = String(totalSeconds % 60).padStart(2, '0');
        if (hours > 0) {
            return `${String(hours).padStart(2, '0')}:${minutes}:${seconds}.${ms}`;
        } else {
            return `${minutes}:${seconds}.${ms}`;
        }
    }

    function parseTimestamp(str) {
        if (!str) return null;
        
        // Try parsing hh:mm:ss.ms
        const partsHhMmSsMs = str.match(/^(\d+):(\d{2}):(\d{2})\.(\d{3})$/);
        if (partsHhMmSsMs) {
            const hours = parseInt(partsHhMmSsMs[1], 10);
            const minutes = parseInt(partsHhMmSsMs[2], 10);
            const seconds = parseInt(partsHhMmSsMs[3], 10);
            const milliseconds = parseInt(partsHhMmSsMs[4], 10);
            if (!isNaN(hours) && !isNaN(minutes) && !isNaN(seconds) && !isNaN(milliseconds) && minutes < 60 && seconds < 60 && milliseconds < 1000) {
                return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000;
            }
        }

        // Try parsing mm:ss.ms
        const partsMmSsMs = str.match(/^(\d+):(\d{2})\.(\d{3})$/);
        if (partsMmSsMs) {
            const minutes = parseInt(partsMmSsMs[1], 10);
            const seconds = parseInt(partsMmSsMs[2], 10);
            const milliseconds = parseInt(partsMmSsMs[3], 10);
            if (!isNaN(minutes) && !isNaN(seconds) && !isNaN(milliseconds) && seconds < 60 && milliseconds < 1000) {
                return minutes * 60 + seconds + milliseconds / 1000;
            }
        }
        
        const floatVal = parseFloat(str);
        return isNaN(floatVal) ? null : floatVal;
    }

    $: displayValue = formatTime(value);

    function handleBlur() {
        const parsed = parseTimestamp(displayValue);
        if (parsed !== null && Math.abs(parsed - value) > 0.0001) {
            dispatch('update', parsed);
        } else {
            // if invalid, revert to original value
            displayValue = formatTime(value);
        }
    }

    function handleKeydown(event) {
        if (event.key === 'Enter') {
            event.target.blur();
        }
    }
</script>

<input
    class="input-field w-[10ch] text-xs p-1 text-center"
    type="text"
    bind:value={displayValue}
    {disabled}
    on:blur={handleBlur}
    on:keydown={handleKeydown}
    aria-label="Timestamp"
    placeholder="00:00.000"
    autocomplete="off"
    autocorrect="off"
/>

<style>
    .input-field {
		@apply text-center bg-transparent border-0 p-0 text-gray-800 dark:text-text-secondary;
	}
	.input-field:not(:disabled) {
		@apply bg-white dark:bg-surface-3 border border-gray-300 dark:border-border text-gray-900 dark:text-text-primary rounded;
	}
</style>