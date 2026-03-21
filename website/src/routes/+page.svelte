<script>
    import { base } from '$app/paths';
    import { ArrowRight, WifiOff, Code, Lock, Download, Github, ChevronLeft, ChevronRight } from '@lucide/svelte';
    import { onMount, onDestroy } from 'svelte';

    let activeTab = 'windows';
    let isMac = false;

    // Carousel logic
    let currentSlide = 0;
    const slides = [
        { title: 'Transcription', description: 'Convert audio and video to text using local AI models.', color: 'bg-blue-500' },
        { title: 'Translation', description: 'Translate your transcripts into English instantly.', color: 'bg-purple-500' },
        { title: 'Model Management', description: 'Download and manage various Whisper and translation models.', color: 'bg-green-500' },
        { title: 'Rich Text Editor', description: 'Edit transcripts and documents with professional tools.', color: 'bg-amber-500' },
        { title: 'Image Annotation', description: 'Mark up images and PDFs directly within the app.', color: 'bg-rose-500' },
        { title: 'Table Management', description: 'View and edit CSV and XLSX files seamlessly.', color: 'bg-emerald-500' },
        { title: 'Media Sync', description: 'Transcripts stay in sync with media playback automatically.', color: 'bg-indigo-500' },
        { title: 'Flexible Export', description: 'Export your work to DOCX, CSV, and other formats.', color: 'bg-slate-700' }
    ];

    let interval;
    onMount(() => {
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
        clearInterval(interval);
        interval = setInterval(nextSlide, 5000);
    }

    function setActiveTab(tab) {
        activeTab = tab;
    }
</script>

<!-- Hero -->
<section class="py-16 lg:py-24 text-center">
    <h1 class="text-4xl lg:text-6xl font-bold tracking-tight text-slate-900 mb-6">
        Qualitative Research, <span class="text-green-500">Reimagined.</span>
    </h1>
    <p class="text-lg lg:text-xl text-slate-600 max-w-2xl mx-auto mb-10 leading-relaxed">
        Harvey is an integrated desktop workspace for transcription, translation, and qualitative analysis.
        Built by researchers, for researchers.
    </p>
    <div class="flex flex-col sm:flex-row gap-4 justify-center">
        <a href="#download" class="inline-flex items-center justify-center px-6 py-3 text-base font-semibold text-white transition-all bg-green-500 rounded-xl hover:bg-green-600 shadow-lg shadow-green-200 hover:shadow-green-300 gap-2">
            <Download class="w-5 h-5" />
            Download for Free
        </a>
        <a href="{base}/about" class="inline-flex items-center justify-center px-6 py-3 text-base font-semibold text-slate-700 transition-all bg-white border border-slate-200 rounded-xl hover:bg-slate-50 hover:text-slate-900 shadow-sm">
            Learn More
        </a>
    </div>
</section>

<!-- Features -->
<section class="py-16 border-t border-slate-100">
    <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
        <!-- Card 1 -->
        <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6">
                <Lock class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Privacy First</h3>
            <p class="text-slate-600 leading-relaxed">
                All AI processing happens locally on your device. Your data never leaves your computer.
            </p>
        </div>

        <!-- Card 2 -->
        <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6">
                <WifiOff class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Offline AI</h3>
            <p class="text-slate-600 leading-relaxed">
                Download state-of-the-art models once and use them forever. Transcription and translation run completely offline on your local machine.
            </p>
        </div>

        <!-- Card 3 -->
        <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-green-500 mb-6">
                <Code class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Open Source</h3>
            <p class="text-slate-600 leading-relaxed">
                Harvey is free and open source. We warmly welcome contributions from researchers and developers to shape the future of qualitative tools.
            </p>
        </div>
    </div>
</section>

<!-- App Showcase Carousel -->
<section class="py-24 border-t border-slate-100 overflow-hidden">
    <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="text-center mb-12">
            <h2 class="text-3xl font-bold text-slate-900 mb-4">Powerful Tools for Qualitative Research</h2>
            <p class="text-slate-600">Explore the features that make Harvey the perfect companion for your research.</p>
        </div>

        <div class="relative group">
            <!-- Carousel Container -->
            <div class="relative aspect-[16/10] w-full overflow-hidden rounded-3xl bg-slate-900 shadow-2xl border-8 border-slate-800">
                {#each slides as slide, i}
                    <div 
                        class="absolute inset-0 transition-all duration-700 ease-in-out flex flex-col items-center justify-center p-12 text-center"
                        style="opacity: {currentSlide === i ? 1 : 0}; transform: translateX({(i - currentSlide) * 20}px); visibility: {currentSlide === i ? 'visible' : 'hidden'}"
                    >
                        <!-- Image Placeholder / Real Image -->
                        {#if i === 0}
                            <img src="{base}/1_welcome-screen.png" alt="Welcome Screen" class="absolute inset-0 w-full h-full object-cover opacity-40" />
                        {/if}
                        
                        <div class="relative z-10 space-y-4 max-w-md">
                            <div class="inline-block px-3 py-1 rounded-full text-xs font-bold uppercase tracking-widest text-white/80 bg-white/10 backdrop-blur-md mb-2">
                                Feature Showcase
                            </div>
                            <h3 class="text-3xl md:text-4xl font-bold text-white tracking-tight">{slide.title}</h3>
                            <p class="text-lg text-slate-300 leading-relaxed">{slide.description}</p>
                        </div>

                        <!-- Gradient Overlay -->
                        <div class="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-900/40 to-transparent"></div>
                    </div>
                {/each}
            </div>

            <!-- Controls -->
            <button 
                on:click={prevSlide}
                class="absolute left-4 top-1/2 -translate-y-1/2 p-2 rounded-full bg-white/10 backdrop-blur-md text-white border border-white/20 opacity-0 group-hover:opacity-100 transition-all hover:bg-white/20"
                aria-label="Previous slide"
            >
                <ChevronLeft class="w-6 h-6" />
            </button>
            <button 
                on:click={nextSlide}
                class="absolute right-4 top-1/2 -translate-y-1/2 p-2 rounded-full bg-white/10 backdrop-blur-md text-white border border-white/20 opacity-0 group-hover:opacity-100 transition-all hover:bg-white/20"
                aria-label="Next slide"
            >
                <ChevronRight class="w-6 h-6" />
            </button>

            <!-- Indicators -->
            <div class="flex justify-center gap-2 mt-8">
                {#each slides as _, i}
                    <button 
                        on:click={() => goToSlide(i)}
                        class="h-1.5 transition-all rounded-full {currentSlide === i ? 'w-8 bg-green-500' : 'w-2 bg-slate-300 hover:bg-slate-400'}"
                        aria-label="Go to slide {i + 1}"
                    ></button>
                {/each}
            </div>
        </div>
    </div>
</section>

<!-- Download Section -->
<section id="download" class="py-24 border-t border-slate-100 scroll-mt-24">
    <div class="max-w-4xl mx-auto text-center">
        <h2 class="text-3xl lg:text-4xl font-bold text-slate-900 mb-4">Get Harvey</h2>
        <p class="text-lg text-slate-600 mb-10">Choose your platform to start your research journey.</p>

        <!-- Tabs -->
        <div class="flex justify-center mb-8 border-b border-slate-200">
            <button
                class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'windows' ? 'border-green-500 text-green-600' : 'border-transparent text-slate-500 hover:text-slate-700'}"
                on:click={() => setActiveTab('windows')}
            >
                Windows
            </button>
            <button
                class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'macos' ? 'border-green-500 text-green-600' : 'border-transparent text-slate-500 hover:text-slate-700'}"
                on:click={() => setActiveTab('macos')}
            >
                macOS
            </button>
            <button
                class="px-6 py-3 font-medium text-sm transition-colors border-b-2 {activeTab === 'source' ? 'border-green-500 text-green-600' : 'border-transparent text-slate-500 hover:text-slate-700'}"
                on:click={() => setActiveTab('source')}
            >
                Build from Source
            </button>
        </div>

        <!-- Tab Content -->
        <div class="bg-slate-50 border border-slate-100 rounded-2xl p-8 lg:p-12 transition-all">

            {#if activeTab === 'windows'}
                <div class="flex flex-col items-center">
                    <div class="h-16 w-16 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mb-6">
                        <svg viewBox="0 0 24 24" fill="currentColor" class="w-8 h-8"><path d="M0 3.449L9.75 2.1v9.451H0V3.449zm10.949-1.323L24 0v11.551H10.949V2.126zm-10.949 9.924H9.75v9.451L0 20.551V12.05zm10.949 0H24v11.551l-13.051-2.126V12.05z"/></svg>
                    </div>
                    <h3 class="text-2xl font-bold text-slate-900 mb-2">Harvey for Windows</h3>
                    <p class="text-slate-600 mb-4 max-w-md">Compatible with Windows 10 and 11.</p>
                    <p class="text-sm text-slate-500 mb-8">For detailed installation instructions, see the <a href="{base}/help/downloads" class="text-green-600 hover:underline">Help Center</a>.</p>
                    <a href="https://github.com/Ethnomethodology/harvey/releases/download/main/Harvey_0.1.0_x64-setup.zip" class="inline-flex items-center justify-center px-8 py-4 text-lg font-bold text-white transition-all bg-green-500 rounded-xl hover:bg-green-600 shadow-lg shadow-green-200 hover:shadow-green-300 gap-2">
                        <Download class="w-6 h-6" />
                        Download .exe Installer
                    </a>
                    <p class="text-xs text-slate-400 mt-4">Version 0.1.0 • 64-bit</p>
                </div>
            {/if}

            {#if activeTab === 'macos'}
                <div class="flex flex-col items-center">
                    <div class="h-16 w-16 bg-gray-100 text-gray-900 rounded-2xl flex items-center justify-center mb-6">
                        <svg viewBox="0 0 24 24" fill="currentColor" class="w-8 h-8"><path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.21-1.98 1.08-3.11-1.06.05-2.31.71-3.06 1.48-.69.72-1.24 1.87-1.09 2.98 1.18.08 2.36-.67 3.07-1.35"/></svg>
                    </div>
                    <h3 class="text-2xl font-bold text-slate-900 mb-2">Harvey for macOS</h3>
                    <p class="text-slate-600 mb-4 max-w-md"> Optimized for Apple Silicon (M1, M2, etc.) and Intel Macs.</p>
                    <p class="text-sm text-slate-500 mb-8">For detailed installation instructions, see the <a href="{base}/help/downloads" class="text-green-600 hover:underline">Help Center</a>.</p>
                    <div class="flex flex-col sm:flex-row gap-4">
                        <a href="https://github.com/Ethnomethodology/harvey/releases/download/main/Harvey_0.1.0_aarch64.dmg" class="inline-flex items-center justify-center px-6 py-3 text-base font-bold text-white transition-all bg-slate-900 rounded-xl hover:bg-slate-800 shadow-lg shadow-slate-200 gap-2">
                            <Download class="w-5 h-5" />
                            Apple Silicon
                        </a>
                        <a href="https://github.com/Ethnomethodology/harvey/releases/download/main/Harvey_0.1.0_x64.dmg" class="inline-flex items-center justify-center px-6 py-3 text-base font-bold text-white transition-all bg-slate-900 rounded-xl hover:bg-slate-800 shadow-lg shadow-slate-200 gap-2">
                            <Download class="w-5 h-5" />
                            Intel Mac
                        </a>
                    </div>
                    <p class="text-xs text-slate-400 mt-4">Version 0.1.0 • Universal Binary available</p>
                </div>
            {/if}

            {#if activeTab === 'source'}
                <div class="flex flex-col items-center">
                    <div class="h-16 w-16 bg-slate-100 text-slate-900 rounded-2xl flex items-center justify-center mb-6">
                        <Github class="w-8 h-8" />
                    </div>
                    <h3 class="text-2xl font-bold text-slate-900 mb-2">Build from Source</h3>
                    <p class="text-slate-600 mb-4 max-w-md">Clone the repository and build Harvey for your Linux distribution or custom environment.</p>
                    <p class="text-sm text-slate-500 mb-8">For detailed build instructions, see the <a href="{base}/help/downloads" class="text-green-600 hover:underline">Help Center</a>.</p>

                    <div class="w-full max-w-lg bg-slate-900 rounded-lg p-4 text-left font-mono text-sm text-green-400 mb-6 overflow-x-auto">
                        <div class="flex select-none mb-2 text-slate-500">
                            <span class="mr-2">$</span>
                            <span># Clone and install dependencies</span>
                        </div>
                        <div class="mb-2">
                            <span class="mr-2 text-slate-500">$</span>
                            <span class="text-white">git clone https://github.com/Ethnomethodology/harvey.git</span>
                        </div>
                        <div class="mb-2">
                            <span class="mr-2 text-slate-500">$</span>
                            <span class="text-white">cd harvey && npm install</span>
                        </div>
                         <div>
                            <span class="mr-2 text-slate-500">$</span>
                            <span class="text-white">npm run tauri build</span>
                        </div>
                    </div>

                    <a href="https://github.com/Ethnomethodology/harvey" target="_blank" class="inline-flex items-center justify-center px-8 py-3 text-base font-bold text-slate-700 transition-all bg-white border border-slate-200 rounded-xl hover:bg-slate-50 gap-2">
                        View on GitHub
                    </a>
                </div>
            {/if}

        </div>
    </div>
</section>
