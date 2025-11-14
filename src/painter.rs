use std::ops::Deref;
use std::sync::Arc;

use egui::epaint::ahash::AHashMap;

use egui::epaint::Primitive;
use egui::{ClippedPrimitive, ImageData, Pos2, TextureId, TexturesDelta};
use skia_safe::surfaces::raster_n32_premul;
use skia_safe::vertices::VertexMode;
use skia_safe::{
    scalar, BlendMode, Canvas, ClipOp, Color, ConditionallySend, Data, Drawable, Image, ImageInfo,
    Paint, PictureRecorder, Point, Rect, Sendable, Vertices,
};

struct PaintHandle {
    paint: Paint,
    image: Image,
}

pub struct Painter {
    paints: AHashMap<TextureId, PaintHandle>,
}

impl Painter {
    pub fn new() -> Painter {
        Self {
            paints: AHashMap::new(),
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        canvas: &Canvas,
        dpi: f32,
        primitives: Vec<ClippedPrimitive>,
        textures_delta: TexturesDelta,
    ) {
        textures_delta.set.iter().for_each(|(id, image_delta)| {
            let delta_image = match &image_delta.image {
                ImageData::Color(color_image) => skia_safe::images::raster_from_data(
                    &ImageInfo::new(
                        skia_safe::ISize::new(
                            color_image.width() as i32,
                            color_image.height() as i32,
                        ),
                        skia_safe::ColorType::RGB888x,
                        skia_safe::AlphaType::Premul,
                        None,
                    ),
                    Data::new_copy(
                        color_image
                            .pixels
                            .iter()
                            .flat_map(|p| p.to_array())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                    color_image.width() * 4,
                )
                .unwrap(),
            };

            let image = match image_delta.pos {
                None => delta_image,
                Some(pos) => {
                    let old_image = self.paints.remove(id).unwrap().image;

                    let mut surface = raster_n32_premul(skia_safe::ISize::new(
                        old_image.width(),
                        old_image.height(),
                    ))
                    .unwrap();

                    let canvas = surface.canvas();

                    canvas.draw_image(&old_image, Point::new(0.0, 0.0), None);

                    canvas.clip_rect(
                        Rect::new(
                            pos[0] as scalar,
                            pos[1] as scalar,
                            (pos[0] as i32 + delta_image.width()) as scalar,
                            (pos[1] as i32 + delta_image.height()) as scalar,
                        ),
                        ClipOp::default(),
                        false,
                    );

                    canvas.clear(Color::TRANSPARENT);
                    canvas.draw_image(&delta_image, Point::new(pos[0] as f32, pos[1] as f32), None);

                    surface.image_snapshot()
                }
            };

            let local_matrix =
                skia_safe::Matrix::scale((1.0 / image.width() as f32, 1.0 / image.height() as f32));

            let sampling_options = {
                use egui::TextureFilter;
                let filter_mode = match image_delta.options.magnification {
                    TextureFilter::Nearest => skia_safe::FilterMode::Nearest,
                    TextureFilter::Linear => skia_safe::FilterMode::Linear,
                };
                let mm_mode = match image_delta.options.minification {
                    TextureFilter::Nearest => skia_safe::MipmapMode::Nearest,
                    TextureFilter::Linear => skia_safe::MipmapMode::Linear,
                };

                skia_safe::SamplingOptions::new(filter_mode, mm_mode)
            };
            let tile_mode = skia_safe::TileMode::Clamp;

            let font_shader = image
                .to_shader((tile_mode, tile_mode), sampling_options, &local_matrix)
                .unwrap();

            image.width();

            let mut paint = Paint::default();
            paint.set_shader(font_shader);
            paint.set_color(Color::WHITE);

            self.paints.insert(
                *id,
                PaintHandle {
                    paint,
                    image,
                },
            );
        });

        for primitive in primitives {
            let skclip_rect = Rect::new(
                primitive.clip_rect.min.x,
                primitive.clip_rect.min.y,
                primitive.clip_rect.max.x,
                primitive.clip_rect.max.y,
            );
            match primitive.primitive {
                Primitive::Mesh(mesh) => {
                    canvas.set_matrix(skia_safe::M44::new_identity().set_scale(dpi, dpi, 1.0));
                    let arc = skia_safe::AutoCanvasRestore::guard(canvas, true);

                    let meshes = mesh.split_to_u16();

                    for mesh in &meshes {
                        let texture_id = mesh.texture_id;

                        let mut pos = Vec::with_capacity(mesh.vertices.len());
                        let mut texs = Vec::with_capacity(mesh.vertices.len());
                        let mut colors = Vec::with_capacity(mesh.vertices.len());

                        mesh.vertices.iter().enumerate().for_each(|(_i, v)| {
                            // Apparently vertices can be NaN and if they are NaN, nothing is rendered.
                            // Replacing them with 0 works around this.
                            // https://github.com/lucasmerlin/egui_skia/issues/4
                            let fixed_pos = if v.pos.x.is_nan() || v.pos.y.is_nan() {
                                Pos2::new(0.0, 0.0)
                            } else {
                                v.pos
                            };

                            pos.push(Point::new(fixed_pos.x, fixed_pos.y));
                            texs.push(Point::new(v.uv.x, v.uv.y));

                            let c = v.color;
                            let c = Color::from_argb(c.a(), c.r(), c.g(), c.b());
                            // Un-premultply color
                            // This fixes some cases of the color-test
                            // https://github.com/lucasmerlin/egui_skia/issues/6
                            // there might be a better solution though?
                            let mut cf = skia_safe::Color4f::from(c);
                            cf.r /= cf.a;
                            cf.g /= cf.a;
                            cf.b /= cf.a;
                            colors.push(Color::from_argb(
                                c.a(),
                                (cf.r * 255.0) as u8,
                                (cf.g * 255.0) as u8,
                                (cf.b * 255.0) as u8,
                            ));
                        });

                        // TODO: Use vertex builder
                        // let mut vertex_builder = Builder::new(
                        //     VertexMode::Triangles,
                        //     mesh.vertices.len(),
                        //     mesh.indices.len(),
                        //     BuilderFlags::HAS_COLORS | BuilderFlags::HAS_TEX_COORDS,
                        // );
                        //
                        // for (i, v) in mesh.vertices.iter().enumerate() {
                        //     vertex_builder.positions()[i] = Point::new(v.pos.x, v.pos.y);
                        //     vertex_builder.tex_coords().unwrap()[i] = Point::new(v.uv.x, v.uv.y);
                        //     vertex_builder.colors().unwrap()[i] = Color::from_argb(
                        //         v.color.a(),
                        //         v.color.r(),
                        //         v.color.g(),
                        //         v.color.b(),
                        //     );
                        // }
                        // let vertices = vertex_builder.detach();

                        let vertices = Vertices::new_copy(
                            VertexMode::Triangles,
                            &pos,
                            &texs,
                            &colors,
                            Some(
                                mesh.indices
                                    .iter()
                                    .map(|index| *index as u16)
                                    .collect::<Vec<u16>>()
                                    .as_slice(),
                            ),
                        );

                        arc.clip_rect(skclip_rect, ClipOp::default(), true);

                        let paint = &self.paints[&texture_id].paint;
                        arc.draw_vertices(&vertices, BlendMode::Modulate, paint);
                    }
                }
                Primitive::Callback(data) => {
                    let callback: Arc<EguiSkiaPaintCallback> = data.callback.downcast().unwrap();
                    let rect = data.rect;

                    let skia_rect = Rect::new(
                        rect.min.x * dpi,
                        rect.min.y * dpi,
                        rect.max.x * dpi,
                        rect.max.y * dpi,
                    );

                    let mut drawable: Drawable = callback.callback.deref()(skia_rect).0.into_inner();

                    let mut arc = skia_safe::AutoCanvasRestore::guard(canvas, true);

                    arc.clip_rect(skclip_rect, ClipOp::default(), true);
                    arc.translate((rect.min.x, rect.min.y));

                    drawable.draw(&mut arc, None);
                }
            }
        }

        textures_delta.free.iter().for_each(|id| {
            self.paints.remove(id);
        });
    }
}

impl Default for Painter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EguiSkiaPaintCallback {
    callback: Box<dyn Fn(Rect) -> SyncSendableDrawable + Send + Sync>,
}

impl EguiSkiaPaintCallback {
    pub fn new<F: Fn(&Canvas) + Send + Sync + 'static>(callback: F) -> EguiSkiaPaintCallback {
        EguiSkiaPaintCallback {
            callback: Box::new(move |rect| {
                let mut pr = PictureRecorder::new();
                let canvas = pr.begin_recording(rect, false);
                callback(canvas);
                SyncSendableDrawable(
                    pr.finish_recording_as_drawable()
                        .unwrap()
                        .wrap_send()
                        .unwrap(),
                )
            }),
        }
    }
}

struct SyncSendableDrawable(pub Sendable<Drawable>);

unsafe impl Sync for SyncSendableDrawable {}


