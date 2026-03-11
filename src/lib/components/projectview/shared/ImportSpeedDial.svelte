<script>
    import { SpeedDial, SpeedDialButton } from 'flowbite-svelte';
    import { Plus, FileAudio, FileVideo, FileText, Image as ImageIcon, Table, MessageSquare } from 'lucide-svelte';
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher();

    function handleImportAction(type) {
        dispatch('requestImportAction', { type });
    }
</script>

<div class="relative z-50">
    <SpeedDial defaultClass="static" trigger="click" tooltip="left" direction="bottom">
        <svelte:fragment slot="icon">
            <Plus class="w-4 h-4" />
        </svelte:fragment>

        <SpeedDialButton name="Audio" on:click={() => handleImportAction('audio')}>
            <FileAudio class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Video" on:click={() => handleImportAction('video')}>
            <FileVideo class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Document" on:click={() => handleImportAction('document')}>
            <FileText class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Image" on:click={() => handleImportAction('image')}>
            <ImageIcon class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Table" on:click={() => handleImportAction('table')}>
            <Table class="w-4 h-4" />
        </SpeedDialButton>
        <SpeedDialButton name="Transcript" on:click={() => handleImportAction('transcript')}>
            <MessageSquare class="w-4 h-4" />
        </SpeedDialButton>
    </SpeedDial>
</div>
