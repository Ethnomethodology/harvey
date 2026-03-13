<!-- src/lib/components/projectview/modals/InsertTableModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Label, Input, Button } from 'flowbite-svelte';
  
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
  
  <Modal bind:open={showModal} size="xs" autoclose={false} outsideclose={true} class="w-full z-[120]" on:close={closeModal}>
    <h2 class="text-xl font-semibold" slot="header">Insert Table</h2>

    <div class="space-y-4" on:keydown={handleKeydown}>
      <div>
        <Label for="table-rows" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Rows:</Label>
        <Input
          type="number"
          id="table-rows"
          bind:this={rowsInput}
          bind:value={rows}
          min="1"
          autocomplete="off"
          autocorrect="off"
        />
      </div>

      <div>
        <Label for="table-columns" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Columns:</Label>
        <Input
          type="number"
          id="table-columns"
          bind:this={columnsInput}
          bind:value={columns}
          min="1"
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