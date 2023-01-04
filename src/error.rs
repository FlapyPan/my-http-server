use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::result::Result as StdResult;

pub type Error = Box<dyn StdError + Sync + Send>;

pub type Result<T> = StdResult<T, Error>;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fail(pub String);

impl Fail {
    pub fn new<E>(err: E) -> Box<Self> where E: Display {
        Box::new(Fail(err.to_string()))
    }
    pub fn from<T, E>(err: E) -> Result<T> where E: Display {
        Err(Self::new(err))
    }
}

impl StdError for Fail {}

impl Display for Fail {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}
