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

use std::rc::Rc;
use std::any::Any;
use std::{fs::File, io::Read};
use core::num::NonZeroU32;

use base64;
use roxmltree::Document;

#[macro_use] extern crate impl_ops;

mod errors;
mod resource_manager;
mod property;
pub mod math;
pub use resource_manager::{ResourceManager, ImageLoader, Provider, FileProvider};
pub use property::{PropertyContainer, Property};
pub use errors::Error;
pub use errors::Result;

const GID_HORIZONTAL_FLIP_FLAG: u32 = 0x80000000;
const GID_VERTICAL_FLIP_FLAG: u32   = 0x40000000;
const GID_DIAGONAL_FLIP_FLAG: u32   = 0x20000000;

const GID_FLIP_MASK: u32 = GID_HORIZONTAL_FLIP_FLAG | GID_VERTICAL_FLIP_FLAG | GID_DIAGONAL_FLIP_FLAG;

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

/// An 8 bit RGB color with alpha value.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub struct Color(u32);

impl Color {
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Color(
              (a as u32) << 24
            | (r as u32) << 16
            | (g as u32) << 8
            | (b as u32) << 0
        )
    }

    pub fn alpha(&self) -> u8 { ((self.0 >> 24) & 0xFF) as u8 }
    pub fn red(&self)   -> u8 { ((self.0 >> 16) & 0xFF) as u8 }
    pub fn green(&self) -> u8 { ((self.0 >>  8) & 0xFF) as u8 }
    pub fn blue(&self)  -> u8 { ((self.0 >>  0) & 0xFF) as u8 }

    pub fn to_u32(&self) -> u32 { self.0 }
}

impl std::str::FromStr for Color {
    type Err = Error;

    /// Parse a color from a hex string.
    ///
    /// ```
    /// let red: tego::Color = "#FF0000".parse()?;
    /// assert_eq!(red.red(), 255);
    /// assert_eq!(red.blue(), 0);
    /// assert_eq!(red.alpha(), 255);
    /// # Ok::<(),tego::Error>(())
    /// ```
    fn from_str(s: &str) -> Result<Self> {
        use Error::*;
        let make_error = || ParseError(format!("Invalid color string, expected #AARRGGBB, got '{}'", s).into());

        let s = s
            .strip_prefix('#')
            .ok_or_else(make_error)?;

        match s.len() {
            8 => {
                let [a, r, g, b] = u32::from_str_radix(s, 16)?.to_be_bytes();
                Ok(Color::from_argb(a, r, g, b))
            },
            6 => {
                let [_, r, g, b] = u32::from_str_radix(s, 16)?.to_be_bytes();
                Ok(Color::from_argb(255, r, g, b))
            },
            _ => Err(make_error())
        }
    }
}

/// Global Tile ID
/// A GID acts as an index into any tileset referenced in the map
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[repr(transparent)]
pub struct GID(NonZeroU32);

impl GID {
    const fn as_raw(&self) -> u32 {
        self.0.get()
    }

    /// Turn this GID into an u32 for indexing.
    /// This function masks the bits used for tile flipping,
    /// to get the flip information use [GID::flip_horizontal], [GID::flip_vertical] and
    /// [GID::flip_diagonal].
    ///
    /// This is a low level function,
    /// for looking up the image for a tile prefer to use [Map::tile_image] instead.
    pub const fn to_id(&self) -> u32 {
        self.as_raw() & !GID_FLIP_MASK
    }

    /// Return whether this tile is flipped horizontally or not
    pub fn flip_horizontal(&self) -> bool {
        (self.as_raw() & GID_HORIZONTAL_FLIP_FLAG) == GID_HORIZONTAL_FLIP_FLAG
    }

    /// Return whether this tile is flipped vertically or not
    pub fn flip_vertical(&self) -> bool {
        (self.as_raw() & GID_VERTICAL_FLIP_FLAG) == GID_VERTICAL_FLIP_FLAG
    }

    /// Return whether this tile is flipped diagonally or not
    pub fn flip_diagonal(&self) -> bool {
        (self.as_raw() & GID_DIAGONAL_FLIP_FLAG) == GID_DIAGONAL_FLIP_FLAG
    }
}

impl std::str::FromStr for GID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(GID(s.parse()?))
    }
}

fn attribute<T>(node: &roxmltree::Node, name: &str) -> Result<T>
    where T: std::str::FromStr,
          T::Err: std::error::Error + 'static,
          Error: From<<T as std::str::FromStr>::Err>
{
    Ok(node.attribute(name).ok_or_else(||{Error::StructureError{
        tag: node.tag_name().name().to_string(),
        msg: format!("Required attribute '{}' missing", name)
    }})?.parse()?)
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

impl math::ivec2 {
    pub(crate) fn from_tmx_or_default(tmx: &roxmltree::Node, x_attr: &str, y_attr: &str) -> Result<Self> {
        Ok(Self::new(
            attribute_or_default(tmx, x_attr)?,
            attribute_or_default(tmx, y_attr)?
        ))
    }
}

impl math::fvec2 {
    pub(crate) fn from_tmx_or_default(tmx: &roxmltree::Node, x_attr: &str, y_attr: &str) -> Result<Self> {
        Ok(Self::new(
            attribute_or_default(tmx, x_attr)?,
            attribute_or_default(tmx, y_attr)?
        ))
    }
}

#[derive(Debug)]
pub enum ImageStorage {
    SpriteSheet(Rc<dyn Any>),
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
    pub properties: PropertyContainer,
}

impl TileSet {
    pub fn from_xml(node: &roxmltree::Node, loader: &mut ResourceManager) -> Result<Self> {
        let mut data_node = *node;

        // Need those two for lifetime reasons related to data_node
        #[allow(unused_assignments)]
        let mut extern_document = None;
        #[allow(unused_assignments)]
        let mut extern_text = None;

        if let Some(source) = node.attribute("source") {
            extern_text = Some(loader.load_text(source)?);
            extern_document = Some(roxmltree::Document::parse(extern_text.as_ref().unwrap())?);
            data_node = extern_document.as_ref().unwrap().root_element();
        }

        let image_storage;
        use ImageStorage::*;
        if let Some(image) = data_node.children().filter(|n| n.tag_name().name() == "image").next() {
            image_storage = SpriteSheet(
                loader.load_image(image.attribute("source").ok_or_else(|| Error::StructureError{
                    tag: image.tag_name().name().into(),
                    msg: "Missing 'source' tag on image".into(),
                })?)?
            );
        } else {
            return Err(Error::UnsupportedFeature("Image collection tilesets are not implemented yet".into()))
        }

        Ok(Self{
            firstgid: attribute(node, "firstgid")?,
            name: attribute(&data_node, "name")?,
            tile_size: math::ivec2::new(
                attribute(&data_node, "tilewidth")?,
                attribute(&data_node, "tileheight")?
            ),
            spacing: attribute_or_default(&data_node, "spacing")?,
            margin: attribute_or_default(&data_node, "margin")?,
            tilecount: attribute(&data_node, "tilecount")?,
            columns: attribute(&data_node, "columns")?,
            image: image_storage,
            properties: PropertyContainer::from_xml(&data_node)?,
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
#[non_exhaustive]
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
#[non_exhaustive]
pub struct GroupLayer {
    pub id: Option<usize>,
    pub name: String,
    pub offset: math::ivec2,
    pub opacity: f32,
    pub visible: bool,
    pub tintcolor: Color,
    pub content: Vec<Layer>,
    pub properties: PropertyContainer,
}

impl GroupLayer {
    /// Load a group layer from a TMX "group" node
    pub fn from_xml(node: &roxmltree::Node) -> Result<Self> {
        assert_eq!(node.tag_name().name(), "group");

        let content = node.children().filter_map(|c| Layer::try_from_xml(&c)).collect::<Result<Vec<_>>>();

        Ok(Self{
            id: node.attribute("id").map(|t| t.parse()).transpose()?,
            name: node.attribute("name").unwrap_or_default().to_string(),
            offset: math::ivec2::from_tmx_or_default(node, "offsetx", "offsety")?,
            opacity: attribute_or(node, "opacity", 1.)?,
            visible: attribute_or(node, "opacity", true)?,
            tintcolor: attribute_or(node, "tintcolor", Color::from_argb(255, 255, 255, 255))?,
            content: content?,
            properties: PropertyContainer::from_xml(node)?,
        })
    }
}

#[non_exhaustive]
pub struct TileLayer {
    pub id: Option<usize>,
    pub name: String,
    pub size: math::ivec2,

    /// Color that is multiplied with the colors of the tiles in this layer.
    /// Defaults to opaque white, which acts as a no-op when multiplied.
    ///
    /// *Note:* Multiplication with the raw values of the [Color] struct would
    /// lead to the wrong result! The colors must first be converted to the
    /// invervall [0-1] (division by 255).
    pub tintcolor: Color,
    pub tiles: Vec<Option<GID>>,

    pub properties: PropertyContainer,
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
            id: tmx.attribute("id").map(|t| t.parse()).transpose()?,
            name: tmx.attribute("name").unwrap_or_default().to_string(),
            size: math::ivec2::new(
                map_attr("width")?.parse()?,
                map_attr("height")?.parse()?
            ),
            tintcolor: attribute_or(tmx, "tintcolor", Color::from_argb(255, 255, 255, 255))?,
            tiles: Self::parse_data(&tmx.children().find(|n| n.tag_name().name() == "data").unwrap())?,
            properties: PropertyContainer::from_xml(tmx)?,
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
#[non_exhaustive]
pub struct ObjectLayer {
    pub id: Option<usize>,
    pub name: String,

    /// Color that is used to render [Objects](Object) in this layer.
    pub color: Color,
    pub opacity: f32,
    pub visible: bool,

    /// Color that is multiplied with the color of tile-objects in this layer.
    /// Acts the same way as [TileLayer::tintcolor].
    pub tintcolor: Color,

    pub offset: math::ivec2,

    /// The [Objects](Object) contained in this layer
    pub content: Vec<Object>,

    pub properties: PropertyContainer,
}

impl ObjectLayer {
    pub fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        assert_eq!(tmx.tag_name().name(), "objectgroup");

        let content = tmx.children()
            .filter(|t| t.tag_name().name() == "object")
            .map(|t| Object::from_xml(&t))
            .collect::<Result<_>>()?
        ;

        Ok(Self{
            id: tmx.attribute("id").map(|t| t.parse()).transpose()?,
            name: tmx.attribute("name").unwrap_or_default().to_string(),
            color: attribute_or(tmx, "color", Color::from_argb(255, 160, 160, 164))?,
            opacity: attribute_or(tmx, "opacity", 1.)?,
            visible: attribute_or(tmx, "opacity", true)?,
            tintcolor: attribute_or(tmx, "tintcolor", Color::from_argb(255, 255, 255, 255))?,
            offset: math::ivec2::from_tmx_or_default(tmx, "offsetx", "offsety")?,
            content,
            properties: PropertyContainer::from_xml(tmx)?,
        })
    }

}

/// An element of an [ObjectLayer].
/// Objects do not need to be aligned to the normal tile grid.
/// Objects can have different kinds,
/// (e.g. rect, ellipse, text).
/// See [ObjectKind] for more info.
#[non_exhaustive]
pub struct Object {
    pub id: usize,
    pub name: String,
    pub type_: String,
    pub pos: math::fvec2,
    pub size: math::fvec2,
    pub rotation: f32,
    pub tile_id: Option<GID>,
    pub visible: bool,
    pub kind: ObjectKind,
    pub properties: PropertyContainer,
}

impl Object {
    fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        let map_attr = |name: &str| {
            tmx.attribute(name).ok_or_else(||{Error::StructureError{
                tag: tmx.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        let tile_id = if let Some(txt) = tmx.attribute("gid") {
            Some(txt.parse()?)
        } else {
            None
        };

        Ok(Object{
            id: map_attr("id")?.parse()?,
            name: attribute_or_default(tmx, "name")?,
            type_: attribute_or_default(tmx, "type")?,
            pos: math::fvec2::from_tmx_or_default(tmx, "x", "y")?,
            size: math::fvec2::from_tmx_or_default(tmx, "width", "height")?,
            rotation: attribute_or_default(tmx, "rotation")?,
            tile_id,
            visible: attribute_or(tmx, "visible", true)?,
            kind: ObjectKind::from_xml(tmx)?,
            properties: PropertyContainer::from_xml(tmx)?,
        })
    }
}

#[non_exhaustive]
pub enum ObjectKind {
    Rect,
    Ellipse,
    Point,
    Polygon {
        points: Vec<math::fvec2>
    },
    Polyline {
        points: Vec<math::fvec2>
    },
    Text {
        content: String
        // todo
    },
}

trait AsPointListExt { fn as_point_list(&self) -> Result<Vec<math::fvec2>>; }

impl AsPointListExt for &str {
    fn as_point_list(&self) -> Result<Vec<math::fvec2>> {
        let mut points = vec![];
        for point in self.split_ascii_whitespace() {
            let mut coords = point.split(',');
            if let (Some(x), Some(y), None) = (coords.next(), coords.next(), coords.next()) {
                points.push(math::fvec2::new(x.parse()?,y.parse()?));
            } else {
                return Err(Error::ParseError(format!("{} is not a valid point", point).into()));
            }
        }
        Ok(points)
    }
}

impl ObjectKind {
    fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        use ObjectKind::*;
        use Error::StructureError;
        for child in tmx.children() {
            match child.tag_name().name() {
                "ellipse" => return Ok(Ellipse),
                "point" => return Ok(Point),
                poly @ ("polygon" | "polyline") => {
                    let points = child
                        .attribute("points")
                        .ok_or(StructureError{
                            tag: child.tag_name().name().into(),
                            msg: "Missing attribute points".into()
                        })?
                        .as_point_list()?
                    ;
                    return Ok(match poly {
                        "polygon" => Polygon { points },
                        "polyline" => Polyline { points },
                        _ => unreachable!(),
                    });
                }
                "text" => {
                    return Ok(Text {
                        content: child.text().unwrap_or_default().into()
                    });
                },
                _ => continue,
            }
        }
        Ok(Rect)
    }
}


/// The Map struct is the top level container for all relevant data inside of a Tiled map.
/// A Map consists of [TileSets](TileSet) and [Layers](Layer).
/// Stacking the layers in iteration order creates the final map image.
/// Each layer contains indices ([GIDs](GID)) referencing a specific tile in a tile sets.
#[non_exhaustive]
pub struct Map {
    pub version: Version,
    pub editor_version: Option<Version>,
    pub orientation: Orientation,
    pub renderorder: Renderorder,
    pub size: math::ivec2,
    pub tile_size: math::ivec2,
    pub tilesets: Vec<TileSet>,

    /// Background color of this map.
    /// By default fully transparent.
    pub backgroundcolor: Color,

    /// The Layers that make up this map.
    /// The final map image is rendered by stacking the layers in iteration order.
    pub layers: Vec<Layer>,

    /// Custom properties contained in this map.
    pub properties: PropertyContainer,
}

impl Map {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        Self::from_file_with_loader(path, &mut ResourceManager::default())
    }

    pub fn from_file_with_loader(path: &std::path::Path, resource_manager: &mut ResourceManager) -> Result<Self> {
        let mut file = File::open(path)?;

        // TODO(texel, 2021-11-10): Change to use resource manager
        let mut file_xml = String::new();
        file.read_to_string(&mut file_xml)?;

        resource_manager.set_base_path(path.parent().unwrap_or(path).to_string_lossy().to_string());
        Self::from_xml_str(&file_xml, resource_manager)
    }

    /// Parse a map from xml data
    pub fn from_xml_str(tmx: &str, resource_manager: &mut ResourceManager) -> Result<Self> {
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
            .map(|n| TileSet::from_xml(&n, resource_manager))
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
            backgroundcolor: attribute_or_default(&map_node, "backgroundcolor")?,
            layers:
                map_node.children().filter_map(|c| Layer::try_from_xml(&c)).collect::<Result<Vec<_>>>()?,
            properties: PropertyContainer::from_xml(&map_node)?,
        };
        if map_node.attribute("tiledversion").is_some() {
            map.editor_version = Some(map_attr("tiledversion")?.parse()?);
        }
        Ok(map)
    }

    /// Fetch the image that belongs to a given GID.
    /// Returns the image and the pixel coordinates where the tile image is inside of that image.
    ///
    /// Important: To correctly draw the tile,
    ///     inspect the [GID] passed to this function to lookup information if/how the tile should
    ///     be flipped.
    pub fn tile_image(&self, id: GID) -> Option<(&dyn std::any::Any, math::Rect)> {
        use math::ivec2;
        let tileset = self.tilesets.iter().rfind(|t| t.firstgid <= id)?;

        let size = ivec2::new(tileset.tile_size.x, tileset.tile_size.y);
        let stride = tileset.spacing as i32;
        let stride = size + ivec2::new(stride, stride);

        let lid = (id.to_id() - tileset.firstgid.to_id()) as i32;
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

        let map = Map::from_xml_str(&map_xml, &mut ResourceManager::default())?;
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
            (tile) => {Tile(TileLayer{
                id: Some(0), name: "".into(), size: math::ivec2::new(0,0),
                tintcolor: Color::default(), tiles: vec![],
                properties: PropertyContainer::new(),
            })};
            (group $layers:expr) => {Group(GroupLayer{
                id: Some(0), name: "".into(), offset: math::ivec2::new(0,0),
                opacity: 0., tintcolor: Color::default(), visible:false,
                content: $layers, properties: PropertyContainer::new(),
            })};
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

    #[test]
    fn test_color_parsing() {
        assert_eq!(Color::from_argb(255,255,255,255), "#FFFFFFFF".parse().unwrap());
        assert_eq!(Color::from_argb(255,  0,255,255),   "#00FFFF".parse().unwrap());

        // missing hashtag (#)
        assert!("00FFFF".parse::<Color>().is_err());

        // not enough components
        assert!("#FFFF".parse::<Color>().is_err());

        // too many enough components
        assert!("#FF00FF00FF".parse::<Color>().is_err());

        // invalid hex
        assert!("#FQ00FF".parse::<Color>().is_err());
    }
}
