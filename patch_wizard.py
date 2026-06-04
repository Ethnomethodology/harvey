import re

with open('src/lib/components/welcome/SetupWizardModal.svelte', 'r') as f:
    content = f.read()

# 1. Imports
content = content.replace(
    'availableWhisperCppModels,\n    availableFasterWhisperModels\n  } from \'$lib/constants/models.js\';',
    'availableWhisperCppModels,\n    availableFasterWhisperModels,\n    availableCrisperWhisperModels\n  } from \'$lib/constants/models.js\';'
)
content = content.replace(
    'downloadFasterWhisperModel,',
    'downloadFasterWhisperModel,\n    downloadCrisperWhisperModel,'
)

# 2. State variables
content = content.replace(
    'let transcriptionEngines = $state({ whisperCpp: false, fasterWhisper: false });',
    'let transcriptionEngines = $state({ whisperCpp: false, fasterWhisper: false, crisperWhisper: false });'
)
content = content.replace(
    'let selectedFasterWhisperModels = $state([\'Systran/faster-whisper-base\']);',
    'let selectedFasterWhisperModels = $state([\'Systran/faster-whisper-base\']);\n  let selectedCrisperWhisperModels = $state([\'nyrahealth/faster_CrisperWhisper\']);'
)

# 3. resetWizard
content = content.replace(
    'transcriptionEngines.fasterWhisper = recommendFasterWhisper;',
    'transcriptionEngines.fasterWhisper = recommendFasterWhisper;\n    transcriptionEngines.crisperWhisper = false;'
)
content = content.replace(
    'selectedFasterWhisperModels = [\'Systran/faster-whisper-base\'];',
    'selectedFasterWhisperModels = [\'Systran/faster-whisper-base\'];\n    selectedCrisperWhisperModels = [\'nyrahealth/faster_CrisperWhisper\'];'
)

# 4. selectedModelsSummary
summary_insert = """
    if (transcriptionEngines.crisperWhisper) {
      selectedCrisperWhisperModels.forEach((name) => {
        const m = availableCrisperWhisperModels.find((am) => am.name === name);
        if (m) {
          models.push({ name: m.name.split('/').pop(), size: m.size, type: 'crisper-whisper' });
          totalSizeMiB += parseSize(m.size);
        }
      });
    }"""
content = content.replace(
    '    if (transcriptionEngines.fasterWhisper) {',
    summary_insert + '\n    if (transcriptionEngines.fasterWhisper) {'
)

# 5. nextStep/prevStep
# It's easier to just replace the whole nextStep/prevStep block.
new_steps = """
  async function nextStep() {
    diarizationError = '';
    if (currentStep === 1) {
      currentStep = 2;
    } else if (currentStep === 2) {
      installProgress.phase = 'idle';
      installProgress.current = 0;
      installProgress.total = 0;
      installProgress.currentItem = '';

      if (transcriptionEngines.whisperCpp) currentStep = 3;
      else if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (translationEngines.helsinki) currentStep = 6;
      else if (translationEngines.nllb) currentStep = 7;
      else currentStep = 8;
    } else if (currentStep === 3) {
      if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (translationEngines.helsinki) currentStep = 6;
      else if (translationEngines.nllb) currentStep = 7;
      else currentStep = 8;
    } else if (currentStep === 4) {
      if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (translationEngines.helsinki) currentStep = 6;
      else if (translationEngines.nllb) currentStep = 7;
      else currentStep = 8;
    } else if (currentStep === 5) {
      if (translationEngines.helsinki) currentStep = 6;
      else if (translationEngines.nllb) currentStep = 7;
      else currentStep = 8;
    } else if (currentStep === 6) {
      if (translationEngines.nllb) currentStep = 7;
      else currentStep = 8;
    } else if (currentStep === 7) {
      currentStep = 8;
    } else if (currentStep === 8) {
      currentStep = 9;
    }
  }

  function prevStep() {
    diarizationError = '';
    if (currentStep === 2) currentStep = 1;
    else if (currentStep === 3) currentStep = 2;
    else if (currentStep === 4) {
      if (transcriptionEngines.whisperCpp) currentStep = 3;
      else currentStep = 2;
    } else if (currentStep === 5) {
      if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.whisperCpp) currentStep = 3;
      else currentStep = 2;
    } else if (currentStep === 6) {
      if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.whisperCpp) currentStep = 3;
      else currentStep = 2;
    } else if (currentStep === 7) {
      if (translationEngines.helsinki) currentStep = 6;
      else if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.whisperCpp) currentStep = 3;
      else currentStep = 2;
    } else if (currentStep === 8) {
      if (translationEngines.nllb) currentStep = 7;
      else if (translationEngines.helsinki) currentStep = 6;
      else if (transcriptionEngines.crisperWhisper) currentStep = 5;
      else if (transcriptionEngines.fasterWhisper) currentStep = 4;
      else if (transcriptionEngines.whisperCpp) currentStep = 3;
      else currentStep = 2;
    } else if (currentStep === 9) {
      currentStep = 8;
    }
  }
"""

content = re.sub(r'async function nextStep\(\) \{.*?\n  \}', new_steps.strip(), content, flags=re.DOTALL)


with open('src/lib/components/welcome/SetupWizardModal.svelte', 'w') as f:
    f.write(content)

