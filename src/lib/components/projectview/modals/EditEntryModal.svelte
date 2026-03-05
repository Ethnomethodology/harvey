<!-- src/lib/components/projectview/modals/EditEntryModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';
    import { 
        Type, 
        Hash, 
        CheckSquare, 
        List, 
        Tags, 
        Link, 
        DollarSign, 
        Percent, 
        Calendar, 
        Clock, 
        Mail, 
        Phone,
        CalendarClock
    } from 'lucide-svelte';

    export let rowData = {};
    export let columns = [];
    export let schema = {};
    export let rowIndex = 0;

    const dispatch = createEventDispatcher();

    function getSubtypeIcon(colSchema) {
        const type = colSchema.type;
        const subType = colSchema.subType;
        
        if (subType === 'Checkbox') return CheckSquare;
        if (subType === 'Selectbox') return List;
        if (subType === 'Tags') return Tags;
        if (subType === 'Project Link') return Link;
        
        if (type === 'Numeric') {
            if (subType === 'Currency') return DollarSign;
            if (subType === 'Percent') return Percent;
            return Hash;
        }
        
        if (type === 'DateTime') {
            if (subType === 'Date') return Calendar;
            if (subType === 'Time') return Clock;
            return CalendarClock;
        }
        
        if (type === 'Contact') {
            if (subType === 'Email') return Mail;
            if (subType === 'Phone') return Phone;
        }
        
        return Type;
    }

    let editedData = { ...rowData };
    let errors = {};

    function validateField(field, value) {
        const colSchema = schema[field];
        if (!colSchema) return null;

        if (colSchema.required && (value === null || value === undefined || value === "")) {
            return "Field is required";
        }

        if (value !== null && value !== undefined && value !== "") {
            const type = colSchema.type;
            const subType = colSchema.subType;

            if (type === 'Numeric') {
                const num = parseFloat(value);
                if (isNaN(num) || !isFinite(value)) {
                    return "Invalid number format";
                } else {
                    if (colSchema.min !== null && num < colSchema.min) {
                        return `Minimum value is ${colSchema.min}`;
                    } else if (colSchema.max !== null && num > colSchema.max) {
                        return `Maximum value is ${colSchema.max}`;
                    }
                }
            } else if (type === 'Contact' && subType === 'Email') {
                if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return "Invalid email format";
            } else if (type === 'Contact' && subType === 'Phone') {
                if (!/^\+?[\d\s-]{7,20}$/.test(value)) return "Invalid phone format";
            } else if (type === 'DateTime') {
                if (subType === 'Time') {
                    if (colSchema.format === 'HH:mm' && !/^([01]\d|2[0-3]):([0-5]\d)$/.test(value)) return "Invalid format (HH:mm)";
                    if (colSchema.format === 'HH:mm:ss' && !/^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/.test(value)) return "Invalid format (HH:mm:ss)";
                    if (colSchema.format === 'hh:mm A' && !/^(0[1-9]|1[0-2]):([0-5]\d)\s?(AM|PM)$/i.test(value)) return "Invalid format (hh:mm AM/PM)";
                } else if (subType === 'Date') {
                    if (colSchema.format === 'YYYY-MM-DD' && !/^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(value)) return "Invalid format (YYYY-MM-DD)";
                    // ... other date formats could be added here, mirroring TableViewerPanel.svelte
                } else {
                    if (isNaN(Date.parse(value))) return "Invalid date & time format";
                }
            }
        }
        return null;
    }

    function validateAll() {
        let newErrors = {};
        columns.forEach(col => {
            if (col.field) {
                const error = validateField(col.field, editedData[col.field]);
                if (error) newErrors[col.field] = error;
            }
        });
        errors = newErrors;
        return Object.keys(newErrors).length === 0;
    }

    function handleSave() {
        if (validateAll()) {
            dispatch('save', { rowData: editedData, rowIndex });
        }
    }

    function handleCancel() {
        dispatch('cancel');
    }

    function getAllProjectAssets() {
        const assets = [];
        function traverse(nodes) {
            if (!Array.isArray(nodes)) return;
            nodes.forEach(node => {
                if (node.path && node.file_type && node.file_type !== 'directory') {
                    assets.push({ label: node.name, value: node.path });
                }
                if (node.children) traverse(node.children);
            });
        }
        traverse(get(project).files);
        return assets.sort((a, b) => a.label.localeCompare(b.label));
    }

    const projectAssets = getAllProjectAssets();

</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-4 backdrop-blur-sm">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col border border-gray-200 dark:border-gray-700">
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center bg-gray-50 dark:bg-gray-800/50 rounded-t-lg">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-gray-100 flex items-center">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" fill="currentColor" class="mr-2 text-blue-500" viewBox="0 0 16 16">
                    <path d="M12.146.146a.5.5 0 0 1 .708 0l3 3a.5.5 0 0 1 0 .708l-10 10a.5.5 0 0 1-.168.11l-5 2a.5.5 0 0 1-.65-.65l2-5a.5.5 0 0 1 .11-.168l10-10zM11.207 2.5 13.5 4.793 14.793 3.5 12.5 1.207 11.207 2.5zm1.586 3L10.5 3.207 4 9.707V10h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.293l6.5-6.5zm-9.761 5.175-.106.106-1.528 3.821 3.821-1.528.106-.106A.5.5 0 0 1 5 12.5V12h-.5a.5.5 0 0 1-.5-.5V11h-.5a.5.5 0 0 1-.468-.325z"/>
                </svg>
                Edit Entry {rowIndex + 1}
            </h3>
            <button on:click={handleCancel} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" viewBox="0 0 16 16">
                    <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
                </svg>
            </button>
        </div>

        <div class="p-6 overflow-y-auto space-y-4">
            {#each columns as col}
                {#if col.field && col.field !== 'harvey_internal_id'}
                    {@const colSchema = schema[col.field] || {}}
                    <div class="space-y-1">
                        <label for="field-{col.field}" class="flex items-center text-sm font-medium text-gray-700 dark:text-gray-300">
                            <span class="mr-2 text-gray-400">
                                <svelte:component this={getSubtypeIcon(colSchema)} size={16} strokeWidth={2} />
                            </span>
                            {col.field}
                            {#if colSchema.required}<span class="text-red-500 ml-1">*</span>{/if}
                        </label>
                        
                        {#if colSchema.subType === 'Checkbox'}
                            <div class="flex items-center mt-1">
                                <input
                                    id="field-{col.field}"
                                    type="checkbox"
                                    bind:checked={editedData[col.field]}
                                    class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded dark:bg-gray-700 dark:border-gray-600"
                                />
                            </div>
                        {:else if colSchema.subType === 'Selectbox' || colSchema.subType === 'Tags'}
                            {#if colSchema.subType === 'Tags'}
                                <select
                                    id="field-{col.field}"
                                    bind:value={editedData[col.field]}
                                    multiple
                                    class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                                >
                                    {#each colSchema.options || [] as opt}
                                        <option value={opt}>{opt}</option>
                                    {/each}
                                </select>
                            {:else}
                                <select
                                    id="field-{col.field}"
                                    bind:value={editedData[col.field]}
                                    class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                                >
                                    <option value="">Select option...</option>
                                    {#each colSchema.options || [] as opt}
                                        <option value={opt}>{opt}</option>
                                    {/each}
                                </select>
                            {/if}
                        {:else if colSchema.subType === 'Project Link'}
                            <select
                                id="field-{col.field}"
                                bind:value={editedData[col.field]}
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            >
                                <option value="">Select asset...</option>
                                {#each projectAssets as asset}
                                    <option value={asset.value}>{asset.label}</option>
                                {/each}
                            </select>
                        {:else if colSchema.type === 'Numeric'}
                            <input
                                id="field-{col.field}"
                                type="number"
                                bind:value={editedData[col.field]}
                                step="any"
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        {:else if colSchema.type === 'DateTime' && !colSchema.format}
                             <input
                                id="field-{col.field}"
                                type={colSchema.subType === 'Time' ? 'time' : 'date'}
                                bind:value={editedData[col.field]}
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        {:else}
                            <textarea
                                id="field-{col.field}"
                                bind:value={editedData[col.field]}
                                rows="2"
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            ></textarea>
                        {/if}
                        
                        {#if errors[col.field]}
                            <p class="mt-1 text-xs text-red-500">{errors[col.field]}</p>
                        {/if}
                        {#if colSchema.description}
                            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">{colSchema.description}</p>
                        {/if}
                    </div>
                {/if}
            {/each}
        </div>

        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3 bg-gray-50 dark:bg-gray-800/50 rounded-b-lg">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors"
                on:click={handleCancel}
            >
                Cancel
            </button>
            <button
                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 shadow-sm transition-colors"
                on:click={handleSave}
            >
                Save Changes
            </button>
        </div>
    </div>
</div>

<style>
    /* Ensure the modal is above everything */
    :global(.tabulator) {
        z-index: 1;
    }
</style>
