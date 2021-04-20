use std::convert::TryInto;
use std::str::FromStr;

use crate::errors::Error;

#[derive(Debug, Clone, Copy)]
pub struct RGB {
    pub vals: [u8; 3],
}

impl RGB {
    pub fn r(&self) -> u8 {
        self.vals[0]
    }
    pub fn g(&self) -> u8 {
        self.vals[1]
    }
    pub fn b(&self) -> u8 {
        self.vals[2]
    }
}

impl FromStr for RGB {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        let vals: [u8; 3] = s
            .trim_start_matches("#")
            .chars()
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|s| u8::from_str_radix(&format!("{}{}", s[0], s[1]), 16))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()?;
        Ok(RGB { vals: vals })
    }
}
