use super::{
  Fraction,
  Resolution,
};

#[derive(Debug)]
pub enum Error {
  ShmError(bifrost::Error),
  BadEnumValue,
  NotFound,
  AlreadyExists,
  NotAFrame,
  BadResolution(Resolution),
  BadFrameRate(Fraction),
  BadSampleRate(u32),
}

impl From<bifrost::Error> for Error {
  fn from(value: bifrost::Error) -> Self {
    Error::ShmError(value)
  }
}


