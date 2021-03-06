//! Flexible custom Framebuffer designed for re-use

use std::slice;
use std::ops::Deref;

use ::color::Color;
use ::geometry::{Dimensions, HasDimensions};
use ::pixels::{PixelBuffer, PixelRead};

use super::Framebuffer;

/// Structure containing a reference to a specific color buffer from a texture buffer object.
///
/// This allows zero-cost access to the buffer as a texture.
pub struct TextureBufferRef<'a, F: Framebuffer, C: Color> {
    buffer: &'a [C],
    parent: &'a F,
}

impl<'a, F: Framebuffer, C: Color> Deref for TextureBufferRef<'a, F, C> {
    type Target = [C];

    #[inline]
    fn deref(&self) -> &[C] { self.buffer }
}

impl<'a, F: Framebuffer, C: Color> TextureBufferRef<'a, F, C> {
    #[doc(hidden)]
    pub fn __new(buffer: &'a [C], parent: &'a F) -> TextureBufferRef<'a, F, C> {
        TextureBufferRef { buffer, parent }
    }
}

impl<'a, F: Framebuffer, C: Color> HasDimensions for TextureBufferRef<'a, F, C> {
    #[inline]
    fn dimensions(&self) -> Dimensions { self.parent.dimensions() }
}

impl<'a, F: Framebuffer, C: Color> PixelBuffer for TextureBufferRef<'a, F, C> {
    type Color = C;
}

impl<'a, F: Framebuffer, C: Color> PixelRead for TextureBufferRef<'a, F, C> {
    #[inline]
    unsafe fn get_pixel_unchecked(&self, index: usize) -> Self::Color {
        *self.buffer.get_unchecked(index)
    }
}

/// Declares a new texture buffer type with the specified color buffer attachments, as if they were a struct.
///
/// Unlike with the simple `RenderBuffer`, texture buffers store each color component in a separate buffer. This allows
/// for the rendered images to be re-used as textures without copying, which can result in a net performance gain.
///
/// However, because of the extra complexity, a macro is used to simplify the creation of these.
///
/// See the `predefined` module for an example of a texture buffer with a single color attachment, which was created with the macro invocation:
///
/// ```ignore
/// declare_texture_buffer! {
///     /// Texture Buffer with a single RGBA 32-bit Floating Point color
///     pub struct RGBAf32TextureBuffer {
///         /// Primary color buffer
///         pub color: RGBAf32Color,
///     }
/// }
/// ```
///
/// Because the inner buffers are an implementation detail,
/// the attributes given to the fields are placed on their accessor functions.
#[macro_export]
macro_rules! declare_texture_buffer {
    (
        $(#[$($struct_attrs:tt)*])*
        pub struct $buffer_name:ident {
            $(
                $(#[$($field_attrs:tt)*])*
                pub $color_name:ident: $color_ty:ty,
            )+
        }
    ) => {
        $(#[$($struct_attrs)*])*
        #[derive(Clone)]
        pub struct $buffer_name<A: $crate::attachments::Attachments>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            $($color_name: Vec<$color_ty>,)+
            __internal_dimensions: $crate::geometry::Dimensions,
            __internal_buffer: Vec<(A::Depth, A::Stencil)>,
        }

        impl<A: $crate::attachments::Attachments> $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            pub fn new() -> $buffer_name<A> {
                $buffer_name {
                    $($color_name: Vec::new(),)+
                    __internal_dimensions: $crate::geometry::Dimensions::new(0, 0),
                    __internal_buffer: Vec::new(),
                }
            }

            pub fn with_dimensions(dimensions: $crate::geometry::Dimensions) -> $buffer_name<A> {
                let pixels = dimensions.area();

                $buffer_name {
                    $($color_name: vec![<$color_ty as $crate::attachments::Color>::empty(); pixels],)+
                    __internal_dimensions: dimensions,
                    __internal_buffer: vec![($crate::attachments::Depth::far(), Default::default()); pixels],
                }
            }

            $(
                $(#[$($field_attrs)*])*
                #[inline(always)]
                pub fn $color_name(&self) -> $crate::framebuffer::texturebuffer::TextureBufferRef<Self, $color_ty> {
                    $crate::framebuffer::texturebuffer::TextureBufferRef::__new(&self.$color_name, self)
                }
            )+
        }

        impl<A: $crate::attachments::Attachments> $crate::geometry::HasDimensions for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            #[inline]
            fn dimensions(&self) -> $crate::geometry::Dimensions {
                self.__internal_dimensions
            }
        }

        impl<A: $crate::attachments::Attachments> $crate::pixels::PixelBuffer for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            /// All texture buffer colors as a tuple
            type Color = ($($color_ty,)+);
        }

        impl<A: $crate::attachments::Attachments> $crate::pixels::PixelRead for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            unsafe fn get_pixel_unchecked(&self, index: usize) -> Self::Color {
                ($(*self.$color_name.get_unchecked(index),)+)
            }
        }

        impl<A: $crate::attachments::Attachments> $crate::pixels::PixelWrite for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            unsafe fn set_pixel_unchecked(&mut self, index: usize, color: Self::Color) {
                let ($($color_name,)+) = color;

                $(*self.$color_name.get_unchecked_mut(index) = $color_name;)+
            }
        }

        impl<A: $crate::attachments::Attachments> $crate::framebuffer::FramebufferBase for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            type Attachments = $crate::attachments::ColorDepthStencilAttachments<($($color_ty,)+), A::Depth, A::Stencil>;
        }

        impl<A: $crate::attachments::Attachments> $crate::framebuffer::UnsafeFramebuffer for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {

            #[inline]
            unsafe fn get_depth_unchecked(&self, index: usize) -> $crate::framebuffer::types::DepthAttachment<Self> {
                self.__internal_buffer.get_unchecked(index).0
            }

            #[inline]
            unsafe fn set_depth_unchecked(&mut self, index: usize, depth: $crate::framebuffer::types::DepthAttachment<Self>) {
                self.__internal_buffer.get_unchecked_mut(index).0 = depth;
            }

            #[inline]
            unsafe fn get_stencil_unchecked(&self, index: usize) -> $crate::framebuffer::types::StencilAttachment<Self> {
                self.__internal_buffer.get_unchecked(index).1
            }

            #[inline]
            unsafe fn set_stencil_unchecked(&mut self, index: usize, stencil: $crate::framebuffer::types::StencilAttachment<Self>) {
                self.__internal_buffer.get_unchecked_mut(index).1 = stencil;
            }
        }

        impl<A: $crate::attachments::Attachments> $crate::framebuffer::Framebuffer for $buffer_name<A>
            where <A as $crate::attachments::Attachments>::Color: $crate::attachments::EmptyAttachment {
            fn clear(&mut self, color: ($($color_ty,)+)) {
                // Destructure the tuple into its individual attachments, by name, so they can be used one by one.
                let ($($color_name,)+) = color;

                $(
                    // Go through each buffer and overwrite everything with the destructured colors, by name.
                    for mut c in &mut self.$color_name {
                        *c = $color_name;
                    }
                )+

                for mut a in &mut self.__internal_buffer {
                    *a = (<A::Depth as $crate::attachments::Depth>::far(), Default::default());
                }
            }
        }
    }
}

pub mod predefined {
    use ::attachments::color::predefined::formats::RGBAf32Color;

    declare_texture_buffer! {
        /// Texture Buffer with a single RGBA 32-bit Floating Point color.
        ///
        /// Unlike the `RenderBuffer`, texture buffers can easily have one or more color attachment
        /// be reused as a texture for a subsequent render.
        pub struct RGBAf32TextureBuffer {
            /// Primary color buffer
            pub color: RGBAf32Color,
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        use ::texture::TextureRead;
        use ::attachments::predefined::EmptyAttachments;

        fn assert_texture<T: TextureRead>(_: T) {}

        #[test]
        fn test_texturebuffer_texture() {
            let t = RGBAf32TextureBuffer::<EmptyAttachments>::new();

            let color = t.color();

            assert_texture(color)
        }
    }
}