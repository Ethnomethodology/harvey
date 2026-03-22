<!-- src/lib/components/projectview/lexical/InsertEquationModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { Modal, Label, Button, Checkbox, Textarea } from 'flowbite-svelte';
    import katex from 'katex';
    import 'katex/dist/katex.min.css';

    export let showModal = false;
    export let initialEquation = '';
    export let initialInline = true;

    let equation = initialEquation;
    let inline = initialInline;
    let equationInput;
    let previewContainer;

    const dispatch = createEventDispatcher();

    function handleConfirm() {
      if (equation.trim()) {
        dispatch('confirm', { equation, inline });
        closeModal();
      }
    }

    function closeModal() {
      equation = '';
      inline = true;
      dispatch('close');
    }

    function handleKeydown(event) {
      if (event.key === 'Escape') {
        closeModal();
      }
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        handleConfirm();
      }
    }

    $: if (showModal && equationInput) {
      setTimeout(() => {
        equationInput.focus();
        equation = initialEquation;
        inline = initialInline;
      }, 50);
    }

    $: if (previewContainer && (equation !== undefined || inline !== undefined)) {
        try {
            previewContainer.innerHTML = '';
            if (equation.trim()) {
                katex.render(equation, previewContainer, {
                    displayMode: !inline,
                    throwOnError: false,
                    errorColor: '#cc0000',
                });
            } else {
                previewContainer.innerHTML = '<span class="text-gray-400 italic">Preview...</span>';
            }
        } catch (e) {
            previewContainer.innerHTML = `<span class="text-red-500 text-sm">${e.message}</span>`;
        }
    }
</script>

<Modal bind:open={showModal} size="md" autoclose={false} outsideclose={true} class="w-full z-[120]" on:close={closeModal}>
  <h2 class="text-xl font-semibold" slot="header">{initialEquation ? 'Edit Equation' : 'Insert Equation'}</h2>

  <div class="space-y-4" on:keydown={handleKeydown}>
    <div>
      <div class="flex justify-between items-center mb-1">
        <Label for="equation-input" class="text-sm font-medium text-gray-700 dark:text-gray-300">LaTeX Equation:</Label>
        <span class="text-xs text-gray-500">Ctrl+Enter to save</span>
      </div>
      <Textarea
        id="equation-input"
        bind:this={equationInput}
        bind:value={equation}
        rows="4"
        class="font-mono text-sm"
        placeholder="e.g., E = mc^2 or \frac{a}{b}"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
      />
    </div>

    <div class="flex items-center space-x-2">
      <Checkbox bind:checked={inline} id="equation-inline" class="text-blue-600 focus:ring-blue-500" />
      <Label for="equation-inline" class="text-sm font-medium text-gray-700 dark:text-gray-300">Inline Equation</Label>
    </div>

    <div>
      <Label class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Preview:</Label>
      <div
        bind:this={previewContainer}
        class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg min-h-[4rem] flex items-center justify-center bg-gray-50 dark:bg-gray-900 overflow-x-auto"
      >
        <span class="text-gray-400 italic">Preview...</span>
      </div>
    </div>
  </div>

  <svelte:fragment slot="footer">
    <div class="flex justify-end space-x-3 w-full">
      <Button color="alternative" on:click={closeModal}>
        Cancel
      </Button>
      <Button color="blue" on:click={handleConfirm} disabled={!equation.trim()}>
        {initialEquation ? 'Update' : 'Insert'}
      </Button>
    </div>
  </svelte:fragment>
</Modal>