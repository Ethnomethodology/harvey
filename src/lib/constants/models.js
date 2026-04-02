// src/lib/constants/models.js

const WHISPER_CPP_INFO_URL = 'https://huggingface.co/ggerganov/whisper.cpp';
const HUGGING_FACE_BASE = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main';

export const availableWhisperCppModels = [
  {
    name: 'ggml-tiny',
    language: 'Multilingual',
    size: '75 MiB',
    description: 'Smallest and fastest multilingual model. Recommended for testing.',
    download_url: `${HUGGING_FACE_BASE}/ggml-tiny.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-tiny.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-tiny.en',
    language: 'English-only',
    size: '75 MiB',
    description: 'Smallest and fastest for English. Ideal for limited resources.',
    download_url: `${HUGGING_FACE_BASE}/ggml-tiny.en.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-tiny.en.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-base',
    language: 'Multilingual',
    size: '142 MiB',
    description: 'Fast and lightweight for multilingual use. Good for clear audio.',
    download_url: `${HUGGING_FACE_BASE}/ggml-base.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-base.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-base.en',
    language: 'English-only',
    size: '142 MiB',
    description: 'Fast and lightweight for English. Good for clear audio.',
    download_url: `${HUGGING_FACE_BASE}/ggml-base.en.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-base.en.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-small',
    language: 'Multilingual',
    size: '466 MiB',
    description: 'Balanced speed and accuracy for multilingual use. Recommended for most users.',
    download_url: `${HUGGING_FACE_BASE}/ggml-small.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-small.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-small.en',
    language: 'English-only',
    size: '466 MiB',
    description: 'Excellent balance of speed and accuracy for English. Recommended for most users.',
    download_url: `${HUGGING_FACE_BASE}/ggml-small.en.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-small.en.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-large-v3-turbo-q5_0',
    language: 'Multilingual',
    size: '1.1 GiB',
    description: 'Efficient high-performance model. Great balance of speed and quality.',
    download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo-q5_0.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3-turbo-q5_0.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-medium',
    language: 'Multilingual',
    size: '1.5 GiB',
    description: 'High accuracy for multilingual use. Requires more memory and time.',
    download_url: `${HUGGING_FACE_BASE}/ggml-medium.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-medium.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-medium.en',
    language: 'English-only',
    size: '1.5 GiB',
    description: 'Highest accuracy for English. Requires more memory and time.',
    download_url: `${HUGGING_FACE_BASE}/ggml-medium.en.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-medium.en.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-large-v3-turbo',
    language: 'Multilingual',
    size: '1.5 GiB',
    description: 'Optimized for speed. High quality and very fast.',
    download_url: `${HUGGING_FACE_BASE}/ggml-large-v3-turbo.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3-turbo.bin`,
    family: 'whisper-cpp'
  },
  {
    name: 'ggml-large-v3',
    language: 'Multilingual',
    size: '2.9 GiB',
    description: 'Best accuracy for multilingual use. Slowest processing.',
    download_url: `${HUGGING_FACE_BASE}/ggml-large-v3.bin`,
    info_url: `${WHISPER_CPP_INFO_URL}/blob/main/ggml-large-v3.bin`,
    family: 'whisper-cpp'
  }
];

export const availableFasterWhisperModels = [
  {
    name: 'Systran/faster-whisper-tiny',
    language: 'Multilingual',
    size: '75 MiB',
    description: 'Smallest and fastest multilingual model. Recommended for testing.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-tiny'
  },
  {
    name: 'Systran/faster-whisper-tiny.en',
    language: 'English-only',
    size: '75 MiB',
    description: 'Smallest and fastest English-only model. Ideal for limited resources.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-tiny.en'
  },
  {
    name: 'Systran/faster-whisper-base',
    language: 'Multilingual',
    size: '145 MiB',
    description: 'Very fast multilingual model. Good for clear audio and basic needs.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-base'
  },
  {
    name: 'Systran/faster-whisper-base.en',
    language: 'English-only',
    size: '145 MiB',
    description: 'Very fast English-only model. Good for clear audio and basic needs.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-base.en'
  },
  {
    name: 'Systran/faster-whisper-distil-small.en',
    language: 'English-only',
    size: '250 MiB',
    description: 'Distilled Small English model. Very fast with good accuracy.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-distil-small.en'
  },
  {
    name: 'Systran/faster-whisper-small',
    language: 'Multilingual',
    size: '484 MiB',
    description: 'Balanced speed and accuracy for multilingual use. Faster than Medium.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-small'
  },
  {
    name: 'Systran/faster-whisper-small.en',
    language: 'English-only',
    size: '484 MiB',
    description: 'Efficient for English audio. Balanced speed and accuracy.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-small.en'
  },
  {
    name: 'Systran/faster-whisper-distil-medium.en',
    language: 'English-only',
    size: '800 MiB',
    description: 'Distilled Medium English model. Extremely fast and highly accurate.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-distil-medium.en'
  },
  {
    name: 'Systran/faster-whisper-medium',
    language: 'Multilingual',
    size: '1.5 GiB',
    description: 'High accuracy for multilingual use. Great for general purpose.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-medium'
  },
  {
    name: 'Systran/faster-whisper-medium.en',
    language: 'English-only',
    size: '1.5 GiB',
    description: 'Highest accuracy for English audio. Requires more resources.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-medium.en'
  },
  {
    name: 'Systran/faster-whisper-distil-large-v3',
    language: 'Multilingual',
    size: '1.6 GiB',
    description: 'Distilled Large V3. significantly faster with minimal accuracy loss.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-distil-large-v3'
  },
  {
    name: 'Systran/faster-whisper-distil-large-v2',
    language: 'Multilingual',
    size: '1.6 GiB',
    description: 'Distilled Large V2. Significantly faster version of Large V2.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-distil-large-v2'
  },
  {
    name: 'Systran/faster-whisper-large-v3',
    language: 'Multilingual',
    size: '3.1 GiB',
    description: 'Best accuracy for multilingual use. Slowest processing.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-large-v3'
  },
  {
    name: 'Systran/faster-whisper-large-v2',
    language: 'Multilingual',
    size: '3.1 GiB',
    description: 'Previous generation large model. Stable and high quality.',
    family: 'faster-whisper',
    info_url: 'https://huggingface.co/Systran/faster-whisper-large-v2'
  }
];
