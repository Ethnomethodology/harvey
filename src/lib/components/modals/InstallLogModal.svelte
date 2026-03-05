<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';

    export let showModal = false;
    export let logs = [];
    export let isInstalling = true;
    export let isChecking = false;
    export let title = "Installation Logs";
    export let inProgressText = "Downloading...";
    export let buttonInProgressText = "Installing...";
    export let checkingText = "Checking library installations...";

    function closeModal() {
        if (!isInstalling && !isChecking) {
            showModal = false;
        }
    }

    let logContainer;

    $: if (logs && logContainer) {
        // Auto-scroll to the bottom
        logContainer.scrollTop = logContainer.scrollHeight;
    }

    function handleKeydown(event) {
        if (event.key === 'Escape') {
            closeModal();
        }
    }

    onMount(() => {
        window.addEventListener('keydown', handleKeydown);
    });

    onDestroy(() => {
        window.removeEventListener('keydown', handleKeydown);
    });
</script>

{#if showModal}
<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-60 backdrop-blur-sm"
    on:click|self={closeModal}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="log-modal-title"
    tabindex="-1"
>
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-2xl flex flex-col" on:click|stopPropagation role="presentation">
        <h2 id="log-modal-title" class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
            {title}
        </h2>

        <div bind:this={logContainer} class="log-container bg-gray-100 dark:bg-gray-900 p-3 rounded-md text-xs font-mono border border-gray-300 dark:border-gray-700 h-64 overflow-y-auto">
            {#each logs as log (log.id)}
                <p class="whitespace-pre-wrap">{log.message}</p>
            {/each}
            {#if isInstalling}
                <div class="flex items-center">
                    <div class="spinner animate-spin"></div>
                    <p class="ml-2">{inProgressText}</p>
                </div>
            {:else if isChecking}
                <div class="flex items-center">
                    <div class="spinner animate-spin"></div>
                    <p class="ml-2">{checkingText}</p>
                </div>
            {/if}
        </div>

        <div class="mt-6 flex justify-end">
            <button
                type="button"
                class="btn-secondary"
                on:click={closeModal}
                disabled={isInstalling || isChecking}
            >
                {#if isInstalling}
                    {buttonInProgressText}
                {:else if isChecking}
                    Checking...
                {:else}
                    Close
                {/if}
            </button>
        </div>
    </div>
</div>
{/if}

<style lang="postcss">
	.btn-secondary {
		@apply px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-gray-100 dark:bg-gray-600 hover:bg-gray-200 dark:hover:bg-gray-500 rounded-md border border-gray-300 dark:border-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50;
	}
    .log-container {
        scrollbar-width: thin;
    }
    .spinner {
        border: 2px solid rgba(0, 0, 0, 0.1);
        width: 1rem;
        height: 1rem;
        border-radius: 50%;
        border-left-color: #09f;
    }
</style>
