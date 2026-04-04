<!-- src/lib/components/shared/AboutBody.svelte -->
<script>
  export let showTitle = true;
</script>

<div>
  {#if showTitle}
    <h2
      class="text-2xl font-bold mb-4 text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800"
    >
      About Harvey
    </h2>
  {/if}

  <p class="text-lg text-gray-700 dark:text-gray-300 mb-4">
    Harvey is a desktop application for qualitative researchers and anyone working with multimedia
    content. It streamlines the research workflow by integrating a powerful suite of tools into a
    single, cohesive environment:
  </p>

  <ul class="list-disc list-inside text-gray-700 dark:text-gray-300 space-y-2 pl-4 mb-6">
    <li><strong>Transcribe</strong> audio and video with local AI models.</li>
    <li><strong>Translate</strong> transcripts into English.</li>
    <li>
      <strong>Edit</strong> and <strong>annotate</strong> transcripts, documents, PDFs, and images.
    </li>
    <li><strong>Manage</strong> all your project files in one place.</li>
  </ul>

  <p class="text-gray-700 dark:text-gray-300 mb-6">
    Harvey is named in honor of Sociologist <a
      href="https://en.wikipedia.org/wiki/Harvey_Sacks"
      target="_blank"
      rel="noopener noreferrer"
      class="text-blue-600 dark:text-blue-400 hover:underline"><strong>Harvey Sacks</strong></a
    > (1935-1975), whose foundational work revolutionized social research methods.
  </p>

  <div class="pt-4">
    <h4 class="text-xl font-bold mb-2 text-gray-900 dark:text-white">Privacy First</h4>
    <p class="text-gray-700 dark:text-gray-300 leading-relaxed text-base">
      Privacy is the core of Harvey's design. All core AI functionalities, like transcription and
      diarization (speaker identification), run <strong>100% locally</strong> on your computer. You download
      the AI models once and can use them forever offline. Your data never leaves your device.
    </p>
  </div>

  <blockquote
    class="border-l-4 border-blue-500 dark:border-blue-600 pl-4 italic text-gray-600 dark:text-gray-400 mt-8 text-lg"
  >
    It's an application built <em>by researchers, for researchers</em>.
  </blockquote>

  <div class="pt-8 mt-8 border-t border-gray-100 dark:border-gray-800 flex items-center space-x-4">
    <a
      href="https://github.com/Ethnomethodology/harvey"
      target="_blank"
      rel="noopener noreferrer"
      class="text-blue-600 dark:text-blue-400 hover:underline">Harvey on GitHub</a
    >
    <span class="text-gray-300">|</span>
    <a
      href="https://github.com/Ethnomethodology/harvey/blob/main/LICENSE.md"
      target="_blank"
      rel="noopener noreferrer"
      class="text-blue-600 dark:text-blue-400 hover:underline">MIT License</a
    >
  </div>
</div>
