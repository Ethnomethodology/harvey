const fs = require('fs');
const file = 'src/lib/components/projectview/data/tables/ChartModal.svelte';
let content = fs.readFileSync(file, 'utf8');

// The create chart button is currently inside the activeTab === 'create' && !isEditingExisting view:
// <div class="mt-auto pt-6 flex justify-end">
//   <Button color="blue" disabled={!selectedChartType} on:click={initialCreate} class="px-6">Create Chart</Button>
// </div>

content = content.replace(
    /<div class="mt-auto pt-6 flex justify-end">\n\s*<Button color="blue" disabled=\{!selectedChartType\} on:click=\{initialCreate\} class="px-6">\n\s*Create Chart\n\s*<\/Button>\n\s*<\/div>/g,
    `<div class="absolute bottom-6 right-6 z-10">
                        <Button color="blue" disabled={!selectedChartType} on:click={initialCreate} class="px-6 shadow-md">
                            Create Chart
                        </Button>
                    </div>`
);

fs.writeFileSync(file, content);
