### 1. The "Native Rust" Approach (`rawpsd`)
As you noted, `rawpsd` is mostly unmaintained. Attempting to maintain a full-featured PSD parser from scratch in Rust is a project in itself that could take months or years.
*   **Verdict:** Avoid, unless you specifically want to contribute to an open-source Rust PSD parser as a side project. It won't get you to your goal quickly.

### 2. The "Industry Standard" Approach (`libpsd` / `ImageMagick` / `GraphicsMagick`)
Most open-source software (GIMP, Krita, etc.) do not write their own PSD parsers from scratch. They use battle-tested C/C++ libraries.

*   **GraphicsMagick / ImageMagick:** These handle PSDs, but they are designed for image processing, not **reconstructive editing**. They will often "flatten" the file or lose complex metadata like non-destructive adjustment layers or text layer properties.
*   **Verdict:** Good for previews, bad for a "Photoshop clone" where you need to preserve editability.

### 3. The Recommended Approach: Leverage GIMP or Krita's Logic

Since you are building for Linux, you have the advantage of being in the same ecosystem as the best open-source image editors.

#### Option A: Bindings to `libpsd` (The "Hard" Route)
There is a project called `libpsd`. It is ancient and largely abandoned. Do not use it.

#### Option B: Embed/Call Krita's `libkritapsd` (The "Pro" Route)
Krita has the best PSD support of any open-source project. They have a dedicated file filter architecture.
*   **The challenge:** Krita is a massive codebase. Extracting their PSD filter logic into a standalone library is difficult, but it is the most robust path if you want high-fidelity support (including layer styles and layer masks).
*   **Action:** Look at the [Krita source code](https://invent.kde.org/graphics/krita/-/tree/master/plugins/impex/psd) and see if the PSD import/export filter can be used as a standalone dependency.

#### Option C: The "Interop" approach (The Practical Route)
If you want to move fast, **do not write the parser yourself.** Instead, use a "bridge" approach:

1.  **Use `psd-tools` (Python):** This is the industry-standard library for PSD parsing. It is extremely mature and updated regularly.
2.  **The Bridge:** Write a small Python service or script that uses `psd-tools` to convert a PSD into an intermediate JSON format (representing your internal document structure) and exports the layers as PNGs.
3.  **Rust side:** Your Rust application reads the JSON and PNGs to reconstruct the document in your GTK4 app.

---

### My Recommendation

If I were building this today, I would pursue a **Hybrid Integration Strategy**:

1.  **Don't try to parse the binary PSD file directly in Rust** unless you are prepared to spend months reverse-engineering the format.
2.  **Use `psd-tools` (via a Python-to-Rust bridge)** for your initial MVP. It will handle the complex heavy lifting of PSD structure, and you can focus on building the GTK4 UI and the editing engine.
3.  **Future-proofing:** Define an "Intermediate File Format" (like an XML or JSON manifest + a folder of assets) for your application. This way, if you eventually find a better C library (or decide to build your own specialized parser), you only need to change the "Importer" part of your code, not your entire application architecture.

#### Essential reading for you:
*   **[PSD File Format Specification (Adobe)](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/):** You will need this as a reference no matter what route you choose.
*   **[PSD-Tools (Python)](https://github.com/psd-tools/psd-tools):** This is the gold standard for parsing. Even if you don't use it, study their documentation to understand the depth of what you are getting into.
*   **Krita's PSD filters:** Spend time reading how Krita handles layer effects (blending modes, masks). This is where most Photoshop clones fail.