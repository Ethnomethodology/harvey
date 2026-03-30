# Constants (`src/lib/constants`)

**Purpose:** Defines static configurations, mappings, options, and layouts used universally across the Harvey frontend to maintain consistency.

## Exported Utilities / Constants

*   **`exportLayouts.js`**: Contains constants mapping to DOCX layout configurations (e.g., column widths for detailed vs. simple transcripts).
*   **`highlightOptions.js`**: Defines the standard color palette used for highlighting text and table cells.
*   **`languageMap.js`**: Maps ISO language codes (e.g., "en") to full language names (e.g., "English") for display.
*   **`models.js`**: Definitions or lists of available machine learning models used in transcription or translation.
*   **`transcriptionOptions.js`**: Lists available options for transcription languages and engines for dropdowns.

## Usage Example

```javascript
import { languageOptions } from '$lib/constants/transcriptionOptions.js';

// Iterate over options to render a select dropdown
languageOptions.forEach(option => {
  console.log(option.value, option.label);
});
```