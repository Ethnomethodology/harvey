<!-- src/lib/components/projectview/shared/ToastNotifications.svelte -->
<script>
  import notificationStore from '$lib/stores/notificationStore.js';
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';

  const typeStyles = {
    success: { bg: 'bg-green-50 dark:bg-green-800/60', border: 'border-green-500 dark:border-green-500', text: 'text-green-700 dark:text-green-200', iconFill: 'fill-green-600 dark:fill-green-400', iconPath: 'M10 15.172l9.192-9.193 1.415 1.414L10 18l-6.364-6.364 1.414-1.414z' },
    error:   { bg: 'bg-red-50 dark:bg-red-800/60',     border: 'border-red-500 dark:border-red-500',     text: 'text-red-700 dark:text-red-200',       iconFill: 'fill-red-600 dark:fill-red-400',       iconPath: 'M12 10.586l4.95-4.95 1.414 1.414-4.95 4.95 4.95 4.95-1.414 1.414-4.95-4.95-4.95 4.95-1.414-1.414 4.95-4.95-4.95-4.95 1.414-1.414z' },
    info:    { bg: 'bg-blue-50 dark:bg-blue-800/60',    border: 'border-blue-500 dark:border-blue-500',    text: 'text-blue-700 dark:text-blue-200',      iconFill: 'fill-blue-600 dark:fill-blue-400',      iconPath: 'M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z' },
    warning: { bg: 'bg-yellow-50 dark:bg-yellow-800/60',border: 'border-yellow-400 dark:border-yellow-500',text: 'text-yellow-700 dark:text-yellow-200',  iconFill: 'fill-yellow-500 dark:fill-yellow-300',  iconPath: 'M1 15h22L12 2 1 15zm12-2h-2v-2h2v2zm0-4h-2V7h2v2z' },
    default: { bg: 'bg-gray-100 dark:bg-gray-800',    border: 'border-gray-500 dark:border-gray-700',    text: 'text-gray-700 dark:text-gray-200',      iconFill: 'fill-gray-600 dark:fill-gray-600',      iconPath: 'M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z' } // Default uses info icon
  };

</script>

{#if $notificationStore.length > 0}
  <div
    class="fixed bottom-4 right-4 z-[10005] w-full max-w-xs sm:max-w-sm space-y-3"
    aria-live="polite"
    aria-relevant="additions removals"
  >
    {#each $notificationStore as notification (notification.id)}
      <div
        animate:flip="{{duration: 300}}"
        in:fly="{{ y: 100, duration: 300, delay:0 }}"
        out:fly="{{ y: 100, duration: 200 }}"
        class="relative flex items-start p-3 pr-8 rounded-md shadow-lg border-l-4 {typeStyles[notification.type]?.border || typeStyles.default.border} {typeStyles[notification.type]?.bg || typeStyles.default.bg} {typeStyles[notification.type]?.text || typeStyles.default.text}"
        role="alert"
      >
        <div class="flex-shrink-0 mr-3">
          <svg class="w-5 h-5 {typeStyles[notification.type]?.iconFill || typeStyles.default.iconFill}" viewBox="0 0 24 24">
            <path d="{typeStyles[notification.type]?.iconPath || typeStyles.default.iconPath}" />
          </svg>
        </div>
        <div class="flex-grow text-sm break-words">
          {notification.message}
        </div>
        <button
          on:click={() => notificationStore.dismiss(notification.id)}
          class="absolute top-1.5 right-1.5 p-0.5 rounded-full hover:bg-black/20 focus:outline-none focus:ring-2 focus:ring-white/50 transition-colors"
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
