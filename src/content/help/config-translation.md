---
id: config-translation
label: Translation Engine
sidebarId: configure
order: 14
---

Harvey provides two distinct model families for local translation. You can switch between them in the **Configure** tab under the **Translation** section.

> **Note:** Only models belonging to the currently selected model family will be available for translation. If you switch families, you must download at least one compatible model for that family.

#### 1. Helsinki-NLP
*   **Pros:** Lightweight and very fast on CPUs. Provides high quality for common language pairs.
*   **Cons:** Requires downloading a separate model for every language pair (e.g., Japanese to English, French to English).
*   **Recommendation:** Best for users who primarily work with a few specific, common languages and value speed on standard hardware.

#### 2. NLLB (Meta)
*   **Pros:** A single model supports over 200 languages. Excellent for rare or less common languages.
*   **Cons:** Much larger file sizes and slower on lower-end CPUs. Performance is best with a dedicated GPU.
*   **Recommendation:** Best for researchers working with a wide variety of languages or rare dialects where specific Helsinki-NLP pairs may not exist.

### CTranslate2 Backend
Both engines leverage the **CTranslate2** backend for optimized inference. If prompted, ensure you install the required dependencies via the "Install Now" button in the Configure tab to enable high-speed translation.
