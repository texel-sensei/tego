//! Test

use std::{fs::File, io::Read};

use thiserror::Error;

use roxmltree::Document;

/// Version number consisting out of a MAJOR and MINOR version number, followed by an optional PATCH
#[derive(Debug, PartialEq, Eq)]
pub struct Version(pub u32,pub u32, pub Option<u32>);

impl std::str::FromStr for Version {
    type Err = MapError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut items = s.split('.');

        use MapError::ParseError;
        let major = items.next().ok_or(ParseError("Major version is required but missing".into()))?.parse()?;
        let minor = items.next().ok_or(ParseError("Minor version is required but missing".into()))?.parse()?;
        let patch = if let Some(content) = items.next() {
            Some(content.parse()?)
        } else { None };

        Ok(Version(major, minor, patch))
    }
}

#[derive(Error, Debug)]
pub enum MapError {
    #[error("")]
    StructureError{ tag: String, msg: String },

    #[error(transparent)]
    ParseError(Box<dyn std::error::Error>),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl From<roxmltree::Error> for MapError {
    fn from(e: roxmltree::Error) -> Self {
        MapError::ParseError(Box::new(e))
    }
}

impl From<std::num::ParseIntError> for MapError {
    fn from(e: std::num::ParseIntError) -> Self {
        MapError::ParseError(Box::new(e))
    }
}

pub enum Orientation {
    Orthogonal,
    Isometric,
    Staggered,
    Hexagonal,
}

pub enum Renderorder {
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}


pub struct Map {
    pub version: Version,
    pub editor_version: Option<Version>,
}

impl Map {
    pub fn from_file(path: &std::path::Path) -> Result<Self, MapError> {
        let mut file = File::open(path)?;

        let mut file_xml = String::new();
        file.read_to_string(&mut file_xml)?;


        Map::from_xml_str(&file_xml)
    }

    /// Parse a map from xml data
    pub fn from_xml_str(tmx: &str) -> Result<Self, MapError> {
        let document = Document::parse(&tmx)?;

        let map_node = document.root_element();

        if map_node.tag_name().name() != "map" {
            return Err(MapError::StructureError{
                tag: map_node.tag_name().name().to_string(),
                msg: format!("Expected tag 'map' at root level, got '{}'.", map_node.tag_name().name())
            });
        }

        let map_attr = |name: &str| {
            map_node.attribute(name).ok_or_else(||{MapError::StructureError{
                tag: map_node.tag_name().name().to_string(),
                msg: format!("Required attribute '{}' missing", name)
            }})
        };

        let mut map = Map {
            version: map_attr("version")?.parse()?,
            editor_version: None,
        };
        if map_node.attribute("tiledversion").is_some() {
            map.editor_version = Some(map_attr("tiledversion")?.parse()?);
        }
        Ok(map)
    }
}
