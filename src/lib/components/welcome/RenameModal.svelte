<!-- src/lib/components/welcome/RenameModal.svelte -->
<script>
  import { createEventDispatcher, afterUpdate } from 'svelte';

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
          inputElement = modalElement.querySelector('#projectNameInput');
          if (inputElement) {
              console.log("RenameModal: afterUpdate - Focusing and selecting input.");
              inputElement.focus();
              inputElement.select();
              needsFocus = false; // Reset the flag so it doesn't run again until next open
          } else {
              console.error("RenameModal: afterUpdate - Could not find input element.");
              needsFocus = false; // Reset flag even on error
          }
      } else if (showModal && !needsFocus) {
          // console.log("RenameModal: afterUpdate - Modal visible but focus not needed.");
      }
  });

</script>

{#if showModal}
  <div
    bind:this={modalElement}
    class="fixed inset-0 z-50 flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
    on:click|self={cancel}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="rename-modal-title"
  >
    <div class="bg-white p-6 rounded-lg shadow-xl w-full max-w-md m-4" role="document">
      <h2 id="rename-modal-title" class="text-lg font-semibold text-gray-800 mb-5">Rename Project</h2>
      <div class="mb-5">
        <label for="projectNameInput" class="block text-sm font-medium text-gray-700 mb-1">
            New project name:
        </label>
        <input
            id="projectNameInput"
            type="text"
            class="block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
            bind:value={newName}
            placeholder="Enter new project name"
        />
        <p class="mt-1 text-xs text-gray-500">Original Path: <span class="truncate inline-block max-w-full align-bottom" title={projectXmlPath}>{projectXmlPath || 'N/A'}</span></p>
      </div>
      <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 mt-5">
        <button
            type="button"
            on:click={cancel}
            class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md shadow-sm hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
            Cancel
        </button>
        <button
            type="button"
            on:click={confirm}
            class="px-4 py-2 bg-blue-600 text-white rounded-md shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium disabled:opacity-50"
            disabled={!newName.trim()}
        >
            Rename
        </button>
      </div>
    </div>
  </div>
{/if}