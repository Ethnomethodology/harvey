import re

with open('src/lib/components/welcome/SetupWizardModal.svelte', 'r') as f:
    text = f.read()

# 1. Fix selectedModelsSummary duplicates
text = re.sub(
    r'(    if \(transcriptionEngines\.crisperWhisper\) \{\n      selectedCrisperWhisperModels\.forEach\(\(name\) => \{\n        const m = availableCrisperWhisperModels\.find\(\(am\) => am\.name === name\);\n        if \(m\) \{\n          models\.push\(\{ name: m\.name\.split\(\'/\'\)\.pop\(\), size: m\.size, type: \'crisper-whisper\' \}\);\n          totalSizeMiB \+= parseSize\(m\.size\);\n        \}\n      \}\);\n    \}\n){2,}',
    r'\1',
    text
)

# 2. Fix startLibrariesInstall where models.push was injected erroneously
text = re.sub(
    r'    if \(transcriptionEngines\.crisperWhisper\) \{\n      selectedCrisperWhisperModels\.forEach\(\(name\) => \{\n        const m = availableCrisperWhisperModels\.find\(\(am\) => am\.name === name\);\n        if \(m\) \{\n          models\.push\(\{ name: m\.name\.split\(\'/\'\)\.pop\(\), size: m\.size, type: \'crisper-whisper\' \}\);\n          totalSizeMiB \+= parseSize\(m\.size\);\n        \}\n      \}\);\n    \}\n(?=    if \(transcriptionEngines\.fasterWhisper\) \{\n        installProgress\.currentItem = \'faster-whisper dependencies\';)',
    r'''    if (transcriptionEngines.crisperWhisper) {
        installProgress.currentItem = 'crisper-whisper dependencies';
        await invoke('install_crisper_whisper_dependencies_command');
      }\n''',
    text
)
# Also remove any remaining erroneous models.push blocks in startLibrariesInstall
text = re.sub(
    r'(    if \(transcriptionEngines\.crisperWhisper\) \{\n      selectedCrisperWhisperModels\.forEach\(\(name\) => \{\n        const m = availableCrisperWhisperModels\.find\(\(am\) => am\.name === name\);\n        if \(m\) \{\n          models\.push\(\{ name: m\.name\.split\(\'/\'\)\.pop\(\), size: m\.size, type: \'crisper-whisper\' \}\);\n          totalSizeMiB \+= parseSize\(m\.size\);\n        \}\n      \}\);\n    \}\n)+    if \(transcriptionEngines\.fasterWhisper\) \{\n        installProgress\.currentItem = \'faster-whisper dependencies\';',
    r'''    if (transcriptionEngines.crisperWhisper) {
        installProgress.currentItem = 'crisper-whisper dependencies';
        await invoke('install_crisper_whisper_dependencies_command');
      }
    if (transcriptionEngines.fasterWhisper) {
        installProgress.currentItem = 'faster-whisper dependencies';''',
    text
)

# 3. Fix startModelDownloads where models.push was injected erroneously
text = re.sub(
    r'(    if \(transcriptionEngines\.crisperWhisper\) \{\n      selectedCrisperWhisperModels\.forEach\(\(name\) => \{\n        const m = availableCrisperWhisperModels\.find\(\(am\) => am\.name === name\);\n        if \(m\) \{\n          models\.push\(\{ name: m\.name\.split\(\'/\'\)\.pop\(\), size: m\.size, type: \'crisper-whisper\' \}\);\n          totalSizeMiB \+= parseSize\(m\.size\);\n        \}\n      \}\);\n    \}\n)+',
    '',
    text
)
# We need to make sure the CORRECT modelsToDownload push is there
models_to_download_correct = r'''    if (transcriptionEngines.crisperWhisper) {
      selectedCrisperWhisperModels.forEach((name) => {
        const m = availableCrisperWhisperModels.find((am) => am.name === name);
        if (m) {
          modelsToDownload.push({ ...m, type: 'crisper-whisper' });
        }
      });
    }'''

if models_to_download_correct not in text:
    # insert it after fasterWhisper
    text = re.sub(
        r'(        \}\);\n      \}\n\n)',
        r'\1' + models_to_download_correct + '\n\n',
        text,
        count=1
    )

with open('src/lib/components/welcome/SetupWizardModal.svelte', 'w') as f:
    f.write(text)

