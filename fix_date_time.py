import re

with open('src/lib/components/projectview/modals/EditEntryModal.svelte', 'r') as f:
    content = f.read()

# Make sure Datepicker and Timepicker are imported
if "Timepicker" not in content:
    content = content.replace("import { Input", "import { Input, Datepicker, Timepicker")

# Replace Date Input
content = re.sub(
    r'<Input type="date"\s+id="(field-[^"]+)"\s+bind:value={([^}]+)}\s+class="[^"]+"\s*/>',
    r'<Datepicker id="\1" bind:value={\2} class="w-full" />',
    content
)

content = re.sub(
    r'<Input type="date"\s+value={([^}]+)}\s+on:input={([^}]+)}\s+class="[^"]+"\s*/>',
    r'<Datepicker value={\1} on:change={\2} class="w-full" />',
    content
)

# Replace Time Input
content = re.sub(
    r'<Input type="time"\s+id="(field-[^"]+)"\s+bind:value={([^}]+)}\s+class="[^"]+"\s*/>',
    r'<Timepicker id="\1" bind:value={\2} class="w-full" />',
    content
)

content = re.sub(
    r'<Input type="time"\s+value={([^}]+)}\s+on:input={([^}]+)}\s+class="[^"]+"\s*/>',
    r'<Timepicker value={\1} on:change={\2} class="w-full" />',
    content
)

with open('src/lib/components/projectview/modals/EditEntryModal.svelte', 'w') as f:
    f.write(content)
