use bytemuck::{
  Pod,
  Zeroable,
};

use std::fmt::{
  self,
  Display,
  Formatter,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct FourCC {
  data: [u8; 4]
}

impl FourCC {
  pub fn empty() -> FourCC {
    FourCC { data: [0; 4] }
  }

  pub fn from_static(data: &'static [u8; 4]) -> FourCC {
    FourCC { data: *data }
  }
}

impl Display for FourCC {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match str::from_utf8(&self.data) {
      Ok(utf8) => write!(f, "{}", utf8),
      Err(_) => write!(f, "<non-utf8>"),
    }
  }
}


