<!-- src/lib/components/projectview/modals/CreateTableModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';

    import { project } from '$lib/stores/projectStore.js';
    import { invoke } from '@tauri-apps/api/core';
    import { Plus, Trash2, X, Sheet } from 'lucide-svelte';
    import { 
        Input, 
        Label, 
        Select, 
        Checkbox, 
        Button, 
        Table, 
        TableHead, 
        TableHeadCell, 
        TableBody, 
        TableBodyRow, 
        TableBodyCell,
        Modal
    } from 'flowbite-svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let fields = []; // { name, type, subType, options, required, primary, min, max, description, format, currency, customCurrency }

    const FIELD_TYPES = {
        'Text': ['Small Text', 'Long Text'],
        'Numeric': ['Number', 'Currency', 'Percent', 'Progress', 'Rating'],
        'DateTime': ['Date', 'Date & Time', 'Time'],
        'Contact': ['Email', 'Phone', 'Hyperlink'],
        'Misc': ['Selectbox', 'Checkbox', 'Multiselect', 'Project Link']
    };

    const DATETIME_FORMATS = {
        'Date': ['None', 'YYYY-MM-DD', 'DD/MM/YYYY', 'MM/DD/YYYY', 'MMMM DD, YYYY', 'YYYY', 'MMMM', 'MMMM YYYY'],
        'Date & Time': ['None', 'YYYY-MM-DD HH:mm', 'DD/MM/YYYY HH:mm', 'MM/DD/YYYY hh:mm A'],
        'Time': ['None', 'HH:mm', 'HH:mm:ss', 'hh:mm A']
    };

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

    function addField() {
        fields = [...fields, {
            name: `Field ${fields.length + 1}`,
            type: 'Text',
            subType: 'Small Text',
            options: '',
            required: false,
            primary: false,
            min: '',
            max: '',
            description: '',
            format: 'None',
            currency: 'USD',
            customCurrency: 'XXX'
        }];
    }

    function removeField(index) {
        if (fields.length > 1) {
            fields = fields.filter((_, i) => i !== index);
        }
    }

    function handlePrimaryChange(index) {
        if (fields[index].primary) {
            fields = fields.map((f, i) => ({
                ...f,
                primary: i === index
            }));
            // If primary, it must be required
            fields[index].required = true;
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
                options: (f.subType === 'Selectbox' || f.subType === 'Multiselect') ? f.options.split(',').map(o => o.trim()).filter(o => o !== '') : [],
                required: f.required,
                primary: f.primary || false,
                min: f.min !== '' ? parseFloat(f.min) : null,
                max: f.max !== '' ? parseFloat(f.max) : null,
                description: f.description.trim(),
                format: f.format !== 'None' ? f.format : null
            };
            if (f.subType === 'Currency') {
                schema[f.name].currency = f.currency === 'OTHER' ? (f.customCurrency || 'XXX').toUpperCase().substring(0, 3) : f.currency;
            }
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

<Modal 
    bind:open={showModal} 
    size="xl" 
    outsideclose 
    on:close={closeModal} 
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col h-[50vh] max-h-[50vh] relative"
>
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <Sheet size={20} class="text-blue-600 dark:text-blue-400" />
        </div>
        <div>
            <h3 class="text-lg font-bold text-gray-900 dark:text-white">Create New Table</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Define your schema and data types</p>
        </div>
    </div>

    <div class="flex-1 overflow-auto p-0 -m-6 h-full">
        <Table hoverable={true} shadow={false} class="border-b border-gray-200 dark:border-gray-800 h-full">
            <TableHead class="bg-gray-50 dark:bg-gray-900/50 sticky top-0 z-10">
                <TableHeadCell class="w-[200px]">Field Name</TableHeadCell>
                <TableHeadCell>Type</TableHeadCell>
                <TableHeadCell>Sub-type</TableHeadCell>
                <TableHeadCell class="text-center">Primary</TableHeadCell>
                <TableHeadCell class="text-center">Req?</TableHeadCell>
                <TableHeadCell class="min-w-[150px]">Constraints</TableHeadCell>
                <TableHeadCell class="min-w-[250px]">Description</TableHeadCell>
                <TableHeadCell class="w-10"></TableHeadCell>
            </TableHead>
            <TableBody>
                {#each fields as field, i}
                    <TableBodyRow class="group">
                        <TableBodyCell class="px-3 py-2">
                            <Input size="sm" bind:value={field.name} placeholder="Name" />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2">
                            <Select size="sm" items={Object.keys(FIELD_TYPES).map(t => ({name: t, value: t}))} bind:value={field.type} on:change={() => handleTypeChange(i)} />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2">
                            <Select size="sm" items={(FIELD_TYPES[field.type] || []).map(st => ({name: st, value: st}))} bind:value={field.subType} on:change={() => handleSubTypeChange(i)} />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2 text-center">
                            <Checkbox bind:checked={field.primary} on:change={() => handlePrimaryChange(i)} />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2 text-center">
                            <Checkbox bind:checked={field.required} disabled={field.primary} />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2">
                            {#if field.subType === 'Selectbox' || field.subType === 'Multiselect'}
                                <Input size="sm" bind:value={field.options} placeholder="Opt 1, Opt 2..." />
                            {:else if field.type === 'Numeric'}
                                <div class="flex flex-col gap-1">
                                    <div class="flex gap-1">
                                        <Input size="sm" type="number" bind:value={field.min} placeholder="Min" />
                                        <Input size="sm" type="number" bind:value={field.max} placeholder="Max" />
                                    </div>
                                    {#if field.subType === 'Currency'}
                                        <div class="flex flex-col gap-1">
                                            <Select size="sm" items={currencyOptions} bind:value={field.currency} />
                                            {#if field.currency === 'OTHER'}
                                                <Input size="sm" type="text" bind:value={field.customCurrency} placeholder="ISO Code" maxlength="3" />
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                            {:else if field.type === 'DateTime'}
                                <Select size="sm" items={DATETIME_FORMATS[field.subType].map(fmt => ({name: fmt, value: fmt}))} bind:value={field.format} />
                            {:else}
                                <span class="text-xs text-gray-400 italic">None</span>
                            {/if}
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2">
                            <Input size="sm" bind:value={field.description} placeholder="Purpose of this field" />
                        </TableBodyCell>
                        <TableBodyCell class="px-3 py-2 text-center">
                            <button
                                on:click={() => removeField(i)}
                                class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-all disabled:opacity-30 disabled:cursor-not-allowed"
                                disabled={fields.length <= 1}
                                title="Remove Field"
                            >
                                <Trash2 size={16} />
                            </button>
                        </TableBodyCell>
                    </TableBodyRow>
                {/each}
            </TableBody>
        </Table>

        <div class="p-6 bg-gray-50/50 dark:bg-gray-800/30">
            <Button color="alternative" size="sm" on:click={addField} class="flex items-center gap-2">
                <Plus size={16} />
                Add Field
            </Button>
        </div>
    </div>

    <!-- Footer -->
    <div slot="footer" class="flex justify-end gap-3 w-full">
        <Button color="alternative" on:click={closeModal}>Cancel</Button>
        <Button color="blue" on:click={handleSubmit}>Create Table</Button>
    </div>
</Modal>

<style>
    .dark .hover\:bg-gray-700:hover {
        background-color: rgba(55, 65, 81, 0.5);
    }
</style>
