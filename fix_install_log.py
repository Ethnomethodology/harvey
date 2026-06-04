import re

with open('src/lib/components/modals/InstallLogModal.svelte', 'r') as f:
    text = f.read()

# 1. Import X from lucide-svelte and ask from plugin-dialog
if "import { ask } from '@tauri-apps/plugin-dialog';" not in text:
    text = text.replace(
        "import { Modal, Button } from 'flowbite-svelte';",
        "import { Modal, Button } from 'flowbite-svelte';\n  import { X } from 'lucide-svelte';\n  import { ask } from '@tauri-apps/plugin-dialog';"
    )

# 2. Modify handleCloseClick to use ask()
old_handle_close = r'''  function handleCloseClick\(\) \{
    if \(isInstalling\) \{
      showCancelConfirm = true;
    \} else if \(\!isChecking\) \{
      showModal = false;
    \}
  \}

  function confirmCancel\(\) \{
    showCancelConfirm = false;
    showModal = false;
    dispatch\('cancel'\);
  \}'''

new_handle_close = r'''  async function handleCloseClick() {
    if (isInstalling) {
      const confirmed = await ask(
        'Are you sure you want to cancel this download? All progress will be lost and temporary files deleted.',
        { title: 'Cancel Download?', kind: 'warning', okLabel: 'Cancel Download', cancelLabel: 'Keep Downloading' }
      );
      if (confirmed) {
        showModal = false;
        dispatch('cancel');
      }
    } else if (!isChecking) {
      showModal = false;
    }
  }'''
text = re.sub(old_handle_close, new_handle_close, text)

# 3. Add dismissable={false} to Modal
if "dismissable={false}" not in text:
    text = text.replace(
        "autoclose={false}",
        "autoclose={false}\n  dismissable={false}"
    )

# 4. Modify header slot to include our custom X button
old_header = r'''  <h2
    id="log-modal-title"
    class="text-lg font-semibold text-gray-900 dark:text-white"
    slot="header"
  >
    \{title\}
  </h2>'''

new_header = r'''  <svelte:fragment slot="header">
    <div class="flex justify-between items-center w-full">
      <h2 id="log-modal-title" class="text-lg font-semibold text-gray-900 dark:text-white">
        {title}
      </h2>
      <button
        class="text-gray-400 bg-transparent hover:bg-gray-200 hover:text-gray-900 rounded-lg text-sm p-1.5 inline-flex items-center dark:hover:bg-gray-600 dark:hover:text-white"
        on:click={handleCloseClick}
      >
        <X class="w-5 h-5" />
      </button>
    </div>
  </svelte:fragment>'''
text = re.sub(old_header, new_header, text)

# 5. Remove the "Cancel Download" button from the footer, and only show "Close" when not installing.
old_footer = r'''  <svelte:fragment slot="footer">
    <div class="flex justify-end w-full">
      <Button color="alternative" on:click=\{handleCloseClick\} disabled=\{isChecking\}>
        \{#if isInstalling\}
          Cancel Download
        \{:else if isChecking\}
          Checking\.\.\.
        \{:else\}
          Close
        \{/if\}
      </Button>
    </div>
  </svelte:fragment>'''

new_footer = r'''  <svelte:fragment slot="footer">
    <div class="flex justify-end w-full">
      {#if !isInstalling}
        <Button color="alternative" on:click={handleCloseClick} disabled={isChecking}>
          {#if isChecking}
            Checking...
          {:else}
            Close
          {/if}
        </Button>
      {/if}
    </div>
  </svelte:fragment>'''
text = re.sub(old_footer, new_footer, text)

# 6. Remove the Svelte ConfirmCancelModal
text = re.sub(r'<Modal bind:open=\{showCancelConfirm\} size="sm" autoclose>.*?</Modal>', '', text, flags=re.DOTALL)

# 7. Remove showCancelConfirm variable declaration
text = re.sub(r'  let showCancelConfirm = false;\n', '', text)

with open('src/lib/components/modals/InstallLogModal.svelte', 'w') as f:
    f.write(text)

