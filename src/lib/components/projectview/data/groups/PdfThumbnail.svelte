<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { FileText, Loader2 } from '@lucide/svelte';
  import { pdfThumbnailQueue } from '$lib/utils/pdfThumbnailQueue';

  export let file; // AssociatedFile object
  export let projectId;

  let canvasRef;
  let isLoading = !file.thumbnail_data;
  let error = null;
  let thumbnailUrl = null;
  let observer;
  let containerRef;
  let isVisible = false;

  $: if (file.thumbnail_data) {
    // If we have binary data from DB, convert to URL
    if (thumbnailUrl) URL.revokeObjectURL(thumbnailUrl);
    const blob = new Blob([new Uint8Array(file.thumbnail_data)], { type: 'image/jpeg' });
    thumbnailUrl = URL.createObjectURL(blob);
    isLoading = false;
  }

  onMount(() => {
    if (!file.thumbnail_data) {
      setupObserver();
    }
  });

  onDestroy(() => {
    if (observer) observer.disconnect();
    if (thumbnailUrl) URL.revokeObjectURL(thumbnailUrl);
  });

  function setupObserver() {
    observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !isVisible) {
          isVisible = true;
          startRenderTask();
        }
      },
      { threshold: 0.1 }
    );

    if (containerRef) observer.observe(containerRef);
  }

  async function startRenderTask() {
    if (file.thumbnail_data || !isVisible) return;

    pdfThumbnailQueue.add(async () => {
      try {
        isLoading = true;
        // Dynamically import pdfjs to keep it lighthouse-friendly
        const pdfjsLib = await import('pdfjs-dist');
        const PDFJSWorker = (await import('pdfjs-dist/build/pdf.worker.min.mjs?url')).default;
        pdfjsLib.GlobalWorkerOptions.workerSrc = PDFJSWorker;

        const assetUrl = convertFileSrc(file.full_path);
        const loadingTask = pdfjsLib.getDocument(assetUrl);
        const pdf = await loadingTask.promise;

        // Render first page
        const page = await pdf.getPage(1);
        const viewport = page.getViewport({ scale: 0.5 }); // Low scale for thumbnail

        const canvas = document.createElement('canvas');
        const context = canvas.getContext('2d');
        canvas.height = viewport.height;
        canvas.width = viewport.width;

        await page.render({ canvasContext: context, viewport }).promise;

        // Convert to blob for DB saving and local display
        const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/jpeg', 0.8));
        const arrayBuffer = await blob.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        // Save to DB
        await invoke('save_pdf_metadata', {
          projectId,
          assetRelativePath: file.relative_path,
          thumbnail: Array.from(uint8Array)
        });

        // Update local UI
        if (thumbnailUrl) URL.revokeObjectURL(thumbnailUrl);
        thumbnailUrl = URL.createObjectURL(blob);

        // Clean up pdf documents
        await pdf.destroy();
      } catch (e) {
        console.error('[PdfThumbnail] Failed to render:', e);
        error = e.message;
      } finally {
        isLoading = false;
      }
    });
  }
</script>

<div
  bind:this={containerRef}
  class="w-full h-full relative overflow-hidden flex items-center justify-center group"
>
  {#if thumbnailUrl}
    <img
      src={thumbnailUrl}
      alt="PDF Preview"
      class="w-full h-full object-cover opacity-90 transition-opacity duration-300 group-hover:opacity-100"
    />
  {:else if isLoading}
    <div class="flex flex-col items-center gap-2 text-gray-400">
      <Loader2 size={24} class="animate-spin text-blue-500/50" />
      <span class="text-[10px] uppercase font-medium tracking-wider">Rendering</span>
    </div>
  {:else}
    <div class="flex flex-col items-center gap-1 text-gray-300 dark:text-gray-700">
      <FileText size={40} strokeWidth={1} />
    </div>
  {/if}

  {#if error}
    <div
      class="absolute inset-0 bg-red-50/90 dark:bg-red-950/90 flex items-center justify-center p-2 text-center"
    >
      <span class="text-[10px] text-red-600 dark:text-red-400 font-medium">Preview Failed</span>
    </div>
  {/if}
</div>

<style>
  img {
    image-rendering: -webkit-optimize-contrast;
  }
</style>
