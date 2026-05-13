use skia_safe::EncodedImageFormat;
use smol_egui_skia::rasterize_ui;
use std::fs::File;
use std::io::Write;

pub fn main() {
    let mut demo = egui_demo_lib::ColorTest::default();

    let mut surface = rasterize_ui(
        (800, 2000),
        |ui| {
            demo.ui(ui);
        },
        None,
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
