#[derive(Debug)]
pub enum Error {
    NoPaletteDefined,
    ParseIntError(std::num::ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
    //NoneError,
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

// impl From<core::option::NoneError> for Error {
//     fn from(e: core::option::NoneError) -> Self {
//         Error::NoneError
//     }
// }
