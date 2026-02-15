//! Core types for the image runner pipeline: builder, context, and error handling.

pub mod builder;
pub mod context;
pub mod error;

pub use builder::{ImageRunner, ImageRunnerBuilder};
pub use context::Context;
pub use error::{Error, Result};
