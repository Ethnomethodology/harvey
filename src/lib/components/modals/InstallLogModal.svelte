<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { Modal, Button } from 'flowbite-svelte';

    export let showModal = false;
    export let logs = [];
    export let isInstalling = true;
    export let isChecking = false;
    export let progress = 0;
    export let currentFile = "";
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

<Modal bind:open={showModal} size="lg" autoclose={false} outsideclose={!isInstalling && !isChecking} backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm" dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center" class="w-full" on:close={closeModal}>
    <h2 id="log-modal-title" class="text-lg font-semibold text-gray-900 dark:text-white" slot="header">
        {title}
    </h2>

    <div bind:this={logContainer} class="log-container bg-gray-100 dark:bg-gray-900 p-3 rounded-md text-xs font-mono border border-gray-300 dark:border-gray-700 h-64 overflow-y-auto">
        {#each logs as log (log.id)}
            <p class="whitespace-pre-wrap">{log.message}</p>
        {/each}
        {#if isInstalling}
            {#if progress > 0}
                <div class="mt-4 px-1">
                    <div class="flex justify-between items-end mb-1.5">
                        <div class="flex flex-col">
                            <span class="text-[10px] font-semibold text-blue-700 dark:text-blue-400 uppercase tracking-wider leading-none mb-1">{inProgressText}</span>
                            {#if currentFile}
                                <span class="text-[11px] font-medium text-gray-700 dark:text-gray-300 truncate max-w-[200px]" title={currentFile}>{currentFile}</span>
                            {/if}
                        </div>
                        <span class="text-[10px] font-mono font-bold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/30 px-1.5 py-0.5 rounded">{progress}%</span>
                    </div>
                    <div class="w-full bg-gray-200 dark:bg-gray-800 rounded-full h-1.5 overflow-hidden border border-gray-300/30 dark:border-gray-700/30">
                        <div class="bg-blue-500 dark:bg-blue-400 h-full rounded-full transition-all duration-300 ease-out shadow-[0_0_8px_rgba(59,130,246,0.5)]" style="width: {progress}%"></div>
                    </div>
                </div>
            {:else}
                <div class="flex items-center mt-3 ml-1">
                    <div class="spinner animate-spin"></div>
                    <p class="ml-2.5 text-gray-600 dark:text-gray-400 font-medium">{inProgressText}</p>
                </div>
            {/if}
        {:else if isChecking}
            <div class="flex items-center mt-3 ml-1">
                <div class="spinner animate-spin"></div>
                <p class="ml-2.5 text-gray-600 dark:text-gray-400 font-medium">{checkingText}</p>
            </div>
        {/if}
    </div>

    <svelte:fragment slot="footer">
        <div class="flex justify-end w-full">
            <Button
                color="alternative"
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
            </Button>
        </div>
    </svelte:fragment>
</Modal>

<style lang="postcss">
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
