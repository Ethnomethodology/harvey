<!-- src/lib/components/projectview/modals/HeaderConfirmationModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let previewData = { fields: [], headers: [], data: [] };
	export let tablePath = '';

	const dispatch = createEventDispatcher();

	let step = 1; // 1: Header Confirmation, 2: Schema Definition
	let hasHeaders = true;
	let fields = []; // { name, type, subType, options, required, min, max, description, format, currency, customCurrency }

	const FIELD_TYPES = {
		'Text': ['Small Text', 'Long Text'],
		'Numeric': ['Number', 'Currency', 'Percent'],
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
	}

	function handleSubTypeChange(index) {
		fields[index].format = 'None';
        if (fields[index].subType === 'Currency') {
            fields[index].currency = fields[index].currency || 'USD';
        } else {
            delete fields[index].currency;
        }
	}

	function goToStep2() {
		step = 2;
	}

	function handleConfirm() {
		if (!tablePath) {
			console.error("[HeaderConfirmationModal] Cannot confirm: tablePath is missing.");
			closeModal();
			return;
		}

		const schema = {};
		fields.forEach(f => {
			schema[f.name] = {
				type: f.type,
				subType: f.subType,
				options: (f.subType === 'Selectbox' || f.subType === 'Multiselect') ? f.options.split(',').map(o => o.trim()).filter(o => o !== '') : [],
                required: f.required,
                min: f.min !== '' ? parseFloat(f.min) : null,
                max: f.max !== '' ? parseFloat(f.max) : null,
                description: f.description.trim(),
				format: f.format !== 'None' ? f.format : null
			};
            if (f.subType === 'Currency') {
                schema[f.name].currency = f.currency === 'OTHER' ? (f.customCurrency || 'XXX').toUpperCase().substring(0, 3) : f.currency;
            }
		});

		dispatch('confirm', { hasHeaders, schema });
		closeModal();
	}

	function closeModal() {
		step = 1;
		fields = [];
		showModal = false;
		dispatch('close');
	}

	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			closeModal();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[130] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm p-4"
		transition:fade={{ duration: 150 }}
		on:click|self={closeModal}
		role="dialog"
		aria-modal="true"
	>
		<div
			class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-6xl flex flex-col text-gray-800 dark:text-gray-200 max-h-[90vh]"
			on:click|stopPropagation
		>
			<h2 class="text-xl font-bold mb-4">
				{step === 1 ? 'Confirm Import Headers' : 'Define Field Types and Validations'}
			</h2>

			{#if step === 1}
				<p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
					Confirm if the first row contains headers. Review the preview below.
				</p>

				<div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-x-auto mb-4 bg-gray-50 dark:bg-gray-800">
					<table class="w-full text-xs text-left">
						<thead class="bg-gray-100 dark:bg-gray-700 sticky top-0">
							<tr>
								{#each availableFields as f, i}
									<th class="px-3 py-2 font-semibold">
										{#if hasHeaders && previewData.data && previewData.data[0] && previewData.data[0][f] != null && String(previewData.data[0][f]).trim() !== ''}
                                            {String(previewData.data[0][f]).trim()}
                                        {:else}
                                            Field {i + 1}
                                        {/if}
									</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each (hasHeaders ? previewData.data.slice(1, 4) : previewData.data.slice(0, 3)) as row}
								<tr class="border-t border-gray-200 dark:border-gray-700">
									{#each availableFields as header}
										<td class="px-3 py-2 whitespace-nowrap truncate max-w-[150px]">
											{row[header] != null ? row[header] : ''}
										</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="space-y-2 mb-6">
					<label class="flex items-center p-3 border rounded-lg cursor-pointer {hasHeaders ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-700'}">
						<input type="radio" bind:group={hasHeaders} value={true} class="h-4 w-4 text-blue-600">
						<div class="ml-3">
							<p class="text-sm font-medium">Yes, the first row is the header.</p>
						</div>
					</label>
					<label class="flex items-center p-3 border rounded-lg cursor-pointer {!hasHeaders ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-700'}">
						<input type="radio" bind:group={hasHeaders} value={false} class="h-4 w-4 text-blue-600">
						<div class="ml-3">
							<p class="text-sm font-medium">No, treat the first row as data.</p>
						</div>
					</label>
				</div>
			{:else}
				<div class="flex-1 overflow-y-auto mb-4 border dark:border-gray-700 rounded">
					<table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
						<thead class="bg-gray-50 dark:bg-gray-800 sticky top-0 z-10">
							<tr>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Field</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Type</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Sub-type</th>
                                <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">Req?</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-[100px] w-[150px]">Options / Constraints</th>
                                <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider min-w-[200px]">Description</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
							{#each fields as field, i}
								<tr>
									<td class="px-3 py-2 text-sm font-medium truncate max-w-[150px]">{field.name}</td>
									<td class="px-3 py-2">
										<select bind:value={field.type} on:change={() => handleTypeChange(i)} class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
											{#each Object.keys(FIELD_TYPES) as type}<option value={type}>{type}</option>{/each}
										</select>
									</td>
									<td class="px-3 py-2">
										<select bind:value={field.subType} on:change={() => handleSubTypeChange(i)} class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
											{#each FIELD_TYPES[field.type] as sub}<option value={sub}>{sub}</option>{/each}
										</select>
									</td>
                                    <td class="px-3 py-2 text-center">
                                        <input type="checkbox" bind:checked={field.required} class="h-4 w-4 text-blue-600" />
                                    </td>
									<td class="px-3 py-2">
										{#if field.subType === 'Selectbox' || field.subType === 'Multiselect'}
											<input type="text" bind:value={field.options} placeholder="Options (comma separated)" class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
										{:else if field.type === 'Numeric'}
                                            <div class="flex flex-col space-y-1">
                                                <div class="flex space-x-1">
                                                    <input type="number" bind:value={field.min} placeholder="Min" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600" />
                                                    <input type="number" bind:value={field.max} placeholder="Max" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600" />
                                                </div>
                                                {#if field.subType === 'Currency'}
                                                    <div class="flex flex-col space-y-1">
                                                        <select bind:value={field.currency} class="w-full text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600">
                                                            {#each currencyOptions as opt}
                                                                <option value={opt.value}>{opt.label}</option>
                                                            {/each}
                                                        </select>
                                                        {#if field.currency === 'OTHER'}
                                                            <input 
                                                                type="text" 
                                                                bind:value={field.customCurrency} 
                                                                placeholder="Code (e.g. BTC)" 
                                                                maxlength="3"
                                                                class="w-full text-[10px] p-1 rounded border dark:bg-gray-700 dark:border-gray-600" 
                                                            />
                                                        {/if}
                                                    </div>
                                                {/if}
                                            </div>
										{:else if field.type === 'DateTime'}
											<select bind:value={field.format} class="w-full text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600">
												{#each DATETIME_FORMATS[field.subType] as fmt}<option value={fmt}>{fmt}</option>{/each}
											</select>
                                        {/if}
									</td>
                                    <td class="px-3 py-2">
                                        <input type="text" bind:value={field.description} placeholder="Tooltip text" class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
                                    </td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}

			<div class="flex justify-between mt-auto pt-4 border-t dark:border-gray-700">
				<div>
					{#if step === 2}
						<button on:click={() => step = 1} class="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:opacity-80">Back</button>
					{/if}
				</div>
				<div class="space-x-2">
					<button on:click={closeModal} class="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-700 rounded hover:opacity-80">Cancel</button>
					<button on:click={step === 1 ? goToStep2 : handleConfirm} class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700">
						{step === 1 ? 'Next: Define Field Types' : 'Confirm and Import'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
