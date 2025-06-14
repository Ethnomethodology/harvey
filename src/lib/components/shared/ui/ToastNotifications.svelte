<!-- src/lib/components/shared/ui/ToastNotifications.svelte -->
<script>
  import notificationStore from '$lib/stores/notificationStore.js';
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';

  const typeClasses = {
    success: 'bg-green-500 border-green-700',
    error: 'bg-red-500 border-red-700',
    info: 'bg-blue-500 border-blue-700',
    warning: 'bg-yellow-500 border-yellow-700',
  };

  const iconPaths = {
    success: 'M10 15.172l9.192-9.193 1.415 1.414L10 18l-6.364-6.364 1.414-1.414z', // Checkmark
    error: 'M12 10.586l4.95-4.95 1.414 1.414-4.95 4.95 4.95 4.95-1.414 1.414-4.95-4.95-4.95 4.95-1.414-1.414 4.95-4.95-4.95-4.95 1.414-1.414z', // Cross
    info: 'M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z', // Info
    warning: 'M1 15h22L12 2 1 15zm12-2h-2v-2h2v2zm0-4h-2V7h2v2z', // Warning triangle
  };

</script>

{#if $notificationStore.length > 0}
  <div
    class="fixed bottom-4 right-4 z-[200] w-full max-w-xs sm:max-w-sm space-y-3"
    aria-live="polite"
    aria-relevant="additions removals"
  >
    {#each $notificationStore as notification (notification.id)}
      <div
        animate:flip="{{duration: 300}}"
        in:fly="{{ y: 100, duration: 300, delay:0 }}"
        out:fly="{{ y: 100, duration: 200 }}"
        class="relative flex items-start p-4 pr-10 rounded-lg shadow-xl text-white border-l-4 {typeClasses[notification.type] || 'bg-gray-500 border-gray-700'}"
        role="alert"
      >
        <div class="flex-shrink-0 mr-3">
          <svg class="w-6 h-6 fill-current" viewBox="0 0 24 24">
            <path d="{iconPaths[notification.type] || iconPaths.info}" />
          </svg>
        </div>
        <div class="flex-grow text-sm sm:text-base break-words">
          {notification.message}
        </div>
        <button
          on:click={() => notificationStore.dismiss(notification.id)}
          class="absolute top-2 right-2 p-1 rounded-full hover:bg-black/20 focus:outline-none focus:ring-2 focus:ring-white/50 transition-colors"
          aria-label="Dismiss notification"
        >
          <svg class="w-4 h-4 fill-current" viewBox="0 0 20 20">
            <path d="M10 8.586L14.95 3.636l1.414 1.414L11.414 10l4.95 4.95-1.414 1.414L10 11.414l-4.95 4.95-1.414-1.414L8.586 10 3.636 5.05l1.414-1.414L10 8.586z" />
          </svg>
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  /* Ensure z-index is high enough */
  .fixed {
    z-index: 200; /* Tailwind's z-50 is often max, this ensures it's above most things */
  }
</style>
