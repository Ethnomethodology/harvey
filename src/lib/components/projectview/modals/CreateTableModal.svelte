<!-- src/lib/components/projectview/modals/CreateTableModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { Plus, Trash2 } from 'lucide-svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let fields = []; // { name, type, subType, options, required, min, max, description, format }

    const FIELD_TYPES = {
        'Text': ['Small Text', 'Long Text'],
        'Numeric': ['Number', 'Currency', 'Percent'],
        'DateTime': ['Date', 'Date & Time', 'Time'],
        'Contact': ['Email', 'Phone', 'Hyperlink'],
        'Misc': ['Selectbox', 'Checkbox', 'Tags', 'Project Link']
    };

    const DATETIME_FORMATS = {
        'Date': ['None', 'YYYY-MM-DD', 'DD/MM/YYYY', 'MM/DD/YYYY', 'MMMM DD, YYYY', 'YYYY', 'MMMM', 'MMMM YYYY'],
        'Date & Time': ['None', 'YYYY-MM-DD HH:mm', 'DD/MM/YYYY HH:mm', 'MM/DD/YYYY hh:mm A'],
        'Time': ['None', 'HH:mm', 'HH:mm:ss', 'hh:mm A']
    };

    function addField() {
        fields = [...fields, {
            name: `Field ${fields.length + 1}`,
            type: 'Text',
            subType: 'Small Text',
            options: '',
            required: false,
            min: '',
            max: '',
            description: '',
            format: 'None'
        }];
    }

    function removeField(index) {
        if (fields.length > 1) {
            fields = fields.filter((_, i) => i !== index);
        }
    }

    // Initialize with one field when modal opens
    $: if (showModal && fields.length === 0) {
        addField();
    }

    function handleTypeChange(index) {
        const type = fields[index].type;
        fields[index].subType = FIELD_TYPES[type][0];
        fields[index].format = 'None';
        if (type !== 'Numeric') {
            fields[index].min = '';
            fields[index].max = '';
        }
    }

    function handleSubTypeChange(index) {
        fields[index].format = 'None';
    }

    async function handleSubmit() {
        const validFields = fields.filter(f => f.name.trim() !== '');
        if (validFields.length === 0) {
            alert('Please provide at least one field name.');
            return;
        }

        const schema = {};
        validFields.forEach(f => {
            schema[f.name] = {
                type: f.type,
                subType: f.subType,
                options: (f.subType === 'Selectbox' || f.subType === 'Tags') ? f.options.split(',').map(o => o.trim()).filter(o => o !== '') : [],
                required: f.required,
                min: f.min !== '' ? parseFloat(f.min) : null,
                max: f.max !== '' ? parseFloat(f.max) : null,
                description: f.description.trim(),
                format: f.format !== 'None' ? f.format : null
            };
        });

        try {
            const newTablePath = await invoke('create_new_table', {
                projectXmlPath: $project.xmlPath,
                headers: validFields.map(f => f.name),
                schema: schema
            });
            closeModal();
            dispatch('tableCreated', { path: newTablePath });
        } catch (error) {
            console.error('Error creating new table:', error);
            alert(`Error creating table: ${error.message || error}`);
        }
    }

    function closeModal() {
        fields = [];
        showModal = false;
        dispatch('close');
    }
</script>

{#if showModal}
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-6xl overflow-hidden flex flex-col max-h-[90vh]">
        <div class="flex justify-between items-center mb-4">
            <h3 class="text-xl font-bold dark:text-white">Create New Table</h3>
        </div>

        <div class="flex-1 overflow-x-auto overflow-y-auto pr-2 border dark:border-gray-700 rounded-md">
            <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0 z-10">
                    <tr>
                        <th class="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-[150px]">Field Name</th>
                        <th class="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Type</th>
                        <th class="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Sub-type</th>
                        <th class="px-3 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">Req?</th>
                        <th class="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-[120px]">Constraints / Options</th>
                        <th class="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-[150px]">Description</th>
                        <th class="px-3 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider w-10"></th>
                    </tr>
                </thead>
                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                    {#each fields as field, i}
                        <tr class="hover:bg-gray-50 dark:hover:bg-gray-750 transition-colors">
                            <td class="px-3 py-2">
                                <input type="text" bind:value={field.name} placeholder="Name" class="w-full text-sm p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none" />
                            </td>
                            <td class="px-3 py-2">
                                <select bind:value={field.type} on:change={() => handleTypeChange(i)} class="w-full text-sm p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none">
                                    {#each Object.keys(FIELD_TYPES) as type}<option value={type}>{type}</option>{/each}
                                </select>
                            </td>
                            <td class="px-3 py-2">
                                <select bind:value={field.subType} on:change={() => handleSubTypeChange(i)} class="w-full text-sm p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none">
                                    {#each FIELD_TYPES[field.type] as sub}<option value={sub}>{sub}</option>{/each}
                                </select>
                            </td>
                            <td class="px-3 py-2 text-center">
                                <input type="checkbox" bind:checked={field.required} class="h-4 w-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500" />
                            </td>
                            <td class="px-3 py-2">
                                {#if field.subType === 'Selectbox' || field.subType === 'Tags'}
                                    <input type="text" bind:value={field.options} placeholder="Opt 1, Opt 2..." class="w-full text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none" />
                                {:else if field.type === 'Numeric'}
                                    <div class="flex space-x-1">
                                        <input type="number" bind:value={field.min} placeholder="Min" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none" />
                                        <input type="number" bind:value={field.max} placeholder="Max" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none" />
                                    </div>
                                {:else if field.type === 'DateTime'}
                                    <select bind:value={field.format} class="w-full text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none">
                                        {#each DATETIME_FORMATS[field.subType] as fmt}<option value={fmt}>{fmt}</option>{/each}
                                    </select>
                                {:else}
                                    <span class="text-xs text-gray-400">None</span>
                                {/if}
                            </td>
                            <td class="px-3 py-2">
                                <input type="text" bind:value={field.description} placeholder="Purpose of this field" class="w-full text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 dark:text-white focus:ring-1 focus:ring-blue-500 outline-none" />
                            </td>
                            <td class="px-3 py-2 text-center">
                                <button 
                                    on:click={() => removeField(i)}
                                    class="text-gray-400 hover:text-red-500 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                                    disabled={fields.length <= 1}
                                    title="Remove Field"
                                >
                                    <Trash2 size={16} />
                                </button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
            
            <div class="p-3 bg-gray-50 dark:bg-gray-900 border-t dark:border-gray-700">
                <button 
                    on:click={addField}
                    class="flex items-center space-x-1 px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 text-sm font-medium rounded-md transition-colors shadow-sm"
                >
                    <Plus size={16} />
                    <span>Add Field</span>
                </button>
            </div>
        </div>

        <div class="mt-6 flex justify-end space-x-2 border-t pt-4 dark:border-gray-700">
            <button class="px-4 py-2 text-sm font-medium rounded-md text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-600 dark:text-gray-200 dark:hover:bg-gray-500" on:click={closeModal}>Cancel</button>
            <button class="px-4 py-2 text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 shadow-sm transition-all" on:click={handleSubmit}>
                Create Table
            </button>
        </div>
    </div>
</div>
{/if}

<style>
    .dark .hover\:bg-gray-750:hover {
        background-color: rgba(55, 65, 81, 0.5);
    }
</style>
