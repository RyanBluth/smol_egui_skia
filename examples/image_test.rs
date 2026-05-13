use egui::load::TexturePoll;
use egui::{SizeHint, TextureOptions, include_image};
use skia_safe::EncodedImageFormat;
use smol_egui_skia::{RasterizeOptions, rasterize_ui};
use std::fs::File;
use std::io::Write;

pub fn main() {
    let mut surface = rasterize_ui(
        (460, 307),
        |ui| {
            let ctx = ui.ctx();
            egui_extras::install_image_loaders(ctx);

            while !matches!(
                include_image!("assets/ferris.jpg").load(
                    ctx,
                    TextureOptions::default(),
                    SizeHint::default()
                ),
                Ok(TexturePoll::Ready { .. })
            ) {
                continue;
            }

            ui.image(include_image!("assets/ferris.jpg"));
        },
        Some(RasterizeOptions {
            pixels_per_point: 1.0,
            frames_before_screenshot: 20,
        }),
    );

    let data = surface
        .image_snapshot()
        .encode(None, EncodedImageFormat::PNG, 100)
        .expect("Failed to encode image");

    File::create("output.png")
        .unwrap()
        .write_all(&data)
        .unwrap();

    println!("wrote output.png");
}
