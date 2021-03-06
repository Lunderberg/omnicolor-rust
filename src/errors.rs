use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    NoStagesDefined,
    NoLayersDefined,
    ParseIntError(std::num::ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
    VecLengthError(usize),
    IoError(std::io::Error),
    //NoneError,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseIntError(e)
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::ParseFloatError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<Vec<u8>> for Error {
    fn from(e: Vec<u8>) -> Self {
        Error::VecLengthError(e.len())
    }
}

// impl From<core::option::NoneError> for Error {
//     fn from(e: core::option::NoneError) -> Self {
//         Error::NoneError
//     }
// }
