const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The original `{:else}` branch closed the `div.space-y-4` before the `{/if}`.
// See line 526: `</div> \n {/if}`
// But wait! In my code:
// ```svelte
// {#if activeTab === 'create'}
//    <div class="space-y-4">
//       <div>...</div>
//       <div>...</div>
//       {#if !isEditingExisting}
//          ...
//       {:else}
//          ...
//          <div class="pt-2"><Toggle...></div>
// </div> <-- This closes space-y-4 inside the ELSE block!
// {/if}
// ```
// Ah! Since I moved `<div class="space-y-4">` OUTSIDE the `{#if !isEditingExisting}`, it needs to be closed AFTER the `{#if !isEditingExisting}` block, NOT inside the `{:else}`!
// The `</div>` on line 526 is currently closing `space-y-4`, but because it is before `{/if}`, it only closes if `isEditingExisting` is true! If it's false, `space-y-4` is NEVER closed, which breaks HTML!
// We need to swap them:
// {/if}
// </div>

content = content.replace(
    /<\/div>\n\s*\{\/if\}\n\s*\{:else if activeTab === 'existing'\}/g,
    `   {/if}\n                    </div>\n                {:else if activeTab === 'existing'}`
);

fs.writeFileSync(file, content);
