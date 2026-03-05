<!-- src/lib/components/projectview/modals/HeaderConfirmationModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let previewData = { fields: [], data: [] };
	export let tablePath = '';

	const dispatch = createEventDispatcher();

	let step = 1; // 1: Header Confirmation, 2: Schema Definition
	let hasHeaders = true;
	let fields = []; // { name, type, subType, options, required, min, max, description }

	const FIELD_TYPES = {
		'Text': ['Small Text', 'Long Text'],
		'Numeric': ['Number', 'Currency', 'Percent'],
		'DateTime': ['Date', 'Date & Time', 'Time'],
		'Contact': ['Email', 'Phone', 'Hyperlink'],
		'Misc': ['Selectbox', 'Checkbox', 'Tags', 'Project Link']
	};

	$: if (showModal && previewData && previewData.fields && fields.length === 0) {
		fields = previewData.fields.map((f, i) => ({
			name: hasHeaders ? f : `Field ${i + 1}`,
			type: 'Text',
			subType: 'Small Text',
			options: '',
            required: false,
            min: '',
            max: '',
            description: ''
		}));
	}

	// Update field names if hasHeaders changes
	$: if (step === 1) {
		fields = previewData.fields.map((f, i) => ({
			...fields[i],
			name: hasHeaders ? f : `Field ${i + 1}`
		}));
	}

	function handleTypeChange(index) {
		const type = fields[index].type;
		fields[index].subType = FIELD_TYPES[type][0];
        if (type !== 'Numeric') {
            fields[index].min = '';
            fields[index].max = '';
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
				options: (f.subType === 'Selectbox' || f.subType === 'Tags') ? f.options.split(',').map(o => o.trim()).filter(o => o !== '') : [],
                required: f.required,
                min: f.min !== '' ? parseFloat(f.min) : null,
                max: f.max !== '' ? parseFloat(f.max) : null,
                description: f.description.trim()
			};
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
								{#each previewData.fields as _, i}
									<th class="px-3 py-2 font-semibold">
										{hasHeaders ? previewData.fields[i] : `Field ${i + 1}`}
									</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each previewData.data.slice(0, 3) as row}
								<tr class="border-t border-gray-200 dark:border-gray-700">
									{#each previewData.fields as header}
										<td class="px-3 py-2 whitespace-nowrap truncate max-w-[150px]">
											{row[header]}
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
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">Field</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">Type</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">Sub-type</th>
                                <th class="px-3 py-2 text-center text-xs font-medium text-gray-500 uppercase">Req?</th>
								<th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">Options / Constraints</th>
                                <th class="px-3 py-2 text-left text-xs font-medium text-gray-500 uppercase">Description</th>
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
										<select bind:value={field.subType} class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
											{#each FIELD_TYPES[field.type] as sub}<option value={sub}>{sub}</option>{/each}
										</select>
									</td>
                                    <td class="px-3 py-2 text-center">
                                        <input type="checkbox" bind:checked={field.required} class="h-4 w-4 text-blue-600" />
                                    </td>
									<td class="px-3 py-2">
										{#if field.subType === 'Selectbox' || field.subType === 'Tags'}
											<input type="text" bind:value={field.options} placeholder="Options (comma separated)" class="text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600 w-full">
										{:else if field.type === 'Numeric'}
                                            <div class="flex space-x-1">
                                                <input type="number" bind:value={field.min} placeholder="Min" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600" />
                                                <input type="number" bind:value={field.max} placeholder="Max" class="w-1/2 text-xs p-1 rounded border dark:bg-gray-700 dark:border-gray-600" />
                                            </div>
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
