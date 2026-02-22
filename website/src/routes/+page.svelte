<script>
    import { base } from '$app/paths';
    import { ArrowRight, Mic, FileText, Lock } from 'lucide-svelte';
</script>

<!-- Hero -->
<section class="py-16 lg:py-24 text-center">
    <h1 class="text-4xl lg:text-6xl font-bold tracking-tight text-slate-900 mb-6">
        Qualitative Research, <span class="text-emerald-500">Reimagined.</span>
    </h1>
    <p class="text-lg lg:text-xl text-slate-600 max-w-2xl mx-auto mb-10 leading-relaxed">
        Harvey is an all-in-one desktop app for transcription, translation, and analysis.
        Built by researchers, for researchers. 100% Offline & Private.
    </p>
    <div class="flex flex-col sm:flex-row gap-4 justify-center">
        <a href="https://github.com/Ethnomethodology/harvey/releases" class="inline-flex items-center justify-center px-6 py-3 text-base font-semibold text-white transition-all bg-emerald-500 rounded-xl hover:bg-emerald-600 shadow-lg shadow-emerald-200 hover:shadow-emerald-300">
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
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-emerald-500 mb-6">
                <Lock class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Privacy First</h3>
            <p class="text-slate-600 leading-relaxed">
                All AI processing happens locally on your device. Your data never leaves your computer.
            </p>
        </div>

        <!-- Card 2 -->
        <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-emerald-500 mb-6">
                <Mic class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Offline AI</h3>
            <p class="text-slate-600 leading-relaxed">
                State-of-the-art transcription (Whisper) and translation models running without internet.
            </p>
        </div>

        <!-- Card 3 -->
        <div class="bg-slate-50 p-8 rounded-2xl border border-slate-100">
            <div class="h-12 w-12 bg-white rounded-xl shadow-sm flex items-center justify-center text-emerald-500 mb-6">
                <FileText class="w-6 h-6" />
            </div>
            <h3 class="text-xl font-bold text-slate-900 mb-3">Rich Editing</h3>
            <p class="text-slate-600 leading-relaxed">
                Interactive transcript editor linked to media playback, with annotation tools built-in.
            </p>
        </div>
    </div>
</section>
