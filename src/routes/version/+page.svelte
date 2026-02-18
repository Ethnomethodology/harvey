<script>
    const buildDate = new Date(__BUILD_DATE__).toLocaleDateString(undefined, {
        dateStyle: 'medium'
    });
</script>

<div class="h-screen w-screen bg-gray-100 dark:bg-gray-950 p-8 flex flex-col items-center justify-center text-center">
    <img src="/logo.png" alt="Harvey Logo" class="w-20 h-20 mb-2 [filter:drop-shadow(0_5px_5px_rgba(0,0,0,0.2))]" />
    <h1 class="text-2xl font-bold mb-4 text-gray-900 dark:text-gray-100">Harvey</h1>
    <p class="text-lg text-gray-700 dark:text-gray-300">Version 0.1.0</p>
    <p class="text-sm text-gray-500 dark:text-gray-400 mt-2">Build Date: {buildDate}</p>
    
    <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700 w-full">
        <p class="text-xs text-gray-600 dark:text-gray-400 mb-2">Check for updated version:</p>
        <a 
            href="https://github.com/Ethnomethodology/harvey/releases/latest" 
            target="_blank" 
            class="text-xs text-blue-600 dark:text-blue-400 hover:underline whitespace-nowrap"
        >
            https://github.com/Ethnomethodology/harvey/releases/latest
        </a>
    </div>
</div>
