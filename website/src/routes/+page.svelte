<script>
  import { base } from '$app/paths';
  import {
    ArrowRight,
    WifiOff,
    Code,
    Lock,
    Download,
    Github,
    ChevronLeft,
    ChevronRight,
    Copy,
    Check,
    Mic,
    Languages,
    Settings,
    Edit3,
    Image,
    Table,
    Video,
    Share2,
    FileText
  } from '@lucide/svelte';
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';

  let activeTab = 'windows';
  let isMac = false;

  // Release and Download logic
  /* @sync-start */
  let version = '0.1.1'; /* @sync-version */
  let downloadLinks = {
    windows:
      'https://github.com/Ethnomethodology/harvey/releases/download/v0.1.1/Harvey_0.1.1_x64-setup.zip' /* @sync-win */,
    macosArm:
      'https://github.com/Ethnomethodology/harvey/releases/download/v0.1.1/Harvey_0.1.1_aarch64.dmg' /* @sync-macos-arm */,
    macosIntel:
      'https://github.com/Ethnomethodology/harvey/releases/download/v0.1.1/Harvey_0.1.1_x64.dmg' /* @sync-macos-x64 */
  };
  /* @sync-end */

  // Carousel logic
  let currentSlide = 0;
  const slides = [
    {
      title: 'Transcription',
      description: 'Convert audio and video into text automatically using state-of-the-art local AI models. Perfect for interviews and focus groups.',
      icon: Mic,
      color: 'blue'
    },
    {
      title: 'Translation',
      description: 'Break language barriers effortlessly. Translate your qualitative data into English, maintaining nuanced meaning across languages.',
      icon: Languages,
      color: 'purple'
    },
    {
      title: 'Model Management',
      description: 'Choose from a variety of Whisper and translation models tailored to your hardware and accuracy needs.',
      icon: Settings,
      color: 'green'
    },
    {
      title: 'Rich Text Editor',
      description: 'Draft reports, edit transcripts, and keep your notes organized in a professional, integrated markdown workspace.',
      icon: Edit3,
      color: 'amber'
    },
    {
      title: 'Image Annotation',
      description: 'Work with visual data seamlessly. Annotate images and PDFs directly within the app to support your findings.',
      icon: Image,
      color: 'rose'
    },
    {
      title: 'Table Management',
      description: 'Manipulate structured data with ease. View and edit CSV and XLSX files without leaving your research environment.',
      icon: Table,
      color: 'emerald'
    },
    {
      title: 'Media Sync',
      description: 'Navigate your transcripts instantly. Media playback stays perfectly synchronized with your text for effortless verification.',
      icon: Video,
      color: 'indigo'
    },
    {
      title: 'Flexible Export',
      description: 'Export your completed work to high-quality DOCX, CSV, or pure text formats, ready for publication or sharing.',
      icon: Share2,
      color: 'slate'
    }
  ];

  let interval;
  onMount(async () => {
    if (typeof navigator !== 'undefined') {
      const platform = navigator.platform.toLowerCase();
      if (platform.includes('mac')) {
        activeTab = 'macos';
        isMac = true;
      } else if (platform.includes('linux')) {
        activeTab = 'source';
      } else {
        activeTab = 'windows';
      }
    }

    // Fetch latest release info from GitHub - skip on localhost to respect hardcoded version
    try {
      if (
        typeof window !== 'undefined' &&
        (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1')
      ) {
        console.log('[Harvey] Localhost detected, using hardcoded version:', version);
      } else {
        const response = await fetch(
          'https://api.github.com/repos/Ethnomethodology/harvey/releases/latest'
        );
        if (response.ok) {
          const data = await response.json();
          version = data.tag_name.replace('v', '');

          const assets = data.assets || [];
          const winAsset = assets.find((a) => a.name.endsWith('.zip') || a.name.endsWith('.exe'));
          const macArmAsset = assets.find(
            (a) => a.name.includes('aarch64.dmg') || a.name.includes('arm64.dmg')
          );
          const macIntelAsset = assets.find((a) => a.name.includes('x64.dmg'));

          if (winAsset) downloadLinks.windows = winAsset.browser_download_url;
          if (macArmAsset) downloadLinks.macosArm = macArmAsset.browser_download_url;
          if (macIntelAsset) downloadLinks.macosIntel = macIntelAsset.browser_download_url;
        }
      }
    } catch (error) {
      console.error('Failed to fetch latest release from GitHub:', error);
    }

    interval = setInterval(() => {
      nextSlide();
    }, 5000);
  });

  onDestroy(() => {
    clearInterval(interval);
  });

  function nextSlide() {
    currentSlide = (currentSlide + 1) % slides.length;
  }

  function prevSlide() {
    currentSlide = (currentSlide - 1 + slides.length) % slides.length;
  }

  function goToSlide(index) {
    currentSlide = index;
    if (interval) {
      clearInterval(interval);
      interval = setInterval(nextSlide, 5000);
    }
  }

  function setActiveTab(tab) {
    activeTab = tab;
  }

  let copyFeedback = {
    mac: false,
    source: false
  };

  function copyToClipboard(text, key) {
    if (typeof navigator !== 'undefined') {
      navigator.clipboard.writeText(text).then(() => {
        copyFeedback[key] = true;
        setTimeout(() => {
          copyFeedback[key] = false;
        }, 2000);
      });
    }
  }
</script>

<!-- Hero -->
<section class="py-16 lg:py-28 text-center">
  <div class="max-w-4xl mx-auto px-4">
    <h1
      class="text-4xl lg:text-6xl font-extrabold tracking-tight text-slate-900 mb-8 leading-[1.15]"
    >
      Your secure, offline workspace for <span class="text-green-500">qualitative research.</span>
    </h1>
    <p
      class="text-lg lg:text-xl text-slate-700 max-w-3xl mx-auto mb-10 leading-relaxed font-medium"
    >
      Harvey is an integrated desktop workspace for transcription and translation with
      secure, state-of-the-art AI models that run entirely on your own machine.
    </p>
    <div class="flex flex-col sm:flex-row gap-4 justify-center mb-20 relative z-10">
      <a
        href="#download"
        class="inline-flex items-center justify-center px-8 py-4 text-lg font-bold text-white transition-all bg-green-500 rounded-xl hover:bg-green-600 shadow-xl shadow-green-200 hover:shadow-green-300 gap-2"
      >
        <Download class="w-6 h-6" />
        Download for Free
      </a>
      <a
        href="{base}/about"
        class="inline-flex items-center justify-center px-8 py-4 text-lg font-bold text-slate-800 transition-all bg-white border border-slate-200 rounded-xl hover:bg-slate-50 shadow-sm"
      >
        Learn More
      </a>
    </div>

    <!-- Product Mockup -->
    <div class="relative max-w-5xl mx-auto group">
      <div
        class="absolute -inset-1.5 bg-gradient-to-r from-green-500/20 to-blue-500/20 rounded-[2.5rem] blur-xl opacity-75 group-hover:opacity-100 transition duration-1000"
      ></div>
      <div
        class="relative rounded-[2rem] overflow-hidden border-8 border-white dark:border-slate-800 shadow-2xl"
      >
        <img
          src="{base}/heroshot.png"
          alt="Harvey Application Workspace Showcasing Synchronized Transcripts and Video Playback"
          class="w-full h-auto transition-transform duration-500 group-hover:scale-[1.01]"
        />
      </div>
    </div>
  </div>
</section>

<!-- Features -->
<section class="py-16 border-t border-slate-100">
  <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
    <!-- Card 1 -->
    <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
      <div
        class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6"
      >
        <Lock class="w-6 h-6" />
      </div>
      <h3 class="text-xl font-bold text-slate-900 mb-3">Privacy First</h3>
      <p class="text-slate-700 leading-relaxed">
        All AI processing happens locally on your device. Your data never leaves your computer.
      </p>
    </div>

    <!-- Card 2 -->
    <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
      <div
        class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6"
      >
        <WifiOff class="w-6 h-6" />
      </div>
      <h3 class="text-xl font-bold text-slate-900 mb-3">Offline AI</h3>
      <p class="text-slate-700 leading-relaxed">
        Download state-of-the-art models once and use them forever. Transcription and translation
        run completely offline on your local machine.
      </p>
    </div>

    <!-- Card 3 -->
    <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
      <div
        class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6"
      >
        <Code class="w-6 h-6" />
      </div>
      <h3 class="text-xl font-bold text-slate-900 mb-3">Open Source</h3>
      <p class="text-slate-700 leading-relaxed">
        Harvey is free and open source. We warmly welcome contributions from researchers and
        developers to shape the future of qualitative tools.
      </p>
    </div>
  </div>
</section>

<!-- App Showcase Carousel -->
<section class="py-24 border-t border-slate-100 overflow-hidden">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="text-center mb-16">
      <h2 class="text-3xl lg:text-5xl font-extrabold text-slate-900 mb-6">
        Powerful Tools for Qualitative Research
      </h2>
      <p class="text-lg text-slate-700 max-w-2xl mx-auto">
        Everything you need to streamline your qualitative workflow in one secure, privacy-first
        environment.
      </p>
    </div>

    <!-- Redesigned Split Carousel -->
    <div
      class="bg-white border border-slate-200 rounded-[2.5rem] shadow-xl overflow-hidden min-h-[500px]"
    >
      <div class="grid grid-cols-1 lg:grid-cols-12 h-full">
        <!-- Sidebar Navigation -->
        <div class="lg:col-span-4 bg-slate-50/50 border-r border-slate-200 p-8 flex flex-col">
          <div class="mb-8 hidden lg:block">
            <span
              class="px-3 py-1 bg-green-100 text-green-700 text-xs font-bold uppercase tracking-widest rounded-full"
            >
              Core Features
            </span>
          </div>

          <div class="flex-1 space-y-2 overflow-y-auto pr-2 custom-scrollbar">
            {#each slides as slide, i}
              <button
                on:click={() => goToSlide(i)}
                class="w-full text-left p-4 rounded-2xl transition-all duration-300 flex items-center gap-4 group {currentSlide ===
                i
                  ? 'bg-white shadow-md border border-slate-200 ring-2 ring-green-500/10'
                  : 'hover:bg-slate-100/80 text-slate-500'}"
              >
                <div
                  class="h-10 w-10 rounded-xl flex items-center justify-center transition-all {currentSlide ===
                  i
                    ? `bg-green-500 text-white shadow-lg shadow-green-200`
                    : 'bg-slate-200 text-slate-500 group-hover:bg-slate-300'}"
                >
                  <svelte:component this={slide.icon} class="w-5 h-5" />
                </div>
                <span class="font-bold text-sm lg:text-base {currentSlide === i ? 'text-slate-900' : ''}"
                  >{slide.title}</span
                >
                {#if currentSlide === i}
                  <ArrowRight class="w-4 h-4 ml-auto text-green-500" />
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Main Display Content -->
        <div class="lg:col-span-8 p-8 lg:p-12 flex flex-col">
          {#each slides as slide, i}
            {#if currentSlide === i}
              <div
                class="flex flex-col h-full space-y-8"
                in:fade={{ duration: 400 }}
                out:fade={{ duration: 200 }}
              >
                <!-- Feature Title & Description -->
                <div class="max-w-2xl">
                  <h3 class="text-3xl lg:text-4xl font-extrabold text-slate-900 mb-4">
                    {slide.title}
                  </h3>
                  <p class="text-lg text-slate-700 leading-relaxed font-medium">
                    {slide.description}
                  </p>
                </div>

                <!-- Feature Visual Area -->
                <div class="flex-1 relative group mt-4 h-full min-h-[300px]">
                  <!-- Backdrop Glow -->
                  <div class="absolute -inset-4 bg-slate-100 rounded-[2.5rem] blur-xl opacity-30 group-hover:opacity-50 transition-opacity duration-500"></div>
                  
                  <!-- Unified Monitor Mockup -->
                  <div
                    class="relative aspect-[16/10] w-full bg-slate-950 rounded-[1.8rem] border-[10px] border-slate-900 shadow-[0_25px_50px_-12px_rgba(0,0,0,0.5)] overflow-hidden"
                  >
                    <!-- Screen Content -->
                    <div class="absolute inset-0 bg-slate-900">
                      {#if i === 0}
                        <img
                          src="{base}/transcription-preview.png"
                          alt="Harvey Transcription Interface"
                          class="w-full h-full object-top object-cover opacity-90 transition-opacity duration-300"
                        />
                      {:else}
                        <!-- Icon Placeholder for other features within the same monitor frame -->
                        <div class="flex flex-col items-center justify-center h-full text-center p-12 bg-gradient-to-br from-slate-900 to-slate-950">
                          <div class="h-24 w-24 rounded-3xl bg-white/5 backdrop-blur-xl flex items-center justify-center text-white mb-8 border border-white/10 shadow-2xl">
                             <svelte:component this={slide.icon} class="w-12 h-12" />
                          </div>
                          <h4 class="text-2xl font-bold text-white mb-3">{slide.title}</h4>
                          <p class="text-slate-400 text-base max-w-sm mx-auto leading-relaxed">{slide.description}</p>
                          <div class="mt-8 flex gap-2">
                             {#each Array(3) as _}
                                <div class="h-1 w-12 bg-white/10 rounded-full"></div>
                             {/each}
                          </div>
                        </div>
                      {/if}
                    </div>

                    <!-- Reflective Overlay for "Screen" feel -->
                    <div class="absolute inset-0 pointer-events-none bg-gradient-to-tr from-white/5 to-transparent opacity-30"></div>
                  </div>
                </div>

                <!-- Controls for Mobile (Quick jump arrows) -->
                <div class="flex lg:hidden justify-between items-center pt-8 border-t border-slate-100">
                   <button on:click={prevSlide} class="p-3 bg-slate-100 rounded-full hover:bg-slate-200 text-slate-700">
                      <ChevronLeft class="w-6 h-6" />
                   </button>
                   <div class="flex gap-2">
                      {#each slides as _, dotIndex}
                         <div class="h-2 w-2 rounded-full {currentSlide === dotIndex ? 'bg-green-500 w-4' : 'bg-slate-300'} transition-all"></div>
                      {/each}
                   </div>
                   <button on:click={nextSlide} class="p-3 bg-slate-100 rounded-full hover:bg-slate-200 text-slate-700">
                      <ChevronRight class="w-6 h-6" />
                   </button>
                </div>
              </div>
            {/if}
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

<!-- Download Section -->
<section id="download" class="py-24 border-t border-slate-100 scroll-mt-24">
  <div class="max-w-4xl mx-auto text-center">
    <h2 class="text-3xl lg:text-4xl font-bold text-slate-900 mb-4">Get Harvey</h2>
    <p class="text-lg text-slate-700 mb-10">Choose your platform to start your research journey.</p>

    <!-- Tabs -->
    <div class="flex justify-center mb-8 border-b border-slate-200">
      <button
        class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'windows'
          ? 'border-green-500 text-green-600'
          : 'border-transparent text-slate-500 hover:text-slate-700'}"
        on:click={() => setActiveTab('windows')}
      >
        Windows
      </button>
      <button
        class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'macos'
          ? 'border-green-500 text-green-600'
          : 'border-transparent text-slate-500 hover:text-slate-700'}"
        on:click={() => setActiveTab('macos')}
      >
        macOS
      </button>
      <button
        class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'source'
          ? 'border-green-500 text-green-600'
          : 'border-transparent text-slate-500 hover:text-slate-700'}"
        on:click={() => setActiveTab('source')}
      >
        Build from Source
      </button>
    </div>

    <!-- Tab Content -->
    <div class="bg-slate-50 border border-slate-100 rounded-2xl p-8 lg:p-12 transition-all">
      {#if activeTab === 'windows'}
        <div class="flex flex-col items-center">
          <div
            class="h-16 w-16 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mb-6"
          >
            <svg viewBox="0 0 24 24" fill="currentColor" class="w-8 h-8"
              ><path
                d="M0 3.449L9.75 2.1v9.451H0V3.449zm10.949-1.323L24 0v11.551H10.949V2.126zm-10.949 9.924H9.75v9.451L0 20.551V12.05zm10.949 0H24v11.551l-13.051-2.126V12.05z"
              /></svg
            >
          </div>
          <h3 class="text-2xl font-bold text-slate-900 mb-2">Harvey for Windows</h3>
          <p class="text-slate-600 mb-4 max-w-md">Compatible with Windows 10 and 11.</p>
          <p class="text-sm text-slate-500 mb-8">
            For detailed installation instructions, see the <a
              href="{base}/help/downloads"
              class="text-green-600 hover:underline">Help Center</a
            >.
          </p>
          <a
            href={downloadLinks.windows}
            class="inline-flex items-center justify-center px-8 py-4 text-lg font-bold text-white transition-all bg-green-500 rounded-xl hover:bg-green-600 shadow-lg shadow-green-200 hover:shadow-green-300 gap-2"
          >
            <Download class="w-6 h-6" />
            Download .exe Installer
          </a>
          <p class="text-xs text-slate-400 mt-4 mb-8">Version {version} • 64-bit</p>

          <div class="pt-8 border-t border-slate-200 w-full max-w-lg text-left">
            <h4 class="text-sm font-bold text-slate-900 mb-3 uppercase tracking-wider">
              Windows Security Warnings
            </h4>
            <p class="text-sm text-slate-600 mb-0 leading-relaxed">
              As an open-source project, Harvey is not yet digitally signed by a commercial
              certificate authority. Because the app is not currently signed, Windows SmartScreen
              will display a warning. You can safely proceed by clicking <strong>"More info"</strong
              >
              and then <strong>"Run anyway"</strong>.
            </p>
          </div>
        </div>
      {/if}

      {#if activeTab === 'macos'}
        <div class="flex flex-col items-center">
          <div
            class="h-16 w-16 bg-gray-100 text-gray-900 rounded-2xl flex items-center justify-center mb-6"
          >
            <svg viewBox="0 0 24 24" fill="currentColor" class="w-8 h-8"
              ><path
                d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.21-1.98 1.08-3.11-1.06.05-2.31.71-3.06 1.48-.69.72-1.24 1.87-1.09 2.98 1.18.08 2.36-.67 3.07-1.35"
              /></svg
            >
          </div>
          <h3 class="text-2xl font-bold text-slate-900 mb-2">Harvey for macOS</h3>
          <p class="text-slate-600 mb-4 max-w-md">
            Optimized for Apple Silicon (M1, M2, etc.) and Intel Macs.
          </p>
          <p class="text-sm text-slate-500 mb-8">
            For detailed installation instructions, see the <a
              href="{base}/help/downloads"
              class="text-green-600 hover:underline">Help Center</a
            >.
          </p>
          <div class="flex flex-col sm:flex-row gap-4">
            <a
              href={downloadLinks.macosArm}
              class="inline-flex items-center justify-center px-6 py-3 text-base font-bold text-white transition-all bg-slate-900 rounded-xl hover:bg-slate-800 shadow-lg shadow-slate-200 gap-2"
            >
              <Download class="w-5 h-5" />
              Apple Silicon
            </a>
            <a
              href={downloadLinks.macosIntel}
              class="inline-flex items-center justify-center px-6 py-3 text-base font-bold text-white transition-all bg-slate-900 rounded-xl hover:bg-slate-800 shadow-lg shadow-slate-200 gap-2"
            >
              <Download class="w-5 h-5" />
              Intel Mac
            </a>
          </div>
          <p class="text-xs text-slate-400 mt-4 mb-8">
            Version {version} • Universal Binary available
          </p>

          <div class="pt-8 border-t border-slate-200 w-full max-w-lg text-left">
            <h4 class="text-sm font-bold text-slate-900 mb-3 uppercase tracking-wider">
              macOS Security Warnings
            </h4>
            <p class="text-sm text-slate-600 mb-4 leading-relaxed">
              As an open-source project, Harvey is not yet digitally signed by a commercial
              certificate authority. Because the app is not currently signed, macOS will prevent it
              from running initially. To authorize it after installation, run the following commands
              in your <strong>Terminal</strong> application:
            </p>
            <div
              class="relative group/term bg-slate-900 rounded-lg p-4 font-mono text-xs text-white overflow-x-auto shadow-inner border border-slate-800"
            >
              <button
                on:click={() =>
                  copyToClipboard(
                    'cd /Applications\nsudo xattr -dr com.apple.quarantine harvey.app',
                    'mac'
                  )}
                class="absolute top-2 right-2 p-1.5 rounded-md bg-white/5 hover:bg-white/10 text-slate-400 hover:text-white transition-all flex items-center gap-1.5"
                title="Copy to clipboard"
              >
                {#if copyFeedback.mac}
                  <Check class="w-3.5 h-3.5 text-green-400" />
                  <span class="text-[10px] font-bold text-green-400 uppercase tracking-tight"
                    >Copied!</span
                  >
                {:else}
                  <Copy class="w-3.5 h-3.5" />
                {/if}
              </button>
              <div class="flex select-none mb-1 text-slate-500">
                <span class="mr-2">$</span>
                <span>cd /Applications</span>
              </div>
              <div class="flex">
                <span class="mr-2 text-slate-500 select-none">$</span>
                <span class="text-green-400">sudo xattr -dr com.apple.quarantine harvey.app</span>
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if activeTab === 'source'}
        <div class="flex flex-col items-center">
          <div
            class="h-16 w-16 bg-slate-100 text-slate-900 rounded-2xl flex items-center justify-center mb-6"
          >
            <Github class="w-8 h-8" />
          </div>
          <h3 class="text-2xl font-bold text-slate-900 mb-2">Build Harvey from Source</h3>
          <p class="text-slate-600 mb-4 max-w-md">
            Use the automated build script to check dependencies and compile Harvey on your
            platform.
          </p>
          <p class="text-sm text-slate-500 mb-8">
            For detailed build instructions, see the <a
              href="{base}/help/downloads"
              class="text-green-600 hover:underline">Help Center</a
            >.
          </p>

          <div
            class="relative group/term w-full max-w-lg bg-slate-900 rounded-lg p-4 text-left font-mono text-sm text-green-400 mb-6 overflow-x-auto border border-slate-800 shadow-inner"
          >
            <button
              on:click={() =>
                copyToClipboard(
                  'curl -sSL https://raw.githubusercontent.com/Ethnomethodology/harvey/main/scripts/bootstrap.sh | bash',
                  'source'
                )}
              class="absolute top-2 right-2 p-1.5 rounded-md bg-white/5 hover:bg-white/10 text-slate-400 hover:text-white transition-all flex items-center gap-1.5"
              title="Copy to clipboard"
            >
              {#if copyFeedback.source}
                <Check class="w-4 h-4 text-green-400" />
                <span class="text-[10px] font-bold text-green-400 uppercase tracking-tight"
                  >Copied!</span
                >
              {:else}
                <Copy class="w-4 h-4" />
              {/if}
            </button>
            <div class="flex select-none mb-2 text-slate-500">
              <span class="mr-2">$</span>
              <span># Run automated bootstrapper</span>
            </div>
            <div class="mb-2">
              <span class="mr-2 text-slate-500 select-none">$</span>
              <span class="text-white"
                >curl -sSL
                https://raw.githubusercontent.com/Ethnomethodology/harvey/main/scripts/bootstrap.sh
                | bash</span
              >
            </div>
          </div>

          <p class="text-sm text-slate-500 mb-4 text-center">Or manually clone and run locally:</p>
          <div
            class="w-full max-w-lg bg-slate-50 border border-slate-200 rounded-lg p-4 text-left font-mono text-xs text-slate-600 mb-8 overflow-x-auto"
          >
            <div class="mb-2 flex items-center gap-2">
              <span class="h-1 w-1 bg-slate-400 rounded-full"></span>
              <span>git clone https://github.com/Ethnomethodology/harvey.git</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="h-1 w-1 bg-slate-400 rounded-full"></span>
              <span>cd harvey && bash scripts/bootstrap.sh</span>
            </div>
          </div>

          <a
            href="https://github.com/Ethnomethodology/harvey"
            target="_blank"
            class="inline-flex items-center justify-center px-8 py-3 text-base font-bold text-slate-700 transition-all bg-white border border-slate-200 rounded-xl hover:bg-slate-50 gap-2"
          >
            View on GitHub
          </a>
        </div>
      {/if}
    </div>
  </div>
</section>
