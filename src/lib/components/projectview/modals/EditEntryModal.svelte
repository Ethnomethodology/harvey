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

    function parseDate(str, colSchema) {
        if (!str || typeof str !== 'string') return null;
        const format = colSchema?.format || '';
        const subType = colSchema?.subType || 'Date';

        // Helper to normalize months
        const months = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
        const getMonthIndex = (m) => months.indexOf(m.toLowerCase());

        // Try standard ISO first
        let d = new Date(str);
        if (!isNaN(d.getTime())) return d;

        // Try format-specific parsing
        if (subType === 'Date') {
            if (format === 'DD/MM/YYYY') {
                const p = str.split('/');
                if (p.length === 3) return new Date(p[2], p[1] - 1, p[0]);
            } else if (format === 'MM/DD/YYYY') {
                const p = str.split('/');
                if (p.length === 3) return new Date(p[2], p[0] - 1, p[1]);
            } else if (format === 'YYYY') {
                if (/^\d{4}$/.test(str)) return new Date(str, 0, 1);
            } else if (format === 'MMMM') {
                const idx = getMonthIndex(str);
                if (idx !== -1) return new Date(new Date().getFullYear(), idx, 1);
            } else if (format === 'MMMM YYYY') {
                const p = str.split(' ');
                const idx = getMonthIndex(p[0]);
                if (p.length === 2 && idx !== -1) return new Date(p[1], idx, 1);
            }
        } else if (subType === 'Time') {
            const is12Hour = format.includes('A') || format.includes('a');
            const ampmMatch = str.match(/(AM|PM)/i);
            const ampm = ampmMatch ? ampmMatch[0].toUpperCase() : null;
            const timeParts = str.replace(/(AM|PM)/i, '').trim().split(':');
            
            if (timeParts.length >= 2) {
                let h = parseInt(timeParts[0]);
                const m = parseInt(timeParts[1]);
                const s = parseInt(timeParts[2] || 0);
                
                if (is12Hour && ampm) {
                    if (ampm === 'PM' && h < 12) h += 12;
                    if (ampm === 'AM' && h === 12) h = 0;
                }
                
                const d = new Date();
                d.setHours(h, m, s);
                return d;
            }
        } else if (subType === 'Date & Time') {
            // Improved split: find first T or space that separates date and time
            let dateStr, timeStr;
            if (str.includes('T')) {
                [dateStr, timeStr] = str.split('T');
            } else {
                // For space separator, we assume the date part is the first block
                // (which works for YYYY-MM-DD, DD/MM/YYYY, MM/DD/YYYY)
                const firstSpace = str.indexOf(' ');
                if (firstSpace !== -1) {
                    dateStr = str.substring(0, firstSpace);
                    timeStr = str.substring(firstSpace + 1);
                }
            }

            if (dateStr && timeStr) {
                const dateD = parseDate(dateStr, { type: 'DateTime', subType: 'Date', format: format.split(/[T ]/)[0] });
                const timeD = parseDate(timeStr, { type: 'DateTime', subType: 'Time', format: format.split(/[T ]/).slice(1).join(' ') });
                
                if (dateD && timeD) {
                    dateD.setHours(timeD.getHours(), timeD.getMinutes(), timeD.getSeconds());
                    return dateD;
                }
            }
        }

        return null;
    }

    function formatDate(d, colSchema) {
        if (!(d instanceof Date) || isNaN(d.getTime())) return '';
        const format = colSchema?.format || '';
        const subType = colSchema?.subType || 'Date';

        const pad = (n) => String(n).padStart(2, '0');
        const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

        if (subType === 'Date') {
            if (format === 'DD/MM/YYYY') return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
            if (format === 'MM/DD/YYYY') return `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${d.getFullYear()}`;
            if (format === 'YYYY') return `${d.getFullYear()}`;
            if (format === 'MMMM') return months[d.getMonth()];
            if (format === 'MMMM YYYY') return `${months[d.getMonth()]} ${d.getFullYear()}`;
            return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
        } else if (subType === 'Time') {
            const h = d.getHours();
            const m = pad(d.getMinutes());
            const s = pad(d.getSeconds());
            if (format === 'HH:mm:ss') return `${pad(h)}:${m}:${s}`;
            if (format === 'hh:mm A') {
                const displayH = h % 12 || 12;
                const ampm = h >= 12 ? 'PM' : 'AM';
                return `${pad(displayH)}:${m} ${ampm}`;
            }
            return `${pad(h)}:${m}`;
        } else if (subType === 'Date & Time') {
            const formatParts = format.split(/[T ]/);
            const datePart = formatDate(d, { subType: 'Date', format: formatParts[0] });
            const timePart = formatDate(d, { subType: 'Time', format: formatParts.slice(1).join(' ') || '' });
            if (format.includes('T')) return `${datePart}T${timePart}`;
            return `${datePart} ${timePart}`;
        }
        return '';
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
        
        const type = colSchema.type;
        const subType = colSchema.subType;
        const isBlank = value === null || value === undefined || (typeof value === 'string' && value.trim() === "") || (Array.isArray(value) && value.length === 0);

        if (colSchema.required && isBlank) {
            return "Field is required";
        } 
        
        if (!isBlank) {
            if (type === 'Numeric') {
                const num = parseFloat(value);
                if (isNaN(num) || !isFinite(value)) return "Must be a valid number";
                if (colSchema.min !== null && colSchema.min !== undefined && num < colSchema.min) return `Value must be at least ${colSchema.min}`;
                if (colSchema.max !== null && colSchema.max !== undefined && num > colSchema.max) return `Value must be at most ${colSchema.max}`;
            } else if (type === 'Contact' && subType === 'Email') {
                if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return "Invalid email format";
            } else if (type === 'Contact' && subType === 'Phone') {
                if (!/^\+?[\d\s-]{7,20}$/.test(value)) return "Invalid phone format";
            } else if (type === 'Contact' && subType === 'Hyperlink') {
                const urlRegex = /^(https?:\/\/)?([\da-z\.-]+)\.([a-z\.]{2,6})([\/\w \.-]*)*\/?$/i;
                if (!urlRegex.test(value)) return "Invalid hyperlink format";
            } else if (type === 'DateTime') {
                if (subType === 'Time') {
                    if (colSchema.format === 'HH:mm' && !/^([01]\d|2[0-3]):([0-5]\d)$/.test(value)) return "Invalid time format (HH:mm)";
                    if (colSchema.format === 'HH:mm:ss' && !/^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/.test(value)) return "Invalid time format (HH:mm:ss)";
                    if (colSchema.format === 'hh:mm A' && !/^(0[1-9]|1[0-2]):([0-5]\d)\s?(AM|PM)$/i.test(value)) return "Invalid time format (hh:mm AM/PM)";
                    if (!/^([01]\d|2[0-3]):?([0-5]\d)/.test(value)) return "Invalid time format";
                } else if (subType === 'Date') {
                    if (colSchema.format === 'YYYY-MM-DD' && !/^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(value)) return "Invalid date format (YYYY-MM-DD)";
                    if (colSchema.format === 'DD/MM/YYYY' && !/^(0[1-9]|[12]\d|3[01])\/(0[1-9]|1[0-2])\/\d{4}$/.test(value)) return "Invalid date format (DD/MM/YYYY)";
                    if (colSchema.format === 'MM/DD/YYYY' && !/^(0[1-9]|1[0-2])\/(0[1-9]|[12]\d|3[01])\/\d{4}$/.test(value)) return "Invalid date format (MM/DD/YYYY)";
                    if (colSchema.format === 'YYYY' && !/^\d{4}$/.test(value)) return "Invalid year format (YYYY)";
                    if (colSchema.format === 'MMMM' && !["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"].includes(value.toLowerCase())) return "Invalid month name";
                    if (colSchema.format === 'MMMM YYYY') {
                        const parts = value.split(' ');
                        const valid = parts.length === 2 && ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"].includes(parts[0].toLowerCase()) && /^\d{4}$/.test(parts[1]);
                        if (!valid) return "Invalid Month YYYY format";
                    }
                    if (!parseDate(value, colSchema)) return "Invalid date";
                } else {
                    if (!parseDate(value, colSchema)) return "Invalid date/time";
                }
            } else if (type === 'Misc') {
                if (subType === 'Selectbox' && Array.isArray(colSchema.options)) {
                    if (value && !colSchema.options.includes(value)) return `Value must be one of: ${colSchema.options.join(', ')}`;
                } else if (subType === 'Multiselect' && Array.isArray(colSchema.options)) {
                    const vals = Array.isArray(value) ? value : String(value).split(',').map(s => s.trim()).filter(Boolean);
                    const invalidVals = vals.filter(v => !colSchema.options.includes(v));
                    if (invalidVals.length > 0) return `Invalid options selected: ${invalidVals.join(', ')}`;
                }
            }
        }
        return null;
    }

    // Reactive validation
    $: {
        let newErrors = {};
        columns.forEach(col => {
            if (col.field && col.field !== 'harvey_internal_id') {
                const error = validateField(col.field, editedData[col.field]);
                if (error) newErrors[col.field] = error;
            }
        });
        errors = newErrors;
    }

    function handleSave() {
        if (Object.keys(errors).length === 0) {
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
        const colSchema = schema[field] || {};
        let currentDateObj = parseDate(editedData[field], colSchema) || new Date();

        if (type === 'date') {
            const dateParts = parseDate(val, { ...colSchema, subType: 'Date', format: colSchema.format?.split(/[T ]/)[0] || 'YYYY-MM-DD' });
            if (dateParts) {
                currentDateObj.setFullYear(dateParts.getFullYear(), dateParts.getMonth(), dateParts.getDate());
            }
        } else if (type === 'time') {
            const timeParts = parseDate(val, { ...colSchema, subType: 'Time', format: colSchema.format?.split(/[T ]/).slice(1).join(' ') || 'HH:mm' });
            if (timeParts) {
                currentDateObj.setHours(timeParts.getHours(), timeParts.getMinutes(), timeParts.getSeconds());
            }
        }

        editedData[field] = formatDate(currentDateObj, colSchema);
    }

    function flowbiteDatepicker(node, { field, isDateTime = false }) {
        let picker = null;
        const colSchema = schema[field] || {};
        const format = colSchema.format || '';
        const datePartFormat = (isDateTime ? (format.split(/[T ]/)[0] || 'YYYY-MM-DD') : (format || 'YYYY-MM-DD')).toLowerCase();

        const initPicker = () => {
            if (picker) return;
            picker = new Datepicker(node, {
                format: datePartFormat,
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
            const d = picker.getDate();
            let dateStr = node.value;
            if (d instanceof Date && !isNaN(d)) {
                dateStr = formatDate(d, { ...colSchema, subType: 'Date', format: isDateTime ? format.split(/[T ]/)[0] : format });
            }
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
    const seconds = Array.from({ length: 60 }, (_, i) => i.toString().padStart(2, '0'));

    function selectTimePart(field, part, val, isDateTime = false) {
        const colSchema = schema[field] || {};
        const format = colSchema.format || '';
        const timePartFormat = isDateTime ? (format.split(/[T ]/).slice(1).join(' ') || 'HH:mm') : (format || 'HH:mm');
        const hasSeconds = format.includes(':ss');
        
        let currentTimeStr = isDateTime ? (formatDate(parseDate(editedData[field], colSchema) || new Date(), { ...colSchema, subType: 'Time', format: timePartFormat })) : editedData[field];
        let d = parseDate(currentTimeStr, { ...colSchema, subType: 'Time', format: timePartFormat }) || new Date();
        
        if (part === 'h') d.setHours(parseInt(val));
        if (part === 'm') d.setMinutes(parseInt(val));
        if (part === 's') d.setSeconds(parseInt(val));
        
        const newTimeStr = formatDate(d, { ...colSchema, subType: 'Time', format: timePartFormat });

        if (isDateTime) {
            handleDateTimeChange(field, 'time', newTimeStr);
        } else {
            editedData[field] = newTimeStr;
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
                                    {@const hasSeconds = (colSchema.format || '').includes(':ss')}
                                    {@const currentD = parseDate(editedData[col.field], colSchema) || new Date()}
                                    {@const curH = currentD.getHours().toString().padStart(2, '0')}
                                    {@const curM = currentD.getMinutes().toString().padStart(2, '0')}
                                    {@const curS = currentD.getSeconds().toString().padStart(2, '0')}
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
                                        <Dropdown triggeredBy="#time_input_{sanitizeId(col.field)}" class="{hasSeconds ? 'w-36' : 'w-24'} p-0 z-[110] shadow-2xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                            <div class="flex h-64">
                                                <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                    {#each hours as h}
                                                        <button 
                                                            class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curH === h ? 'bg-blue-500 text-white font-bold' : ''}"
                                                            on:click={() => selectTimePart(col.field, 'h', h)}
                                                        >{h}</button>
                                                    {/each}
                                                </div>
                                                <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                <div class="flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900">
                                                    {#each minutes as m}
                                                        <button 
                                                            class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curM === m ? 'bg-blue-500 text-white font-bold' : ''}"
                                                            on:click={() => selectTimePart(col.field, 'm', m)}
                                                        >{m}</button>
                                                    {/each}
                                                </div>
                                                {#if hasSeconds}
                                                    <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                    <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                        {#each seconds as s}
                                                            <button 
                                                                class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curS === s ? 'bg-blue-500 text-white font-bold' : ''}"
                                                                on:click={() => selectTimePart(col.field, 's', s)}
                                                            >{s}</button>
                                                        {/each}
                                                    </div>
                                                {/if}
                                            </div>
                                        </Dropdown>
                                    </div>
                                {:else}
                                    {@const hasSeconds = (colSchema.format || '').includes(':ss')}
                                    {@const currentD = parseDate(editedData[col.field], colSchema) || new Date()}
                                    {@const curH = currentD.getHours().toString().padStart(2, '0')}
                                    {@const curM = currentD.getMinutes().toString().padStart(2, '0')}
                                    {@const curS = currentD.getSeconds().toString().padStart(2, '0')}
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
                                                    value={formatDate(parseDate(editedData[col.field], colSchema) || new Date(), { ...colSchema, subType: 'Date', format: (colSchema.format || '').split(/[T ]/)[0] })} 
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
                                                    value={formatDate(parseDate(editedData[col.field], colSchema) || new Date(), { ...colSchema, subType: 'Time', format: (colSchema.format || '').split(/[T ]/).slice(1).join(' ') })} 
                                                    class="cursor-pointer bg-white border border-gray-300 text-gray-900 text-xs rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2 pe-7 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                    on:keydown={(e) => e.preventDefault()}
                                                />
                                                <Dropdown triggeredBy="#dt_time_input_{sanitizeId(col.field)}" class="{hasSeconds ? 'w-36' : 'w-24'} p-0 z-[110] shadow-2xl border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                                    <div class="flex h-64">
                                                        <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                            {#each hours as h}
                                                                <button 
                                                                    class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curH === h ? 'bg-blue-500 text-white font-bold' : ''}"
                                                                    on:click={() => selectTimePart(col.field, 'h', h, true)}
                                                                >{h}</button>
                                                            {/each}
                                                        </div>
                                                        <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                        <div class="flex-1 overflow-y-auto custom-scrollbar bg-white dark:bg-gray-900">
                                                            {#each minutes as m}
                                                                <button 
                                                                    class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curM === m ? 'bg-blue-500 text-white font-bold' : ''}"
                                                                    on:click={() => selectTimePart(col.field, 'm', m, true)}
                                                                >{m}</button>
                                                            {/each}
                                                        </div>
                                                        {#if hasSeconds}
                                                            <div class="w-px bg-gray-200 dark:border-gray-700"></div>
                                                            <div class="flex-1 overflow-y-auto custom-scrollbar bg-gray-50 dark:bg-gray-800">
                                                                {#each seconds as s}
                                                                    <button 
                                                                        class="w-full py-2 text-sm transition-colors hover:bg-blue-100 dark:hover:bg-blue-900/30 {curS === s ? 'bg-blue-500 text-white font-bold' : ''}"
                                                                        on:click={() => selectTimePart(col.field, 's', s, true)}
                                                                    >{s}</button>
                                                                {/each}
                                                            </div>
                                                        {/if}
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
                                <div class="flex items-center gap-3 h-10 relative">
                                    <div class="relative flex-grow h-full flex items-center group">
                                        <input
                                            type="range"
                                            {min} {max} step="1"
                                            bind:value={editedData[col.field]}
                                            style="background: linear-gradient(to right, #3b82f6 {percentage}%, #e5e7eb {percentage}%);"
                                            class="progress-range w-full h-2 rounded-lg appearance-none cursor-pointer dark:bg-gray-700"
                                        />
                                        <div
                                            class="absolute -top-6 -ml-3 w-8 text-center text-xs font-semibold text-white bg-gray-900 dark:bg-gray-700 rounded py-0.5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
                                            style="left: {percentage}%;"
                                        >
                                            {val}
                                            <!-- Tooltip caret -->
                                            <div class="absolute w-2 h-2 bg-gray-900 dark:bg-gray-700 rotate-45 -bottom-1 left-1/2 -translate-x-1/2"></div>
                                        </div>
                                    </div>
                                    <span class="text-sm font-medium text-gray-700 dark:text-gray-300 min-w-[3rem] text-right">
                                        {val}/{max}
                                    </span>
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
                                color={errors[col.field] ? 'red' : 'base'} class="resize-none"
                                on:keydown={(e) => { if (e.key === 'Enter') e.stopPropagation(); }} />
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
