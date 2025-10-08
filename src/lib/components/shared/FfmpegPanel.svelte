<script>
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open as openExternal } from '@tauri-apps/plugin-shell';

	let isPanelOpen = false;
	let isFfmpegInstalled = false;
	let platform = '';
	let isLoading = true;
	let error = '';

	onMount(async () => {
		try {
			isLoading = true;
			isFfmpegInstalled = await invoke('check_ffmpeg_installed');
			platform = await invoke('get_platform_info');
		} catch (e) {
			console.error('Error checking FFmpeg status or platform:', e);
			error = `Failed to check dependency status: ${e.message || e}`;
			// Assume not installed if check fails
			isFfmpegInstalled = false;
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
			<h3 class="block text-sm font-medium text-gray-700">FFmpeg</h3>
		</div>
		<div class="flex items-center">
			{#if isLoading}
				<span class="text-xs text-gray-500 mr-2">Checking...</span>
			{:else if isFfmpegInstalled}
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
			A complete, cross-platform solution to record, convert and stream audio and video.
		</p>

		{#if isFfmpegInstalled}
			<p class="text-green-600 mb-2">Already installed, no further action required.</p>
		{:else}
			<p class="text-red-600 mb-2">
				Required installation. Please follow the guide for your operating system.
			</p>
		{/if}

		<p class="mb-4">
			This application uses FFmpeg to convert various audio and video formats into a standard
			format required for transcription.
		</p>

		<h4 class="font-semibold mb-2">Installation Guide</h4>

		{#if platform === 'linux'}
			<div>
				<p>On Debian/Ubuntu-based systems, you can install it using the terminal:</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs"
					>sudo apt update && sudo apt install ffmpeg</code
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
					is the recommended way to install FFmpeg:
				</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs">brew install ffmpeg</code>
			</div>
		{:else if platform === 'windows'}
			<div>
				<p>
					You can use a package manager like
					<a
						href="https://chocolatey.org/"
						on:click|preventDefault={() => openLink('https://chocolatey.org/')}
						class="text-blue-600 hover:underline">Chocolatey</a
					>
					or
					<a
						href="https://scoop.sh/"
						on:click|preventDefault={() => openLink('https://scoop.sh/')}
						class="text-blue-600 hover:underline">Scoop</a
					>:
				</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs">choco install ffmpeg</code>
				<p class="my-2">or</p>
				<code class="block bg-gray-200 p-2 rounded my-2 text-xs">scoop install ffmpeg</code>
				<p class="mt-2">
					Alternatively, download the binaries from the
					<a
						href="https://ffmpeg.org/download.html"
						on:click|preventDefault={() => openLink('https://ffmpeg.org/download.html')}
						class="text-blue-600 hover:underline">official FFmpeg website</a
					>
					and add the `bin` directory to your system's PATH.
				</p>
			</div>
		{:else}
			<p>
				Could not determine your operating system. Please visit the
				<a
					href="https://ffmpeg.org/download.html"
					on:click|preventDefault={() => openLink('https://ffmpeg.org/download.html')}
					class="text-blue-600 hover:underline">official FFmpeg website</a
				>
				for installation instructions.
			</p>
		{/if}
		{#if error}
			<p class="text-red-600 mt-4">{error}</p>
		{/if}
	</div>
{/if}
