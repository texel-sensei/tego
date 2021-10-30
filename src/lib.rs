//! [tego](https://github.com/texel-sensei/tego) is a simple library for loading
//! [Tiled](https://mapeditor.org) maps.
//!
//! It aims to provide a simple, yet flexible API,
//! without forcing any special image format to the user
//! or assuming that data is provided as a file.
//!
//! As a starting point,
//! load a map using any of the *from_\** functions provided in the [Map] type.
//! And inspect the [Layers](Map::layers) inside of it.
//!
//! ```no_run
//! let path = std::path::Path::new("example-maps/default/default_map.tmx");
//! let mymap = tego::Map::from_file(&path)?;
//!
//! println!(
//!     "Map {} is {} by {} pixels.", path.display(),
//!     mymap.size.x * mymap.tile_size.x, mymap.size.y * mymap.tile_size.y
//! );
//!
//! # Ok::<(),tego::Error>(())
//! ```

use std::any::Any;
use std::{fs::File, io::Read};
use core::num::NonZeroU32;

use base64;
use roxmltree::Document;

#[macro_use] extern crate impl_ops;

mod errors;
mod resource_manager;
pub mod math;
pub use resource_manager::ImageLoader;
pub use errors::Error;
pub use errors::Result;

/// Version number consisting out of a MAJOR and MINOR version number, followed by an optional PATCH
#[derive(Debug, PartialEq, Eq)]
pub struct Version(
    /// Major version
    pub u32,
    /// Minor version
    pub u32,
    /// Patch version
    pub Option<u32>
);

impl std::str::FromStr for Version {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut items = s.split('.');

        use Error::ParseError;
        let major = items.next().ok_or(ParseError("Major version is required but missing".into()))?.parse()?;
        let minor = items.next().ok_or(ParseError("Minor version is required but missing".into()))?.parse()?;
        let patch = if let Some(content) = items.next() {
            Some(content.parse()?)
        } else { None };

        Ok(Version(major, minor, patch))
    }
}

pub enum Orientation {
    Orthogonal,
    Isometric,
    Staggered,
    Hexagonal,
}

impl std::str::FromStr for Orientation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        use Orientation::*;
        match s {
            "orthogonal" => Ok(Orthogonal),
            "isometric" => Ok(Isometric),
            "staggered" => Ok(Staggered),
            "hexagonal" => Ok(Hexagonal),
            _ => Err(Error::ParseError(format!("Invalid orientation '{}'", s).into()))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Renderorder {
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}

impl std::str::FromStr for Renderorder {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        use Renderorder::*;
        match s {
            "right-down" => Ok(RightDown),
            "right-up" => Ok(RightUp),
            "left-down" => Ok(LeftDown),
            "left-up" => Ok(LeftUp),
            _ => Err(Error::ParseError(format!("Invalid render order '{}'", s).into()))
        }
    }
}

impl Default for Renderorder {
    fn default() -> Self {
        Renderorder::RightDown
    }
}

/// Global Tile ID
/// A GID acts as an index into any tileset referenced in the map
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[repr(transparent)]
pub struct GID(NonZeroU32);

impl std::str::FromStr for GID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(GID(s.parse()?))
    }
}

fn attribute_or<T>(node: &roxmltree::Node, name: &str, alternative: T) -> Result<T>
    where T: Copy + std::str::FromStr,
          T::Err: std::error::Error + 'static
{
    match node.attribute(name) {
        None => Ok(alternative),
        Some(text) => text.parse().map_err(|e: T::Err| Error::ParseError(Box::new(e)))
    }
}

fn attribute_or_default<T>(node: &roxmltree::Node, name: &str) -> Result<T>
    where T: Default + std::str::FromStr,
          T::Err: std::error::Error + 'static
{
    match node.attribute(name) {
        None => Ok(T::default()),
        Some(text) => text.parse().map_err(|e: T::Err| Error::ParseError(Box::new(e)))
    }
}

#[derive(Debug)]
pub enum ImageStorage {
    SpriteSheet(Box<dyn Any>),
}

pub struct TileSet {
    pub firstgid: GID,
    pub name: String,
    pub tile_size: math::ivec2,
    pub spacing: usize,
    pub margin: usize,
    pub tilecount: usize,
    pub columns: usize,
    pub image: ImageStorage,
}

impl TileSet {
    pub fn from_xml(node: &roxmltree::Node, loader: &mut dyn ImageLoader) -> Result<Self> {
        let map_attr = |name: &str| {
            node.attribute(name).ok_or_else(||{Error::StructureError{
                tag: node.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        if let Some(source) = node.attribute("source") {
            return Err(Error::UnsupportedFeature(format!("Extern tileset at: {}", source)));
        }

        let image_storage;
        use ImageStorage::*;
        if let Some(image) = node.children().filter(|n| n.tag_name().name() == "image").next() {
            image_storage = SpriteSheet(
                loader.load(image.attribute("source").ok_or_else(|| Error::StructureError{
                    tag: image.tag_name().name().into(),
                    msg: "Missing 'source' tag on image".into(),
                })?)?
            );
        } else {
            return Err(Error::UnsupportedFeature("Image collection tilesets are not implemented yet".into()))
        }

        Ok(Self{
            firstgid: map_attr("firstgid")?.parse()?,
            name: map_attr("name")?.into(),
            tile_size: math::ivec2::new(
                map_attr("tilewidth")?.parse()?,
                map_attr("tileheight")?.parse()?
            ),
            spacing: attribute_or_default(node, "spacing")?,
            margin: attribute_or_default(node, "margin")?,
            tilecount: map_attr("tilecount")?.parse()?,
            columns: map_attr("columns")?.parse()?,
            image: image_storage,
        })
    }
}

/// Helper function to read the binary data contained in a "data" tag
/// # Panics
/// The given node has no "encoding" attribute
fn read_data_tag(data_node: &roxmltree::Node) -> Result<Vec<u8>> {
    assert_eq!(data_node.tag_name().name(), "data");
    assert!(data_node.attribute("encoding").is_some());

    match data_node.attribute("encoding").unwrap() {
        "csv" => todo!{"Implement csv parsing"},
        "base64" => {
            // helper macro for decoding compressed data using libflate
            macro_rules! decode_with {
                ($input:ident $compression:ident) => {{
                    use std::io::Read;
                    let mut decoded = Vec::new();
                    let mut decoder = libflate::$compression::Decoder::new(&$input[..])?;
                    decoder.read_to_end(&mut decoded)?;
                    decoded
                }};
            }

            let raw_bytes = base64::decode(data_node.text().unwrap_or_default().trim())
                .map_err(|e| Error::ParseError(Box::new(e)))?
                ;
            let raw_bytes = match data_node.attribute("compression") {
                None => raw_bytes,
                Some("zlib") => decode_with!(raw_bytes zlib),
                Some("gzip") => decode_with!(raw_bytes gzip),
                Some(compression) => Err(Error::StructureError{
                    tag: data_node.tag_name().name().to_string(),
                    msg: format!("Unsupported data compression '{}'", compression)
                })?,
            };
            Ok(raw_bytes)
        },
        encoding => Err(Error::StructureError{
            tag: data_node.tag_name().name().to_string(),
            msg: format!("Unsupported data encoding '{}'", encoding)
        })
    }
}

/// This enum contains the different types of layers that can be found in a map
pub enum Layer{
    /// A layer containing a grid of tiles
    Tile(TileLayer),

    /// A layer grouping mutiple other layer together.
    /// Group Layers may be nested,
    /// forming a tree of layers.
    Group(GroupLayer),

    /// A layer containing objects.
    /// Objects are not aligned to the tile grid.
    /// They can be used for example to mark regions of interest.
    ///
    /// Object layers are also called object  groups.
    Object(ObjectLayer),
}

impl Layer {
    pub fn try_from_xml(node: &roxmltree::Node) -> Option<Result<Self>> {
        use Layer::*;
        match node.tag_name().name() {
            "layer" => Some(TileLayer::from_xml(node).map(|l| Tile(l))),
            "group" => Some(GroupLayer::from_xml(node).map(|l| Group(l))),
            "objectgroup" => Some(ObjectLayer::from_xml(node).map(|l| Object(l))),
            _ => None,
        }
    }
}

pub struct TileIterator<'map, 'layer> {
    map: &'map Map,
    layer: &'layer TileLayer,
    pos: math::ivec2,
}

impl<'map, 'layer> TileIterator<'map, 'layer> {
    pub(crate) fn new(map: &'map Map, layer: &'layer TileLayer) -> Self { Self { map, layer, pos: math::ivec2::new(0, 0) } }
}

impl<'a,'b> Iterator for TileIterator<'a,'b> {
    type Item = (math::ivec2, Option<GID>);

    fn next(&mut self) -> Option<Self::Item> {
        assert_eq!(
            self.map.renderorder,
            Renderorder::RightDown,
            "Only right-down renderorder is implemented right now"
        );
        if self.pos.x >= self.layer.size.x {
            self.pos.x = 0;
            self.pos.y += 1;
        }
        if self.pos.y >= self.layer.size.y {
            return None;
        }

        let idx = self.pos.x + self.layer.size.x * self.pos.y;
        let element = Some((self.pos, self.layer.tiles[idx as usize]));
        self.pos.x += 1;
        element
    }
}

/// A layer to group multiple sub-layers
pub struct GroupLayer {
    pub id: usize,
    pub name: String,
    pub offset: math::ivec2,
    pub opacity: f32,
    pub visible: bool,
    // pub tintcolor: TODO
    pub content: Vec<Layer>
}

impl GroupLayer {
    /// Load a group layer from a TMX "group" node
    pub fn from_xml(node: &roxmltree::Node) -> Result<Self> {
        assert_eq!(node.tag_name().name(), "group");
        let map_attr = |name: &str| {
            node.attribute(name).ok_or_else(||{Error::StructureError{
                tag: node.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        let content = node.children().filter_map(|c| Layer::try_from_xml(&c)).collect::<Result<Vec<_>>>();

        Ok(Self{
            id: map_attr("id")?.parse()?,
            name: node.attribute("name").unwrap_or_default().to_string(),
            offset: math::ivec2::new(
                attribute_or_default(node, "offsetx")?,
                attribute_or_default(node, "offsety")?
            ),
            opacity: attribute_or(node, "opacity", 1.)?,
            visible: attribute_or(node, "opacity", true)?,
            content: content?,
        })
    }
}

pub struct TileLayer {
    pub id: usize,
    pub name: String,
    pub size: math::ivec2,
    pub tiles: Vec<Option<GID>>
}

impl TileLayer {

    fn parse_data(data_node: &roxmltree::Node) -> Result<Vec<Option<GID>>> {
        assert_eq!(data_node.tag_name().name(), "data");

        match data_node.attribute("encoding") {
            None => todo!{"Tag based tile data loading not yet implemented"},
            Some(_) => {
                let raw_bytes = read_data_tag(data_node)?;

                const BYTE_SIZE: usize = std::mem::size_of::<u32>();
                assert!(raw_bytes.len() % BYTE_SIZE == 0);

                // convert chunk of bytes into GIDS (via u32)
                use std::convert::TryInto;
                Ok(
                    raw_bytes.chunks_exact(BYTE_SIZE)
                    .map(|c| Some(GID(NonZeroU32::new(u32::from_le_bytes(c.try_into().unwrap()))?)))
                    .collect()
                )
            }
        }
    }

    pub fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        let map_attr = |name: &str| {
            tmx.attribute(name).ok_or_else(||{Error::StructureError{
                tag: tmx.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };
        Ok(Self{
            id: map_attr("id")?.parse()?,
            name: tmx.attribute("name").unwrap_or_default().to_string(),
            size: math::ivec2::new(
                map_attr("width")?.parse()?,
                map_attr("height")?.parse()?
            ),
            tiles: Self::parse_data(&tmx.children().find(|n| n.tag_name().name() == "data").unwrap())?,
        })
    }


    /// Iterate over the tiles inside of this layer in the order in which they would be rendered.
    /// See [Map::renderorder]. This iterator yields the GID and xy coordinates of the tiles in the
    /// layer, with a None GID for empty tiles.
    ///
    /// # Panics
    ///
    /// At the moment, this function is only implemented for a renderorder of RightDown.
    /// Other render orders result in a panic.
    pub fn tiles_in_renderorder<'a, 'b>(&'b self, map: &'a Map) -> TileIterator<'a, 'b> {
        TileIterator::new(map, &self)
    }
}

/// An ObjectLayer is a container of Objects.
/// Objects are not aligned to the tile grid,
/// and can be used to include extra information in a map.
///
/// Check the [Tiled Documentation](https://doc.mapeditor.org/en/stable/manual/objects/)
/// for more information on objects.
pub struct ObjectLayer {
    pub id: usize,
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub offset: math::ivec2,
}

impl ObjectLayer {
    pub fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        assert_eq!(tmx.tag_name().name(), "objectgroup");

        let map_attr = |name: &str| {
            tmx.attribute(name).ok_or_else(||{Error::StructureError{
                tag: tmx.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };
        Ok(Self{
            id: map_attr("id")?.parse()?,
            name: tmx.attribute("name").unwrap_or_default().to_string(),
            opacity: attribute_or(tmx, "opacity", 1.)?,
            visible: attribute_or(tmx, "opacity", true)?,
            offset: math::ivec2::new(
                attribute_or_default(tmx, "offsetx")?,
                attribute_or_default(tmx, "offsety")?
            ),
        })
    }

}


/// The Map struct is the top level container for all relevant data inside of a Tiled map.
/// A Map consists of [TileSets](TileSet) and [Layers](Layer).
/// Stacking the layers in iteration order creates the final map image.
/// Each layer contains indices ([GIDs](GID)) referencing a specific tile in a tile sets.
pub struct Map {
    pub version: Version,
    pub editor_version: Option<Version>,
    pub orientation: Orientation,
    pub renderorder: Renderorder,
    pub size: math::ivec2,
    pub tile_size: math::ivec2,
    pub tilesets: Vec<TileSet>,
    /// The Layers that make up this map.
    /// The final map image is rendered by stacking the layers in iteration order.
    pub layers: Vec<Layer>,
}

impl Map {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let mut loader = resource_manager::LazyLoader {};
        Self::from_file_with_loader(path, &mut loader)
    }

    pub fn from_file_with_loader(path: &std::path::Path, image_loader: &mut dyn resource_manager::ImageLoader) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut file_xml = String::new();
        file.read_to_string(&mut file_xml)?;

        Self::from_xml_str(&file_xml, image_loader)
    }

    /// Parse a map from xml data
    pub fn from_xml_str(tmx: &str, image_loader: &mut dyn resource_manager::ImageLoader) -> Result<Self> {
        let document = Document::parse(&tmx)?;

        let map_node = document.root_element();

        if map_node.tag_name().name() != "map" {
            return Err(Error::StructureError{
                tag: map_node.tag_name().name().to_string(),
                msg: format!("Expected tag 'map' at root level, got '{}'.", map_node.tag_name().name())
            });
        }

        let map_attr = |name: &str| {
            map_node.attribute(name).ok_or_else(||{Error::StructureError{
                tag: map_node.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        let tilesets = map_node.children()
            .filter(|n| n.tag_name().name() == "tileset")
            .map(|n| TileSet::from_xml(&n, image_loader))
            .collect::<Result<Vec<_>>>()?
        ;

        let mut map = Map {
            version: map_attr("version")?.parse()?,
            editor_version: None,
            orientation: map_attr("orientation")?.parse()?,
            renderorder: attribute_or_default(&map_node, "renderorder")?,
            size: math::ivec2::new(
                map_attr("width")?.parse()?,
                map_attr("height")?.parse()?
            ),
            tile_size: math::ivec2::new(
                map_attr("tilewidth")?.parse()?,
                map_attr("tileheight")?.parse()?
            ),
            tilesets,
            layers:
                map_node.children().filter_map(|c| Layer::try_from_xml(&c)).collect::<Result<Vec<_>>>()?
        };
        if map_node.attribute("tiledversion").is_some() {
            map.editor_version = Some(map_attr("tiledversion")?.parse()?);
        }
        Ok(map)
    }

    /// Fetch the image that belongs to a given GID. Returns the image and the pixel coordinates
    /// where the tile image is inside of that image.
    pub fn tile_image(&self, id: GID) -> Option<(&dyn std::any::Any, math::Rect)> {
        use math::ivec2;
        let tileset = self.tilesets.iter().rfind(|t| t.firstgid <= id)?;


        let size = ivec2::new(tileset.tile_size.x, tileset.tile_size.y);
        let stride = tileset.spacing as i32;
        let stride = size + ivec2::new(stride, stride);

        let lid = (id.0.get() - tileset.firstgid.0.get()) as i32;
        let tile_id = ivec2::new(lid % tileset.columns as i32, lid / tileset.columns as i32);
        let upper_left = ivec2::new(tileset.margin as i32, tileset.margin as i32) + tile_id * stride;

        match &tileset.image {
            ImageStorage::SpriteSheet(spritesheet) => {
                Some((&**spritesheet, math::Rect::new(upper_left, size)))
            },
        }
    }

    /// Iterate over all the layers in this map recursively.
    /// All layers are visited in depth-first pre-order manner.
    /// The iterator yields the group layers, as well as all of their sub-layers.
    ///
    /// In addition to the layer, a number of "pops" is also returned with each item.
    /// This is the number of group layers that was left with this iteration step.
    /// Some attributes of group layers affect all containing layers.
    /// If those attributes are accumulated in a stack,
    /// then the number of pops is the number of elemets to remove from the top of the stack.
    ///
    /// # Example
    ///
    /// Rendering layers under consideration of the group opacity:
    ///
    /// ```ignore
    /// let opacities = vec![1.];
    /// for (layer, pops) in map.iter_layers() {
    ///     opacities.truncate(opacities.len() - pops);
    ///     match layer {
    ///         Layer::Group(group) => { opacities.push(opacities.last().unwrap() * group.opacity) },
    ///         Layer::Tile(tile) => { render_layer(tile, opacities.last()) }
    ///     }
    /// }
    /// ```
    pub fn iter_layers(&self) -> impl Iterator<Item=(&Layer, usize)> {
        LayerIterator::new(&self.layers)
    }
}

struct LayerIterator<'a> {
    iter_stack: Vec<std::slice::Iter<'a, Layer>>
}

impl<'a> LayerIterator<'a> {
    fn new(layers: &'a [Layer]) -> Self { Self { iter_stack: vec![layers.iter()] } }
}

impl<'a> Iterator for LayerIterator<'a> {
    type Item = (&'a Layer, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut pops = 0;
        while let Some(iter) = self.iter_stack.last_mut() {
            if let Some(layer) = iter.next() {
                if let Layer::Group(group) = layer {
                    self.iter_stack.push(group.content.iter());
                }
                return Some((layer, pops));
            } else {
                pops += 1;
                self.iter_stack.pop();
            }
        }
        None
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_version_parsing() -> Result<()> {
        assert_eq!("1.0".parse::<Version>()?, Version(1,0,None));
        assert_eq!("4.5.3".parse::<Version>()?, Version(4,5,Some(3)));
        Ok(())
    }

    #[test]
    fn test_default_render_order() -> Result<()> {
        // explicitly no renderorder
        let map_xml = r#"
            <map
                version="1.5"
                orientation="orthogonal"
                width="1"
                height="1"
                tilewidth="1"
                tileheight="1"
            />
        "#;

        let map = Map::from_xml_str(&map_xml, &mut resource_manager::LazyLoader{})?;
        assert_eq!(map.renderorder, Renderorder::RightDown);
        Ok(())
    }

    #[test]
    fn test_gid_size_optimization() {
        use std::mem::size_of;
        assert_eq!(size_of::<Option<GID>>(), size_of::<u32>());
    }

    #[test]
    fn test_layer_iterator() {
        use Layer::*;
        macro_rules! layer {
            (tile) => {Tile(TileLayer{id: 0, name: "".into(), size: math::ivec2::new(0,0), tiles: vec![]})};
            (group $layers:expr) => {Group(GroupLayer{id:0, name: "".into(), offset: math::ivec2::new(0,0), opacity: 0., visible:false, content: $layers})};
        }

        let layers = vec![
            layer!(tile),
            layer!(group vec![]),
            layer!(tile),
            layer!(group vec![
                layer!(group vec![
                    layer!(tile),
                    layer!(tile),
                ])
            ]),
        ];

        let result: Vec<_> = LayerIterator::new(&layers).collect();
        assert_eq!(result.len(), 7);

        // check that we get the group
        assert!(std::ptr::eq(result[2].0, &layers[2]));

        // no pops on a top level tile layer
        assert_eq!(result[0].1, 0);

        // one pop after the empty group
        assert_eq!(result[2].1, 1);
    }
}
