// src/lib/workers/pdfAnnotationMatcher.worker.js

// --- Helper: Escape Regex (copied from PDFViewerPanel.svelte) ---
function escapeRegExp(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// --- Helper: Normalize text for matching (copied from PDFViewerPanel.svelte) ---
function normalizeTextForMatching(text) {
    if (!text) return "";
    let result = text.normalize('NFKC');
    result = result.replace(/\u00AD/g, '');
    result = result.replace(/[\u00A0\u2007\u202F]/g, ' ');
    result = result
        .replace(/\u2026/g, '...')
        .replace(/[–—]/g, '-')
        .replace(/[‘’]/g, "'")
        .replace(/[“”]/g, '"');
    result = result.replace(/-\s+/g, '');
    return result;
}

self.onmessage = function(event) {
    const {
        pageIndex,
        annotationId,
        annotationText,
        annotationPrefix,
        annotationSuffix,
        annotationOccurrence,
        pageTextContentItems
    } = event.data;

    try {
        // Reconstruct and normalize full page text from items
        let rawFullPageText = pageTextContentItems.map(item => item.str).join('');
        // Basic whitespace normalization similar to what was done with textLayer.textContent
        rawFullPageText = rawFullPageText.replace(/\u00A0/g, ' ').replace(/-\s+/g, '');
        const fullPageTextNormalized = normalizeTextForMatching(rawFullPageText).replace(/\s+/g, ' ');

        const searchStrNormalized = normalizeTextForMatching(annotationText).replace(/\s+/g, ' ');
        if (!searchStrNormalized) {
            postMessage({ pageIndex, annotationId, error: 'Normalized search text is empty.' });
            return;
        }

        const prefixNorm = annotationPrefix ? normalizeTextForMatching(annotationPrefix).replace(/\s+/g, ' ') : '';
        const suffixNorm = annotationSuffix ? normalizeTextForMatching(annotationSuffix).replace(/\s+/g, ' ') : '';

        let pattern = '';
        if (prefixNorm) pattern += `(?<=${escapeRegExp(prefixNorm)})`;
        pattern += escapeRegExp(searchStrNormalized);
        if (suffixNorm) pattern += `(?=${escapeRegExp(suffixNorm)})`;

        let regex = new RegExp(pattern, 'g');
        let match, currentOccurrences = 0, foundStartIndex = -1;
        const targetOccurrence = annotationOccurrence || 0;

        while ((match = regex.exec(fullPageTextNormalized)) !== null) {
            if (currentOccurrences === targetOccurrence) {
                foundStartIndex = match.index;
                break;
            }
            currentOccurrences++;
        }

        // Fallback simple search if context search failed and search string is not empty
        if (foundStartIndex === -1 && searchStrNormalized.length > 0) {
            console.warn(`Worker: Context search failed for "${searchStrNormalized.substring(0,30)}" on page ${pageIndex + 1}. Trying simple search.`);
            const simpleRegex = new RegExp(escapeRegExp(searchStrNormalized), 'g');
            let simpleMatch, simpleCount = 0;
            while ((simpleMatch = simpleRegex.exec(fullPageTextNormalized)) !== null) {
                if (simpleCount === targetOccurrence) {
                    foundStartIndex = simpleMatch.index;
                    console.log(`Worker: Simple search found match at occurrence ${targetOccurrence} for "${searchStrNormalized.substring(0,30)}"`);
                    break;
                }
                simpleCount++;
            }
        }

        if (foundStartIndex !== -1) {
            postMessage({
                pageIndex,
                annotationId,
                startIndex: foundStartIndex,
                matchLength: searchStrNormalized.length, // Length of the normalized search string
                error: null
            });
        } else {
            postMessage({ pageIndex, annotationId, error: `Text not found for ID ${annotationId} (norm: "${searchStrNormalized.substring(0,30)}", occ: ${targetOccurrence}) on page ${pageIndex + 1}.` });
        }

    } catch (e) {
        console.error(`[pdfAnnotationMatcher.worker.js] Error processing annotation ${annotationId} for page ${pageIndex + 1}:`, e);
        postMessage({ pageIndex, annotationId, error: e.message || 'Unknown worker error' });
    }
};
