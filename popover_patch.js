// Snippet for URL popover implementation

import { openUrl } from '@tauri-apps/plugin-opener';

let showUrlPopover = false;
let popoverUrl = '';
let popoverX = 0;
let popoverY = 0;
let isUrlCopied = false;

async function handleOpenUrl() {
    try {
        await openUrl(popoverUrl);
        showUrlPopover = false;
    } catch (e) {
        console.error("Failed to open URL:", e);
    }
}

function handleCopyUrl() {
    navigator.clipboard.writeText(popoverUrl);
    isUrlCopied = true;
    setTimeout(() => {
        showUrlPopover = false;
        isUrlCopied = false;
    }, 2000);
}
