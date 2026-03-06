<!-- src/lib/components/projectview/modals/EditFieldModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { 
        Type as TypeIcon, 
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
        AlertCircle,
        X,
        Link2,
        TextInitial
    } from 'lucide-svelte';

    export let fieldName = '';
    export let colSchema = {};

    const dispatch = createEventDispatcher();

    let editedName = fieldName;
    let editedSchema = { 
        type: 'Text',
        subType: 'Small Text',
        required: false,
        min: null,
        max: null,
        options: [],
        description: '',
        format: '',
        ...colSchema 
    };

    if (!editedSchema.subType) editedSchema.subType = 'Small Text';

    const types = ['Text', 'Numeric', 'DateTime', 'Contact', 'Misc'];
    const subTypes = {
        'Text': ['Small Text', 'Long Text'],
        'Numeric': ['Number', 'Currency', 'Percent'],
        'DateTime': ['Date', 'Date & Time', 'Time'],
        'Contact': ['Email', 'Phone', 'Hyperlink'],
        'Misc': ['Selectbox', 'Checkbox', 'Multiselect', 'Project Link']
    };

    const dateFormats = [
        { label: 'None', value: '' },
        { label: 'YYYY-MM-DD', value: 'YYYY-MM-DD' },
        { label: 'DD/MM/YYYY', value: 'DD/MM/YYYY' },
        { label: 'MM/DD/YYYY', value: 'MM/DD/YYYY' },
        { label: 'MMMM DD, YYYY', value: 'MMMM DD, YYYY' },
        { label: 'YYYY', value: 'YYYY' },
        { label: 'MMMM', value: 'MMMM' },
        { label: 'MMMM YYYY', value: 'MMMM YYYY' }
    ];

    const dateTimeFormats = [
        { label: 'None', value: '' },
        { label: 'YYYY-MM-DD HH:mm', value: 'YYYY-MM-DD HH:mm' },
        { label: 'DD/MM/YYYY HH:mm', value: 'DD/MM/YYYY HH:mm' },
        { label: 'MM/DD/YYYY hh:mm A', value: 'MM/DD/YYYY hh:mm A' }
    ];

    const timeFormats = [
        { label: 'None', value: '' },
        { label: 'HH:mm', value: 'HH:mm' },
        { label: 'HH:mm:ss', value: 'HH:mm:ss' },
        { label: 'hh:mm A', value: 'hh:mm A' }
    ];

    const currencyOptions = [
        { label: 'USD ($) - US Dollar', value: 'USD', symbol: '$' },
        { label: 'EUR (€) - Euro', value: 'EUR', symbol: '€' },
        { label: 'GBP (£) - British Pound', value: 'GBP', symbol: '£' },
        { label: 'JPY (¥) - Japanese Yen', value: 'JPY', symbol: '¥' },
        { label: 'INR (₹) - Indian Rupee', value: 'INR', symbol: '₹' },
        { label: 'CNY (¥) - Chinese Yuan', value: 'CNY', symbol: '¥' },
        { label: 'AUD ($) - Australian Dollar', value: 'AUD', symbol: '$' },
        { label: 'CAD ($) - Canadian Dollar', value: 'CAD', symbol: '$' },
        { label: 'CHF (CHF) - Swiss Franc', value: 'CHF', symbol: 'CHF' },
        { label: 'SGD ($) - Singapore Dollar', value: 'SGD', symbol: '$' }
    ];

    let optionsText = (editedSchema.options || []).join(', ');

    function handleTypeChange() {
        editedSchema.subType = subTypes[editedSchema.type][0];
        if (editedSchema.type !== 'Numeric') {
            editedSchema.min = null;
            editedSchema.max = null;
        }
        if (editedSchema.type !== 'DateTime') {
            editedSchema.format = '';
        }
        if (editedSchema.subType === 'Currency') {
            editedSchema.currency = editedSchema.currency || 'USD';
        } else {
            delete editedSchema.currency;
        }
    }

    function handleSave() {
        if (!editedName.trim()) {
            alert('Field name cannot be empty');
            return;
        }

        if (editedSchema.subType === 'Selectbox' || editedSchema.subType === 'Multiselect') {
            editedSchema.options = optionsText.split(',').map(s => s.trim()).filter(Boolean);
        } else {
            editedSchema.options = [];
        }

        dispatch('save', { 
            oldName: fieldName,
            newName: editedName.trim(), 
            schema: { ...editedSchema } 
        });
    }

    function getIcon(type, subType) {
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
        
        return TypeIcon;
    }

</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-4 backdrop-blur-sm">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-2xl w-full max-w-xl max-h-[90vh] flex flex-col border border-gray-200 dark:border-gray-700">
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center bg-gray-50 dark:bg-gray-800/50 rounded-t-lg">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-gray-100 flex items-center">
                <svelte:component this={getIcon(editedSchema.type, editedSchema.subType)} size={20} class="mr-2 text-blue-500" />
                Edit Field: {fieldName}
            </h3>
            <button on:click={() => dispatch('cancel')} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors">
                <X size={24} />
            </button>
        </div>

        <div class="p-6 overflow-y-auto space-y-6">
            <!-- Field Name -->
            <div class="space-y-1">
                <label for="field-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    FIELD NAME <span class="text-red-500">*</span>
                </label>
                <input
                    id="field-name"
                    type="text"
                    bind:value={editedName}
                    class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <!-- Type -->
                <div class="space-y-1">
                    <label for="field-type" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        TYPE
                    </label>
                    <select
                        id="field-type"
                        bind:value={editedSchema.type}
                        on:change={handleTypeChange}
                        class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                    >
                        {#each types as t}
                            <option value={t}>{t}</option>
                        {/each}
                    </select>
                </div>

                <!-- Sub-type -->
                <div class="space-y-1">
                    <label for="field-subtype" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        SUB-TYPE
                    </label>
                    <select
                        id="field-subtype"
                        bind:value={editedSchema.subType}
                        class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                    >
                        {#each subTypes[editedSchema.type] || [] as st}
                            <option value={st}>{st}</option>
                        {/each}
                    </select>
                </div>
            </div>

            <!-- Required -->
            <div class="flex items-center space-x-2">
                <input
                    id="field-required"
                    type="checkbox"
                    bind:checked={editedSchema.required}
                    class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded dark:bg-gray-700 dark:border-gray-600"
                />
                <label for="field-required" class="text-sm font-medium text-gray-700 dark:text-gray-300">
                    REQUIRED?
                </label>
            </div>

            <!-- Constraints / Options -->
            <div class="border-t border-gray-100 dark:border-gray-700 pt-4 space-y-4">
                <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider">Constraints / Options</h4>
                
                {#if editedSchema.type === 'Numeric'}
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label for="field-min" class="block text-sm font-medium text-gray-700 dark:text-gray-300">MIN VALUE</label>
                            <input
                                id="field-min"
                                type="number"
                                step="any"
                                bind:value={editedSchema.min}
                                class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        </div>
                        <div class="space-y-1">
                            <label for="field-max" class="block text-sm font-medium text-gray-700 dark:text-gray-300">MAX VALUE</label>
                            <input
                                id="field-max"
                                type="number"
                                step="any"
                                bind:value={editedSchema.max}
                                class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        </div>
                    </div>
                    {#if editedSchema.subType === 'Currency'}
                        <div class="space-y-1">
                            <label for="field-currency" class="block text-sm font-medium text-gray-700 dark:text-gray-300">CURRENCY / COUNTRY</label>
                            <select
                                id="field-currency"
                                bind:value={editedSchema.currency}
                                class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            >
                                {#each currencyOptions as opt}
                                    <option value={opt.value}>{opt.label}</option>
                                {/each}
                            </select>
                        </div>
                    {/if}
                {:else if editedSchema.subType === 'Selectbox' || editedSchema.subType === 'Multiselect'}
                    <div class="space-y-1">
                        <label for="field-options" class="block text-sm font-medium text-gray-700 dark:text-gray-300">OPTIONS (Comma separated)</label>
                        <textarea
                            id="field-options"
                            bind:value={optionsText}
                            rows="2"
                            placeholder="Option 1, Option 2, Option 3"
                            class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                        ></textarea>
                    </div>
                {:else if editedSchema.type === 'DateTime'}
                    <div class="space-y-1">
                        <label for="field-format" class="block text-sm font-medium text-gray-700 dark:text-gray-300">FORMAT</label>
                        <select
                            id="field-format"
                            bind:value={editedSchema.format}
                            class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                        >
                            {#if editedSchema.subType === 'Date'}
                                {#each dateFormats as f}
                                    <option value={f.value}>{f.label}</option>
                                {/each}
                            {:else if editedSchema.subType === 'Time'}
                                {#each timeFormats as f}
                                    <option value={f.value}>{f.label}</option>
                                {/each}
                            {:else if editedSchema.subType === 'Date & Time'}
                                {#each dateTimeFormats as f}
                                    <option value={f.value}>{f.label}</option>
                                {/each}
                            {:else}
                                <option value="">Default (Browser Local)</option>
                            {/if}
                        </select>
                    </div>
                {:else}
                    <div class="text-sm text-gray-500 dark:text-gray-400 italic">No specific constraints for this sub-type.</div>
                {/if}
            </div>

            <!-- Description -->
            <div class="space-y-1">
                <label for="field-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    DESCRIPTION
                </label>
                <textarea
                    id="field-description"
                    bind:value={editedSchema.description}
                    rows="2"
                    class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                ></textarea>
            </div>
        </div>

        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3 bg-gray-50 dark:bg-gray-800/50 rounded-b-lg">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none"
                on:click={() => dispatch('cancel')}
            >
                Cancel
            </button>
            <button
                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none shadow-sm transition-colors"
                on:click={handleSave}
            >
                Save Field
            </button>
        </div>
    </div>
</div>
