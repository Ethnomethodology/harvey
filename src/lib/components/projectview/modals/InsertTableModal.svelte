<!-- src/lib/components/projectview/modals/InsertTableModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Label, Input, Button } from 'flowbite-svelte';
    import { Table as TableIcon, Rows, Columns } from '@lucide/svelte';
  
    export let showModal = false;
  
    let rows = 2;
    let columns = 3;
    let rowsInput;
    let columnsInput;
  
    const dispatch = createEventDispatcher();
  
    function handleConfirm() {
      if (rows > 0 && columns > 0) {
        dispatch('confirm', { rows, columns });
        closeModal();
      } else {
        alert('Please enter valid numbers for rows and columns (must be greater than 0).');
      }
    }
  
    function closeModal() {
      dispatch('close');
    }
  
    function handleKeydown(event) {
      if (event.key === 'Escape') {
        closeModal();
      }
      if (event.key === 'Enter') {
          if (document.activeElement === rowsInput || document.activeElement === columnsInput) {
              handleConfirm();
          }
      }
    }
  
    $: if (showModal && rowsInput) {
      setTimeout(() => rowsInput.focus(), 50);
    }
  </script>
  
<Modal
    bind:open={showModal}
    size="xs"
    autoclose={false}
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
    on:close={closeModal}
>
    <div slot="header" class="flex items-center gap-2">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <TableIcon class="w-5 h-5 text-blue-600 dark:text-blue-400" />
        </div>
        <div class="flex flex-col">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white leading-tight">Insert Table</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Specify grid dimensions</p>
        </div>
    </div>

    <div class="space-y-5 py-2" on:keydown={handleKeydown}>
      <div class="space-y-2">
        <Label for="table-rows" class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
            <Rows size={14} class="text-gray-400" />
            Rows
        </Label>
        <Input
          type="number"
          id="table-rows"
          bind:this={rowsInput}
          bind:value={rows}
          min="1"
          class="bg-gray-50 dark:bg-gray-800"
          autocomplete="off"
          autocorrect="off"
        />
      </div>

      <div class="space-y-2">
        <Label for="table-columns" class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
            <Columns size={14} class="text-gray-400" />
            Columns
        </Label>
        <Input
          type="number"
          id="table-columns"
          bind:this={columnsInput}
          bind:value={columns}
          min="1"
          class="bg-gray-50 dark:bg-gray-800"
          autocomplete="off"
          autocorrect="off"
        />
      </div>
    </div>

    <svelte:fragment slot="footer">
      <div class="flex justify-end space-x-3 w-full">
        <Button color="alternative" on:click={closeModal}>
          Cancel
        </Button>
        <Button color="blue" on:click={handleConfirm}>
          Insert
        </Button>
      </div>
    </svelte:fragment>
</Modal>