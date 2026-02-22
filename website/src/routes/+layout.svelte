<script>
    import "../app.css";
    import { base } from '$app/paths';
    import { page } from '$app/stores';
    import { Github, Menu, X } from 'lucide-svelte';

    let isMenuOpen = false;

    function toggleMenu() {
        isMenuOpen = !isMenuOpen;
    }

    // Close menu on navigation
    $: if ($page.url.pathname) {
        isMenuOpen = false;
    }
</script>

<div class="flex flex-col min-h-screen font-sans text-slate-600 antialiased selection:bg-emerald-100 selection:text-emerald-900">
    <!-- Navbar -->
    <header class="sticky top-0 z-40 w-full backdrop-blur flex-none transition-colors duration-500 lg:z-50 lg:border-b lg:border-slate-900/10 bg-white/95 supports-backdrop-blur:bg-white/60">
        <div class="max-w-7xl mx-auto">
            <div class="py-4 border-b border-slate-900/10 lg:px-8 lg:border-0 mx-4 lg:mx-0">
                <div class="relative flex items-center justify-between">
                    <!-- Logo -->
                    <a class="mr-3 flex-none w-[2.0625rem] overflow-hidden md:w-auto text-slate-900 hover:text-emerald-600 transition-colors font-bold text-xl tracking-tighter flex items-center gap-2" href="{base}/">
                        <div class="h-8 w-8 bg-emerald-500 rounded-lg flex items-center justify-center text-white font-bold text-lg shadow-sm shadow-emerald-200">H</div>
                        <span class="hidden md:inline">Harvey</span>
                    </a>

                    <!-- Desktop Nav -->
                    <div class="hidden lg:flex items-center gap-8 font-medium text-sm">
                        <a href="{base}/about" class="hover:text-emerald-600 transition-colors {$page.url.pathname === base + '/about' ? 'text-emerald-600' : ''}">About</a>
                        <a href="{base}/help" class="hover:text-emerald-600 transition-colors {$page.url.pathname.startsWith(base + '/help') ? 'text-emerald-600' : ''}">Help Center</a>
                        <a href="{base}/contribute" class="hover:text-emerald-600 transition-colors {$page.url.pathname === base + '/contribute' ? 'text-emerald-600' : ''}">Contribute</a>
                    </div>

                    <!-- CTA & GitHub -->
                    <div class="hidden lg:flex items-center gap-4">
                        <a href="https://github.com/Ethnomethodology/harvey" target="_blank" rel="noreferrer" class="text-slate-400 hover:text-slate-600 transition-colors">
                            <span class="sr-only">GitHub</span>
                            <Github class="w-5 h-5" />
                        </a>
                        <a href="https://github.com/Ethnomethodology/harvey/releases" target="_blank" rel="noreferrer" class="bg-emerald-500 hover:bg-emerald-600 text-white px-4 py-2 rounded-xl text-sm font-semibold transition-colors shadow-sm shadow-emerald-200">
                            Download
                        </a>
                    </div>

                    <!-- Mobile Menu Button -->
                    <button type="button" class="lg:hidden p-2 text-slate-500" on:click={toggleMenu}>
                        <span class="sr-only">Open main menu</span>
                        {#if isMenuOpen}
                            <X class="w-6 h-6" />
                        {:else}
                            <Menu class="w-6 h-6" />
                        {/if}
                    </button>
                </div>
            </div>
        </div>

        <!-- Mobile Menu -->
        {#if isMenuOpen}
            <div class="lg:hidden border-b border-slate-900/10 bg-white px-4 pt-2 pb-6 shadow-xl absolute w-full top-full left-0 z-50 flex flex-col gap-4">
                <a href="{base}/about" class="block text-base font-medium text-slate-700 hover:text-emerald-600">About</a>
                <a href="{base}/help" class="block text-base font-medium text-slate-700 hover:text-emerald-600">Help Center</a>
                <a href="{base}/contribute" class="block text-base font-medium text-slate-700 hover:text-emerald-600">Contribute</a>
                <div class="pt-4 mt-2 border-t border-slate-100 flex items-center gap-4">
                     <a href="https://github.com/Ethnomethodology/harvey/releases" class="flex-1 bg-emerald-500 text-center py-2 rounded-lg text-white font-medium">Download</a>
                     <a href="https://github.com/Ethnomethodology/harvey" class="p-2 text-slate-500"><Github class="w-6 h-6"/></a>
                </div>
            </div>
        {/if}
    </header>

    <!-- Main Content -->
    <main class="flex-auto w-full max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 lg:py-12">
        <slot />
    </main>

    <!-- Footer -->
    <footer class="border-t border-slate-200 bg-slate-50 py-12">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 flex flex-col md:flex-row justify-between items-center gap-6">
            <div class="flex items-center gap-2">
                <div class="h-6 w-6 bg-slate-200 rounded flex items-center justify-center text-slate-500 font-bold text-xs">H</div>
                <span class="text-sm font-semibold text-slate-900">Harvey Project</span>
            </div>
            <p class="text-sm text-slate-500">
                &copy; {new Date().getFullYear()} Project Harvey. Open Source (MIT).
            </p>
             <div class="flex gap-4 text-sm text-slate-500">
                <a href="{base}/about" class="hover:text-slate-900">About</a>
                <a href="{base}/help" class="hover:text-slate-900">Help</a>
            </div>
        </div>
    </footer>
</div>
