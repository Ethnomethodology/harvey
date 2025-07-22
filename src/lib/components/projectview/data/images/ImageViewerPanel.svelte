<script>
    import { onMount, onDestroy, tick } from 'svelte';
    import { convertFileSrc, invoke } from '@tauri-apps/api/core';
    import { dirname, join, sep } from '@tauri-apps/api/path'; // ensure sep is imported
    import { get } from 'svelte/store';
    import { project } from '$lib/stores/projectStore.js';
    import OpenSeadragon from 'openseadragon';
    import OpenSeadragonAnnotator from '@recogito/annotorious-openseadragon';
    import Toolbar from '@recogito/annotorious-toolbar';

    export let imagePath = '';

    let osdViewerElement;
    let osdViewer = null;
    let anno = null;
    let toolbar = null;

    let isLoading = true;
    let error = null;
    let currentLoadedPath = null;

    

    import AnnotationCreationDialog from '$lib/components/modals/AnnotationCreationDialog.svelte';

    let showAnnotationCreationDialog = false;
    let dialogX = 0;
    let dialogY = 0;
    let annotationBeingCreated = null; // Stores the annotation data before it's formally added

    let currentAnnotations = [];

    const DELETE_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="w-4 h-4" viewBox="0 0 16 16"><path d="M5.5 5.5A.5.5 0 0 1 6 5h4a.5.5 0 0 1 0 1H6a.5.5 0 0 1-.5-.5m2.5 3a.5.5 0 0 0-.5.5v4a.5.5 0 0 0 1 0v-4a.5.5 0 0 0-.5-.5"/><path d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4zM2.5 3h11V2h-11z"/></svg>`;

    async function loadAnnotationsForImage(imgPath) {
        if (!anno) {
            console.warn("[ImageViewerPanel loadAnnotationsForImage] Annotorious not initialized.");
            currentAnnotations = [];
            return;
        }
        anno.clearAnnotations();
        currentAnnotations = [];

        const currentProj = get(project);
        if (!currentProj || !currentProj.id || typeof currentProj.id !== 'string' || currentProj.id.trim() === '') {
            console.error('[ImageViewerPanel loadAnnotationsForImage] project ID (from $project.id) is missing or invalid.');
            return;
        }
        const projectId = currentProj.id;

        const projectBaseDir = currentProj.baseDirectory;
        let relativeImagePath = imgPath;

        if (projectBaseDir && imgPath.startsWith(projectBaseDir)) {
            relativeImagePath = imgPath.substring(projectBaseDir.length);
            if (relativeImagePath.startsWith(sep) || relativeImagePath.startsWith('/') || relativeImagePath.startsWith('\\')) {
                relativeImagePath = relativeImagePath.substring(1);
            }
        } else {
            console.warn(`[ImageViewerPanel loadAnnotationsForImage] imgPath "${imgPath}" may not be within projectBaseDir "${projectBaseDir}". Using path as is for DB key, this might fail if not truly relative.`);
        }
        relativeImagePath = relativeImagePath.replace(/\\/g, '/');

        console.log(`[ImageViewerPanel loadAnnotationsForImage] Attempting for project ${projectId} (from $project.id), image relative path: ${relativeImagePath} (absolute: ${imgPath})`);

        try {
            const annotationsJsonString = await invoke('load_image_annotations', {
                projectId: projectId,
                imageRelativePathStr: relativeImagePath
            });
            if (annotationsJsonString && typeof annotationsJsonString === 'string') {
                const loaded = JSON.parse(annotationsJsonString);
                if (Array.isArray(loaded)) {
                    anno.setAnnotations(loaded);
                    currentAnnotations = JSON.parse(JSON.stringify(loaded));
                    console.log(`[ImageViewerPanel loadAnnotationsForImage] Loaded ${loaded.length} annotations for ${relativeImagePath}.`);
                } else {
                    console.warn(`[ImageViewerPanel loadAnnotationsForImage] Loaded data for ${relativeImagePath} is not an array.`);
                }
            } else {
                 console.log(`[ImageViewerPanel loadAnnotationsForImage] No annotations found or empty content for ${relativeImagePath}.`);
            }
        } catch (err) {
            console.error(`[ImageViewerPanel loadAnnotationsForImage] Error for ${relativeImagePath} (absolute: ${imgPath}):`, err);
            currentAnnotations = [];
        }
    }

    async function saveAnnotationsForImage() {
        if (!currentLoadedPath) {
            console.warn("[ImageViewerPanel saveAnnotationsForImage] No image path, cannot save.");
            return;
        }
        const currentProj = get(project);
        if (!currentProj || !currentProj.id || typeof currentProj.id !== 'string' || currentProj.id.trim() === '') {
            console.error('[ImageViewerPanel saveAnnotationsForImage] project ID (from $project.id) is missing or invalid.');
            await message('Cannot save image annotations: Project identifier is missing or invalid.', { title: 'Save Error', type: 'error' });
            return;
        }
        const projectId = currentProj.id;

        const projectBaseDir = currentProj.baseDirectory;
        let relativeImagePath = currentLoadedPath;

        if (projectBaseDir && currentLoadedPath.startsWith(projectBaseDir)) {
            relativeImagePath = currentLoadedPath.substring(projectBaseDir.length);
            if (relativeImagePath.startsWith(sep) || relativeImagePath.startsWith('/') || relativeImagePath.startsWith('\\')) {
                relativeImagePath = relativeImagePath.substring(1);
            }
        } else {
            console.error(`[ImageViewerPanel saveAnnotationsForImage] Cannot determine relative path for ${currentLoadedPath} using base ${projectBaseDir}. Cannot save.`);
            return;
        }
        relativeImagePath = relativeImagePath.replace(/\\/g, '/');

        console.log(`[ImageViewerPanel saveAnnotationsForImage] Saving ${currentAnnotations.length} annotations for project ${projectId} (from $project.id), image relative path: ${relativeImagePath}`);

        try {
            await invoke('save_image_annotations', {
                projectId: projectId,
                imageRelativePathStr: relativeImagePath,
                annotationsJsonString: JSON.stringify(currentAnnotations, null, 2)
            });
            console.log(`[ImageViewerPanel saveAnnotationsForImage] Annotations saved for project ${projectId} (from $project.id), image ${relativeImagePath}`);
        } catch (err) {
            console.error(`[ImageViewerPanel saveAnnotationsForImage] Error saving for project ${projectId} (from $project.id), image ${relativeImagePath}:`, err);
        }
    }


    async function initializeViewer(pathForImage) {
        console.log(`[ImageViewerPanel initializeViewer] Attempting for path: ${pathForImage}`);
        if (!pathForImage || !osdViewerElement) {
            console.warn('[ImageViewerPanel initializeViewer] Skipping: no path or osdViewerElement.', { pathForImage, osdViewerElementExists: !!osdViewerElement });
            isLoading = false; error = 'Viewer element not ready or path missing.'; return;
        }

        console.log(`[ImageViewerPanel initializeViewer] Proceeding with initialization for: ${pathForImage}`);
        currentLoadedPath = pathForImage; isLoading = true; error = null;
        currentAnnotations = [];

        if (anno) { try { anno.destroy(); console.log("Previous Annotorious instance destroyed."); } catch (e) { console.warn("Error destroying previous Annotorious instance:", e); } anno = null; }
        if (toolbar) { try { toolbar.destroy(); console.log("Previous Toolbar instance destroyed."); } catch(e) { console.warn("Error destroying toolbar", e)} toolbar = null; }
        if (osdViewer) { try { osdViewer.destroy(); console.log("Previous OpenSeadragon instance destroyed."); } catch (e) { console.warn("Error destroying previous OpenSeadragon instance:", e); } osdViewer = null; }
        if(osdViewerElement) osdViewerElement.innerHTML = '';

        try {
            const assetUrl = convertFileSrc(pathForImage);
            console.log(`[ImageViewerPanel] Converted asset URL: ${assetUrl}`);
            await tick();
            if (!osdViewerElement) {
                error = 'OSD container element became null during async operations.';
                console.error(`[ImageViewerPanel initializeViewer] ${error}`);
                isLoading = false; currentLoadedPath = null; return;
            }

            osdViewer = OpenSeadragon({
                element: osdViewerElement,
                prefixUrl: '/openseadragon-icons/',
                crossOriginPolicy: 'Anonymous',
                loadTilesWithAjax: true,
                ajaxWithCredentials: false,
                tileSources: { type: 'image', url: assetUrl },
                animationTime: 0.5, blendTime: 0.1, constrainDuringPan: true,
                maxZoomPixelRatio: 2, minZoomImageRatio: 0.8, visibilityRatio: 1,
                zoomPerScroll: 1.2,
                gestureSettingsMouse: { scrollToZoom: true, clickToZoom: false, dblClickToZoom: false, pinchToZoom: true },
                showNavigator: false,
                showNavigationControl: false,
            });

            osdViewer.addHandler('open', async () => {
                console.log('[ImageViewerPanel OSD Event] "open" event triggered.');
                isLoading = false;

                if (typeof OpenSeadragonAnnotator === 'undefined') {
                    console.error("OpenSeadragonAnnotator is undefined! Check import.");
                    error = "Annotorious library component failed to load."; isLoading = false; return;
                }
                anno = OpenSeadragonAnnotator(osdViewer, {
                    disableEditor: true, // Disable the default Annotorious editor
                    allowEmpty: true,
                    formatter: (annotation) => {
                        if (annotation.body) {
                            const colorBody = annotation.body.find(b => b.purpose === 'highlighting' && b.type === 'Color');
                            if (colorBody && colorBody.value) {
                                return { 'style': `stroke-width:2; stroke: ${adjustOpacity(colorBody.value, 1)}; fill: ${colorBody.value}` };
                            }
                        }
                        return { 'style': `stroke-width:2; stroke: ${adjustOpacity('rgba(255, 242, 117, 0.5)', 1)}; fill: rgba(255, 242, 117, 0.5)` }; // Default yellow // Default yellow
                    }
                });
                if (!anno) { error = "Failed to initialize Annotorious."; isLoading = false; return; }

                const toolbarContainerEl = document.getElementById('image-annotation-toolbar-container');
                if (toolbarContainerEl) {
                     if (typeof Toolbar === 'undefined') {
                        console.error("Toolbar (from @recogito/annotorious-toolbar) is undefined! Check import.");
                        error = "Annotorious Toolbar library component failed to load."; return;
                    }
                    toolbar = new Toolbar(anno, toolbarContainerEl);
                } else {
                    console.error("[ImageViewerPanel] CRITICAL: Toolbar container 'image-annotation-toolbar-container' NOT found!");
                }

                anno.setDrawingTool('rect');
                setupAnnotationEvents();
                await loadAnnotationsForImage(pathForImage);
                console.log('[ImageViewerPanel] Annotorious setup complete.');
            });

            osdViewer.addHandler('open-failed', (event) => {
                console.error('[ImageViewerPanel OSD Event] "open-failed" event triggered:', event);
                let detail = event.message || 'Unknown OSD error';
                if (event.source && event.source.url) detail += ` (URL: ${event.source.url})`;
                error = `OpenSeadragon failed to open image: ${detail}`;
                isLoading = false;
            });
        } catch (err) {
            console.error(`[ImageViewerPanel initializeViewer] Top-level error for path ${pathForImage}:`, err);
            error = `Failed to load image viewer: ${err.message || err}`;
            currentLoadedPath = null; isLoading = false;
        }
    }

    function adjustOpacity(rgbaColor, newOpacity) {
        if (!rgbaColor || typeof rgbaColor !== 'string' || !rgbaColor.startsWith('rgba(')) { return rgbaColor; }
        const parts = rgbaColor.substring(5, rgbaColor.length - 1).split(',');
        if (parts.length !== 4) return rgbaColor;
        return `rgba(${parts[0].trim()}, ${parts[1].trim()}, ${parts[2].trim()}, ${newOpacity})`;
    }

    function setupAnnotationEvents() {
        if (!anno) { console.warn("setupAnnotationEvents: Annotorious instance not available."); return; }
        console.log("[ImageViewerPanel] Setting up annotation events.");
        anno.off('createAnnotation'); anno.off('selectAnnotation'); anno.off('cancelSelected');
        anno.off('updateAnnotation'); anno.off('deleteAnnotation');
        anno.off('createSelection'); // Ensure this is off to prevent multiple listeners

        anno.on('createSelection', (selection) => {
            console.log('[ImageViewerPanel createSelection] Event fired. Selection:', JSON.parse(JSON.stringify(selection)));
            // Store the selection temporarily
            annotationBeingCreated = selection;

            // Get the coordinates for the dialog.
            // For rectangles, use the end of the drag. For polygons, use the last double-click point.
            // Annotorious selection provides screen coordinates in .rendered.
            let clientX, clientY;
            if (selection.rendered && selection.rendered.geometry) {
                if (selection.target.selector.type === 'FragmentSelector') { // Rectangle
                    const rect = selection.rendered.geometry;
                    clientX = rect.x + rect.width;
                    clientY = rect.y + rect.height;
                } else if (selection.target.selector.type === 'SvgSelector') { // Polygon
                    const points = selection.rendered.geometry.points;
                    if (points && points.length > 0) {
                        let sumX = 0, sumY = 0;
                        for (const p of points) {
                            sumX += p.x;
                            sumY += p.y;
                        }
                        clientX = sumX / points.length;
                        clientY = sumY / points.length;
                    } else {
                        clientX = osdViewerElement.clientWidth / 2;
                        clientY = osdViewerElement.clientHeight / 2;
                    }
                } else {
                    clientX = osdViewerElement.clientWidth / 2;
                    clientY = osdViewerElement.clientHeight / 2;
                }
            } else {
                // Fallback if rendered geometry is not immediately available
                console.warn("Selection rendered geometry not available at createSelection event. Using fallback dialog position.");
                clientX = osdViewerElement.clientWidth / 2;
                clientY = osdViewerElement.clientHeight / 2;
            }

            // Convert client coordinates to coordinates relative to the osdViewerElement
            const osdRect = osdViewerElement.getBoundingClientRect();
            dialogX = clientX - osdRect.left;
            dialogY = clientY - osdRect.top;

            showAnnotationCreationDialog = true;
        });

        

        anno.on('updateAnnotation', async (annotation, _previous) => {
            console.log('[ImageViewerPanel updateAnnotation] Annotation updated:', JSON.parse(JSON.stringify(annotation)));
            const updatedAnnotationData = JSON.parse(JSON.stringify(annotation));
            const index = currentAnnotations.findIndex(a => a.id === updatedAnnotationData.id);
            if (index > -1) {
                currentAnnotations[index] = updatedAnnotationData;
            } else {
                console.warn("[ImageViewerPanel updateAnnotation] Updated annotation not found in local cache, adding it. ID:", updatedAnnotationData.id);
                currentAnnotations.push(updatedAnnotationData);
            }
            await saveAnnotationsForImage();
        });

        anno.on('deleteAnnotation', async (annotation) => {
            console.log('[ImageViewerPanel deleteAnnotation] Annotation deleted:', JSON.parse(JSON.stringify(annotation)));
            currentAnnotations = currentAnnotations.filter(a => a.id !== annotation.id);
            await saveAnnotationsForImage();
            });
    }

    function fitToViewer() { if (osdViewer) { osdViewer.viewport.goHome(false); } }

    async function handleAnnotationDialogSave(event) {
        const { title, description, color } = event.detail;
        if (annotationBeingCreated) {
            const newBody = [];
            if (title) {
                newBody.push({ type: 'Title', value: title, purpose: 'commenting' });
            }
            if (description) {
                newBody.push({ type: 'Description', value: description, purpose: 'commenting' });
            }
            newBody.push({ type: 'Color', value: color, purpose: 'highlighting' });

            const annotationToSave = {
                ...annotationBeingCreated,
                body: newBody
            };

            // Add the annotation to Annotorious
            anno.addAnnotation(annotationToSave);
            // Annotorious's 'createAnnotation' event listener will handle saving to DB

            // Clear the temporary selection and hide the dialog
            annotationBeingCreated = null;
            showAnnotationCreationDialog = false;
        }
    }

    function handleAnnotationDialogCancel() {
        // Clear the temporary selection and hide the dialog
        annotationBeingCreated = null;
        showAnnotationCreationDialog = false;
        // If the user cancels, we might want to clear the drawing tool selection
        // or reset the state in Annotorious, depending on desired UX.
        // For now, just hide the dialog.
    }

    

    

    onMount(() => {
        console.log('[ImageViewerPanel] Mounted. Initial Path:', imagePath);
        if (imagePath && osdViewerElement) { initializeViewer(imagePath); }
        else { isLoading = false; console.log("[ImageViewerPanel onMount] No imagePath or osdViewerElement, not initializing."); }
        
    });

    $: {
        if (imagePath && imagePath !== currentLoadedPath && osdViewerElement) {
            console.log(`[ImageViewerPanel reactive] imagePath changed from '${currentLoadedPath || 'null'}' to '${imagePath}'`);
            initializeViewer(imagePath);
        } else if (imagePath && imagePath !== currentLoadedPath && !osdViewerElement) {
            console.log(`[ImageViewerPanel reactive] imagePath changed to ${imagePath}, but osdViewerElement not ready. Deferring init.`);
            if (!isLoading) isLoading = true;
        }
         else if (!imagePath && (osdViewer || anno || toolbar) ) {
            console.log(`[ImageViewerPanel reactive] imagePath cleared, destroying viewer and annotorious instances.`);
            if (anno) { try { anno.destroy(); } catch(e){console.warn("Error destroying anno on path clear", e)} anno = null; }
            if (toolbar) { try { toolbar.destroy(); } catch(e){console.warn("Error destroying toolbar on path clear", e)} toolbar = null; }
            if (osdViewer) { try { osdViewer.destroy(); } catch(e){console.warn("Error destroying osdViewer", e)} osdViewer = null; }
                        isLoading = false; error = null; currentLoadedPath = null;
            currentAnnotations = [];
        }
    }
</script>

<svelte:head>
    <link href="/annotorious/annotorious.min.css" rel="stylesheet" />
    <link href="/annotorious/annotorious-toolbar.css" rel="stylesheet" />
</svelte:head>

<div class="flex flex-col h-full w-full bg-white dark:bg-gray-800 rounded-md shadow overflow-hidden">
    <div class="flex items-center justify-between px-1 border-b border-gray-200 dark:border-gray-600 flex-shrink-0 text-xs">
        <div id="image-annotation-toolbar-container" class="flex items-center h-9 border border-transparent">
            <span class="text-xs font-medium pr-1">Highlight:</span>
            {#if !isLoading && !error && !toolbar && anno}
                <span class="text-xs text-red-500 italic px-2">Toolbar failed to load but Annotorious might be active.</span>
            {:else if !isLoading && !error && !toolbar && !anno}
                <span class="text-xs text-gray-400 italic px-2">Annotation tools unavailable.</span>
            {/if}
        </div>
    </div>

    <div class="flex-grow overflow-hidden min-h-0 relative">
        {#if isLoading && !error}
            <div class="absolute inset-0 flex items-center justify-center text-gray-500 dark:text-gray-400 z-10 bg-white/50 dark:bg-gray-800/50">Loading image viewer...</div>
        {:else if error}
            <div class="absolute inset-0 flex items-center justify-center text-red-600 dark:text-red-400 p-4 text-center z-10 bg-white/80 dark:bg-gray-800/80">{error}</div>
        {/if}
        <div bind:this={osdViewerElement} class="w-full h-full osd-viewer-container" class:opacity-0={isLoading || error}>
        </div>

        {#if showAnnotationCreationDialog}
            <AnnotationCreationDialog
                x={dialogX}
                y={dialogY}
                on:save={handleAnnotationDialogSave}
                on:cancel={handleAnnotationDialogCancel}
            />
        {/if}

        
    </div>
</div>

<style lang="postcss">
    .min-h-0 { min-height: 0; }
    .osd-viewer-container {
        background-color: theme('colors.gray.300');
    }
    :global(html.dark) .osd-viewer-container {
        background-color: theme('colors.gray.700');
    }
    .opacity-0 { opacity: 0; }

    #image-annotation-toolbar-container:empty {
    }

    :global(#image-annotation-toolbar-container .a9s-toolbar button) {
        /* Tailwind‑style control button – matches other toolbars */
        @apply inline-flex items-center justify-center px-2 py-1 border
                border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800
                text-gray-700 dark:text-gray-300 text-xs rounded-md
                hover:bg-gray-100 dark:hover:bg-gray-700
                focus:outline-none focus:ring-2 focus:ring-blue-500;
        height: 28px;          
        min-width: 28px;       
    }
    :global(#image-annotation-toolbar-container .a9s-toolbar) {
        list-style: none; 
        padding: 0;
        margin: 0;
    }
    :global(#image-annotation-toolbar-container .a9s-toolbar svg) {
        width: 1rem;
        height: 1rem;
        stroke: currentColor;
        fill: none;
    }
    :global(#image-annotation-toolbar-container .a9s-toolbar button.a9s-selected svg) {
        @apply text-blue-500 dark:text-blue-400;
    }
    :global(#image-annotation-toolbar-container .a9s-toolbar button.active) {
        @apply bg-blue-100 dark:bg-blue-900; /* Light blue highlight for active button */
    }
    :global(#image-annotation-toolbar-container .a9s-toolbar button.active svg g,
            #image-annotation-toolbar-container .a9s-toolbar button.active svg rect,
            #image-annotation-toolbar-container .a9s-toolbar button.active svg circle,
            #image-annotation-toolbar-container .a9s-toolbar button.active svg path) {
        stroke: theme('colors.blue.500') !important; /* Deeper blue stroke for active SVG elements */
    }
    :global(.openseadragon-container .openseadragon-canvas) {
        outline: none !important;
    }

    :global(.openseadragon-container div) {
        box-sizing: content-box;
    }
  
  :global(#image-annotation-toolbar-container .a9s-toolbar) {
      display: inline-flex;
      flex-direction: row;
      gap: 0.25rem; 
  }
  :global(#image-annotation-toolbar-container .a9s-toolbar li) {
      margin: 0; 
  }
</style>