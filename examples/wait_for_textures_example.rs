/// Example showing how to wait for textures to load before painting
use egui::{CentralPanel, include_image, Pos2};
use smol_egui_skia::EguiSkia;
use skia_safe::surface::surfaces::raster_n32_premul;

fn main() {
    let size = (800, 600);
    let mut surface = raster_n32_premul(size).expect("Failed to create surface");
    let canvas = surface.canvas();

    let mut backend = EguiSkia::new(1.0);

    // Install image loaders to enable async loading
    egui_extras::install_image_loaders(&backend.egui_ctx);

    let input = egui::RawInput {
        screen_rect: Some([
            Pos2::default(),
            Pos2::new(size.0 as f32, size.1 as f32),
        ].into()),
        ..Default::default()
    };

    // Method 1: Use paint_when_ready - simplest approach
    let success = backend.paint_when_ready(
        canvas,
        input.clone(),
        |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                ui.heading("Image loaded and ready!");
                ui.image(include_image!("../examples/assets/ferris.jpg"));
            });
        },
        Some(100), // Wait up to 100 frames
    );

    if success {
        println!("✓ All textures loaded and painted successfully!");
    } else {
        println!("✗ Textures did not load in time");
    }

    // Method 2: Manual control with wait_for_textures
    let mut backend2 = EguiSkia::new(1.0);
    egui_extras::install_image_loaders(&backend2.egui_ctx);

    // Wait for textures first
    if backend2.wait_for_textures(
        input.clone(),
        |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                ui.image(include_image!("../examples/assets/ferris.jpg"));
            });
        },
        Some(100),
    ) {
        println!("✓ All textures ready!");
        // Now paint
        backend2.paint(canvas);
    } else {
        println!("✗ Timeout waiting for textures");
    }

    // Method 3: Check texture status manually
    let mut backend3 = EguiSkia::new(1.0);
    egui_extras::install_image_loaders(&backend3.egui_ctx);

    backend3.run(input.clone(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            ui.image(include_image!("../examples/assets/ferris.jpg"));
        });
    });

    if backend3.are_textures_loaded() {
        println!("✓ Textures are loaded, safe to paint");
        backend3.paint(canvas);
    } else {
        println!("⚠ Textures not ready yet, need to run more frames");
    }

    // Save the surface
    let image = surface.image_snapshot();
    let data = image.encode(None, skia_safe::EncodedImageFormat::PNG, None).unwrap();
    std::fs::write("wait_for_textures_example.png", data.as_bytes()).unwrap();
    println!("Saved to wait_for_textures_example.png");
}
