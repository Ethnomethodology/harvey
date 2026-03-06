<!-- src/lib/components/projectview/modals/EditFieldModal.svelte -->
<script>
    import { createEventDispatcher } from 'svelte';
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

    const currencyOptions = [
        { label: 'USD ($) - US Dollar', value: 'USD' },
        { label: 'EUR (€) - Euro', value: 'EUR' },
        { label: 'GBP (£) - British Pound', value: 'GBP' },
        { label: 'JPY (¥) - Japanese Yen', value: 'JPY' },
        { label: 'INR (₹) - Indian Rupee', value: 'INR' },
        { label: 'CNY (¥) - Chinese Yuan', value: 'CNY' },
        { label: 'AUD ($) - Australian Dollar', value: 'AUD' },
        { label: 'CAD ($) - Canadian Dollar', value: 'CAD' },
        { label: 'CHF (CHF) - Swiss Franc', value: 'CHF' },
        { label: 'SGD ($) - Singapore Dollar', value: 'SGD' },
        { label: 'HKD ($) - Hong Kong Dollar', value: 'HKD' },
        { label: 'NZD ($) - New Zealand Dollar', value: 'NZD' },
        { label: 'KRW (₩) - South Korean Won', value: 'KRW' },
        { label: 'NOK (kr) - Norwegian Krone', value: 'NOK' },
        { label: 'MXN ($) - Mexican Peso', value: 'MXN' },
        { label: 'RUB (₽) - Russian Ruble', value: 'RUB' },
        { label: 'ZAR (R) - South African Rand', value: 'ZAR' },
        { label: 'TRY (₺) - Turkish Lira', value: 'TRY' },
        { label: 'BRL (R$) - Brazilian Real', value: 'BRL' },
        { label: 'TWD (NT$) - Taiwan Dollar', value: 'TWD' },
        { label: 'DKK (kr) - Danish Krone', value: 'DKK' },
        { label: 'PLN (zł) - Polish Zloty', value: 'PLN' },
        { label: 'THB (฿) - Thai Baht', value: 'THB' },
        { label: 'IDR (Rp) - Indonesian Rupiah', value: 'IDR' },
        { label: 'PHP (₱) - Philippine Peso', value: 'PHP' },
        { label: 'Other (Custom Code)', value: 'OTHER' }
    ];

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
        currency: 'USD',
        ...colSchema 
    };

    if (!editedSchema.subType) editedSchema.subType = 'Small Text';

    // Handle custom currency display/initialization
    let isOtherCurrency = editedSchema.subType === 'Currency' && !currencyOptions.find(o => o.value === editedSchema.currency && o.value !== 'OTHER');
    let customCurrencyCode = isOtherCurrency ? editedSchema.currency : 'XXX';
    let selectedCurrency = isOtherCurrency ? 'OTHER' : (editedSchema.currency || 'USD');

    const types = ['Text', 'Numeric', 'DateTime', 'Contact', 'Misc'];
    const subTypes = {
        'Text': ['Small Text', 'Long Text'],
        'Numeric': ['Number', 'Currency', 'Percent'],
        'DateTime': ['Date', 'Date & Time', 'Time'],
        'Contact': ['Email', 'Phone', 'Hyperlink'],
        'Misc': ['Selectbox', 'Checkbox', 'Multiselect', 'Project Link']
    };

    const dateTimeFormats = {
        'Date': [
            { label: 'Default', value: '' },
            { label: 'YYYY-MM-DD', value: 'YYYY-MM-DD' },
            { label: 'DD/MM/YYYY', value: 'DD/MM/YYYY' },
            { label: 'MM/DD/YYYY', value: 'MM/DD/YYYY' },
            { label: 'Full Date', value: 'MMMM DD, YYYY' },
            { label: 'Year Only', value: 'YYYY' },
            { label: 'Month Only', value: 'MMMM' },
            { label: 'Month Year', value: 'MMMM YYYY' }
        ],
        'Date & Time': [
            { label: 'Default', value: '' },
            { label: 'ISO', value: 'YYYY-MM-DD HH:mm' },
            { label: 'British', value: 'DD/MM/YYYY HH:mm' },
            { label: 'American', value: 'MM/DD/YYYY hh:mm A' }
        ],
        'Time': [
            { label: 'Default', value: '' },
            { label: '24 Hour', value: 'HH:mm' },
            { label: '24 Hour + Sec', value: 'HH:mm:ss' },
            { label: '12 Hour', value: 'hh:mm A' }
        ]
    };

    let optionsText = (editedSchema.options || []).join(', ');

    function handleTypeChange() {
        editedSchema.subType = subTypes[editedSchema.type][0];
        handleSubTypeChange();
    }

    function handleSubTypeChange() {
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
            alert('Field name cannot be empty.');
            return;
        }
        
        const finalSchema = { ...editedSchema };
        if (finalSchema.subType === 'Selectbox' || finalSchema.subType === 'Multiselect') {
            finalSchema.options = optionsText.split(',').map(o => o.trim()).filter(o => o !== '');
        } else {
            delete finalSchema.options;
        }

        if (finalSchema.subType === 'Currency') {
            finalSchema.currency = selectedCurrency === 'OTHER' ? customCurrencyCode.toUpperCase().substring(0, 3) : selectedCurrency;
        }

        dispatch('save', { oldName: fieldName, newName: editedName.trim(), schema: finalSchema });
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

<div class="fixed inset-0 z-[150] flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm p-4">
    <div class="bg-white dark:bg-gray-900 rounded-lg shadow-2xl w-full max-w-lg flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden">
        <!-- Header -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50 dark:bg-gray-800/50">
            <h3 class="text-lg font-bold text-gray-900 dark:text-gray-100 flex items-center">
                <svelte:component this={getIcon(editedSchema.type, editedSchema.subType)} class="mr-2 text-blue-500" size={20} />
                Edit Field Settings
            </h3>
            <button on:click={() => dispatch('cancel')} class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300">
                <X size={20} />
            </button>
        </div>

        <!-- Content -->
        <div class="p-6 overflow-y-auto space-y-5 max-h-[70vh]">
            <!-- Field Name -->
            <div class="space-y-1">
                <label for="field-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">FIELD NAME</label>
                <input
                    id="field-name"
                    type="text"
                    bind:value={editedName}
                    placeholder="Enter field name"
                    class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <!-- Type Selection -->
                <div class="space-y-1">
                    <label for="field-type" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">DATA TYPE</label>
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

                <!-- SubType Selection -->
                <div class="space-y-1">
                    <label for="field-subtype" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">SUB-TYPE</label>
                    <select
                        id="field-subtype"
                        bind:value={editedSchema.subType}
                        on:change={handleSubTypeChange}
                        class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                    >
                        {#each subTypes[editedSchema.type] || [] as st}
                            <option value={st}>{st}</option>
                        {/each}
                    </select>
                </div>
            </div>

            <!-- Required Toggle -->
            <div class="flex items-center space-x-3 bg-gray-50 dark:bg-gray-800/30 p-3 rounded-md border border-gray-100 dark:border-gray-800">
                <input
                    id="field-required"
                    type="checkbox"
                    bind:checked={editedSchema.required}
                    class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="field-required" class="text-sm font-medium text-gray-700 dark:text-gray-300">This field is required</label>
            </div>

            <!-- Constraints Area -->
            <div class="space-y-4 pt-2">
                {#if editedSchema.type === 'Numeric'}
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label for="field-min" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">MIN VALUE</label>
                            <input
                                id="field-min"
                                type="number"
                                bind:value={editedSchema.min}
                                class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        </div>
                        <div class="space-y-1">
                            <label for="field-max" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">MAX VALUE</label>
                            <input
                                id="field-max"
                                type="number"
                                bind:value={editedSchema.max}
                                class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                            />
                        </div>
                    </div>
                    {#if editedSchema.subType === 'Currency'}
                        <div class="space-y-1">
                            <label for="field-currency" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">CURRENCY / COUNTRY</label>
                            <div class="space-y-2">
                                <select
                                    id="field-currency"
                                    bind:value={selectedCurrency}
                                    class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                                >
                                    {#each currencyOptions as opt}
                                        <option value={opt.value}>{opt.label}</option>
                                    {/each}
                                </select>
                                {#if selectedCurrency === 'OTHER'}
                                    <input 
                                        type="text" 
                                        bind:value={customCurrencyCode} 
                                        placeholder="Enter 3-letter ISO Code (e.g. BTC)" 
                                        maxlength="3"
                                        class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                                    />
                                {/if}
                            </div>
                        </div>
                    {/if}
                {:else if editedSchema.subType === 'Selectbox' || editedSchema.subType === 'Multiselect'}
                    <div class="space-y-1">
                        <label for="field-options" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">OPTIONS (Comma separated)</label>
                        <input
                            id="field-options"
                            type="text"
                            bind:value={optionsText}
                            placeholder="Option 1, Option 2, Option 3..."
                            class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                        />
                    </div>
                {:else if editedSchema.type === 'DateTime'}
                    <div class="space-y-1">
                        <label for="field-format" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">DISPLAY FORMAT</label>
                        <select
                            id="field-format"
                            bind:value={editedSchema.format}
                            class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                        >
                            {#each (dateTimeFormats[editedSchema.subType] || []) as f}
                                <option value={f.value}>{f.label}</option>
                            {/each}
                        </select>
                    </div>
                {/if}

                <!-- Description -->
                <div class="space-y-1 pt-2">
                    <label for="field-desc" class="block text-sm font-medium text-gray-700 dark:text-gray-300 uppercase tracking-wider text-xs">TOOLTIP / DESCRIPTION</label>
                    <textarea
                        id="field-desc"
                        bind:value={editedSchema.description}
                        rows="2"
                        placeholder="Explain the purpose of this field..."
                        class="block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm dark:text-gray-100"
                    ></textarea>
                </div>
            </div>
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end space-x-3 bg-gray-50 dark:bg-gray-800/50">
            <button
                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors"
                on:click={() => dispatch('cancel')}
            >
                Cancel
            </button>
            <button
                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 shadow-sm transition-colors"
                on:click={handleSave}
            >
                Save Settings
            </button>
        </div>
    </div>
</div>
