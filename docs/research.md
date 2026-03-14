### The "Super Fluid" Architecture

To keep the UI responsive while manipulating large textures, you need a **hybrid-retained** architecture. Use GTK for the "chrome" (buttons, layers, menus) and a raw GPU context for the canvas.

* **Language:** **Rust**.
* **GUI Framework:** **GTK4** via the `gtk4-rs` crate.
* **Canvas Rendering:** **Wgpu**. This is the standard Rust abstraction over Vulkan. Since you are on Plasma 6.6 (Wayland), Wgpu will talk directly to the compositor via the Vulkan backend for zero-latency drawing.
* **Image Processing Engine:** **`image`** crate (for CPU basics) and **custom WGSL shaders** (for GPU-accelerated filters like blurs or color corrections).

---

### Key Technical Implementation Details

#### 1. The Wayland "Subsurface" Bridge

GTK4 generally prefers to handle its own rendering via GSK (Gtk Scene Graph). To get "Photoshop-level" speed, you shouldn't let GTK draw your pixels.

* **Approach:** Use a `GtkPaintable` or a `GdkTexture`. However, for the absolute best performance on Wayland, you can use the **`raw-window-handle`** crate to pass the Wayland surface from your GTK window directly to **Wgpu**.
* **Benefit:** This allows your canvas to refresh at 144Hz+ without waiting for the GTK UI loop to catch up.

#### 2. Handling High-DPI and Scaling

Since you are on Plasma 6.6, you'll likely encounter **fractional scaling** (e.g., 125% or 150%).

* GTK4 handles this well, but your **Wgpu canvas** must listen for the `surface.configure` events to resize its internal buffers.
* In Rust, you will calculate your internal buffer size as:

{Buffer Width} = {Logical Width} x {Scale Factor}



#### 3. CachyOS Optimizations

Since you are on CachyOS, you can compile your app specifically for your hardware:

* Add `-C target-cpu=native` to your `RUSTFLAGS`.
* This will enable SIMD (Single Instruction, Multiple Data) instructions, which are vital for CPU-based image processing tasks that aren't offloaded to the GPU.

---

### The MVP Component Stack

| Layer | Technology | Purpose |
| --- | --- | --- |
| **Windowing** | `gtk4` | Native window, menus, and file dialogs. |
| **State** | `relm4` | An idiomatic "Elm-like" wrapper for Rust GTK4 (highly recommended for MVP speed). |
| **Canvas** | `wgpu` | Hardware-accelerated drawing, panning, and zooming. |
| **Layers** | `vulkan` (via Wgpu) | Blending multiple textures (layers) using GPU fragment shaders. |
| **Input** | `libinput` (via GTK) | Native support for drawing tablet pressure and tilt on Wayland. |

---