use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    /// An error in the structure of the data, e.g. a required tag is missing.
    #[error(r#"Error in the map data at '{tag}': "{msg}""#)]
    StructureError{ tag: String, msg: String },

    /// An error that happened while parsing the map, e.g. the tmx file is not valid xml.
    #[error(transparent)]
    ParseError(Box<dyn std::error::Error>),

    /// A general IO error, e.g. opening a file failed
    #[error(transparent)]
    IO(#[from] std::io::Error),

    /// Map uses features that are not (yet) supported
    #[error("Feature not supported: {0}")]
    UnsupportedFeature(String)
}

impl From<roxmltree::Error> for Error {
    fn from(e: roxmltree::Error) -> Self {
        Error::ParseError(Box::new(e))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseError(Box::new(e))
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::ParseError(Box::new(e))
    }
}

impl From<std::str::ParseBoolError> for Error {
    fn from(e: std::str::ParseBoolError) -> Self {
        Error::ParseError(Box::new(e))
    }
}
