// --- Helper Functions (extracted and potentially simplified from PDFViewerPanel.svelte) ---

import { v4 as uuidv4 } from 'uuid'; // Assuming uuid is available

function getBoundingBoxForQuads(quadsArray) {
    if (!quadsArray || quadsArray.length === 0) return null;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const quad of quadsArray) {
        minX = Math.min(minX, quad[0], quad[2], quad[4], quad[6]);
        minY = Math.min(minY, quad[1], quad[3], quad[5], quad[7]);
        maxX = Math.max(maxX, quad[0], quad[2], quad[4], quad[6]);
        maxY = Math.max(maxY, quad[1], quad[3], quad[5], quad[7]);
    }
    if (minX === Infinity) return null;
    return {
        x1: minX, y1: minY, x2: maxX, y2: maxY,
        x_center: (minX + maxX) / 2, // Added for convenience if needed by logic
        y_center: (minY + maxY) / 2  // Added for convenience
    };
}

function doBoundingBoxesIntersect(boxA, boxB) {
    if (!boxA || !boxB) return false;
    return !(boxA.x1 >= boxB.x2 || boxA.x2 <= boxB.x1 || boxA.y1 >= boxB.y2 || boxA.y2 <= boxB.y1);
}

function quadToRect(quad) {
    return { x1: quad[0], y1: quad[1], x2: quad[2], y2: quad[5] }; // Assuming TL, TR, BL, BR order for points
}

function rectToQuad(rect) {
    return [rect.x1, rect.y1, rect.x2, rect.y1, rect.x1, rect.y2, rect.x2, rect.y2];
}

function _subtractSingleRect(rectA, rectB) {
    const resultRects = [];
    const TOLERANCE = 0.01;

    if (rectA.x1 >= rectB.x2 - TOLERANCE || rectA.x2 <= rectB.x1 + TOLERANCE || rectA.y1 >= rectB.y2 - TOLERANCE || rectA.y2 <= rectB.y1 + TOLERANCE) {
        return [rectA];
    }

    const ix1 = Math.max(rectA.x1, rectB.x1);
    const ix2 = Math.min(rectA.x2, rectB.x2);
    const iy1 = Math.max(rectA.y1, rectB.y1);
    const iy2 = Math.min(rectA.y2, rectB.y2);

    if (rectA.y1 < iy1 - TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: rectA.y1, x2: rectA.x2, y2: iy1 });
    }
    if (rectA.y2 > iy2 + TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: iy2, x2: rectA.x2, y2: rectA.y2 });
    }
    if (rectA.x1 < ix1 - TOLERANCE) {
        resultRects.push({ x1: rectA.x1, y1: iy1, x2: ix1, y2: iy2 });
    }
    if (rectA.x2 > ix2 + TOLERANCE) {
        resultRects.push({ x1: ix2, y1: iy1, x2: rectA.x2, y2: iy2 });
    }
    return resultRects.filter(r => r.x2 - r.x1 > TOLERANCE && r.y2 - r.y1 > TOLERANCE);
}

function _simplifyAndMergeRects(rectsToSimplify) {
    if (!rectsToSimplify || rectsToSimplify.length === 0) return [];
    let rects = [...rectsToSimplify.filter(r => r.x2 - r.x1 > 0.01 && r.y2 - r.y1 > 0.01)];
    rects.sort((a, b) => (a.y1 !== b.y1 ? a.y1 - b.y1 : a.x1 - b.x1));
    if (rects.length === 0) return [];

    const lines = [];
    let currentLine = [rects[0]];
    for (let i = 1; i < rects.length; i++) {
        const currentRect = rects[i];
        const firstRectOfCurrentLine = currentLine[0];
        const approxLineHeight = firstRectOfCurrentLine.y2 - firstRectOfCurrentLine.y1;
        if (currentRect.y1 > firstRectOfCurrentLine.y1 + approxLineHeight * 0.7) {
            lines.push(currentLine);
            currentLine = [currentRect];
        } else {
            currentLine.push(currentRect);
        }
    }
    lines.push(currentLine);

    const mergedLines = [];
    for (const line of lines) {
        if (line.length === 0) continue;
        line.sort((a, b) => a.x1 - b.x1);
        let mergedRectsOnLine = [{ ...line[0] }];
        for (let i = 1; i < line.length; i++) {
            const currentRect = line[i];
            let lastMerged = mergedRectsOnLine[mergedRectsOnLine.length - 1];
            if (currentRect.x1 < lastMerged.x2 + 5) {
                lastMerged.x2 = Math.max(lastMerged.x2, currentRect.x2);
                lastMerged.y1 = Math.min(lastMerged.y1, currentRect.y1);
                lastMerged.y2 = Math.max(lastMerged.y2, currentRect.y2);
            } else {
                mergedRectsOnLine.push({ ...currentRect });
            }
        }
        mergedLines.push(...mergedRectsOnLine);
    }
    return mergedLines;
}

function subtractQuads(originalQuads, subtractingQuads) {
    if (!originalQuads || originalQuads.length === 0) return [];
    if (!subtractingQuads || subtractingQuads.length === 0) return [...originalQuads];
    let currentRects = originalQuads.map(quadToRect);
    const subtractingRects = subtractingQuads.map(quadToRect);
    for (const subRect of subtractingRects) {
        let nextResultRects = [];
        for (const currentRect of currentRects) {
            const diffRects = _subtractSingleRect(currentRect, subRect);
            nextResultRects.push(...diffRects);
        }
        currentRects = nextResultRects;
    }
    const finalCleanedRects = _simplifyAndMergeRects(currentRects);
    return finalCleanedRects.map(rectToQuad);
}

// Mock dispatch function
let dispatchedEvents = [];
function mockDispatch(event, payload) {
    dispatchedEvents.push({ event, payload });
}

// The core logic from handleHighlightAction (modified for testing)
// We assume `selectionQuads` and `selectionBBox` are pre-calculated and passed in.
// `initialHighlights` is the global list of all highlights.
function testableHandleHighlightRemoval(existingHl, selectionQuads, selectionBBox, selectionPageIndex, dispatchFunction) {
    const originalExistingQuads = JSON.parse(JSON.stringify(existingHl.quadPoints));
    let quadsBeforeSelection = [];
    let quadsAfterSelection = [];

    for (const exQuad of existingHl.quadPoints) {
        const exQuadBBox = getBoundingBoxForQuads([exQuad]);
        if (!exQuadBBox) continue;

        if (!doBoundingBoxesIntersect(exQuadBBox, selectionBBox)) {
            if (exQuadBBox.y2 <= selectionBBox.y1) {
                quadsBeforeSelection.push(exQuad);
            } else if (exQuadBBox.y1 >= selectionBBox.y2) {
                quadsAfterSelection.push(exQuad);
            } else {
                if (exQuadBBox.x2 <= selectionBBox.x1) {
                    quadsBeforeSelection.push(exQuad);
                } else if (exQuadBBox.x1 >= selectionBBox.x2) {
                    quadsAfterSelection.push(exQuad);
                } else {
                    console.warn('[Testable Logic] Non-intersecting exQuad overlaps selection horizontally. Defaulting to quadsBeforeSelection. exQuadBBox:', exQuadBBox, 'selectionBBox:', selectionBBox);
                    quadsBeforeSelection.push(exQuad);
                }
            }
        } else {
            const remnants = subtractQuads([exQuad], selectionQuads);
            for (const remnantQuad of remnants) {
                const remnantBBox = getBoundingBoxForQuads([remnantQuad]);
                if (!remnantBBox) continue;

                if (remnantBBox.x2 <= selectionBBox.x1) {
                    quadsBeforeSelection.push(remnantQuad);
                } else if (remnantBBox.x1 >= selectionBBox.x2) {
                    quadsAfterSelection.push(remnantQuad);
                } else {
                    console.warn('[Testable Logic] Remnant quad overlaps selection horizontally. exQuadBBox:', exQuadBBox, 'remnantBBox:', remnantBBox, 'selectionBBox:', selectionBBox);
                     if (exQuadBBox.x2 - exQuadBBox.x1 > exQuadBBox.y2 - exQuadBBox.y1) { // exQuad is wider than tall
                        if (remnantBBox.y2 <= selectionBBox.y1) {
                            quadsBeforeSelection.push(remnantQuad);
                        } else if (remnantBBox.y1 >= selectionBBox.y2) {
                            quadsAfterSelection.push(remnantQuad);
                        } else {
                            const remnantCenterX = (remnantBBox.x1 + remnantBBox.x2) / 2;
                            const selectionCenterX = (selectionBBox.x1 + selectionBBox.x2) / 2;
                            if (remnantCenterX < selectionCenterX) {
                                quadsBeforeSelection.push(remnantQuad);
                            } else {
                                quadsAfterSelection.push(remnantQuad);
                            }
                        }
                    } else {
                        const remnantCenterX = (remnantBBox.x1 + remnantBBox.x2) / 2;
                        const selectionCenterX = (selectionBBox.x1 + selectionBBox.x2) / 2;
                        if (remnantCenterX < selectionCenterX) {
                            quadsBeforeSelection.push(remnantQuad);
                        } else {
                            quadsAfterSelection.push(remnantQuad);
                        }
                    }
                }
            }
        }
    }

    const finalQuadsBefore = quadsBeforeSelection.length > 0 ? _simplifyAndMergeRects(quadsBeforeSelection.map(q => quadToRect(q))).map(r => rectToQuad(r)) : [];
    const finalQuadsAfter = quadsAfterSelection.length > 0 ? _simplifyAndMergeRects(quadsAfterSelection.map(q => quadToRect(q))).map(r => rectToQuad(r)) : [];

    if (finalQuadsBefore.length > 0) {
        const updatedHighlightData = { ...existingHl, quadPoints: finalQuadsBefore };
        // Ensure 'type' from existingHl (which is 'pdfHighlight') doesn't overwrite the action type 'update'
        delete updatedHighlightData.type;
        dispatchFunction('pdfhighlightevent', { type: 'update', data: { ...updatedHighlightData, type: existingHl.type || 'pdfHighlight'} });
    } else {
        dispatchFunction('pdfhighlightevent', { type: 'remove', id: existingHl.id });
    }

    if (finalQuadsAfter.length > 0) {
        const newSplitHighlightId = `hl-${uuidv4()}`;
        const newHighlightDataObject = {
            type: 'pdfHighlight', // This is the object's type
            id: newSplitHighlightId,
            color: existingHl.color,
            pageIndex: existingHl.pageIndex,
            text: existingHl.text,
            quadPoints: finalQuadsAfter,
            prefix: existingHl.prefix,
            suffix: existingHl.suffix,
        };
        dispatchFunction('pdfhighlightevent', { type: 'add', data: newHighlightDataObject });
    }
}

// --- Test Scenarios ---
const scenarios = [
    {
        description: "Scenario 1: Single Line Split",
        existingHl: { id: 'old_id_1', color: 'yellow', pageIndex: 0, text: "ABCDEFG", prefix: "", suffix: "", quadPoints: [[0, 10, 100, 10, 0, 30, 100, 30]] },
        selectionQuads: [[30, 10, 70, 10, 30, 30, 70, 30]],
        expectedDispatch: [
            { type: 'update', id: 'old_id_1', quadPoints: [[0, 10, 30, 10, 0, 30, 30, 30]] },
            { type: 'add', idRegex: /^hl-/, quadPoints: [[70, 10, 100, 10, 70, 30, 100, 30]] }
        ]
    },
    {
        description: "Scenario 2: Multi-Line Split (Remove from First Line)",
        existingHl: {
            id: 'old_id_para', color: 'blue', pageIndex: 0, text: "Line1\nLine2\nLine3",
            quadPoints: [
                [0, 10, 100, 10, 0, 30, 100, 30], // Line 1
                [0, 35, 100, 35, 0, 55, 100, 55], // Line 2
                [0, 60, 100, 60, 0, 80, 100, 80]  // Line 3
            ]
        },
        selectionQuads: [[30, 10, 70, 10, 30, 30, 70, 30]], // Middle of first line
        expectedDispatch: [
            { type: 'update', id: 'old_id_para', quadPoints: [[0, 10, 30, 10, 0, 30, 30, 30]] },
            { type: 'add', idRegex: /^hl-/, quadPoints: [[70, 10, 100, 10, 70, 30, 100, 30], [0, 35, 100, 35, 0, 55, 100, 55], [0, 60, 100, 60, 0, 80, 100, 80]] }
        ]
    },
    {
        description: "Scenario 3: Edge Case - Remove from Start of Line",
        existingHl: { id: 'old_id_edge1', color: 'green', pageIndex: 0, quadPoints: [[0, 10, 100, 10, 0, 30, 100, 30]] },
        selectionQuads: [[0, 10, 40, 10, 0, 30, 40, 30]],
        expectedDispatch: [
            { type: 'remove', id: 'old_id_edge1' }, // finalQuadsBefore should be empty
            { type: 'add', idRegex: /^hl-/, color: 'green', quadPoints: [[40, 10, 100, 10, 40, 30, 100, 30]] }
        ]
    },
    {
        description: "Scenario 4: Edge Case - Remove until End of Line",
        existingHl: { id: 'old_id_edge2', color: 'purple', pageIndex: 0, quadPoints: [[0, 10, 100, 10, 0, 30, 100, 30]] },
        selectionQuads: [[60, 10, 100, 10, 60, 30, 100, 30]],
        expectedDispatch: [
            { type: 'update', id: 'old_id_edge2', color: 'purple', quadPoints: [[0, 10, 60, 10, 0, 30, 60, 30]] }
            // No 'add' event expected here as finalQuadsAfter should be empty
        ]
    }
];

// --- Test Runner ---
let allTestsPassed = true;

scenarios.forEach(scenario => {
    console.log(`\nRunning: ${scenario.description}`);
    dispatchedEvents = []; // Reset for each scenario

    const selectionBBox = getBoundingBoxForQuads(scenario.selectionQuads);
    // Simulate the loop over existingHl. In real code, this is `for (const existingHl of highlightsToProcess)`
    // For this test, we call the function directly with one `existingHl`.
    testableHandleHighlightRemoval(scenario.existingHl, scenario.selectionQuads, selectionBBox, scenario.existingHl.pageIndex, mockDispatch);

    let scenarioPassed = true;
    if (dispatchedEvents.length !== scenario.expectedDispatch.length) {
        scenarioPassed = false;
        console.error(`  FAIL: Expected ${scenario.expectedDispatch.length} dispatched events, but got ${dispatchedEvents.length}`);
        console.error('  Got:', JSON.stringify(dispatchedEvents, null, 2));
    } else {
        for (let i = 0; i < scenario.expectedDispatch.length; i++) {
            const expected = scenario.expectedDispatch[i];
            const actualPayload = dispatchedEvents[i].payload; // This is { type: 'action', data: {...} } or { type: 'remove', id: '...' }
            let eventDetailPassed = true;

            if (expected.type !== actualPayload.type) {
                eventDetailPassed = false;
                console.error(`  FAIL Event ${i}: Action Type mismatch. Expected: ${expected.type}, Got: ${actualPayload.type}`);
            }

            const actualData = actualPayload.data; // This is the highlight object for 'add'/'update'

            if (expected.type === 'add' || expected.type === 'update') {
                if (!actualData) {
                    eventDetailPassed = false;
                    console.error(`  FAIL Event ${i}: Missing 'data' field in payload for ${expected.type} event.`);
                } else {
                    if (expected.id && expected.id !== actualData.id) {
                        eventDetailPassed = false;
                        console.error(`  FAIL Event ${i}: ID mismatch. Expected: ${expected.id}, Got: ${actualData.id}`);
                    }
                    if (expected.idRegex && !expected.idRegex.test(actualData.id)) {
                        eventDetailPassed = false;
                        console.error(`  FAIL Event ${i}: ID regex mismatch. Regex: ${expected.idRegex}, Got: ${actualData.id}`);
                    }
                    if (expected.color && expected.color !== actualData.color) {
                        eventDetailPassed = false;
                        console.error(`  FAIL Event ${i}: Color mismatch. Expected: ${expected.color}, Got: ${actualData.color}`);
                    }
                    if (expected.quadPoints && JSON.stringify(expected.quadPoints) !== JSON.stringify(actualData.quadPoints)) {
                        eventDetailPassed = false;
                        console.error(`  FAIL Event ${i}: QuadPoints mismatch.`);
                        console.error(`    Expected: ${JSON.stringify(expected.quadPoints)}`);
                        console.error(`    Got:      ${JSON.stringify(actualData.quadPoints)}`);
                    }
                     if (actualData.type !== 'pdfHighlight') { // Check the type within the data object
                        eventDetailPassed = false;
                        console.error(`  FAIL Event ${i}: Highlight object type mismatch. Expected 'pdfHighlight', Got: ${actualData.type}`);
                    }
                }
            } else if (expected.type === 'remove') {
                if (expected.id && expected.id !== actualPayload.id) {
                    eventDetailPassed = false;
                    console.error(`  FAIL Event ${i}: ID mismatch for remove. Expected: ${expected.id}, Got: ${actualPayload.id}`);
                }
            }
            if (!eventDetailPassed) scenarioPassed = false;
        }
    }

    if (scenarioPassed) {
        console.log("  PASS");
    } else {
        allTestsPassed = false;
    }
});

console.log(`\n------------------------------------`);
if (allTestsPassed) {
    console.log("All scenarios passed successfully!");
} else {
    console.log("Some scenarios FAILED.");
    process.exit(1); // Indicate failure
}
process.exit(0); // Indicate success
