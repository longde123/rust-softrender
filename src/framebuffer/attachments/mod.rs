use ::behavior::ThreadSafeCopyable;

pub use ::color;
pub use ::stencil;

pub mod depth;

pub trait Attachment: ThreadSafeCopyable {}

impl<T> Attachment for T where T: ThreadSafeCopyable {}

pub use self::color::Color;
pub use self::depth::Depth;
pub use self::stencil::{Stencil, StencilOp, StencilTest, StencilConfig, GenericStencilConfig};

/// Marker trait only defined for `()`, an empty tuple.
pub trait EmptyAttachment {}

impl EmptyAttachment for () {}

/// Trait defining associated types of framebuffer attachments
pub trait Attachments: Attachment {
    type Color: Attachment + Color;
    type Depth: Attachment + Depth;
    type Stencil: Attachment + Stencil;
}

pub mod predefined;

pub use self::predefined::*;