<!-- src/lib/components/projectview/lexical/InsertEquationModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { Modal, Label, Button, Checkbox, Textarea } from 'flowbite-svelte';
    import { Sigma, CheckCircle2 } from '@lucide/svelte';
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
        document.getElementById('equation-input')?.focus();
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

<Modal
    bind:open={showModal}
    size="md"
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
            <Sigma class="w-5 h-5 text-blue-600 dark:text-blue-400" />
        </div>
        <div class="flex flex-col">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white leading-tight">
                {initialEquation ? 'Edit Equation' : 'Insert Equation'}
            </h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Add mathematical expressions via LaTeX</p>
        </div>
    </div>

  <div class="space-y-5 py-2" on:keydown={handleKeydown}>
    <div class="space-y-2">
      <div class="flex justify-between items-center mb-1">
        <Label for="equation-input" class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
            <Sigma size={14} class="text-gray-400" />
            LaTeX Expression
        </Label>
        <span class="text-xs text-gray-500">Ctrl+Enter to save</span>
      </div>
      <Textarea
        id="equation-input"
        bind:this={equationInput}
        bind:value={equation}
        rows="4"
        class="font-mono text-sm bg-gray-50 dark:bg-gray-800"
        placeholder={"e.g., E = mc^2 or \\frac{a}{b}"}
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
      />
    </div>

    <div class="pt-2">
      <Checkbox bind:checked={inline} id="equation-inline" class="text-sm font-medium text-gray-700 dark:text-gray-300">
        Inline Equation
      </Checkbox>
    </div>

    <!-- Preview Result -->
    <div class="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl border border-blue-100 dark:border-blue-800/50">
        <div class="flex items-center justify-between mb-2">
            <span class="text-[10px] font-extrabold uppercase tracking-widest text-blue-600/60">Preview</span>
            <CheckCircle2 size={14} class="text-blue-500" />
        </div>
        <div
            bind:this={previewContainer}
            class="text-sm font-semibold text-blue-800 dark:text-blue-300 flex items-center justify-center min-h-[3rem] overflow-x-auto"
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
        {initialEquation ? 'Update' : 'Confirm'}
      </Button>
    </div>
  </svelte:fragment>
</Modal>
