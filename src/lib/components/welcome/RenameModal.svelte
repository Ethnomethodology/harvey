<!-- src/lib/components/welcome/RenameModal.svelte -->
<script>
  import { createEventDispatcher, afterUpdate, tick } from 'svelte';
  import { Modal, Label, Input, Button } from 'flowbite-svelte';

  export let showModal = false;
  export let projectToRename = null;

  let newName = '';
  let projectXmlPath = '';
  let modalElement; // Reference to the modal container DOM element
  let inputElement; // Reference to the input element

  // Flag to control focus/select logic
  let needsFocus = false;

  const dispatch = createEventDispatcher();

  // Update local component state when the project prop changes
  function updateLocalState() {
    if (projectToRename) {
      newName = projectToRename.name || '';
      projectXmlPath = projectToRename.path || '';
       // Signal that focus is needed when the project is set (modal is about to open)
      needsFocus = true;
      console.log("RenameModal: Project set, needsFocus=true", projectToRename);
    } else {
      newName = '';
      projectXmlPath = '';
      needsFocus = false; // Reset flag if project is cleared
    }
  }

  // Reactively update local state when the prop changes
  $: if (projectToRename !== undefined) { // Trigger on any change, including null
      updateLocalState();
  }

  // --- MODAL ACTIONS ---
  function confirm() {
      if (!projectXmlPath) {
          console.error("RenameModal: Cannot confirm rename, projectXmlPath is missing.");
          alert("Error: Could not identify the project to rename.");
          cancel(); // Close modal on error
          return;
      }
      const trimmedNewName = newName.trim();
      if (!trimmedNewName) {
          alert("Project name cannot be empty.");
          return; // Keep modal open for correction
      }
      console.log("RenameModal: Dispatching confirm event:", { projectXmlPath, newName: trimmedNewName });
      dispatch('confirm', { projectXmlPath: projectXmlPath, newName: trimmedNewName });
      needsFocus = false; // Reset focus flag after confirm
  }

  function cancel() {
    console.log("RenameModal: Dispatching cancel event.");
    dispatch('cancel'); // Parent handles closing the modal via bind:showModal
    needsFocus = false; // Reset focus flag on cancel
  }

  // --- KEYBOARD HANDLING ---
  function handleKeydown(event) {
    if (event.key === 'Escape') {
      cancel();
    }
    if (event.key === 'Enter' && newName.trim()) {
        confirm();
    }
  }

  // --- FOCUS & SELECT LOGIC using afterUpdate + Flag ---
  afterUpdate(() => {
      // Only run if the modal is visible AND we explicitly need to set focus
      if (showModal && needsFocus && modalElement) {
          tick().then(() => {
            const el = modalElement.querySelector('input');
            if (el) {
                console.log("RenameModal: afterUpdate - Focusing and selecting input.");
                el.focus();
                el.select();
                needsFocus = false; // Reset the flag so it doesn't run again until next open
            }
          })
      }
  });

</script>

<Modal bind:open={showModal} size="md" autoclose={false} outsideclose={true} class="w-full z-50" on:close={cancel}>
    <h2 id="rename-modal-title" class="text-lg font-semibold text-gray-800" slot="header">Rename Project</h2>

    <div bind:this={modalElement} class="space-y-4" on:keydown={handleKeydown}>
        <div>
            <Label for="projectNameInput" class="mb-1 text-sm font-medium text-gray-700">New project name:</Label>
            <Input
                id="projectNameInput"
                type="text"
                bind:value={newName}
                placeholder="Enter new project name"
                autocomplete="off"
                autocorrect="off"
            />
            <p class="mt-1 text-xs text-gray-500">Original Path: <span class="truncate inline-block max-w-full align-bottom" title={projectXmlPath}>{projectXmlPath || 'N/A'}</span></p>
        </div>
    </div>

    <svelte:fragment slot="footer">
        <div class="flex justify-end space-x-3 w-full">
            <Button color="alternative" on:click={cancel}>Cancel</Button>
            <Button color="blue" on:click={confirm} disabled={!newName.trim()}>Rename</Button>
        </div>
    </svelte:fragment>
</Modal>