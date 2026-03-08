<!-- src/lib/components/projectview/modals/EditEntryModal.svelte -->
<script>
    import { createEventDispatcher, onMount, tick } from 'svelte';
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
        AlertCircle,
        Calendar,
        Star
    } from 'lucide-svelte';
    import ProgressIcon from '$lib/components/projectview/data/tables/icons/ProgressIcon.svelte';
    import { 
        Input, 
        Label, 
        Select, 
        Textarea, 
        Button, 
        Helper,
        MultiSelect,
        Dropdown
    } from 'flowbite-svelte';
    import { Datepicker } from 'flowbite-datepicker';

    export let rowData = {};
    export let columns = [];
    export let schema = {};
    export let rowIndex = 0;

    const dispatch = createEventDispatcher();
    let modalInner;

    function sanitizeId(id) {
        return String(id).replace(/[^a-zA-Z0-9]/g, '_');
    }

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
        }
        if (type === 'Text' && subType === 'Long Text') return TextInitial;
        return Type;
    }

    let editedData = { ...rowData };
    for (const field in schema) {
        if (schema[field].type === 'Misc' && schema[field].subType === 'Multiselect' && typeof editedData[field] === 'string') {
            editedData[field] = editedData[field].split(',').map(s => s.trim()).filter(Boolean);
        }
        if (schema[field].type === 'Numeric' && schema[field].subType === 'Progress') {
            if (editedData[field] === undefined || editedData[field] === null || editedData[field] === "") {
                editedData[field] = schema[field].min ?? 0;
            }
        }
    }

    let errors = {};

    function validateField(field, value) {
        const colSchema = schema[field];
        if (!colSchema) return null;
        if (colSchema.required && (value === null || value === undefined || value === "")) return "Field is required";
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

    function handleDateTimeChange(field, type, val) {
        let currentVal = editedData[field] || "";
        let datePart = "2026-03-07"; 
        let timePart = "00:00";

        if (currentVal.includes('T')) {
            [datePart, timePart] = currentVal.split('T');
        } else if (currentVal.includes(' ')) {
             [datePart, timePart] = currentVal.split(' ');
        } else if (/^\d{4}-\d{2}-\d{2}$/.test(currentVal)) {
            datePart = currentVal;
        }

        if (type === 'date') datePart = val;
        if (type === 'time') timePart = val;

        if (datePart) {
            editedData[field] = `${datePart}T${timePart || "00:00"}`;
        } else {
            editedData[field] = "";
        }
    }

    function flowbiteDatepicker(node, { field, isDateTime = false }) {
        let picker = null;

        const initPicker = () => {
            if (picker) return;
            picker = new Datepicker(node, {
                format: 'yyyy-mm-dd',
                autohide: true,
                orientation: 'auto',
                todayBtn: true,
                clearBtn: true,
                container: 'body'
            });
            // Ensure picker shows immediately when initialized
            picker.show();
        };

        const destroyPicker = () => {
            if (picker) {
                picker.hide();
                picker.destroy();
                picker = null;
            }
        };

        const handleChange = (e) => {
            if (!picker) return;
            const dateStr = picker.getDate('yyyy-mm-dd');
            if (isDateTime) {
                handleDateTimeChange(field, 'date', dateStr);
            } else {
                editedData[field] = dateStr;
            }
            // Auto-hide will hide it, but we can also destroy it
            destroyPicker();
            node.blur();
        };

        const handleOutsideClick = (event) => {
            if (!picker) return;
            const isClickInsideInput = node.contains(event.target) || node === event.target;
            
            let isClickInsidePicker = false;
            if (event.target instanceof Element) {
                isClickInsidePicker = event.target.closest('.datepicker-dropdown') || event.target.closest('.datepicker');
            }

            if (!isClickInsideInput && !isClickInsidePicker) {
                destroyPicker();
                node.blur(); // Ensure the input loses focus so clicking it again reopens the picker
            }
        };

        node.addEventListener('focus', initPicker);
        node.addEventListener('click', initPicker);
        node.addEventListener('changeDate', handleChange);
        document.addEventListener('mousedown', handleOutsideClick, true);

        return {
            destroy() {
                node.removeEventListener('focus', initPicker);
                node.removeEventListener('click', initPicker);
                node.removeEventListener('changeDate', handleChange);
                document.removeEventListener('mousedown', handleOutsideClick, true);
                destroyPicker();
            }
        };
    }

    // Rolling Timepicker constants
    const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
    const minutes = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

    function selectTimePart(field, part, val, isDateTime = false) {
        let currentVal = (isDateTime ? (editedData[field] || "").split('T')[1] : editedData[field]) || "00:00";
        let [h, m] = currentVal.split(':');
        if (part === 'h') h = val;
        if (part === 'm') m = val;
        const newTime = `${h}:${m}`;

        if (isDateTime) {
            handleDateTimeChange(field, 'time', newTime);
        } else {
            editedData[field] = newTime;
        }
    }
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-[100] p-4 backdrop-blur-sm" on:click|self={() => dispatch('cancel')}>
    <div bind:this={modalInner} class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden">
        <!-- Header -->
        <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                    <Pencil size={20} class="text-blue-600 dark:text-blue-400" />
                </div>
                <div>
                    <h3 class="text-lg font-bold text-gray-900 dark:text-white">Edit Entry</h3>
                    <p class="text-xs text-gray-500 dark:text-gray-400">Update the fields below</p>
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
                        <Label class="mb-2 flex items-center gap-2">
                            <svelte:component this={getSubtypeIcon(colSchema)} size={14} class="text-gray-500 dark:text-gray-400" />
                            {col.field}
                            {#if colSchema.required}<span class="text-red-500">*</span>{/if}
                        </Label>

                        {#if colSchema.type === 'DateTime'}
                            <div class="space-y-3">
                                {#if colSchema.subType === 'Date'}
                                    <div class="relative">
                                        <div class="absolute inset-y-0 start-0 flex items-center ps-3.5 pointer-events-none">
                                            <svg class="w-4 h-4 text-gray-500 dark:text-gray-400" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20"><path d="M20 4a2 2 0 0 0-2-2h-2V1a1 1 0 0 0-2 0v1h-3V1a1 1 0 0 0-2 0v1H6V1a1 1 0 0 0-2 0v1H2a2 2 0 0 0-2 2v2h20V4ZM0 18a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8H0v10Zm5-8h10a1 1 0 0 1 0 2H5a1 1 0 0 1 0-2Z"/></svg>
                                        </div>
                                        <input 
                                            use:flowbiteDatepicker={{field: col.field}}
                                            type="text" 
                                            autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                            value={editedData[col.field] || ""} 
                                            placeholder="Select date"
                                            class="cursor-pointer bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full ps-10 p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500 {errors[col.field] ? 'border-red-500 ring-red-500' : ''}"
                                            on:keydown={(e) => e.preventDefault()}
                                        />
                                    </div>
                                {:else if colSchema.subType === 'Time'}
                                    <div class="relative max-w-[12rem]">
                                        <div class="absolute inset-y-0 end-0 top-0 flex items-center pe-3.5 pointer-events-none">
                                            <svg class="w-4 h-4 text-gray-500 dark:text-gray-400" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 24 24">
                                                <path fill-rule="evenodd" d="M2 12C2 6.477 6.477 2 12 2s10 4.477 10 10-4.477 10-10 10S2 17.523 2 12Zm11-4a1 1 0 1 0-2 0v4a1 1 0 0 0 .293.707l3 3a1 1 0 0 0 1.414-1.414L13 11.586V8Z" clip-rule="evenodd"/>
                                            </svg>
                                        </div>
                                        <input 
                                            id="time_input_{sanitizeId(col.field)}"
                                            type="text" 
                                            autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                            bind:value={editedData[col.field]} 
                                            placeholder="00:00"
                                            class="cursor-pointer bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500 {errors[col.field] ? 'border-red-500' : ''}" 
                                            on:keydown={(e) => e.preventDefault()}
                                        />
                                        <Dropdown triggeredBy="#time_input_{sanitizeId(col.field)}" class="w-24 p-0 z-[110] shadow-2xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                            <div class="flex h-64">
                                                <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                    {#each hours as h}
                                                        <button 
                                                            class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {(editedData[col.field] || '').startsWith(h) ? 'bg-blue-500 text-white font-bold' : 'text-gray-700 dark:text-gray-300'}"
                                                            on:click={() => selectTimePart(col.field, 'h', h)}
                                                        >{h}</button>
                                                    {/each}
                                                </div>
                                                <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                <div class="flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900">
                                                    {#each minutes as m}
                                                        <button 
                                                            class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {(editedData[col.field] || '').endsWith(m) ? 'bg-blue-500 text-white font-bold' : 'text-gray-700 dark:text-gray-300'}"
                                                            on:click={() => selectTimePart(col.field, 'm', m)}
                                                        >{m}</button>
                                                    {/each}
                                                </div>
                                            </div>
                                        </Dropdown>
                                    </div>
                                {:else}
                                    <div class="grid grid-cols-2 gap-4 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700">
                                        <div class="space-y-1.5">
                                            <Label class="text-[10px] font-extrabold uppercase tracking-widest text-gray-400">Date</Label>
                                            <div class="relative">
                                                <div class="absolute inset-y-0 start-0 flex items-center ps-3 pointer-events-none">
                                                    <svg class="w-3.5 h-3.5 text-blue-500" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20"><path d="M20 4a2 2 0 0 0-2-2h-2V1a1 1 0 0 0-2 0v1h-3V1a1 1 0 0 0-2 0v1H6V1a1 1 0 0 0-2 0v1H2a2 2 0 0 0-2 2v2h20V4ZM0 18a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8H0v10Zm5-8h10a1 1 0 0 1 0 2H5a1 1 0 0 1 0-2Z"/></svg>
                                                </div>
                                                <input 
                                                    use:flowbiteDatepicker={{field: col.field, isDateTime: true}}
                                                    type="text" 
                                                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                                    value={(editedData[col.field] || "").split('T')[0] || ""} 
                                                    class="cursor-pointer bg-white border border-gray-300 text-gray-900 text-xs rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full ps-8 p-2 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                    on:keydown={(e) => e.preventDefault()}
                                                />
                                            </div>
                                        </div>
                                        <div class="space-y-1.5">
                                            <Label class="text-[10px] font-extrabold uppercase tracking-widest text-gray-400">Time</Label>
                                            <div class="relative">
                                                <div class="absolute inset-y-0 end-0 top-0 flex items-center pe-2.5 pointer-events-none">
                                                    <svg class="w-3.5 h-3.5 text-blue-500" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 24 24">
                                                        <path fill-rule="evenodd" d="M2 12C2 6.477 6.477 2 12 2s10 4.477 10 10-4.477 10-10 10S2 17.523 2 12Zm11-4a1 1 0 1 0-2 0v4a1 1 0 0 0 .293.707l3 3a1 1 0 0 0 1.414-1.414L13 11.586V8Z" clip-rule="evenodd"/>
                                                    </svg>
                                                </div>
                                                <input 
                                                    id="dt_time_input_{sanitizeId(col.field)}"
                                                    type="text" 
                                                    autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                                    value={(editedData[col.field] || "").split('T')[1] || "00:00"} 
                                                    class="cursor-pointer bg-white border border-gray-300 text-gray-900 text-xs rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2 pe-7 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                    on:keydown={(e) => e.preventDefault()}
                                                />
                                                <Dropdown triggeredBy="#dt_time_input_{sanitizeId(col.field)}" class="w-24 p-0 z-[110] shadow-2xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                                    <div class="flex h-64">
                                                        <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                            {#each hours as h}
                                                                <button 
                                                                    class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {((editedData[col.field] || '').split('T')[1] || '').startsWith(h) ? 'bg-blue-500 text-white font-bold' : 'text-gray-700 dark:text-gray-300'}"
                                                                    on:click={() => selectTimePart(col.field, 'h', h, true)}
                                                                >{h}</button>
                                                            {/each}
                                                        </div>
                                                        <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                        <div class="flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900">
                                                            {#each minutes as m}
                                                                <button 
                                                                    class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {((editedData[col.field] || '').split('T')[1] || '').endsWith(m) ? 'bg-blue-500 text-white font-bold' : 'text-gray-700 dark:text-gray-300'}"
                                                                    on:click={() => selectTimePart(col.field, 'm', m, true)}
                                                                >{m}</button>
                                                            {/each}
                                                        </div>
                                                    </div>
                                                </Dropdown>
                                            </div>
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {:else if colSchema.type === 'Misc'}
                            {#if colSchema.subType === 'Checkbox'}
                                <div class="flex items-center h-10 ps-1">
                                    <input 
                                        type="checkbox" 
                                        bind:checked={editedData[col.field]} 
                                        class="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600" 
                                    />
                                </div>
                            {:else if colSchema.subType === 'Multiselect'}
                                <MultiSelect items={(colSchema.options || []).map(o => ({name: o, value: o}))} bind:value={editedData[col.field]} placeholder="Select options..." />
                            {:else if colSchema.subType === 'Selectbox'}
                                <Select items={[{name: '-- None --', value: ''}, ...(colSchema.options || []).map(o => ({name: o, value: o}))]} bind:value={editedData[col.field]} placeholder="Select option..." color={errors[col.field] ? 'red' : 'base'} />
                            {:else if colSchema.subType === 'Project Link'}
                                <Select items={[{name: '-- None --', value: ''}, ...projectAssets.map(a => ({name: a.label, value: a.value}))]} bind:value={editedData[col.field]} placeholder="Select asset..." color={errors[col.field] ? 'red' : 'base'} />
                            {/if}
                        {:else if colSchema.type === 'Numeric'}
                            {#if colSchema.subType === 'Progress'}
                                {@const min = colSchema.min ?? 0}
                                {@const max = colSchema.max ?? 100}
                                {@const val = editedData[col.field] ?? min}
                                {@const percentage = ((val - min) / (max - min)) * 100}
                                <div class="flex items-center gap-3 h-10">
                                    <input 
                                        type="range" 
                                        {min} {max} step="1" 
                                        bind:value={editedData[col.field]} 
                                        style="background: linear-gradient(to right, #3b82f6 {percentage}%, #e5e7eb {percentage}%);"
                                        title="{val} / {max}"
                                        class="progress-range w-full h-2 rounded-lg appearance-none cursor-pointer dark:bg-gray-700" 
                                    />
                                </div>
                            {:else if colSchema.subType === 'Rating'}
                                <div class="flex items-center gap-1 h-10">
                                    {#each Array(colSchema.max || 5) as _, i}
                                        <button
                                            type="button"
                                            class="focus:outline-none transition-colors"
                                            on:click={() => editedData[col.field] = i + 1}
                                        >
                                            <svg class="w-6 h-6 {(editedData[col.field] || 0) > i ? 'text-yellow-400 dark:text-yellow-300' : 'text-gray-300 dark:text-gray-600'}" viewBox="0 0 24 24" fill="currentColor">
                                                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                            </svg>
                                        </button>
                                    {/each}
                                </div>
                            {:else}
                                <div class="relative group/input">
                                    {#if colSchema.subType === 'Currency'}
                                        <div class="absolute inset-y-0 start-0 flex items-center ps-3.5 pointer-events-none">
                                            <span class="text-gray-500 dark:text-gray-400 font-bold">{getCurrencySymbol(colSchema.currency)}</span>
                                        </div>
                                    {/if}
                                    <input
                                        type="number"
                                        autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                        step="any"
                                        id="field-{col.field}"
                                        bind:value={editedData[col.field]}
                                        class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500 {colSchema.subType === 'Currency' ? 'ps-10' : ''} {colSchema.subType === 'Percent' ? 'pe-10' : ''} {errors[col.field] ? 'border-red-500' : ''}"
                                    />
                                    {#if colSchema.subType === 'Percent'}
                                        <div class="absolute inset-y-0 end-0 flex items-center pe-3.5 pointer-events-none">
                                            <span class="text-gray-500 dark:text-gray-400 font-bold">%</span>
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        {:else if colSchema.type === 'Contact'}
                            <Input type={colSchema.subType === 'Email' ? 'email' : (colSchema.subType === 'Phone' ? 'tel' : 'url')} 
                                id="field-{col.field}" bind:value={editedData[col.field]} 
                                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                color={errors[col.field] ? 'red' : 'base'}>
                                <svelte:fragment slot="left">
                                    {#if colSchema.subType === 'Email'}
                                        <Mail size={18} class="text-gray-400" />
                                    {:else if colSchema.subType === 'Phone'}
                                        <Phone size={18} class="text-gray-400" />
                                    {:else}
                                        <Link2 size={18} class="text-gray-400" />
                                    {/if}
                                </svelte:fragment>
                            </Input>
                        {:else if colSchema.type === 'Text' && colSchema.subType === 'Small Text'}
                            <Input type="text" id="field-{col.field}" bind:value={editedData[col.field]} 
                                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                color={errors[col.field] ? 'red' : 'base'} />
                        {:else}
                            <Textarea id="field-{col.field}" bind:value={editedData[col.field]} rows="3"
                                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
                                color={errors[col.field] ? 'red' : 'base'} class="resize-none" />
                        {/if}

                        {#if errors[col.field]}
                            <Helper color="red" class="mt-2 flex items-center gap-1">
                                <AlertCircle size={12} /> {errors[col.field]}
                            </Helper>
                        {/if}
                        {#if colSchema.description}
                            <Helper class="italic text-gray-400 dark:text-gray-500">{colSchema.description}</Helper>
                        {/if}
                    </div>
                {/if}
            {/each}
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
            <Button color="alternative" on:click={() => dispatch('cancel')}>Cancel</Button>
            <Button color="blue" on:click={handleSave}>Save Changes</Button>
        </div>
    </div>
</div>

<style lang="postcss">
    .custom-scrollbar::-webkit-scrollbar {
        @apply w-1.5;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        @apply bg-transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        @apply bg-gray-300 dark:bg-gray-700 rounded-full;
    }

    /* Target inputs specifically in this modal to hide spin buttons */
    input::-webkit-outer-spin-button,
    input::-webkit-inner-spin-button {
        -webkit-appearance: none;
        margin: 0;
    }
    input[type=number] {
        -moz-appearance: textfield;
    }

    /* Beautiful focus states for the native pickers */
    input[type="date"]:focus, input[type="time"]:focus {
        @apply ring-2 ring-blue-500/20 border-blue-500 outline-none;
    }

    /* Hide the native date icon to let the Flowbite one shine */
    input[type="date"]::-webkit-calendar-picker-indicator {
        @apply opacity-0 absolute inset-0 cursor-pointer;
    }

    /* Progress Editor Styling */
    .progress-range {
        -webkit-appearance: none;
        background: #e5e7eb;
        height: 6px !important;
        border-radius: 3px;
        outline: none;
        margin: 0;
        padding: 0;
    }
    .progress-range::-webkit-slider-thumb {
        -webkit-appearance: none;
        width: 14px;
        height: 14px;
        background: #3b82f6;
        border-radius: 50%;
        cursor: pointer;
        border: 2px solid white;
        box-shadow: 0 0 2px rgba(0,0,0,0.3);
        margin-top: -4px; /* Center thumb on track */
    }
    .progress-range::-moz-range-thumb {
        width: 14px;
        height: 14px;
        background: #3b82f6;
        border-radius: 50%;
        cursor: pointer;
        border: 2px solid white;
        box-shadow: 0 0 2px rgba(0,0,0,0.3);
    }
</style>

