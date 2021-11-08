//! This module provides functionality for custom
//! [properties](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#properties)

use crate::{Color, Error, Result};

/// Reference type to an object stored in this map.
pub struct ObjectReference(i64);

#[non_exhaustive]
pub enum PropertyValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Color(Color),
    File(String),
    Object(ObjectReference),
}

impl PropertyValue {
    fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        use PropertyValue::*;

        macro_rules! parse {
            () => {
                tmx.attribute("value").map(|t| t.parse()).transpose()?.unwrap_or_default()
            };
        }

        match tmx.attribute("type").unwrap_or("string") {
            "string" => todo!{},
            "int" => Ok(Int(parse!())),
            "float" => Ok(Float(parse!())),
            "bool" => Ok(Bool(parse!())),
            "color" => Ok(Color(parse!())),
            "file" => todo!{},
            "object" => todo!{},
            other => Err(Error::StructureError{
                    tag: tmx.tag_name().name().into(),
                    msg: format!(
                        "Unknown property type '{}' for property '{}'",
                        other,
                        tmx.attribute("name").unwrap_or("")
                    ),
                }),
        }
    }
}

pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

pub struct PropertyContainer {
    properties: Vec<Property>
}

impl PropertyContainer {
    /// Parse the properties from an tmx xml node.
    /// This function takes any node in the tmx file that supports properties
    /// and looks for a child node "properties".
    pub(crate) fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        let mut container = PropertyContainer{properties: Vec::new()};

        let properties = tmx.children().find(|c| c.tag_name().name() == "properties");

        if properties.is_none() {
            return Ok(container);
        }
        let properties = properties.unwrap();

        for property in properties.children() {
            let name = match property.attribute("name") {
                Some(name) => name,
                None => return Err(
                    Error::StructureError{
                        tag: property.tag_name().name().into(),
                        msg: format!("Property is missing a name!")
                    }
                )
            };

            container.properties.push(Property{name: name.into(), value: PropertyValue::from_xml(&property)?});
        }

        Ok(container)
    }
}
