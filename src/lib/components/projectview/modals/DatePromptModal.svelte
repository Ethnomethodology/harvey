<!-- src/lib/components/projectview/modals/DatePromptModal.svelte -->
<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { 
        Modal, 
        Button, 
        Label, 
        Toggle, 
        Select,
        Dropdown,
        Input
    } from 'flowbite-svelte';
    import { 
        CalendarDays, 
        Clock, 
        Settings2,
        X,
        CheckCircle2,
        Trash2
    } from '@lucide/svelte';
    import { Datepicker } from 'flowbite-datepicker';

    export let showModal = false;
    export let initialDate = null; // Date object or ISO string
    export let initialFormat = 'YYYY-MM-DD';
    export let initialShowTime = false;
    export let initialTimeFormat = 'HH:mm';
    export let isEditing = false;
    export let initialInsertAsText = false;

    const dispatch = createEventDispatcher();

    let dateValue = initialDate ? new Date(initialDate) : new Date();
    let format = initialFormat || 'YYYY-MM-DD';
    let showTime = initialShowTime || false;
    let timeFormat = initialTimeFormat || 'HH:mm';
    let insertAsText = initialInsertAsText;

    const dateFormats = [
        { name: 'YYYY-MM-DD (2024-03-22)', value: 'YYYY-MM-DD' },
        { name: 'DD/MM/YYYY (22/03/2024)', value: 'DD/MM/YYYY' },
        { name: 'MM/DD/YYYY (03/22/2024)', value: 'MM/DD/YYYY' },
        { name: 'MMMM DD, YYYY (March 22, 2024)', value: 'MMMM DD, YYYY' },
        { name: 'MMM DD, YYYY (Mar 22, 2024)', value: 'MMM DD, YYYY' }
    ];

    const timeFormats = [
        { name: '24 Hour (14:30)', value: 'HH:mm' },
        { name: '24 Hour w/ Secs (14:30:15)', value: 'HH:mm:ss' },
        { name: '12 Hour (02:30 PM)', value: 'hh:mm A' }
    ];

    function pad(n) { return String(n).padStart(2, '0'); }

    function formatDate(d, fmt, st, tfmt) {
        if (!(d instanceof Date) || isNaN(d)) return '';
        
        const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        const shortMonths = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        
        let result = '';
        if (fmt === 'YYYY-MM-DD') result = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
        else if (fmt === 'DD/MM/YYYY') result = `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
        else if (fmt === 'MM/DD/YYYY') result = `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${d.getFullYear()}`;
        else if (fmt === 'MMMM DD, YYYY') result = `${months[d.getMonth()]} ${pad(d.getDate())}, ${d.getFullYear()}`;
        else if (fmt === 'MMM DD, YYYY') result = `${shortMonths[d.getMonth()]} ${pad(d.getDate())}, ${d.getFullYear()}`;
        else result = d.toLocaleDateString();

        if (st) {
            const h = d.getHours();
            const m = pad(d.getMinutes());
            const s = pad(d.getSeconds());
            let tStr = '';
            
            if (tfmt === 'HH:mm:ss') tStr = `${pad(h)}:${m}:${s}`;
            else if (tfmt === 'hh:mm A') {
                const displayH = h % 12 || 12;
                const ampm = h >= 12 ? 'PM' : 'AM';
                tStr = `${pad(displayH)}:${m} ${ampm}`;
            } else {
                tStr = `${pad(h)}:${m}`;
            }
            const separator = (fmt.includes('MMM')) ? ' at ' : ', ';
            result += `${separator}${tStr}`;
        }
        return result;
    }

    function handleConfirm() {
        const displayValue = formatDate(dateValue, format, showTime, timeFormat);
        dispatch('confirm', {
            date: dateValue.toISOString(),
            format,
            showTime,
            timeFormat,
            displayValue,
            insertAsText
        });
        showModal = false;
    }

    // Datepicker integration
    function flowbiteDatepicker(node) {
        let picker = null;

        const destroyPicker = () => {
            if (picker) {
                picker.destroy();
                picker = null;
            }
        };

        const initPicker = () => {
            if (picker) return;
            picker = new Datepicker(node, {
                autohide: true,
                format: 'yyyy-mm-dd',
                todayBtn: true,
                clearBtn: false,
                container: 'body'
            });
            picker.show();
        };

        const handleChange = (e) => {
            if (!picker) return;
            const d = picker.getDate();
            if (d) {
                // Preserve time when updating date
                const newD = new Date(d);
                newD.setHours(dateValue.getHours(), dateValue.getMinutes(), dateValue.getSeconds());
                dateValue = newD;
                destroyPicker();
                node.blur();
            }
        };

        const handleOutside = (event) => {
            if (!picker) return;
            const isClickInsideInput = node.contains(event.target) || node === event.target;
            
            let isClickInsidePicker = false;
            if (event.target instanceof Element) {
                isClickInsidePicker = event.target.closest('.datepicker-dropdown') || event.target.closest('.datepicker');
            }

            if (!isClickInsideInput && !isClickInsidePicker) {
                destroyPicker();
                node.blur();
            }
        };

        node.addEventListener('focus', initPicker);
        node.addEventListener('click', initPicker);
        node.addEventListener('changeDate', handleChange);
        document.addEventListener('mousedown', handleOutside, true);

        return {
            destroy() {
                node.removeEventListener('focus', initPicker);
                node.removeEventListener('click', initPicker);
                node.removeEventListener('changeDate', handleChange);
                document.removeEventListener('mousedown', handleOutside, true);
                destroyPicker();
            }
        };
    }

    const hours = Array.from({ length: 24 }, (_, i) => i.toString().padStart(2, '0'));
    const minutes = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));
    const seconds = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

    function selectTimePart(part, val) {
        const newD = new Date(dateValue);
        if (part === 'h') newD.setHours(parseInt(val));
        if (part === 'm') newD.setMinutes(parseInt(val));
        if (part === 's') newD.setSeconds(parseInt(val));
        dateValue = newD;
    }

    $: previewText = formatDate(dateValue, format, showTime, timeFormat);
</script>

<Modal 
    bind:open={showModal} 
    size="xs" 
    autoclose={false} 
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
>
    <div slot="header" class="flex items-center gap-2">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <CalendarDays class="w-5 h-5 text-blue-600 dark:text-blue-400" />
        </div>
        <div class="flex flex-col">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white leading-tight">Insert Date</h3>
            <p class="text-xs text-gray-500 dark:text-gray-400">Configure your dynamic date</p>
        </div>
    </div>

    <div class="space-y-5 py-2">
        <!-- Date Picker Section -->
        <div class="space-y-2">
            <Label class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
                <CalendarDays size={14} class="text-gray-400" />
                Select Date
            </Label>
            <div class="relative">
                <div class="absolute inset-y-0 start-0 flex items-center ps-3.5 pointer-events-none">
                    <CalendarDays class="w-4 h-4 text-gray-500" />
                </div>
                <input 
                    use:flowbiteDatepicker
                    type="text" 
                    readonly
                    value={dateValue.toISOString().split('T')[0]}
                    class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full ps-10 p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:text-white cursor-pointer"
                    placeholder="Select date"
                />
            </div>
        </div>

        <!-- Format Selection -->
        <div class="space-y-2">
            <Label class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
                <Settings2 size={14} class="text-gray-400" />
                Date Format
            </Label>
            <Select items={dateFormats} bind:value={format} class="bg-gray-50" />
        </div>

        <!-- Time Toggle -->
        <div class="pt-2 flex flex-col gap-3">
            <Toggle bind:checked={showTime} class="text-sm font-medium text-gray-700 dark:text-gray-300">
                Include Time
            </Toggle>
            {#if !isEditing}
            <Toggle bind:checked={insertAsText} class="text-sm font-medium text-gray-700 dark:text-gray-300">
                Insert as Text
            </Toggle>
            {/if}
        </div>

        {#if showTime}
            <div class="space-y-4 animate-in fade-in slide-in-from-top-2 duration-200">
                <div class="grid grid-cols-2 gap-3 items-end">
                    <div class="space-y-2">
                        <Label class="text-xs font-bold uppercase tracking-wider text-gray-500 flex items-center gap-2">
                            <Clock size={14} class="text-gray-400" />
                            Select Time
                        </Label>
                        <div class="relative">
                            <Input 
                                id="time_picker_input"
                                value={`${pad(dateValue.getHours())}:${pad(dateValue.getMinutes())}${timeFormat.includes(':ss') ? ':' + pad(dateValue.getSeconds()) : ''}`}
                                readonly
                                class="cursor-pointer bg-gray-50 ps-10"
                            >
                                <Clock slot="left" size={16} class="text-gray-400" />
                            </Input>
                            <Dropdown triggeredBy="#time_picker_input" class="w-48 p-0 z-[10002]" strategy="fixed">
                                <div class="flex h-48 border dark:border-gray-700 rounded-lg shadow-xl overflow-hidden bg-white dark:bg-gray-800">
                                    <div class="flex-1 overflow-y-auto scrollbar-hide">
                                        {#each hours as h}
                                            <button 
                                                class="w-full py-1.5 text-xs transition-colors hover:bg-blue-50 dark:hover:bg-blue-900/20 {pad(dateValue.getHours()) === h ? 'bg-blue-500 text-white font-bold' : ''}"
                                                on:click={() => selectTimePart('h', h)}
                                            >{h}</button>
                                        {/each}
                                    </div>
                                    <div class="w-px bg-gray-100 dark:bg-gray-700"></div>
                                    <div class="flex-1 overflow-y-auto scrollbar-hide">
                                        {#each minutes as m}
                                            <button 
                                                class="w-full py-1.5 text-xs transition-colors hover:bg-blue-50 dark:hover:bg-blue-900/20 {pad(dateValue.getMinutes()) === m ? 'bg-blue-500 text-white font-bold' : ''}"
                                                on:click={() => selectTimePart('m', m)}
                                            >{m}</button>
                                        {/each}
                                    </div>
                                    {#if timeFormat.includes(':ss')}
                                        <div class="w-px bg-gray-100 dark:bg-gray-700"></div>
                                        <div class="flex-1 overflow-y-auto scrollbar-hide">
                                            {#each seconds as s}
                                                <button 
                                                    class="w-full py-1.5 text-xs transition-colors hover:bg-blue-50 dark:hover:bg-blue-900/20 {pad(dateValue.getSeconds()) === s ? 'bg-blue-500 text-white font-bold' : ''}"
                                                    on:click={() => selectTimePart('s', s)}
                                                >{s}</button>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                            </Dropdown>
                        </div>
                    </div>
                    <div class="space-y-2">
                        <Label class="text-xs font-bold uppercase tracking-wider text-gray-500">Time Format</Label>
                        <Select items={timeFormats} bind:value={timeFormat} class="bg-gray-50" />
                    </div>
                </div>
            </div>
        {/if}

        <!-- Preview Result -->
        <div class="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl border border-blue-100 dark:border-blue-800/50">
            <div class="flex items-center justify-between mb-2">
                <span class="text-[10px] font-extrabold uppercase tracking-widest text-blue-600/60">Preview</span>
                <CheckCircle2 size={14} class="text-blue-500" />
            </div>
            <div class="text-sm font-semibold text-blue-800 dark:text-blue-300">
                {previewText}
            </div>
        </div>
    </div>

    <svelte:fragment slot="footer">
        {#if isEditing}
            <Button color="red" outline class="px-3" on:click={() => { showModal = false; dispatch('delete'); }} title="Delete Date Node">
                <Trash2 size={16} class="mr-2" /> Delete
            </Button>
        {/if}
        <div class="flex space-x-3">
            <Button color="alternative" on:click={() => { showModal = false; dispatch('cancel'); }}>
                Cancel
            </Button>
            <Button color="blue" on:click={handleConfirm}>
                Confirm
            </Button>
        </div>
    </svelte:fragment>
</Modal>

<style>
    :global(.datepicker) {
        z-index: 100000 !important;
    }

    .scrollbar-hide::-webkit-scrollbar {
        display: none;
    }
    .scrollbar-hide {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
