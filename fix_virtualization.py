import re

with open('src/lib/components/projectview/transcription/RichTextPreview.svelte', 'r') as f:
    content = f.read()

# Replace Virtualization calculations
old_virt = """  $: if (allSegmentsData) {
    if (previewScrollContainerRef && allSegmentsData.length > 0) {
      const totalItems = allSegmentsData.length;
      visibleStartIndex = Math.max(
        0,
        Math.floor(scrollTop / ESTIMATED_SEGMENT_HEIGHT) - OVERSCAN_COUNT
      );
      visibleEndIndex = Math.min(
        totalItems - 1,
        Math.ceil((scrollTop + containerHeight) / ESTIMATED_SEGMENT_HEIGHT) + OVERSCAN_COUNT
      );

      paddingTop = visibleStartIndex * ESTIMATED_SEGMENT_HEIGHT;
      paddingBottom = (totalItems - 1 - visibleEndIndex) * ESTIMATED_SEGMENT_HEIGHT;"""

new_virt = """  // --- Dynamic Virtualization ---
  let segmentHeights = [];

  function measureHeight(node, index) {
    let resizeObserver = new ResizeObserver((entries) => {
      for (let entry of entries) {
        const height = entry.borderBoxSize ? entry.borderBoxSize[0].blockSize : entry.contentRect.height;
        if (height > 0 && segmentHeights[index] !== height) {
          segmentHeights[index] = height;
          // Reactivity trigger: assigning to itself forces Svelte to re-evaluate dependent $ blocks
          segmentHeights = segmentHeights;
        }
      }
    });
    resizeObserver.observe(node);

    return {
      update(newIndex) {
        index = newIndex;
      },
      destroy() {
        resizeObserver.disconnect();
      }
    };
  }

  function getSegmentHeight(i) {
    return segmentHeights[i] || ESTIMATED_SEGMENT_HEIGHT;
  }

  function getCumulativeTop(index) {
    let top = 0;
    for (let i = 0; i < index; i++) {
      top += getSegmentHeight(i);
    }
    return top;
  }

  $: if (allSegmentsData) {
    if (previewScrollContainerRef && allSegmentsData.length > 0) {
      const totalItems = allSegmentsData.length;

      // Dynamic scanning to find visibleStartIndex
      let currentTop = 0;
      let startIdx = 0;
      while (startIdx < totalItems) {
        const h = getSegmentHeight(startIdx);
        if (currentTop + h >= scrollTop) break;
        currentTop += h;
        startIdx++;
      }

      // Find visibleEndIndex
      let endIdx = startIdx;
      let currentBottom = currentTop;
      while (endIdx < totalItems) {
        currentBottom += getSegmentHeight(endIdx);
        if (currentBottom >= scrollTop + containerHeight) break;
        endIdx++;
      }

      visibleStartIndex = Math.max(0, startIdx - OVERSCAN_COUNT);
      visibleEndIndex = Math.min(totalItems - 1, endIdx + OVERSCAN_COUNT);

      paddingTop = 0;
      for (let i = 0; i < visibleStartIndex; i++) {
        paddingTop += getSegmentHeight(i);
      }

      paddingBottom = 0;
      for (let i = visibleEndIndex + 1; i < totalItems; i++) {
        paddingBottom += getSegmentHeight(i);
      }"""

content = content.replace(old_virt, new_virt)

# Also update the scroll math in the auto-scroller block:
# old_scroll_top = "($transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex) * ESTIMATED_SEGMENT_HEIGHT;"
old_scroll_top = """      const itemTop =
        ($transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex) *
        ESTIMATED_SEGMENT_HEIGHT;"""

new_scroll_top = """      const itemIndex = $transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex;
      const itemTop = getCumulativeTop(itemIndex);"""

content = content.replace(old_scroll_top, new_scroll_top)

old_actual_top = """        let actualItemTop =
          ($transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex) *
          ESTIMATED_SEGMENT_HEIGHT;
        let actualItemBottom = actualItemTop + ESTIMATED_SEGMENT_HEIGHT;
        let actualItemHeight = ESTIMATED_SEGMENT_HEIGHT;"""

new_actual_top = """        let actualItemTop = getCumulativeTop($transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex);
        let actualItemHeight = getSegmentHeight($transcriptStore.isDualModeActive ? activeSegmentIndex * 2 : activeSegmentIndex);
        let actualItemBottom = actualItemTop + actualItemHeight;"""

content = content.replace(old_actual_top, new_actual_top)

old_search_scroll = """    const itemTop =
      ($transcriptStore.isDualModeActive
        ? res.segmentIndex * 2 + (res.isPrimary ? 0 : 1)
        : res.segmentIndex) * ESTIMATED_SEGMENT_HEIGHT;
    const targetScrollTop = Math.max(
      0,
      itemTop - containerHeight / 2 + ESTIMATED_SEGMENT_HEIGHT / 2
    );"""

new_search_scroll = """    const itemIndex = $transcriptStore.isDualModeActive
        ? res.segmentIndex * 2 + (res.isPrimary ? 0 : 1)
        : res.segmentIndex;
    const itemTop = getCumulativeTop(itemIndex);
    const itemHeight = getSegmentHeight(itemIndex);
    const targetScrollTop = Math.max(
      0,
      itemTop - containerHeight / 2 + itemHeight / 2
    );"""

content = content.replace(old_search_scroll, new_search_scroll)

# Add use:measureHeight to the rendered segments.
# They are rendered in the HTML block. Let's find it.
old_html_segment = """          <div
            class="flex items-start transition-colors duration-200 segment-row relative pl-6 mr-1 pr-1 group"""

new_html_segment = """          <div
            use:measureHeight={idx}
            class="flex items-start transition-colors duration-200 segment-row relative pl-6 mr-1 pr-1 group"""

content = content.replace(old_html_segment, new_html_segment)

# Wait, idx might be wrong if visibleSegments is mapped.
# let's check the #each loop for visibleSegments

with open('src/lib/components/projectview/transcription/RichTextPreview.svelte', 'w') as f:
    f.write(content)
