<!-- src/lib/components/projectview/modals/ColorPickerModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
  
    export let showModal = false;
    export let initialColor = '#FFFFFF'; // Default to white or transparent? Let's use white
    export let title = 'Select Color';

    let selectedColor = initialColor || '#FFFFFF';

    const dispatch = createEventDispatcher();

    // Basic palette
    const colors = [
      '#FFFFFF', '#F3F4F6', '#E5E7EB', '#D1D5DB', '#9CA3AF', '#6B7280', '#4B5563', '#374151', '#1F2937', '#11182C', // Grays
      '#FEE2E2', '#FECACA', '#FCA5A5', '#F87171', '#EF4444', '#DC2626', '#B91C1C', '#991B1B', '#7F1D1D', '#fee2e2', // Reds
      '#FFEDD5', '#FED7AA', '#FDBA74', '#FB923C', '#F97316', '#EA580C', '#C2410C', '#9A3412', '#7C2D12', '#fed7aa', // Oranges
      '#FEF3C7', '#FDE68A', '#FCD34D', '#FBBF24', '#F59E0B', '#D97706', '#B45309', '#92400E', '#78350F', '#fde68a', // Ambers
      '#FEFCE8', '#FEF08A', '#FACC15', '#EAB308', '#CA8A04', '#A16207', '#854D0E', '#713F12', '#422006', '#fef08a', // Yellows
      '#ECFCCB', '#D9F99D', '#BEF264', '#A3E635', '#84CC16', '#65A30D', '#4D7C0F', '#3F6212', '#365314', '#d9f99d', // Limes
      '#DCFCE7', '#BBF7D0', '#86EFAC', '#4ADE80', '#22C55E', '#16A34A', '#15803D', '#166534', '#14532D', '#bbf7d0', // Greens
      '#D1FAE5', '#A7F3D0', '#6EE7B7', '#34D399', '#10B981', '#059669', '#047857', '#065F46', '#064E3B', '#a7f3d0', // Emeralds
      '#CCFBF1', '#99F6E4', '#5EEAD4', '#2DD4BF', '#14B8A6', '#0D9488', '#0F766E', '#134E4A', '#115E59', '#99f6e4', // Teals
      '#CFFAFE', '#A5F3FC', '#67E8F9', '#22D3EE', '#06B6D4', '#0891B2', '#0E7490', '#164E63', '#155E75', '#a5f3fc', // Cyans
      '#E0F2FE', '#BAE6FD', '#7DD3FC', '#38BDF8', '#0EA5E9', '#0284C7', '#0369A1', '#075985', '#0C4A6E', '#bae6fd', // Sky Blues
      '#DBEAFE', '#BFDBFE', '#93C5FD', '#60A5FA', '#3B82F6', '#2563EB', '#1D4ED8', '#1E40AF', '#1E3A8A', '#bfdbfe', // Blues
      '#E0E7FF', '#C7D2FE', '#A5B4FC', '#818CF8', '#6366F1', '#4F46E5', '#4338CA', '#3730A3', '#312E81', '#c7d2fe', // Indigos
      '#E5E0FF', '#DDD6FE', '#C4B5FD', '#A78BFA', '#8B5CF6', '#7C3AED', '#6D28D9', '#5B21B6', '#4C1D95', '#ddd6fe', // Violets
      '#F3E8FF', '#E9D5FF', '#D8B4FE', '#C084FC', '#A855F7', '#9333EA', '#7E22CE', '#6B21A8', '#581C87', '#e9d5ff', // Purples
      '#FAE8FF', '#F5D0FE', '#F0ABFC', '#E879F9', '#D946EF', '#C026D3', '#A21CAF', '#86198F', '#701A75', '#f5d0fe', // Fuchsias
      '#FCE7F3', '#FBCFE8', '#F9A8D4', '#F472B6', '#EC4899', '#DB2777', '#BE185D', '#9D174D', '#831843', '#fbcfe8' // Pinks
    ];
  
    function handleConfirm() {
      dispatch('confirm', { color: selectedColor });
    }
  
    function handleClose() {
      dispatch('close');
    }
  
    function selectColor(color) {
      selectedColor = color;
    }
  
    function keydown(event) {
      if (event.key === 'Escape') {
        handleClose();
      }
       if (event.key === 'Enter' && !['INPUT', 'BUTTON'].includes(event.target.tagName)) {
          handleConfirm();
       }
    }
  </script>
  
  <svelte:window on:keydown={keydown}/>
  
  {#if showModal}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" on:click|self={handleClose}>
      <div class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation>
        <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-gray-100">{title}</h2>
  
        <div class="grid grid-cols-10 gap-1 mb-4">
          {#each colors as color}
            <button
              type="button"
              class="w-6 h-6 rounded-full border border-gray-300 dark:border-border focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-900"
              style:background-color={color}
              class:ring-2={selectedColor === color}
              class:ring-blue-500={selectedColor === color}
              class:ring-offset-2={selectedColor === color}
              aria-label={`Select color ${color}`}
              on:click={() => selectColor(color)}
            ></button>
          {/each}
        </div>
  
         <div class="flex items-center mb-6">
             <span class="w-8 h-8 rounded mr-3 border border-gray-400" style:background-color={selectedColor}></span>
             <input type="text" bind:value={selectedColor} class="w-full px-3 py-1 border border-gray-300 dark:border-border rounded-md shadow-sm text-sm bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100">
         </div>
  
        <div class="flex justify-end space-x-3">
          <button
            type="button"
            class="px-4 py-2 bg-gray-200 dark:bg-gray-600 text-gray-800 dark:text-gray-200 rounded-md hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:focus:ring-offset-gray-900"
            on:click={handleClose}
          >
            Cancel
          </button>
          <button
            type="button"
            class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-900"
            on:click={handleConfirm}
          >
            Apply
          </button>
        </div>
      </div>
    </div>
  {/if}