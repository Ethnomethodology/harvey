<!-- src/lib/components/projectview/modals/EditEntryModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
    import { project } from '$lib/stores/projectStore.js';
    import { get } from 'svelte/store';
    import { 
        Type, 
        Hash, 
        CheckSquare, 
        SquareMenu, 
        Tags, 
        Link, 
        DollarSign, 
        Percent, 
        CalendarDays, 
        Clock, 
        Mail, 
        Phone,
        CalendarClock,
        Link2,
        Pencil,
        X,
        TextInitial
    } from 'lucide-svelte';

    export let rowData = {};
    export let columns = [];
    export let schema = {};
    export let rowIndex = 0;

    const dispatch = createEventDispatcher();

    function getCurrencySymbol(currencyCode) {
        const symbols = {
            'USD': '$',
            'EUR': '€',
            'GBP': '£',
            'JPY': '¥',
            'INR': '₹',
            'CNY': '¥',
            'AUD': '$',
            'CAD': '$',
            'CHF': 'CHF',
            'SGD': '$',
            'HKD': '$',
            'NZD': '$',
            'KRW': '₩',
            'NOK': 'kr',
            'MXN': '$',
            'RUB': '₽',
            'ZAR': 'R',
            'TRY': '₺',
            'BRL': 'R$',
            'TWD': 'NT$',
            'DKK': 'kr',
            'PLN': 'zł',
            'THB': '฿',
            'IDR': 'Rp',
            'PHP': '₱'
        };
        if (symbols[currencyCode]) return symbols[currencyCode];
        return currencyCode || 'XXX';
    }

    function getSubtypeIcon(colSchema) {
        const type = colSchema.type || 'Text';
        const subType = colSchema.subType || 'Small Text';
        
        if (type === 'Misc') {
            if (subType === 'Checkbox') return CheckSquare;
            if (subType === 'Selectbox') return SquareMenu;
            if (subType === 'Multiselect') return Tags;
            if (subType === 'Project Link') return Link;
        }
        
        if (type === 'Numeric') {
            if (subType === 'Currency') return DollarSign;
            if (subType === 'Percent') return Percent;
            return Hash;
        }
        
        if (type === 'DateTime') {
            if (subType === 'Date') return CalendarDays;
            if (subType === 'Time') return Clock;
            return CalendarClock;
        }
        
        if (type === 'Contact') {
            if (subType === 'Email') return Mail;
            if (subType === 'Phone') return Phone;
            if (subType === 'Hyperlink') return Link2;
            return Mail;
        }

        if (type === 'Text') {
            if (subType === 'Long Text') return TextInitial;
        }
        
        return Type;
    }

    let editedData = { ...rowData };
    
    // Ensure Multiselect fields are arrays for the multiple select
    for (const field in schema) {
        if (schema[field].type === 'Misc' && schema[field].subType === 'Multiselect' && typeof editedData[field] === 'string') {
            editedData[field] = editedData[field].split(',').map(s => s.trim()).filter(Boolean);
        }
    }

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
        const AUDIO_EXTENSIONS = new Set(['mp3', 'wav', 'm4a', 'ogg', 'aac', 'flac']);
        const VIDEO_EXTENSIONS = new Set(['mp4', 'mov', 'avi', 'mkv', 'webm']);

        function traverse(nodes, parentMediaCategory = null) {
            if (!Array.isArray(nodes)) return;
            nodes.forEach(node => {
                let currentMediaCategory = parentMediaCategory;
                
                if (node.file_type === 'directory_media_stem') {
                    const findMediaFile = (n) => {
                        if (n.file_type === 'media' && !n.is_directory) return n;
                        if (n.children) {
                            for (const child of n.children) {
                                const found = findMediaFile(child);
                                if (found) return found;
                            }
                        }
                        return null;
                    };
                    const mediaFile = findMediaFile(node);
                    if (mediaFile) {
                        const ext = mediaFile.name.split('.').pop()?.toLowerCase() ?? '';
                        if (VIDEO_EXTENSIONS.has(ext)) currentMediaCategory = 'Videos';
                        else if (AUDIO_EXTENSIONS.has(ext)) currentMediaCategory = 'Audios';
                    }
                }

                if (node.path && node.file_type && node.file_type !== 'directory' && node.file_type !== 'directory_media_stem') {
                    let category = '';
                    const path = node.path.replaceAll('\\', '/');
                    
                    if (node.file_type === 'media') {
                        const ext = node.name.split('.').pop()?.toLowerCase() ?? '';
                        if (VIDEO_EXTENSIONS.has(ext)) category = 'Videos';
                        else if (AUDIO_EXTENSIONS.has(ext)) category = 'Audios';
                    } else if (node.file_type === 'transcript') {
                        category = currentMediaCategory || 'Transcripts';
                    } else if (node.file_type === 'doc' || path.includes('/Documents/')) {
                        category = 'Documents';
                    } else if (node.file_type === 'table' || path.includes('/Tables/')) {
                        category = 'Tables';
                    } else if (node.file_type === 'image' || path.includes('/Images/')) {
                        category = 'Images';
                    } else if (node.file_type === 'imported_transcript' || path.includes('/Transcripts/')) {
                        category = 'Transcripts';
                    } else if (path.includes('/Media/')) {
                         if (currentMediaCategory) category = currentMediaCategory;
                         else {
                            if (path.toLowerCase().includes('video')) category = 'Videos';
                            else if (path.toLowerCase().includes('audio')) category = 'Audios';
                            else category = 'Audios';
                         }
                    }

                    if (category) {
                        assets.push({ 
                            label: `${category} - ${node.name}`, 
                            value: node.path,
                            category: category
                        });
                    }
                }
                
                if (node.children) {
                    traverse(node.children, currentMediaCategory);
                }
            });
        }
        
        traverse(get(project).files);
        
        return assets.sort((a, b) => {
            if (a.category !== b.category) {
                return a.category.localeCompare(b.category);
            }
            return a.label.localeCompare(b.label);
        });
    }

    const projectAssets = getAllProjectAssets();

</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-4 backdrop-blur-sm">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col border border-gray-200 dark:border-gray-700">
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center bg-gray-50 dark:bg-gray-800/50 rounded-t-lg">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-gray-100 flex items-center">
                <Pencil size={20} class="mr-2 text-blue-500" />
                Edit Entry {rowIndex + 1}
            </h3>
            <button on:click={handleCancel} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors">
                <X size={24} />
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
                        
                        {#if colSchema.type === 'Contact'}
                            <input
                                id="field-{col.field}"
                                type={colSchema.subType === 'Email' ? 'email' : (colSchema.subType === 'Phone' ? 'tel' : 'url')}
                                bind:value={editedData[col.field]}
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        {:else if colSchema.type === 'Misc'}
                            {#if colSchema.subType === 'Checkbox'}
                                <div class="flex items-center mt-1">
                                    <input
                                        id="field-{col.field}"
                                        type="checkbox"
                                        bind:checked={editedData[col.field]}
                                        class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded dark:bg-gray-700 dark:border-gray-600"
                                    />
                                </div>
                            {:else if colSchema.subType === 'Selectbox' || colSchema.subType === 'Multiselect'}
                                {#if colSchema.subType === 'Multiselect'}
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
                            {/if}
                        {:else if colSchema.type === 'Numeric'}
                            <div class="relative mt-1">
                                {#if colSchema.subType === 'Currency'}
                                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                                        <span class="text-gray-500 sm:text-sm">{getCurrencySymbol(colSchema.currency)}</span>
                                    </div>
                                {/if}
                                <input
                                    id="field-{col.field}"
                                    type="number"
                                    bind:value={editedData[col.field]}
                                    step="any"
                                    class="block w-full {colSchema.subType === 'Currency' ? 'pl-7' : 'px-3'} py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                                />
                                {#if colSchema.subType === 'Percent'}
                                    <div class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
                                        <span class="text-gray-500 sm:text-sm">%</span>
                                    </div>
                                {/if}
                            </div>
                        {:else if colSchema.type === 'DateTime' && !colSchema.format}
                             <input
                                id="field-{col.field}"
                                type={colSchema.subType === 'Time' ? 'time' : (colSchema.subType === 'Date' ? 'date' : 'datetime-local')}
                                bind:value={editedData[col.field]}
                                class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border {errors[col.field] ? 'border-red-500' : 'border-gray-300 dark:border-gray-600'} rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        {:else if colSchema.type === 'Text' && colSchema.subType === 'Small Text'}
                            <input
                                id="field-{col.field}"
                                type="text"
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
