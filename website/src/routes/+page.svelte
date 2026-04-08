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
    Share2,
    FileText,
    FileSpreadsheet,
    Video,
    Table,
    Tag
  } from '@lucide/svelte';
  import { onMount, onDestroy } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { browser } from '$app/environment';

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
  let previousSlide = 0;
  const SLIDE_DURATION = 6000;

  const slides = [
    {
      title: 'Transcription',
      description: 'Automatically transcribe audio and video files entirely offline using state-of-the-art models. Harvey also uses open-source diarization model to identify different speakers and keeps your transcript perfectly in sync with the recording.',
      icon: Mic,
      accent: 'bg-blue-500',
      image: 'transcription-preview.png'
    },
    {
      title: 'Translation',
      description: 'Translate your transcripts and documents from one language to another using state-of-the-art models. Harvey uses open-source machine translation to run your project entirely offline.',
      icon: Languages,
      accent: 'bg-purple-500',
      image: 'translation-preview.png'
    },
    {
      title: 'Model Management',
      description: 'Take full control of your AI environment. Harvey lets you download and manage various open-source models for transcription and translation, optimized for your computer\'s hardware and specific research needs.',
      icon: Settings,
      accent: 'bg-green-500',
      image: 'model-management.png'
    },
    {
      title: 'Qualitative Coding',
      description: 'Apply highlights and tags across transcripts, documents, images, and tables to support your qualitative coding process. Harvey provides a unified workspace for rigorous analysis while keeping all of your research data strictly private.',
      icon: Tag,
      accent: 'bg-indigo-500',
      image: 'qualitative-coding.png'
    },
    {
      title: 'Image Annotation',
      description: 'Add detailed annotations to high-resolution images in addition to tagging them. Harvey includes built-in tools to censor confidential information and mark up visual evidence for your research files.',
      icon: Image,
      accent: 'bg-rose-500',
      image: 'image-annotation.png'
    },
    {
      title: 'Table Management',
      description: 'Import CSV and XLSX datasets, such as survey responses, and edit them directly within your project. Harvey provides advanced tools to create specialized data views and generate visualizations tailored to your research needs.',
      icon: FileSpreadsheet,
      accent: 'bg-emerald-500',
      image: 'table-management.png'
    },
    {
      title: 'Rich Text Editor',
      description: 'Edit transcripts and imported documents, and draft reports in a professional, integrated Lexical editor. Export your final research products to your preferred formats, including DOCX and Markdown.',
      icon: Edit3,
      accent: 'bg-amber-500',
      image: 'richtext-editing.png'
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

    // Auto-rotation logic
    if (browser) {
      interval = setInterval(nextSlide, SLIDE_DURATION);
    }
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });

  function nextSlide() {
    currentSlide = (currentSlide + 1) % slides.length;
    resetInterval();
  }

  function prevSlide() {
    currentSlide = (currentSlide - 1 + slides.length) % slides.length;
    resetInterval();
  }

  function goToSlide(index) {
    if (index === currentSlide) return;
    currentSlide = index;
    resetInterval();
  }

  function resetInterval() {
    if (interval) {
      clearInterval(interval);
      interval = setInterval(nextSlide, SLIDE_DURATION);
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
<section class="py-24 border-t border-slate-100 overflow-hidden bg-slate-50/30">
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

    <!-- Redesigned 75/25 Split Carousel -->
    <div
      class="bg-white border border-slate-200 rounded-[2.5rem] shadow-2xl overflow-hidden min-h-[600px] flex flex-col lg:flex-row p-6 lg:p-12 gap-8 lg:gap-16"
    >
      <!-- Carousel Area (75%) -->
      <div class="lg:w-3/4 flex flex-col justify-center">
        <div 
          class="relative bg-black px-2 py-1.5 lg:px-2.5 lg:py-2 rounded-2xl shadow-[0_40px_80px_-20px_rgba(0,0,0,0.6)] border border-white/5 w-full transition-all duration-500"
        >
          <div class="relative w-full aspect-video overflow-hidden rounded-xl group/carousel">
            {#each slides as slide, i}
              {#if currentSlide === i}
                <div 
                  class="absolute inset-0 flex items-center justify-center p-2"
                  in:fade={{ duration: 600 }}
                  out:fade={{ duration: 300 }}
                >
                  <img 
                    src="{base}/{slide.image}" 
                    alt={slide.title}
                    class="w-full h-full object-contain"
                  />
                </div>
              {/if}
            {/each}

            <!-- Custom Controls -->
            <button 
              on:click={prevSlide}
              class="absolute left-4 top-1/2 -translate-y-1/2 p-2 bg-black/20 hover:bg-black/40 text-white rounded-full transition-all opacity-0 group-hover/carousel:opacity-100 backdrop-blur-sm"
              aria-label="Previous"
            >
              <ChevronLeft class="w-6 h-6" />
            </button>
            <button 
              on:click={nextSlide}
              class="absolute right-4 top-1/2 -translate-y-1/2 p-2 bg-black/20 hover:bg-black/40 text-white rounded-full transition-all opacity-0 group-hover/carousel:opacity-100 backdrop-blur-sm"
              aria-label="Next"
            >
              <ChevronRight class="w-6 h-6" />
            </button>

            <!-- Indicators -->
            <div class="absolute bottom-6 left-1/2 -translate-x-1/2 flex gap-2 z-10">
              {#each slides as _, i}
                <button 
                  on:click={() => goToSlide(i)}
                  class="h-1.5 rounded-full transition-all duration-300 {currentSlide === i ? 'bg-white w-8' : 'bg-white/30 w-3 hover:bg-white/50'}"
                  aria-label="Go to slide {i + 1}"
                ></button>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- Content Area (25%) -->
      <div class="lg:w-1/4 flex flex-col justify-center py-8">
        {#key currentSlide}
          <div in:fade={{ duration: 400 }}>
            <div class="flex items-center gap-3 mb-6">
              <div class="h-10 w-10 rounded-xl {slides[currentSlide].accent} bg-opacity-10 flex items-center justify-center text-slate-900 shadow-sm border border-slate-100">
                <svelte:component this={slides[currentSlide].icon} class="w-5 h-5" />
              </div>
              <span class="text-xs font-bold text-green-600 uppercase tracking-widest leading-none">Feature</span>
            </div>
            
            <h3 class="text-3xl lg:text-4xl font-black text-slate-900 tracking-tight mb-6 leading-tight">
              {slides[currentSlide].title}
            </h3>
            
            <p class="text-lg text-slate-600 leading-relaxed font-medium mb-8">
              {slides[currentSlide].description}
            </p>

            <div class="flex gap-2">
              {#each slides as _, i}
                <button 
                  on:click={() => goToSlide(i)}
                  class="h-1.5 rounded-full transition-all duration-500 {currentSlide === i ? 'bg-green-500 w-8' : 'bg-slate-200 w-4 hover:bg-slate-300'}"
                  aria-label="Go to slide {i + 1}"
                ></button>
              {/each}
            </div>
          </div>
        {/key}
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
