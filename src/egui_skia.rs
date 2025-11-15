use egui::{Context, Pos2};
use skia_safe::{surface::surfaces::raster_n32_premul, Canvas, Surface};

use crate::painter::Painter;

pub struct RasterizeOptions {
    pub pixels_per_point: f32,
    /// The number of frames to render before a screenshot is taken.
    /// Default is 2, so egui will be able to display windows
    pub frames_before_screenshot: usize,
}

impl Default for RasterizeOptions {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.0,
            frames_before_screenshot: 2,
        }
    }
}

pub fn rasterize(
    size: (i32, i32),
    ui: impl FnMut(&Context),
    options: Option<RasterizeOptions>,
) -> Surface {
    let mut surface = raster_n32_premul(size).expect("Failed to create surface");
    draw_onto_canvas(surface.canvas(), ui, options);
    surface
}

pub fn draw_onto_canvas(
    canvas: &Canvas,
    mut ui: impl FnMut(&Context),
    options: Option<RasterizeOptions>,
) {
    let RasterizeOptions {
        pixels_per_point,
        frames_before_screenshot,
    } = options.unwrap_or_default();
    let mut backend = EguiSkia::new(pixels_per_point);

    let image_info = canvas.image_info();

    let input = egui::RawInput {
        screen_rect: Some(
            [
                Pos2::default(),
                Pos2::new(image_info.width() as f32, image_info.height() as f32),
            ]
            .into(),
        ),
        ..Default::default()
    };

    for _ in 0..frames_before_screenshot {
        backend.run(input.clone(), &mut ui);
    }
    backend.paint(canvas);
}

/// Convenience wrapper for using [`egui`] from a [`skia`] app.
pub struct EguiSkia {
    pub egui_ctx: Context,
    pub painter: Painter,

    pixels_per_point: f32,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiSkia {
    pub fn new(pixels_per_point: f32) -> Self {
        let painter = Painter::new();
        Self {
            pixels_per_point,
            egui_ctx: Default::default(),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    /// Returns a duration after witch egui should repaint.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(
        &mut self,
        input: egui::RawInput,
        run_ui: impl FnMut(&Context),
    ) -> egui::PlatformOutput {
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            ..
        } = self.egui_ctx.run(input, run_ui);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);

        platform_output
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, canvas: &Canvas) {
        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_primitives = self.egui_ctx.tessellate(shapes, self.pixels_per_point);
        self.painter.paint_and_update_textures(
            canvas,
            self.egui_ctx.pixels_per_point(),
            clipped_primitives,
            textures_delta,
        );
    }

    /// Check if all textures required for rendering are currently loaded.
    ///
    /// This checks if all textures referenced in the current shapes/primitives
    /// are available in the painter's texture cache.
    ///
    /// Returns `true` if all textures are ready, `false` if any are missing.
    pub fn are_textures_loaded(&self) -> bool {
        let clipped_primitives = self.egui_ctx.tessellate(self.shapes.clone(), self.pixels_per_point);
        self.painter.all_textures_loaded(&clipped_primitives)
    }

    /// Wait for all textures to load by running the UI repeatedly until ready.
    ///
    /// This method calls `run_ui` repeatedly (up to `max_iterations` times) until
    /// all textures referenced in the UI are loaded. This is useful when you need
    /// to ensure images are fully loaded before painting.
    ///
    /// # Arguments
    /// * `input` - The raw input to pass to egui
    /// * `run_ui` - The UI function to run (same as passed to `run`)
    /// * `max_iterations` - Maximum number of frames to wait (default: 100)
    ///
    /// # Returns
    /// `true` if all textures loaded, `false` if max iterations reached
    ///
    /// # Example
    /// ```no_run
    /// # use egui::Context;
    /// # use smol_egui_skia::EguiSkia;
    /// # let mut backend = EguiSkia::new(1.0);
    /// # let input = egui::RawInput::default();
    /// // Install image loaders first
    /// egui_extras::install_image_loaders(&backend.egui_ctx);
    ///
    /// // Wait for textures to load
    /// backend.wait_for_textures(input.clone(), |ctx| {
    ///     egui::CentralPanel::default().show(ctx, |ui| {
    ///         ui.image(egui::include_image!("../assets/ferris.jpg"));
    ///     });
    /// }, Some(100));
    /// ```
    pub fn wait_for_textures(
        &mut self,
        input: egui::RawInput,
        mut run_ui: impl FnMut(&Context),
        max_iterations: Option<usize>,
    ) -> bool {
        let max_iterations = max_iterations.unwrap_or(100);

        for _ in 0..max_iterations {
            self.run(input.clone(), &mut run_ui);

            if self.are_textures_loaded() {
                return true;
            }

            // Request repaint to trigger another frame
            self.egui_ctx.request_repaint();
        }

        false // Timed out
    }

    /// Paint only after waiting for all textures to load.
    ///
    /// This is a convenience method that combines `wait_for_textures` and `paint`.
    /// It will run the UI repeatedly until all textures are loaded, then paint the result.
    ///
    /// # Arguments
    /// * `canvas` - The canvas to paint onto
    /// * `input` - The raw input to pass to egui
    /// * `run_ui` - The UI function to run
    /// * `max_iterations` - Maximum number of frames to wait (default: 100)
    ///
    /// # Returns
    /// `true` if painted successfully, `false` if textures didn't load in time
    pub fn paint_when_ready(
        &mut self,
        canvas: &Canvas,
        input: egui::RawInput,
        run_ui: impl FnMut(&Context),
        max_iterations: Option<usize>,
    ) -> bool {
        if self.wait_for_textures(input, run_ui, max_iterations) {
            self.paint(canvas);
            true
        } else {
            false
        }
    }
}

impl Default for EguiSkia {
    fn default() -> Self {
        Self::new(1.0)
    }
}
