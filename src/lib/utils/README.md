# Utilities (`src/lib/utils`)

**Purpose:** Houses standalone, pure utility functions and helper modules that perform generic data processing, formatting, or queue management independently of Svelte components.

## Exported Utilities / Constants

- **`pdfThumbnailQueue.js`**: A specialized async queue manager for generating PDF thumbnails sequentially. It prevents the application from overwhelming the system or PDF.js by trying to render dozens of thumbnails concurrently.

## Usage Example

```javascript
import { queuePdfThumbnail } from '$lib/utils/pdfThumbnailQueue.js';

// Enqueue a thumbnail generation task
queuePdfThumbnail(pdfPath, canvasElement)
  .then(() => console.log('Thumbnail rendered'))
  .catch((err) => console.error(err));
```
