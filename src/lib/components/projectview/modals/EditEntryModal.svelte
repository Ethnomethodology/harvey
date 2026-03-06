<!-- src/lib/components/projectview/modals/EditEntryModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
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
        TextInitial,
        Calendar as CalendarIcon
    } from 'lucide-svelte';

    export let rowData = {};
    export let columns = [];
    export let schema = {};
    export let rowIndex = 0;

    const dispatch = createEventDispatcher();

    function getCurrencySymbol(currencyCode) {
        const symbols = {
            'USD': '$', 'EUR': '€', 'GBP': '£', 'JPY': '¥', 'INR': '₹', 'CNY': '¥',
            'AUD': '$', 'CAD': '$', 'CHF': 'CHF', 'SGD': '$', 'HKD': '$', 'NZD': '$',
            'KRW': '₩', 'NOK': 'kr', 'MXN': '$', 'RUB': '₽', 'ZAR': 'R', 'TRY': '₺',
            'BRL': 'R$', 'TWD': 'NT$', 'DKK': 'kr', 'PLN': 'zł', 'THB': '฿', 'IDR': 'Rp', 'PHP': '₱'
        };
        return symbols[currencyCode] || currencyCode || 'XXX';
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
        }
        if (type === 'Text' && subType === 'Long Text') return TextInitial;
        return Type;
    }

    let editedData = { ...rowData };
    for (const field in schema) {
        if (schema[field].type === 'Misc' && schema[field].subType === 'Multiselect' && typeof editedData[field] === 'string') {
            editedData[field] = editedData[field].split(',').map(s => s.trim()).filter(Boolean);
        }
    }

    let errors = {};

    function validateField(field, value) {
        const colSchema = schema[field];
        if (!colSchema) return null;
        if (colSchema.required && (value === null || value === undefined || value === "")) return "Field is required";
        if (value !== null && value !== undefined && value !== "") {
            const { type, subType, min, max, format } = colSchema;
            if (type === 'Numeric') {
                const num = parseFloat(value);
                if (isNaN(num) || !isFinite(value)) return "Invalid number format";
                if (min !== null && num < min) return `Minimum value is ${min}`;
                if (max !== null && num > max) return `Maximum value is ${max}`;
            } else if (type === 'Contact') {
                if (subType === 'Email' && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return "Invalid email format";
                if (subType === 'Phone' && !/^\+?[\d\s-]{7,20}$/.test(value)) return "Invalid phone format";
            } else if (type === 'DateTime') {
                if (subType === 'Time') {
                    if (format === 'HH:mm' && !/^([01]\d|2[0-3]):([0-5]\d)$/.test(value)) return "Invalid format (HH:mm)";
                    if (format === 'HH:mm:ss' && !/^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/.test(value)) return "Invalid format (HH:mm:ss)";
                    if (format === 'hh:mm A' && !/^(0[1-9]|1[0-2]):([0-5]\d)\s?(AM|PM)$/i.test(value)) return "Invalid format (hh:mm AM/PM)";
                } else if (subType === 'Date') {
                    if (format === 'YYYY-MM-DD' && !/^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(value)) return "Invalid format (YYYY-MM-DD)";
                } else if (isNaN(Date.parse(value))) return "Invalid date & time format";
            }
        }
        return null;
    }

    function handleSave() {
        let newErrors = {};
        columns.forEach(col => {
            if (col.field) {
                const error = validateField(col.field, editedData[col.field]);
                if (error) newErrors[col.field] = error;
            }
        });
        errors = newErrors;
        if (Object.keys(newErrors).length === 0) {
            dispatch('save', { rowData: editedData, rowIndex });
        }
    }

    let projectAssets = [];
    onMount(async () => {
        const currentProject = get(project);
        if (currentProject?.id) {
            const { getProjectAssetsForLink } = await import('$lib/services/projectService.js');
            projectAssets = await getProjectAssetsForLink(currentProject.id);
        }
    });

    // Helper to sync date/time parts for datetime-local
    function handleDateTimeChange(field, type, event) {
        let currentVal = editedData[field] || "";
        let datePart = "";
        let timePart = "00:00";

        if (currentVal.includes('T')) {
            [datePart, timePart] = currentVal.split('T');
        } else if (currentVal.includes(' ')) {
             [datePart, timePart] = currentVal.split(' ');
        } else if (/^\d{4}-\d{2}-\d{2}$/.test(currentVal)) {
            datePart = currentVal;
        }

        if (type === 'date') datePart = event.target.value;
        if (type === 'time') timePart = event.target.value;

        if (datePart) {
            editedData[field] = `${datePart}T${timePart || "00:00"}`;
        } else {
            editedData[field] = "";
        }
    }
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-[100] p-4 backdrop-blur-sm">
    <div class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden" on:click|stopPropagation>
        <!-- Header -->
        <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                    <Pencil size={20} class="text-blue-600 dark:text-blue-400" />
                </div>
                <div>
                    <h3 class="text-lg font-bold text-gray-900 dark:text-white">Edit Entry</h3>
                    <p class="text-xs text-gray-500 dark:text-gray-400">Row index: {rowIndex + 1}</p>
                </div>
            </div>
            <button on:click={() => dispatch('cancel')} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all">
                <X size={20} />
            </button>
        </div>

        <!-- Form Content -->
        <div class="flex-1 overflow-y-auto p-6 space-y-6 custom-scrollbar">
            {#each columns as col}
                {#if col.field && col.field !== 'harvey_internal_id'}
                    {@const colSchema = schema[col.field] || {}}
                    <div class="group space-y-2">
                        <label for="field-{col.field}" class="flex items-center text-xs font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wider group-focus-within:text-blue-500 transition-colors">
                            <svelte:component this={getSubtypeIcon(colSchema)} size={14} class="mr-2" />
                            {col.field}
                            {#if colSchema.required}<span class="text-red-500 ml-1">*</span>{/if}
                        </label>
                        
                        <div class="relative">
                            {#if colSchema.type === 'DateTime'}
                                {#if colSchema.subType === 'Date'}
                                    <div class="relative">
                                        <input type="date" id="field-{col.field}" bind:value={editedData[col.field]} 
                                            class="input-base pr-10 {errors[col.field] ? 'input-error' : ''}" />
                                        <CalendarIcon size={16} class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" />
                                    </div>
                                {:else if colSchema.subType === 'Time'}
                                    <div class="relative">
                                        <input type="time" id="field-{col.field}" bind:value={editedData[col.field]} 
                                            class="input-base pr-10 {errors[col.field] ? 'input-error' : ''}" />
                                        <Clock size={16} class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" />
                                    </div>
                                {:else}
                                    <!-- Date & Time: Better UX using two adjacent inputs -->
                                    <div class="flex space-x-2">
                                        <div class="relative flex-1">
                                            <input type="date" value={(editedData[col.field] || "").split('T')[0] || ""} 
                                                on:input={(e) => handleDateTimeChange(col.field, 'date', e)}
                                                class="input-base pr-10 {errors[col.field] ? 'input-error' : ''}" />
                                            <CalendarIcon size={14} class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" />
                                        </div>
                                        <div class="relative w-32">
                                            <input type="time" value={(editedData[col.field] || "").split('T')[1] || "00:00"} 
                                                on:input={(e) => handleDateTimeChange(col.field, 'time', e)}
                                                class="input-base pr-10 {errors[col.field] ? 'input-error' : ''}" />
                                            <Clock size={14} class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none" />
                                        </div>
                                    </div>
                                {/if}
                            {:else if colSchema.type === 'Misc'}
                                {#if colSchema.subType === 'Checkbox'}
                                    <label class="flex items-center space-x-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer hover:bg-blue-50 dark:hover:bg-blue-900/10 transition-colors">
                                        <input type="checkbox" bind:checked={editedData[col.field]} class="h-5 w-5 text-blue-600 rounded border-gray-300 dark:border-gray-600 focus:ring-blue-500" />
                                        <span class="text-sm text-gray-700 dark:text-gray-300">Enabled</span>
                                    </label>
                                {:else if colSchema.subType === 'Selectbox' || colSchema.subType === 'Multiselect'}
                                    <select id="field-{col.field}" bind:value={editedData[col.field]} multiple={colSchema.subType === 'Multiselect'}
                                        class="input-base {errors[col.field] ? 'input-error' : ''} {colSchema.subType === 'Multiselect' ? 'h-32' : ''}">
                                        {#if colSchema.subType !== 'Multiselect'}<option value="">Select option...</option>{/if}
                                        {#each colSchema.options || [] as opt}<option value={opt}>{opt}</option>{/each}
                                    </select>
                                {:else if colSchema.subType === 'Project Link'}
                                    <select id="field-{col.field}" bind:value={editedData[col.field]} class="input-base {errors[col.field] ? 'input-error' : ''}">
                                        <option value="">Select asset...</option>
                                        {#each projectAssets as asset}<option value={asset.value}>{asset.label}</option>{/each}
                                    </select>
                                {/if}
                            {:else if colSchema.type === 'Numeric'}
                                <div class="relative">
                                    {#if colSchema.subType === 'Currency'}
                                        <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                                            <span class="text-gray-400 text-sm font-bold">{getCurrencySymbol(colSchema.currency)}</span>
                                        </div>
                                    {/if}
                                    <input type="number" step="any" id="field-{col.field}" bind:value={editedData[col.field]} 
                                        class="input-base {colSchema.subType === 'Currency' ? 'pl-8' : ''} {colSchema.subType === 'Percent' ? 'pr-8' : ''} {errors[col.field] ? 'input-error' : ''}" />
                                    {#if colSchema.subType === 'Percent'}
                                        <div class="absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none">
                                            <span class="text-gray-400 text-sm font-bold">%</span>
                                        </div>
                                    {/if}
                                </div>
                            {:else if colSchema.type === 'Contact'}
                                <input type={colSchema.subType === 'Email' ? 'email' : (colSchema.subType === 'Phone' ? 'tel' : 'url')} 
                                    id="field-{col.field}" bind:value={editedData[col.field]} 
                                    class="input-base {errors[col.field] ? 'input-error' : ''}" />
                            {:else if colSchema.type === 'Text' && colSchema.subType === 'Small Text'}
                                <input type="text" id="field-{col.field}" bind:value={editedData[col.field]} 
                                    class="input-base {errors[col.field] ? 'input-error' : ''}" />
                            {:else}
                                <textarea id="field-{col.field}" bind:value={editedData[col.field]} rows="3"
                                    class="input-base py-3 resize-none custom-scrollbar {errors[col.field] ? 'input-error' : ''}"></textarea>
                            {/if}
                        </div>
                        
                        {#if errors[col.field]}
                            <p class="flex items-center text-[11px] text-red-500 font-medium animate-in fade-in slide-in-from-top-1">
                                <AlertCircle size={12} class="mr-1" /> {errors[col.field]}
                            </p>
                        {/if}
                        {#if colSchema.description}
                            <p class="text-[11px] text-gray-400 dark:text-gray-500 leading-relaxed italic">{colSchema.description}</p>
                        {/if}
                    </div>
                {/if}
            {/each}
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end space-x-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
            <button class="btn-secondary" on:click={() => dispatch('cancel')}>Cancel</button>
            <button class="btn-primary" on:click={handleSave}>Save Changes</button>
        </div>
    </div>
</div>

<style lang="postcss">
    .input-base {
        @apply block w-full px-4 py-2.5 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg shadow-sm text-sm dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-all duration-200;
    }
    .input-error {
        @apply border-red-500 ring-2 ring-red-500/10 focus:border-red-500 focus:ring-red-500/20;
    }
    .btn-primary {
        @apply px-5 py-2.5 text-sm font-bold text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:ring-4 focus:ring-blue-500/30 shadow-lg shadow-blue-500/20 transition-all active:scale-95;
    }
    .btn-secondary {
        @apply px-5 py-2.5 text-sm font-bold text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-750 transition-all active:scale-95;
    }
    .custom-scrollbar::-webkit-scrollbar {
        @apply w-1.5;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-300 dark:bg-gray-700 rounded-full;
    }
    /* Hide native calendar icon to use Lucide one for consistent aesthetic */
    input[type="date"]::-webkit-calendar-picker-indicator,
    input[type="time"]::-webkit-calendar-picker-indicator {
        @apply opacity-0 absolute right-0 w-8 h-full cursor-pointer;
    }
</style>
