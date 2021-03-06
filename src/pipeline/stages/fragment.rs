use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use num_traits::{Float, One, Zero, NumCast, cast};
use nalgebra::coordinates::XYZW;

use ::error::RenderResult;

use ::numeric::utils::min;
use ::color::{Color, ColorAlpha};
use ::color::blend::Blend;
use ::pixels::{PixelRead, PixelWrite};
use ::framebuffer::{UnsafeFramebuffer, Framebuffer};
use ::attachments::depth::Depth;
use ::stencil::StencilConfig;
use ::primitive::Primitive;
use ::mesh::{Vertex, Mesh};
use ::geometry::{Dimensions, HasDimensions, Coordinate, ScreenVertex, FaceWinding};
use ::interpolate::Interpolate;
use ::pipeline::storage::SeparableScreenPrimitiveStorage;

use ::pipeline::PipelineObject;

use ::framebuffer::types::DepthAttachment;
use ::pipeline::types::{PipelineUniforms, Pixel, StencilValue};

pub const DEFAULT_TILE_SIZE: Dimensions = Dimensions { width: 128, height: 128 };

/// Fragment shader stage.
///
/// The fragment shader is responsible for determining the color of pixels where the underlying geometry has been projected onto.
/// Usually this is individual triangles that are rasterized and shaded by the fragment shader, but it also supports point-cloud
/// and lines (pairs of vertices considered as endpoints for lines).
///
/// The fragment shader runs several tests before executing the given shader program, including a depth test.
/// If the depth of the geometry (from the camera), is farther away than geometry that has already been rendered,
/// the shader program isn't run at all, since it wouldn't be visible anyway. Additionally,
/// if the geometry is nearer than an existing fragment, the existing fragment is overwritten.
///
/// Uniforms passed from the vertex shader are interpolating inside the triangles using Interpolate interpolation,
/// which is why it must satisfy the [`Interpolate`](../uniform/trait.Interpolate.html) trait, which can be automatically implemented for many types using the
/// `declare_uniforms!` macro. See the documentation on that for more information on how to use it.
pub struct FragmentShader<'a, P: 'a, V: Vertex, T, K, B> where P: PipelineObject {
    pub ( in ::pipeline) pipeline: &'a mut P,
    pub ( in ::pipeline) mesh: Arc<Mesh<V>>,
    pub ( in ::pipeline) indexed_primitive: PhantomData<T>,
    pub ( in ::pipeline) stencil_value: StencilValue<P>,
    pub ( in ::pipeline) indexed_vertices: Arc<Option<Vec<ScreenVertex<V::Scalar, K>>>>,
    pub ( in ::pipeline) generated_primitives: Arc<SeparableScreenPrimitiveStorage<V::Scalar, K>>,
    pub ( in ::pipeline) cull_faces: Option<FaceWinding>,
    pub ( in ::pipeline) blend: B,
    pub ( in ::pipeline) antialiased_lines: bool,
    pub ( in ::pipeline) tile_size: Dimensions,
}

/// Fragment returned by the fragment shader, which can either be a color
/// value for the pixel or a discard flag to skip that fragment altogether.
#[derive(Debug, Clone, Copy)]
pub enum Fragment<C> where C: Color {
    /// Discard the fragment altogether, as if it was never there.
    Discard,
    /// Desired color for the pixel
    Color(C)
}

impl<'a, P: 'a, V, T, K, B> Deref for FragmentShader<'a, P, V, T, K, B>
    where P: PipelineObject, V: Vertex, B: Blend<Pixel<P>> {
    type Target = B;
    fn deref(&self) -> &B { &self.blend }
}

impl<'a, P: 'a, V, T, K, B> DerefMut for FragmentShader<'a, P, V, T, K, B>
    where P: PipelineObject, V: Vertex, B: Blend<Pixel<P>> {
    fn deref_mut(&mut self) -> &mut B { &mut self.blend }
}

impl<'a, P: 'a, V, T, K, B> FragmentShader<'a, P, V, T, K, B> where P: PipelineObject, V: Vertex {
    /// Cull faces based on winding order. For more information on how and why this works,
    /// check out the documentation for the [`FaceWinding`](../geometry/winding/enum.FaceWinding.html) enum.
    pub fn cull_faces(&mut self, cull: Option<FaceWinding>) {
        self.cull_faces = cull;
    }

    pub fn with_faces_culled(self, cull: Option<FaceWinding>) -> Self {
        FragmentShader {
            cull_faces: cull,
            ..self
        }
    }

    /// Enables drawing antialiased lines for `Line` primitives
    /// primitives using Xiaolin Wu's algorithm,
    /// otherwise Bresenham's Algorithm is used.
    pub fn antialiased_lines(&mut self, enable: bool) {
        self.antialiased_lines = enable;
    }

    pub fn with_antialiased_lines(self, enable: bool) -> Self {
        FragmentShader {
            antialiased_lines: enable,
            ..self
        }
    }

    pub fn tile_size(&mut self, tile_size: Dimensions) {
        self.tile_size = tile_size;
    }

    pub fn with_tile_size(self, tile_size: Dimensions) -> Self {
        FragmentShader {
            tile_size,
            ..self
        }
    }

    /// Duplicates all references to internal state to return a cloned fragment shader,
    /// which can be used to efficiently render the same geometry with different
    /// rasterization methods in quick succession.
    #[must_use]
    pub fn duplicate<'b>(&'b mut self) -> FragmentShader<'b, P, V, T, K, B> where 'a: 'b, B: Clone {
        FragmentShader {
            pipeline: self.pipeline,
            mesh: self.mesh.clone(),
            indexed_primitive: PhantomData,
            stencil_value: self.stencil_value,
            indexed_vertices: self.indexed_vertices.clone(),
            generated_primitives: self.generated_primitives.clone(),
            cull_faces: self.cull_faces.clone(),
            blend: self.blend.clone(),
            antialiased_lines: self.antialiased_lines,
            tile_size: self.tile_size,
        }
    }
}

impl<'a, P: 'a, V, T, K, O> FragmentShader<'a, P, V, T, K, O> where P: PipelineObject, V: Vertex {
    #[must_use]
    pub fn with_blend<B>(self, blend: B) -> FragmentShader<'a, P, V, T, K, B>
        where B: Blend<Pixel<P>> {
        FragmentShader {
            pipeline: self.pipeline,
            mesh: self.mesh,
            indexed_primitive: PhantomData,
            stencil_value: self.stencil_value,
            indexed_vertices: self.indexed_vertices,
            generated_primitives: self.generated_primitives,
            cull_faces: self.cull_faces,
            blend: blend,
            antialiased_lines: self.antialiased_lines,
            tile_size: self.tile_size,
        }
    }

    #[must_use]
    pub fn with_default_blend<B>(self) -> FragmentShader<'a, P, V, T, K, B>
        where B: Blend<Pixel<P>> + Default {
        self.with_blend(B::default())
    }
}

impl<'a, P: 'a, V, T, K, B> FragmentShader<'a, P, V, T, K, B> where P: PipelineObject,
                                                                    V: Vertex,
                                                                    T: Primitive,
                                                                    K: Send + Sync + Interpolate,
                                                                    B: Blend<Pixel<P>> {
    pub fn run<S>(self, fragment_shader: S)
        where S: Fn(&ScreenVertex<V::Scalar, K>, &PipelineUniforms<P>) -> Fragment<Pixel<P>> + Send + Sync {
        let FragmentShader {
            pipeline,
            mesh,
            indexed_vertices,
            stencil_value,
            generated_primitives,
            cull_faces,
            blend,
            antialiased_lines,
            tile_size,
            ..
        } = self;

        // Basically constant
        let one_half = <V::Scalar as NumCast>::from(0.5).unwrap();

        let dimensions = pipeline.framebuffer().dimensions();

        let tiles = {
            let mut tiles = Vec::new();

            let xmax = dimensions.width - 1;
            let ymax = dimensions.height - 1;

            let mut y = 0;

            while y < ymax {
                let mut x = 0;

                let next_y = min(y + tile_size.height, ymax);

                while x < xmax {
                    let next_x = min(x + tile_size.width, xmax);

                    tiles.push((
                        Coordinate::new(x, y),
                        Coordinate::new(next_x, next_y)
                    ));

                    x = next_x;
                }

                y = next_y;
            }

            tiles
        };

        // Fetch stencil test and operation before tile loop
        let stencil_test = pipeline.stencil_config().get_test();
        let stencil_op = pipeline.stencil_config().get_op();

        /// There is simply no way around this right now. The only reason I'm comfortable doing it is because
        /// all the code using the pipeline is my own and not available to the user.
        ///
        /// Additionally, although the framebuffer access is totally unsafe, the uniforms are requires to be `Send + Sync`, so they
        /// are fine. More or less.
        #[derive(Clone, Copy)]
        struct NeverDoThis<P> { pipeline: *mut P }

        unsafe impl<P> Send for NeverDoThis<P> {}
        unsafe impl<P> Sync for NeverDoThis<P> {}

        /// Create unsafe mutable point to the pipeline
        let seriously_dont = NeverDoThis { pipeline: pipeline as *mut P };

        let (_, _, pool) = pipeline.all_mut();

        let thread_count = pool.thread_count();

        let i = AtomicUsize::new(0);

        pool.scoped(|scope| {
            for _ in 0..thread_count {
                scope.execute(|| {
                    use super::rasterization::{RasterArguments, rasterize_triangle, rasterize_line, rasterize_point};

                    // Get the unsafe mutable reference to the pipeline
                    let pipeline: &mut P = unsafe { &mut *seriously_dont.pipeline };

                    loop {
                        let i = i.fetch_add(1, Ordering::Relaxed);

                        if i < tiles.len() {
                            let tile = tiles[i];

                            let mut args: RasterArguments<P, V> = RasterArguments {
                                dimensions,
                                tile: tile,
                                bounds: ((cast(tile.0.x).unwrap(), cast(tile.0.y).unwrap()),
                                         (cast(tile.1.x).unwrap(), cast(tile.1.y).unwrap())),
                                stencil_value,
                                stencil_test,
                                stencil_op,
                                antialiased_lines,
                                cull_faces,
                            };

                            if T::is_triangle() {
                                if let Some(ref indexed_vertices) = *indexed_vertices {
                                    for triangle in mesh.indices.chunks(3) {
                                        let a = &indexed_vertices[triangle[0]];
                                        let b = &indexed_vertices[triangle[1]];
                                        let c = &indexed_vertices[triangle[2]];

                                        rasterize_triangle(&args, pipeline, &blend, &fragment_shader, a, b, c);
                                    }
                                }
                            }

                            for triangle in generated_primitives.tris.chunks(3) {
                                rasterize_triangle(&args, pipeline, &blend, &fragment_shader, &triangle[0], &triangle[1], &triangle[2]);
                            }

                            if T::is_line() {
                                if let Some(ref indexed_vertices) = *indexed_vertices {
                                    for line in mesh.indices.chunks(2) {
                                        let start = &indexed_vertices[line[0]];
                                        let end = &indexed_vertices[line[1]];

                                        rasterize_line(&args, pipeline, &blend, &fragment_shader, start, end);
                                    }
                                }
                            }

                            for line in generated_primitives.lines.chunks(2) {
                                rasterize_line(&args, pipeline, &blend, &fragment_shader, &line[0], &line[1]);
                            }

                            if T::is_point() {
                                if let Some(ref indexed_vertices) = *indexed_vertices {
                                    for index in &mesh.indices {
                                        let point = &indexed_vertices[*index];

                                        rasterize_point(&args, pipeline, &blend, &fragment_shader, point);
                                    }
                                }
                            }

                            for point in &generated_primitives.points {
                                rasterize_point(&args, pipeline, &blend, &fragment_shader, point);
                            }
                        } else {
                            break;
                        }
                    }
                });
            }
        });
    }
}