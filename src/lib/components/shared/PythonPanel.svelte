<script>
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openExternal } from '@tauri-apps/plugin-shell';

	let isPanelOpen = false;
	let isPythonInstalled = false;
	let platform = '';
	let isLoading = true;
	let error = '';

	onMount(async () => {
		try {
			isLoading = true;
			isPythonInstalled = await invoke('check_python_installed');
			platform = await invoke('get_platform_info');
		} catch (e) {
			console.error('Error checking Python status or platform:', e);
			error = `Failed to check dependency status: ${e.message || e}`;
			// Assume not installed if check fails
			isPythonInstalled = false;
		} finally {
			isLoading = false;
		}
	});

	function openLink(url) {
		openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
	}
</script>

<div class="border-y border-gray-200">
	<button
		on:click={() => (isPanelOpen = !isPanelOpen)}
		class="w-full flex justify-between items-center py-3 text-left focus:outline-none"
	>
		<div class="flex items-center">
			<h3 class="block text-sm font-medium text-gray-700">Python 3</h3>
		</div>
		<div class="flex items-center">
			{#if isLoading}
				<span class="text-xs text-gray-500 mr-2">Checking...</span>
			{:else if isPythonInstalled}
				<span class="text-sm font-medium text-green-600">Installed</span>
			{:else}
				<span class="text-sm font-medium text-red-600">Required</span>
			{/if}
			<svg
				class="w-6 h-6 transform transition-transform duration-200 ease-in-out {isPanelOpen
					? 'rotate-180'
					: ''}"
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="2"
				stroke="currentColor"
			>
				<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
			</svg>
		</div>
	</button>
</div>

{#if isPanelOpen}
	<div class="p-4 bg-gray-50 border-b border-gray-200 text-sm">
		<p class="mb-2">
			Python is a high-level, general-purpose programming language.
		</p>

		{#if isPythonInstalled}
			<p class="text-green-600 mb-2">Already installed, no further action required.</p>
		{:else}
			<p class="text-red-600 mb-2">
				Required installation. Please follow the guide for your operating system.
			</p>
		{/if}

		<p class="mb-4">
			This application uses Python for speaker diarization (identifying different speakers in an audio file).
		</p>

		<h4 class="font-semibold mb-2">Installation Guide</h4>

		{#if platform === 'linux'}
			<div>
				<p>On Debian/Ubuntu-based systems, you can install it using the terminal:</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs"
					>sudo apt update && sudo apt install python3</code
				>
			</div>
		{:else if platform === 'macos'}
			<div>
				<p>
					Using
					<a
						href="https://brew.sh/"
						on:click|preventDefault={() => openLink('https://brew.sh/')}
						class="text-blue-600 hover:underline">Homebrew</a
					>
					is the recommended way to install Python:
				</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs">brew install python</code>
			</div>
		{:else if platform === 'windows'}
			<div>
				<p>
					Download the installer from the
					<a
						href="https://www.python.org/downloads/"
						on:click|preventDefault={() => openLink('https://www.python.org/downloads/')}
						class="text-blue-600 hover:underline">official Python website</a
					>
					and make sure to check "Add Python to PATH" during installation.
				</p>
			</div>
		{:else}
			<p>
				Could not determine your operating system. Please visit the
				<a
					href="https://www.python.org/downloads/"
					on:click|preventDefault={() => openLink('https://www.python.org/downloads/')}
					class="text-blue-600 hover:underline">official Python website</a
				>
				for installation instructions.
			</p>
		{/if}
		{#if error}
			<p class="text-red-600 mt-4">{error}</p>
		{/if}
	</div>
{/if}
