//! Test

use std::{fs::File, io::Read};
use core::num::NonZeroU32;

use base64;
use roxmltree::Document;

mod errors;
mod resource_manager;
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

pub struct TileSet {
    pub firstgid: GID,
    pub name: String,
    pub tilewidth: usize,
    pub tileheight: usize,
    pub spacing: usize,
    pub margin: usize,
    pub tilecount: usize,
    pub columns: usize,
    // TODO(texel, 2021-10-15): somehow handle the image data
}

impl TileSet {
    pub fn from_xml(node: &roxmltree::Node) -> Result<Self> {
        let map_attr = |name: &str| {
            node.attribute(name).ok_or_else(||{Error::StructureError{
                tag: node.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        if let Some(source) = node.attribute("source") {
            return Err(Error::UnsupportedFeature(format!("Extern tileset at: {}", source)));
        }

        Ok(Self{
            firstgid: map_attr("firstgid")?.parse()?,
            name: map_attr("name")?.into(),
            tilewidth: map_attr("tilewidth")?.parse()?,
            tileheight: map_attr("tileheight")?.parse()?,
            spacing: attribute_or_default(node, "spacing")?,
            margin: attribute_or_default(node, "margin")?,
            tilecount: map_attr("tilecount")?.parse()?,
            columns: map_attr("columns")?.parse()?,
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
    Group(GroupLayer),
}

impl Layer {
    pub fn try_from_xml(node: &roxmltree::Node) -> Option<Result<Self>> {
        use Layer::*;
        match node.tag_name().name() {
            "layer" => Some(TileLayer::from_xml(node).map(|l| Tile(l))),
            "group" => Some(GroupLayer::from_xml(node).map(|l| Group(l))),
            _ => None,
        }
    }
}

pub struct TileIterator<'map, 'layer> {
    map: &'map Map,
    layer: &'layer TileLayer,
    x: usize,
    y: usize,
}

impl<'map, 'layer> TileIterator<'map, 'layer> {
    pub(crate) fn new(map: &'map Map, layer: &'layer TileLayer) -> Self { Self { map, layer, x: 0, y: 0 } }
}

impl<'a,'b> Iterator for TileIterator<'a,'b> {
    type Item = (usize, usize, Option<GID>);

    fn next(&mut self) -> Option<Self::Item> {
        assert_eq!(
            self.map.renderorder,
            Renderorder::RightDown,
            "Only right-down renderorder is implemented right now"
        );
        if self.x >= self.layer.width {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= self.layer.height {
            return None;
        }

        let idx = self.x + self.layer.width * self.y;
        let element = Some((self.x, self.y, self.layer.tiles[idx]));
        self.x += 1;
        element
    }
}

/// A layer to group multiple sub-layers
pub struct GroupLayer {
    pub id: usize,
    pub name: String,
    pub offsetx: usize,
    pub offsety: usize,
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
            offsetx: attribute_or_default(node, "offsetx")?,
            offsety: attribute_or_default(node, "offsety")?,
            opacity: attribute_or(node, "opacity", 1.)?,
            visible: attribute_or(node, "opacity", true)?,
            content: content?,
        })
    }
}

pub struct TileLayer {
    pub id: usize,
    pub name: String,
    pub width: usize,
    pub height: usize,
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
            width: map_attr("width")?.parse()?,
            height: map_attr("height")?.parse()?,
            tiles: Self::parse_data(&tmx.children().find(|n| n.tag_name().name() == "data").unwrap())?,
        })
    }


    /// Iterate over the tiles inside of this layer in the order in which they would be rendered.
    /// See [Map::renderorder]. This iterator yields the GID and xy coordinates of the tiles in the
    /// layer, with an empty GID ([GID::is_empty()]) for empty tiles.
    ///
    /// # Panics
    ///
    /// At the moment, this function is only implemented for a renderorder of RightDown.
    /// Other render orders result in a panic.
    pub fn tiles_in_renderorder<'a, 'b>(&'b self, map: &'a Map) -> TileIterator<'a, 'b> {
        TileIterator::new(map, &self)
    }
}


pub struct Map {
    pub version: Version,
    pub editor_version: Option<Version>,
    pub orientation: Orientation,
    pub renderorder: Renderorder,
    pub width: usize,
    pub height: usize,
    pub tilewidth: usize,
    pub tileheight: usize,
    pub tilesets: Vec<TileSet>,
    pub layers: Vec<Layer>,
}

impl Map {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut file_xml = String::new();
        file.read_to_string(&mut file_xml)?;


        Map::from_xml_str(&file_xml)
    }

    /// Parse a map from xml data
    pub fn from_xml_str(tmx: &str) -> Result<Self> {
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

        let tilesets = map_node.children().filter(|n| n.tag_name().name() == "tileset")
            .map(|n| TileSet::from_xml(&n)).collect::<Result<Vec<_>>>()?;

        let mut map = Map {
            version: map_attr("version")?.parse()?,
            editor_version: None,
            orientation: map_attr("orientation")?.parse()?,
            renderorder: attribute_or_default(&map_node, "renderorder")?,
            width: map_attr("width")?.parse()?,
            height: map_attr("height")?.parse()?,
            tilewidth: map_attr("tilewidth")?.parse()?,
            tileheight: map_attr("tileheight")?.parse()?,
            tilesets,
            layers:
                map_node.children().filter_map(|c| Layer::try_from_xml(&c)).collect::<Result<Vec<_>>>()?
        };
        if map_node.attribute("tiledversion").is_some() {
            map.editor_version = Some(map_attr("tiledversion")?.parse()?);
        }
        Ok(map)
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

        let map = Map::from_xml_str(&map_xml)?;
        assert_eq!(map.renderorder, Renderorder::RightDown);
        Ok(())
    }

    #[test]
    fn test_gid_size_optimization() {
        use std::mem::size_of;
        assert_eq!(size_of::<Option<GID>>(), size_of::<u32>());
    }
}
