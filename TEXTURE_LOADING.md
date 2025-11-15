# Texture Loading Guide

This document explains how to handle texture loading in `smol_egui_skia`.

## The Problem

When rendering images/textures with egui, textures may be loaded asynchronously. If you try to paint before textures are loaded, it can cause:
- Crashes when accessing missing textures
- Incomplete/blank images in the output

## The Solution

`smol_egui_skia` now provides three new methods to handle texture loading:

### 1. `are_textures_loaded()` - Check Status

Check if all textures are currently ready:

```rust
if backend.are_textures_loaded() {
    backend.paint(canvas);
} else {
    println!("Textures not ready yet");
}
```

### 2. `wait_for_textures()` - Wait Until Ready

Run the UI repeatedly until all textures load:

```rust
egui_extras::install_image_loaders(&backend.egui_ctx);

let success = backend.wait_for_textures(
    input,
    |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(egui::include_image!("ferris.jpg"));
        });
    },
    Some(100), // Max iterations
);

if success {
    backend.paint(canvas);
}
```

### 3. `paint_when_ready()` - One-Step Solution ⭐ RECOMMENDED

Combines waiting and painting in one call:

```rust
egui_extras::install_image_loaders(&backend.egui_ctx);

let success = backend.paint_when_ready(
    canvas,
    input,
    |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(egui::include_image!("ferris.jpg"));
        });
    },
    Some(100), // Max iterations (optional, defaults to 100)
);
```

## Complete Example

```rust
use egui::{CentralPanel, include_image, Pos2};
use smol_egui_skia::EguiSkia;
use skia_safe::surface::surfaces::raster_n32_premul;

fn main() {
    let size = (800, 600);
    let mut surface = raster_n32_premul(size).unwrap();
    let canvas = surface.canvas();

    let mut backend = EguiSkia::new(1.0);

    // IMPORTANT: Install loaders for async image loading
    egui_extras::install_image_loaders(&backend.egui_ctx);

    let input = egui::RawInput {
        screen_rect: Some([
            Pos2::default(),
            Pos2::new(size.0 as f32, size.1 as f32),
        ].into()),
        ..Default::default()
    };

    // Paint when ready - simplest approach!
    backend.paint_when_ready(
        canvas,
        input,
        |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                ui.heading("My Image");
                ui.image(include_image!("assets/image.jpg"));
            });
        },
        None, // Use default max iterations (100)
    );

    // Save result
    let image = surface.image_snapshot();
    let data = image.encode(None, skia_safe::EncodedImageFormat::PNG, None).unwrap();
    std::fs::write("output.png", data.as_bytes()).unwrap();
}
```

## Important Notes

1. **Install image loaders first**: Always call `egui_extras::install_image_loaders(&ctx)` before loading images
2. **Max iterations**: Default is 100 frames. Increase if loading very large images over network
3. **Return value**: Methods return `true` if successful, `false` if timeout
4. **Crash prevention**: The painter now skips rendering meshes with missing textures instead of crashing

## How It Works

1. When you run the UI, egui requests textures to be loaded (async)
2. `wait_for_textures()` runs the UI repeatedly, giving time for loaders to fetch data
3. After each frame, it checks if all referenced textures are in the painter's cache
4. Once all textures are loaded, it returns `true`
5. If max iterations is reached, it returns `false` (timeout)

## See Also

- `examples/wait_for_textures_example.rs` - Demonstrates all three methods
- `examples/image_test.rs` - Original manual polling approach (still works)
