//! This module provides functionality for custom
//! [properties](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#properties)

use std::collections::HashMap;

use crate::{Color, Error, Result};

/// Reference type to an object stored in this map.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct ObjectReference(i64);

fn parse_string_value<'a>(tmx: &'a roxmltree::Node) -> &'a str {
    match tmx.attribute("value") {
        Some(text) => text,
        None => tmx.text().unwrap_or_default()
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
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
    /// Parse a single property value from an `<property>` xml node
    fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        use PropertyValue::*;

        // Helper to parse a value if the attribute "value" exists or return the default if not
        macro_rules! parse {
            () => {
                tmx.attribute("value").map(|t| t.parse()).transpose()?.unwrap_or_default()
            };
        }

        match tmx.attribute("type").unwrap_or("string") {
            "string" => Ok(String(parse_string_value(tmx).into())),
            "int" => Ok(Int(parse!())),
            "float" => Ok(Float(parse!())),
            "bool" => Ok(Bool(parse!())),
            "color" => Ok(Color(parse!())),
            "file" => Ok(File(tmx.attribute("value").unwrap_or_default().into())),
            "object" => Ok(Object(ObjectReference(parse!()))),
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

#[derive(Debug, PartialEq, Clone)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

impl Property {
    /// Try to get the value of this property as a [&str].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_str(&self) -> Result<&str> {
        match &self.value {
            PropertyValue::String(text) => Ok(&text),
            _ => Err(Error::PropertyTypeError)
        }
    }

    /// Try to get the value of this property as a [i64].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_i64(&self) -> Result<i64> {
        match &self.value {
            PropertyValue::Int(val) => Ok(*val),
            _ => Err(Error::PropertyTypeError)
        }
    }

    /// Try to get the value of this property as a [f64].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_f64(&self) -> Result<f64> {
        match &self.value {
            PropertyValue::Float(val) => Ok(*val),
            _ => Err(Error::PropertyTypeError)
        }
    }


    /// Try to get the value of this property as a [bool].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_bool(&self) -> Result<bool> {
        match &self.value {
            PropertyValue::Bool(val) => Ok(*val),
            _ => Err(Error::PropertyTypeError)
        }
    }

    /// Try to get the value of this property as a [Color].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_color(&self) -> Result<Color> {
        match &self.value {
            PropertyValue::Color(val) => Ok(*val),
            _ => Err(Error::PropertyTypeError)
        }
    }

    /// Try to get the value of this property as a file path.
    /// This returns the path as a [&str].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_file(&self) -> Result<&str> {
        match &self.value {
            PropertyValue::File(val) => Ok(val),
            _ => Err(Error::PropertyTypeError)
        }
    }

    /// Try to get the value of this property as an [ObjectReference].
    ///
    /// Returns a [Error::PropertyTypeError] if it contains a different type.
    pub fn as_object_ref(&self) -> Result<ObjectReference> {
        match &self.value {
            PropertyValue::Object(val) => Ok(*val),
            _ => Err(Error::PropertyTypeError)
        }
    }
}

#[derive(Clone, Debug)]
pub struct PropertyContainer {
    properties: HashMap<String, Property>,
}

impl PropertyContainer {

    pub(crate) fn new() -> Self { Self{ properties: HashMap::new() } }

    pub(crate) fn from_xml(tmx: &roxmltree::Node) -> Result<Self> {
        let mut this = Self::new();
        this.update_from_xml(tmx)?;
        Ok(this)
    }

    /// Parse the properties from an tmx xml node.
    /// This function takes any node in the tmx file that supports properties
    /// and looks for a child node "properties".
    ///
    /// Properties from the xml node will overwrite properties with the same name
    /// in self.
    pub(crate) fn update_from_xml(&mut self, tmx: &roxmltree::Node) -> Result<()> {
        let properties = tmx.children().find(|c| c.tag_name().name() == "properties");

        if properties.is_none() {
            return Ok(());
        }
        let properties = properties.unwrap();

        for property in properties.children().filter(|c| c.tag_name().name() == "property") {
            let name = match property.attribute("name") {
                Some(name) => name,
                None => return Err(
                    Error::StructureError{
                        tag: property.tag_name().name().into(),
                        msg: format!("Property is missing a name!")
                    }
                )
            };

            self.properties.insert(
                name.to_string(),
                Property{name: name.into(), value: PropertyValue::from_xml(&property)?}
            );
        }

        Ok(())
    }

    /// Iterate over all the properties stored in this container.
    pub fn iter(&self) -> impl Iterator<Item=&Property> {
        self.properties.values()
    }
}

impl std::ops::Index<&str> for PropertyContainer {
    type Output = PropertyValue;

    /// Get the (first) property with the given name if it exists
    ///
    /// # Panics
    /// If the given property does not exist, this function will panic.
    fn index(&self, index: &str) -> &Self::Output {
        &self.properties[index].value
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore] // TODO(texel, 2021-11-22): fix testcase, failing because of property order
    fn test_property_parser() {
        let tmx = r##"
            <map>
                <properties>
                    <property name="all_defaults"/>
                    <property name="string_value" type="string" value="Hello"/>
                    <property name="color_value" type="color" value="#FFcc00"/>
                    <property name="contained_text">Hello World</property>
                </properties>
            </map>
        "##;

        let tmx = roxmltree::Document::parse(tmx).unwrap();

        let properties = PropertyContainer::from_xml(&tmx.root_element()).unwrap();
        let properties: Vec<_> = properties.iter().collect();

        assert_eq!(properties.len(), 4);
        use PropertyValue::*;
        assert_eq!(
            properties,
            vec![
                &Property{name: "all_defaults".into(), value: String("".into())},
                &Property{name: "string_value".into(), value: String("Hello".into())},
                &Property{name: "color_value".into(), value: Color(crate::Color::from_argb(0xFF, 0xFF, 0xCC, 0x00))},
                &Property{name: "contained_text".into(), value: String("Hello World".into())},
            ]
        );
    }

    #[test]
    fn test_property_index_access() {
        let tmx = r##"
            <map>
                <properties>
                    <property name="all_defaults"/>
                    <property name="string_value" type="string" value="Hello"/>
                    <property name="color_value" type="color" value="#FFcc00"/>
                    <property name="contained_text">Hello World</property>
                </properties>
            </map>
        "##;

        let tmx = roxmltree::Document::parse(tmx).unwrap();

        let properties = PropertyContainer::from_xml(&tmx.root_element()).unwrap();
        assert_eq!(properties["all_defaults"], PropertyValue::String("".into()));
    }
}
