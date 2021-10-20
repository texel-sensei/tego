use std::any::Any;

use crate::Result;

pub trait ImageLoader {
    fn load(&mut self, path: &str) -> Result<Box<dyn Any>>;
}

/// Trivial Image loader implementation that only stores paths for manual loading later.
/// It does not actually load any image data or touches any files.
pub struct LazyLoader {}

impl ImageLoader for LazyLoader {
    fn load(&mut self, path: &str) -> Result<Box<dyn Any>> {
        Ok(Box::new(path.to_string()))
    }
}
