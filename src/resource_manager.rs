use std::{any::Any, collections::HashMap, io::Read, path::Path, rc::Rc};

use crate::{Result, Error};

pub struct ResourceManager {
    base_path: String,
    image_loader: Box<dyn ImageLoader>,
    file_provider: Box<dyn Provider>,
    image_cache: HashMap<String, Rc<dyn Any>>
}

impl ResourceManager {
    /// Create a new resource manager with a given image loader and data provider.
    /// Defaults the base path to the current directory (`.`).
    pub fn new<L: ImageLoader + 'static, P: Provider + 'static>(image_loader: L, file_provider: P) -> Self {
        Self {
            base_path: ".".into(),
            image_loader: Box::new(image_loader),
            file_provider: Box::new(file_provider),
            image_cache: HashMap::new(),
        }
    }

    pub fn load_image(&mut self, path: &str) -> Result<Rc<dyn Any>>{
        // TODO(texel, 2021-11-10): Use file provider
        let path = format!("{}/{}", &self.base_path, path);
        let entry = self.image_cache.entry(path.clone());
        use std::collections::hash_map::Entry::*;
        Ok(match entry {
            Occupied(slot) => slot.get().clone(),
            Vacant(slot) => {
                let data = self.image_loader.load(&path)?.into();
                slot.insert(data).clone()
            },
        })
    }

    pub fn load_text(&mut self, path: &str) -> Result<String> {
        let data = self.file_provider.read(&self.base_path, path)?;
        Ok(String::from_utf8(data).map_err(|e| Error::ParseError(Box::new(e)))?)
    }

    /// Get a reference to the resource manager's base path.
    pub fn base_path(&self) -> &str {
        self.base_path.as_ref()
    }

    /// Set the resource manager's base path.
    pub fn set_base_path(&mut self, base_path: String) {
        self.base_path = base_path;
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        ResourceManager::new(LazyLoader{},  FileProvider{})
    }
}

pub trait ImageLoader {
    fn load(&mut self, path: &str) -> Result<Box<dyn Any>>;
}

/// Trait to provide external data.
pub trait Provider {
    /// Open a file that is located at base_path/path and return its contents.
    fn read(&mut self, base_path: &str, path: &str) -> Result<Vec<u8>>;
}

/// Trivial Image loader implementation that only stores paths for manual loading later.
/// It does not actually load any image data or touches any files.
pub struct LazyLoader {}

impl ImageLoader for LazyLoader {
    fn load(&mut self, path: &str) -> Result<Box<dyn Any>> {
        Ok(Box::new(path.to_string()))
    }
}

/// [Provider] that reads the data from files on the file system.
pub struct FileProvider {}

impl Provider for FileProvider {
    fn read(&mut self, base_path: &str, path: &str) -> Result<Vec<u8>> {
        let path = Path::new(base_path).join(Path::new(path));

        let mut file = std::fs::File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        Ok(content)
    }
}
