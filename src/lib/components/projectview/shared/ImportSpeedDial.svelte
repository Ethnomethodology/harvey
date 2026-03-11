<script>
    import { SpeedDial, SpeedDialButton } from 'flowbite-svelte';
    import { Plus, Music, FileText, Image as ImageIcon, Sheet, MessageSquareText, Film } from 'lucide-svelte';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    function handleImportAction(type) {
        dispatch('requestImportAction', { type });
    }
</script>

<div class="relative z-50">
    <SpeedDial defaultClass="static" trigger="click" tooltip="right" direction="bottom">
        <svelte:fragment slot="icon">
            <Plus class="w-4 h-4" />
        </svelte:fragment>

        <SpeedDialButton name="Audio" on:click={() => handleImportAction('audio')}>
            <Music class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Document" on:click={() => handleImportAction('document')}>
            <FileText class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Image" on:click={() => handleImportAction('image')}>
            <ImageIcon class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Table" on:click={() => handleImportAction('table')}>
            <Sheet class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Transcript" on:click={() => handleImportAction('transcript')}>
            <MessageSquareText class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Video" on:click={() => handleImportAction('video')}>
            <Film class="w-4 h-4" />
        </SpeedDialButton>
    </SpeedDial>
</div>
