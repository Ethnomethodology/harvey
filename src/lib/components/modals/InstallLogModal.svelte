<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { Modal, Button } from 'flowbite-svelte';

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

<Modal bind:open={showModal} size="lg" autoclose={false} outsideclose={!isInstalling && !isChecking} backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm" dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center" class="w-full" on:close={closeModal}>
    <h2 id="log-modal-title" class="text-lg font-semibold text-gray-900 dark:text-white" slot="header">
        {title}
    </h2>

    <div bind:this={logContainer} class="log-container bg-gray-100 dark:bg-gray-900 p-3 rounded-md text-xs font-mono border border-gray-300 dark:border-gray-700 h-64 overflow-y-auto">
        {#each logs as log (log.id)}
            <p class="whitespace-pre-wrap">{log.message}</p>
        {/each}
        {#if isInstalling}
            <div class="flex items-center mt-2">
                <div class="spinner animate-spin"></div>
                <p class="ml-2 text-gray-700 dark:text-gray-300">{inProgressText}</p>
            </div>
        {:else if isChecking}
            <div class="flex items-center mt-2">
                <div class="spinner animate-spin"></div>
                <p class="ml-2 text-gray-700 dark:text-gray-300">{checkingText}</p>
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
