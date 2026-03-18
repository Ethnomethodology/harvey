<!-- src/lib/components/projectview/modals/HeaderConfirmationModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
import { Modal } from 'flowbite-svelte';
	import { fade } from 'svelte/transition';
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
        Alert
    } from 'flowbite-svelte';
    import { Info } from 'lucide-svelte';

	export let showModal = false;
	export let previewData = { fields: [], headers: [], data: [] };
	export let tablePath = '';

	const dispatch = createEventDispatcher();

	let step = 1; // 1: Header Confirmation, 2: Schema Definition
	let hasHeaders = true;
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

    $: availableFields = previewData?.fields || previewData?.headers || [];

	$: if (showModal && availableFields.length > 0 && fields.length === 0) {
		fields = availableFields.map((f, i) => {
            let name = `Field ${i + 1}`;
            if (hasHeaders && previewData.data && previewData.data[0]) {
                const val = previewData.data[0][f];
                if (val != null && String(val).trim() !== '') {
                    name = String(val).trim();
                }
            }
            return {
                name,
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
            };
        });
	}

	// Update field names if hasHeaders changes
	$: if (step === 1 && availableFields.length > 0 && fields.length > 0) {
		fields = availableFields.map((f, i) => {
            let name = `Field ${i + 1}`;
            if (hasHeaders && previewData.data && previewData.data[0]) {
                const val = previewData.data[0][f];
                if (val != null && String(val).trim() !== '') {
                    name = String(val).trim();
                }
            }
            return {
                ...(fields[i] || {}),
                name
            };
        });
	}

	function handleTypeChange(index) {
		const type = fields[index].type;
		fields[index].subType = FIELD_TYPES[type][0];
		fields[index].format = 'None';
        if (type !== 'Numeric') {
            fields[index].min = '';
            fields[index].max = '';
        }
        if (fields[index].subType === 'Currency') {
            fields[index].currency = fields[index].currency || 'USD';
        } else {
            delete fields[index].currency;
        }
        // Force Svelte to re-render the row immediately to fix the dropdown items
        fields = [...fields];
	}

	function handleSubTypeChange(index) {
		fields[index].format = 'None';
        if (fields[index].subType === 'Currency') {
            fields[index].currency = fields[index].currency || 'USD';
        } else {
            delete fields[index].currency;
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

	function goToStep2() {
		step = 2;
	}

    let confirmed = false;
    let wasOpen = false;

    $: if (showModal) {
        wasOpen = true;
    }

	function handleConfirm() {
		if (!tablePath) {
			console.error("[HeaderConfirmationModal] Cannot confirm: tablePath is missing.");
			handleCancelClick();
			return;
		}

		const schema = {};
		fields.forEach(f => {
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

        confirmed = true;
		dispatch('confirm', { hasHeaders, schema });
        showModal = false; // Trigger close.
	}

	function handleModalClose() {
        // Reset states for the next open.
        step = 1;
		fields = [];
        confirmed = false;
        wasOpen = false;
	}

	function handleCancelClick() {
        dispatch('cancel');
		showModal = false; // Will trigger `on:close` natively.
	}
</script>

<Modal bind:open={showModal} size="xl" outsideclose on:close={handleModalClose} dismissable={false} class="w-full p-0 overflow-hidden flex flex-col max-h-[90vh] z-[130]">
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <Info size={20} class="text-blue-600 dark:text-blue-400" />
        </div>
        <div>
            <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                {step === 1 ? 'Confirm Import Headers' : 'Define Field Types and Validations'}
            </h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">
                {step === 1 ? 'Step 1 of 2: Header detection' : 'Step 2 of 2: Schema definition'}
            </p>
        </div>
    </div>

    <div class="flex-1 overflow-auto -m-6 p-6">
        {#if step === 1}
            <Alert color="blue" class="mb-6">
                Confirm if the first row contains headers. Review the preview below.
            </Alert>

            <div class="border border-gray-200 dark:border-gray-700 rounded-xl overflow-hidden mb-6 bg-white dark:bg-gray-800 shadow-sm">
                <Table hoverable={true} shadow={false}>
                    <TableHead class="bg-gray-50 dark:bg-gray-900/50">
                        {#each availableFields as f, i}
                            <TableHeadCell>
                                {#if hasHeaders && previewData.data && previewData.data[0] && previewData.data[0][f] != null && String(previewData.data[0][f]).trim() !== ''}
                                    {String(previewData.data[0][f]).trim()}
                                {:else}
                                    Field {i + 1}
                                {/if}
                            </TableHeadCell>
                        {/each}
                    </TableHead>
                    <TableBody>
                        {#each (hasHeaders ? previewData.data.slice(1, 4) : previewData.data.slice(0, 3)) as row}
                            <TableBodyRow>
                                {#each availableFields as header}
                                    <TableBodyCell class="truncate max-w-[150px]">
                                        {row[header] != null ? row[header] : ''}
                                    </TableBodyCell>
                                {/each}
                            </TableBodyRow>
                        {/each}
                    </TableBody>
                </Table>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
                <!-- Option 1: Yes -->
                <div class="p-4 border rounded-xl cursor-pointer transition-all {hasHeaders ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-700'}" on:click={() => hasHeaders = true}>
                    <div class="flex">
                        <div class="flex items-center h-5">
                            <input id="headers-yes" type="radio" bind:group={hasHeaders} value={true} class="w-4 h-4 text-neutral-primary border-default-medium bg-neutral-secondary-medium rounded-full checked:border-brand focus:ring-2 focus:outline-none focus:ring-brand-subtle border border-default appearance-none">
                        </div>
                        <div class="ms-2 text-sm select-none">
                            <label for="headers-yes" class="font-medium text-heading mb-1">Yes, the first row is the header.</label>
                            <p class="text-xs font-normal text-body">Use values from the first row as field names.</p>
                        </div>
                    </div>
                </div>
                <!-- Option 2: No -->
                <div class="p-4 border rounded-xl cursor-pointer transition-all {!hasHeaders ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-700'}" on:click={() => hasHeaders = false}>
                    <div class="flex">
                        <div class="flex items-center h-5">
                            <input id="headers-no" type="radio" bind:group={hasHeaders} value={false} class="w-4 h-4 text-neutral-primary border-default-medium bg-neutral-secondary-medium rounded-full checked:border-brand focus:ring-2 focus:outline-none focus:ring-brand-subtle border border-default appearance-none">
                        </div>
                        <div class="ms-2 text-sm select-none">
                            <label for="headers-no" class="font-medium text-heading mb-1">No, treat the first row as data.</label>
                            <p class="text-xs font-normal text-body">Generate generic field names (Field 1, Field 2, etc.).</p>
                        </div>
                    </div>
                </div>
            </div>
        {:else}
            <div class="border border-gray-200 dark:border-gray-700 rounded-xl overflow-hidden bg-white dark:bg-gray-800 shadow-sm h-full">
                <Table hoverable={true} shadow={false} class="h-full">
                    <TableHead class="bg-gray-50 dark:bg-gray-900/50 sticky top-0 z-10">
                        <TableHeadCell class="w-[150px]">Field</TableHeadCell>
                        <TableHeadCell>Type</TableHeadCell>
                        <TableHeadCell>Sub-type</TableHeadCell>
                        <TableHeadCell class="text-center">Primary</TableHeadCell>
                        <TableHeadCell class="text-center">Req?</TableHeadCell>
                        <TableHeadCell class="min-w-[150px]">Options / Constraints</TableHeadCell>
                        <TableHeadCell class="min-w-[200px]">Description</TableHeadCell>
                    </TableHead>
                    <TableBody>
                        {#each fields as field, i}
                            <TableBodyRow>
                                <TableBodyCell class="font-bold truncate max-w-[150px]">{field.name}</TableBodyCell>
                                <TableBodyCell>
                                    <Select size="sm" bind:value={field.type} items={Object.keys(FIELD_TYPES).map(t => ({name: t, value: t}))} on:change={() => handleTypeChange(i)} />
                                </TableBodyCell>
                                <TableBodyCell>
                                    <Select size="sm" bind:value={field.subType} items={FIELD_TYPES[field.type].map(st => ({name: st, value: st}))} on:change={() => handleSubTypeChange(i)} />
                                </TableBodyCell>
                                <TableBodyCell class="text-center">
                                    <Checkbox bind:checked={field.primary} on:change={() => handlePrimaryChange(i)} />
                                </TableBodyCell>
                                <TableBodyCell class="text-center">
                                    <Checkbox bind:checked={field.required} disabled={field.primary} />
                                </TableBodyCell>
                                <TableBodyCell>
                                    {#if field.subType === 'Selectbox' || field.subType === 'Multiselect'}
                                        <Input size="sm" bind:value={field.options} placeholder="Opt 1, Opt 2..." autocomplete="off" autocorrect="off" />
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
                                                        <Input size="sm" type="text" bind:value={field.customCurrency} placeholder="ISO" maxlength="3" autocomplete="off" autocorrect="off" />
                                                    {/if}
                                                </div>
                                            {/if}
                                        </div>
                                    {:else if field.type === 'DateTime'}
                                        <Select size="sm" bind:value={field.format} items={(DATETIME_FORMATS[field.subType] || DATETIME_FORMATS['Date']).map(fmt => ({name: fmt, value: fmt}))} />
                                    {/if}
                                </TableBodyCell>
                                <TableBodyCell>
                                    <Input size="sm" bind:value={field.description} placeholder="Tooltip text" autocomplete="off" autocorrect="off" />
                                </TableBodyCell>
                            </TableBodyRow>
                        {/each}
                    </TableBody>
                </Table>
            </div>
        {/if}
    </div>

    <!-- Footer -->
    <div slot="footer" class="flex justify-between w-full">
        <div>
            {#if step === 2}
                <Button color="alternative" on:click={() => step = 1}>Back</Button>
            {/if}
        </div>
        <div class="flex gap-3">
            <Button color="alternative" on:click={handleCancelClick}>Cancel</Button>
            <Button color="blue" on:click={step === 1 ? goToStep2 : handleConfirm}>
                {step === 1 ? 'Next: Define Field Types' : 'Confirm and Import'}
            </Button>
        </div>
    </div>
</Modal>
