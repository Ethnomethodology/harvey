<!-- src/lib/components/projectview/data/tables/TableHeaderIcon.svelte -->
<script>
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
        TextInitial
    } from 'lucide-svelte';

    export let colSchema = {};
    export let header = '';

    function getIcon() {
        const type = colSchema.type || 'Text';
        const subType = colSchema.subType || 'Small Text';
        
        if (type === 'Misc') {
            if (subType === 'Checkbox') return CheckSquare;
            if (subType === 'Selectbox') return SquareMenu;
            if (subType === 'Tags') return Tags;
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
        
        return Type;
    }

    $: Icon = getIcon();
</script>

<div class="flex items-center">
    <span class="inline-flex items-center mr-1.5 text-gray-400">
        <svelte:component this={Icon} size={14} strokeWidth={2} />
    </span>
    <span>{header}</span>
</div>
