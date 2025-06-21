<!-- src/lib/components/projectview/shared/FileContextMenu.svelte -->
<script>
  import { createEventDispatcher } from 'svelte';

  export let item = null; // File object { name, path, relativePath, file_type, media_xml_identifier? }
  export let x = 0;
  export let y = 0;
  export let isVisible = false;
  export let revealLabel = 'Reveal in File System'; // New prop with default

  const dispatch = createEventDispatcher();

  const AUDIO_EXTENSIONS = ['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac'];
  const VIDEO_EXTENSIONS = ['mp4', 'mov', 'avi', 'mkv', 'webm'];

  function isMedia(fileItem) {
    if (!fileItem || !fileItem.name) return false;
    const ext = fileItem.name.split('.').pop()?.toLowerCase() || '';
    return AUDIO_EXTENSIONS.includes(ext) || VIDEO_EXTENSIONS.includes(ext);
  }

  function emitAction(actionName) {
    if (item) {
      dispatch(actionName, { item });
    }
    // Automatically hide the menu after an action is dispatched
    isVisible = false;
  }
</script>

{#if isVisible && item}
  <div
    class="fixed z-[100] bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md shadow-xl py-1 text-xs min-w-[180px]"
    style="left: {x}px; top: {y}px;"
    on:click|stopPropagation
    role="menu"
  >
    <ul>
      <li>
        <button on:click={() => emitAction('open')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Open</button>
      </li>
      {#if isMedia(item)}
        <li>
          <button on:click={() => emitAction('transcribe')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Transcribe</button>
        </li>
      {/if}
      <li>
        <button on:click={() => emitAction('addToGroup')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Add to Group...</button>
      </li>
      <li>
        <button on:click={() => emitAction('removeFromGroup')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Remove from this Group</button>
      </li>
      <hr class="my-1 border-gray-200 dark:border-gray-600" />
      <li>
        <button on:click={() => emitAction('reveal')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">{revealLabel}</button>
      </li>
      <hr class="my-1 border-gray-200 dark:border-gray-600" />
      <li>
        <button on:click={() => emitAction('rename')} class="block w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-200">Rename...</button>
      </li>
      <li>
        <button on:click={() => emitAction('delete')} class="block w-full text-left px-3 py-1.5 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/50 dark:text-red-500">Delete...</button>
      </li>
    </ul>
  </div>
{/if}

<style>
  /* Basic styling, can be enhanced */
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  button {
    transition: background-color 0.1s ease-in-out;
  }
</style>
