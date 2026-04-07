<!-- src/lib/components/projectview/data/tables/TableIcon.svelte -->
<script>
  export let icon;
  export let size = 14;
  export let color = 'currentColor';
  export let strokeWidth = 2;
</script>

<div class="inline-flex items-center justify-center">
  <svelte:component this={icon} {size} {color} {strokeWidth} />
</div>
