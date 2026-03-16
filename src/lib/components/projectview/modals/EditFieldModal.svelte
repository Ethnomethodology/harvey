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
        TextInitial,
        Star,
		Settings2
    } from 'lucide-svelte';
    import ProgressIcon from '$lib/components/projectview/data/tables/icons/ProgressIcon.svelte';
    import { 
        Input, 
        Label, 
        Select, 
        Checkbox, 
        Textarea, 
        Button, 
        Toggle,
        Helper,
        Modal
    } from 'flowbite-svelte';

    export let fieldName = '';
    export let colSchema = {};
    export let currentPrimaryField = null; // New prop to know if there's already a primary field

    const dispatch = createEventDispatcher();

    const currencyOptions = [
        { name: 'USD ($) - US Dollar', value: 'USD' },
        { name: 'EUR (€) - Euro', value: 'EUR' },
        { name: 'GBP (£) - British Pound', value: 'GBP' },
        { name: 'JPY (¥) - Japanese Yen', value: 'JPY' },
        { name: 'INR (₹) - Indian Rupee', value: 'INR' },
        { name: 'CNY (¥) - Chinese Yuan', value: 'CNY' },
        { name: 'AUD ($) - Australian Dollar', value: 'AUD' },
        { name: 'CAD ($) - Canadian Dollar', value: 'CAD' },
        { name: 'CHF (CHF) - Swiss Franc', value: 'CHF' },
        { name: 'SGD ($) - Singapore Dollar', value: 'SGD' },
        { name: 'HKD ($) - Hong Kong Dollar', value: 'HKD' },
        { name: 'NZD ($) - New Zealand Dollar', value: 'NZD' },
        { name: 'KRW (₩) - South Korean Won', value: 'KRW' },
        { name: 'NOK (kr) - Norwegian Krone', value: 'NOK' },
        { name: 'MXN ($) - Mexican Peso', value: 'MXN' },
        { name: 'RUB (₽) - Russian Ruble', value: 'RUB' },
        { name: 'ZAR (R) - South African Rand', value: 'ZAR' },
        { name: 'TRY (₺) - Turkish Lira', value: 'TRY' },
        { name: 'BRL (R$) - Brazilian Real', value: 'BRL' },
        { name: 'TWD (NT$) - Taiwan Dollar', value: 'TWD' },
        { name: 'DKK (kr) - Danish Krone', value: 'DKK' },
        { name: 'PLN (zł) - Polish Zloty', value: 'PLN' },
        { name: 'THB (฿) - Thai Baht', value: 'THB' },
        { name: 'IDR (Rp) - Indonesian Rupiah', value: 'IDR' },
        { name: 'PHP (₱) - Philippine Peso', value: 'PHP' },
        { name: 'Other (Custom Code)', value: 'OTHER' }
    ];

    let editedName = fieldName;
    let editedSchema = { 
        type: 'Text',
        subType: 'Small Text',
        required: false,
        primary: false,
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

    const types = ['Text', 'Numeric', 'DateTime', 'Contact', 'Misc'].map(t => ({name: t, value: t}));
    const subTypes = {
        'Text': ['Small Text', 'Long Text'],
        'Numeric': ['Number', 'Currency', 'Percent', 'Progress', 'Rating'],
        'DateTime': ['Date', 'Date & Time', 'Time'],
        'Contact': ['Email', 'Phone', 'Hyperlink'],
        'Misc': ['Selectbox', 'Checkbox', 'Multiselect', 'Project Link']
    };

    const dateTimeFormats = {
        'Date': [
            { name: 'Default (YYYY-MM-DD)', value: '' },
            { name: 'YYYY-MM-DD', value: 'YYYY-MM-DD' },
            { name: 'DD/MM/YYYY', value: 'DD/MM/YYYY' },
            { name: 'MM/DD/YYYY', value: 'MM/DD/YYYY' },
            { name: 'YYYY', value: 'YYYY' },
            { name: 'MMMM', value: 'MMMM' },
            { name: 'MMMM YYYY', value: 'MMMM YYYY' }
        ],
        'Date & Time': [
            { name: 'Default (YYYY-MM-DD HH:mm)', value: '' },
            { name: 'YYYY-MM-DD HH:mm', value: 'YYYY-MM-DD HH:mm' },
            { name: 'DD/MM/YYYY HH:mm', value: 'DD/MM/YYYY HH:mm' },
            { name: 'MM/DD/YYYY hh:mm A', value: 'MM/DD/YYYY hh:mm A' }
        ],
        'Time': [
            { name: 'Default (HH:mm)', value: '' },
            { name: 'HH:mm', value: 'HH:mm' },
            { name: 'HH:mm:ss', value: 'HH:mm:ss' },
            { name: 'hh:mm A', value: 'hh:mm A' }
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

    function handlePrimaryChange() {
        if (editedSchema.primary) {
            editedSchema.required = true;
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
            if (subType === 'Progress') return ProgressIcon;
            if (subType === 'Rating') return Star;
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

    // A field can be made primary ONLY if no other field is primary, or if it IS the current primary.
    $: isPrimaryDisabled = currentPrimaryField && currentPrimaryField !== fieldName && !editedSchema.primary;
</script>

<Modal
    open={true}
    size="md"
    autoclose={false}
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-5 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
    on:close={() => dispatch('cancel')}
>
	<div slot="header" class="flex items-center gap-2">
		<svelte:component this={getIcon(editedSchema.type, editedSchema.subType)} class="w-5 h-5 text-blue-500" />
		<h3 class="text-lg font-semibold text-gray-900 dark:text-white">
			Edit Field Settings
		</h3>
	</div>

    <div class="space-y-5 overflow-y-auto max-h-[65vh] custom-scrollbar pr-2">
        <!-- Field Name -->
        <div class="space-y-1">
            <Label for="field-name" class="text-xs font-bold uppercase tracking-wider text-gray-500">Field Name</Label>
            <Input
                id="field-name"
                type="text"
                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                bind:value={editedName}
                placeholder="Enter field name"
            />
        </div>

        <div class="grid grid-cols-2 gap-4">
            <!-- Type Selection -->
            <div class="space-y-1">
                <Label for="field-type" class="text-xs font-bold uppercase tracking-wider text-gray-500">Data Type</Label>
                <Select
                    id="field-type"
                    items={types}
                    bind:value={editedSchema.type}
                    on:change={handleTypeChange}
                />
            </div>

            <!-- SubType Selection -->
            <div class="space-y-1">
                <Label for="field-subtype" class="text-xs font-bold uppercase tracking-wider text-gray-500">Sub-Type</Label>
                <Select
                    id="field-subtype"
                    items={(subTypes[editedSchema.type] || []).map(st => ({name: st, value: st}))}
                    bind:value={editedSchema.subType}
                    on:change={handleSubTypeChange}
                />
            </div>
        </div>

        <!-- Toggles Area -->
        <div class="grid grid-cols-2 gap-4">
            <!-- Primary Toggle -->
            <div class="flex items-center space-x-3 bg-gray-50 dark:bg-gray-800/30 p-3 rounded-xl border border-gray-100 dark:border-gray-800"
                    class:opacity-50={isPrimaryDisabled}
                    title={isPrimaryDisabled ? "Another field is already primary" : ""}>
                <Checkbox
                    id="field-primary"
                    bind:checked={editedSchema.primary}
                    on:change={handlePrimaryChange}
                    disabled={isPrimaryDisabled}
                >
                    Primary Field
                </Checkbox>
            </div>

            <!-- Required Toggle -->
            <div class="flex items-center space-x-3 bg-gray-50 dark:bg-gray-800/30 p-3 rounded-xl border border-gray-100 dark:border-gray-800">
                <Checkbox
                    id="field-required"
                    bind:checked={editedSchema.required}
                    disabled={editedSchema.primary}
                >
                    Required
                </Checkbox>
            </div>
        </div>

        <!-- Constraints Area -->
        <div class="space-y-4 pt-2">
            {#if editedSchema.type === 'Numeric'}
                <div class="grid grid-cols-2 gap-4">
                    <div class="space-y-1">
                        <Label for="field-min" class="text-xs font-bold uppercase tracking-wider text-gray-500">{editedSchema.subType === 'Rating' ? 'Min Stars' : 'Min Value'}</Label>
                        <Input
                            id="field-min"
                            type="number"
                            autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                            bind:value={editedSchema.min}
                        />
                    </div>
                    <div class="space-y-1">
                        <Label for="field-max" class="text-xs font-bold uppercase tracking-wider text-gray-500">{editedSchema.subType === 'Rating' ? 'Max Stars' : 'Max Value'}</Label>
                        <Input
                            id="field-max"
                            type="number"
                            autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                            bind:value={editedSchema.max}
                        />
                    </div>
                </div>
                {#if editedSchema.subType === 'Currency'}
                    <div class="space-y-1">
                        <Label for="field-currency" class="text-xs font-bold uppercase tracking-wider text-gray-500">Currency / Country</Label>
                        <div class="space-y-2">
                            <Select
                                id="field-currency"
                                items={currencyOptions}
                                bind:value={selectedCurrency}
                            />
                            {#if selectedCurrency === 'OTHER'}
                                <Input
                                    type="text"
                                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                    bind:value={customCurrencyCode}
                                    placeholder="Enter 3-letter ISO Code (e.g. BTC)"
                                    maxlength="3"
                                />
                            {/if}
                        </div>
                    </div>
                {/if}
            {:else if editedSchema.subType === 'Selectbox' || editedSchema.subType === 'Multiselect'}
                <div class="space-y-1">
                    <Label for="field-options" class="text-xs font-bold uppercase tracking-wider text-gray-500">Options (Comma separated)</Label>
                    <Input
                        id="field-options"
                        type="text"
                        autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                        bind:value={optionsText}
                        placeholder="Option 1, Option 2, Option 3..."
                    />
                </div>
            {:else if editedSchema.type === 'DateTime'}
                <div class="space-y-1">
                    <Label for="field-format" class="text-xs font-bold uppercase tracking-wider text-gray-500">Display Format</Label>
                    <Select
                        id="field-format"
                        items={dateTimeFormats[editedSchema.subType] || []}
                        bind:value={editedSchema.format}
                    />
                </div>
            {/if}

            <!-- Description -->
            <div class="space-y-1">
                <Label for="field-desc" class="text-xs font-bold uppercase tracking-wider text-gray-500">Description</Label>
                <Textarea
                    id="field-desc"
                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                    bind:value={editedSchema.description}
                    rows="2"
                    placeholder="Explain the purpose of this field..."
                />
            </div>
        </div>
    </div>

    <svelte:fragment slot="footer">
		<Button color="alternative" on:click={() => dispatch('cancel')} title="Cancel changes">
			Cancel
		</Button>
		<Button color="blue" on:click={handleSave} title="Save field settings">
			Save Settings
		</Button>
    </svelte:fragment>
</Modal>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        width: 6px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-200 dark:bg-gray-700 rounded-full;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        @apply bg-gray-300 dark:bg-gray-600;
    }
</style>