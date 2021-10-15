//! Test

use std::{fs::File, io::Read};

use base64;
use roxmltree::Document;

mod errors;
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

#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GID(u32);

pub struct TileLayer {
    pub id: usize,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<GID>
}

impl TileLayer {

    fn parse_data(data_node: &roxmltree::Node) -> Result<Vec<GID>> {
        assert_eq!(data_node.tag_name().name(), "data");

        match data_node.attribute("encoding") {
            None => todo!{"Tag based tile data loading not yet implemented"},
            Some("csv") => todo!{"Implement csv parsing"},
            Some("base64") => {
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

                const BYTE_SIZE: usize = std::mem::size_of::<u32>();
                assert!(raw_bytes.len() % BYTE_SIZE == 0);

                // convert chunk of bytes into GIDS (via u32)
                use std::convert::TryInto;
                Ok(raw_bytes.chunks_exact(BYTE_SIZE).map(|c| GID(u32::from_le_bytes(c.try_into().unwrap()))).collect())
            },
            Some(encoding) => Err(Error::StructureError{
                tag: data_node.tag_name().name().to_string(),
                msg: format!("Unsupported data encoding '{}'", encoding)
            })
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
    pub layers: Vec<TileLayer>,
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

        let renderorder = if let Some(attr) = map_node.attribute("renderorder") {
            attr.parse()?
        } else { Renderorder::default() };

        let mut map = Map {
            version: map_attr("version")?.parse()?,
            editor_version: None,
            orientation: map_attr("orientation")?.parse()?,
            renderorder,
            width: map_attr("width")?.parse()?,
            height: map_attr("height")?.parse()?,
            tilewidth: map_attr("tilewidth")?.parse()?,
            tileheight: map_attr("tileheight")?.parse()?,
            layers:
                map_node.children().filter(|n| n.tag_name().name() == "layer")
                .map(|n| TileLayer::from_xml(&n)).collect::<Result<Vec<_>>>()?
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
}
