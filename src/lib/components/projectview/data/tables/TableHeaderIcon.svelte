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
    TextInitial,
    Star
  } from '@lucide/svelte';
  import ProgressIcon from './icons/ProgressIcon.svelte';

  let { colSchema = {}, header = '', onResizeStart = null } = $props();

  function getIcon(schema) {
    const type = schema.type || 'Text';
    const subType = schema.subType || 'Small Text';

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

    return Type;
  }

  let Icon = $derived(getIcon(colSchema));

  function handleMouseDown(e) {
    if (onResizeStart) {
      e.preventDefault();
      e.stopPropagation();
      onResizeStart(e);
    }
  }
</script>

<div class="header-container flex items-center justify-center relative w-full h-full">
  <div class="flex items-center justify-center flex-1 px-2">
    <span class="inline-flex items-center mr-1.5 text-gray-400">
      <Icon size={14} strokeWidth={2} />
    </span>
    <span class="truncate font-semibold text-gray-700 dark:text-gray-200">{header}</span>
  </div>

  {#if onResizeStart && header !== '+'}
    <!-- Manual Resize Handle -->
    <div
      class="manual-resize-handle absolute right-0 top-0 bottom-0 w-[6px] cursor-ew-resize hover:bg-blue-400/30 transition-colors z-[500]"
      onmousedown={handleMouseDown}
      role="button"
      tabindex="-1"
      aria-label="Resize Column"
    ></div>
  {/if}
</div>

<style>
  .header-container {
    user-select: none;
  }
  .manual-resize-handle {
    border-right: 1px solid transparent;
  }
  .header-container:hover .manual-resize-handle {
    border-right-color: rgba(59, 130, 246, 0.2);
  }
</style>
