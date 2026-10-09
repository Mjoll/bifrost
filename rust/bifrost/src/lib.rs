pub mod derive {
  pub use memory_mappable_derive::MemoryMappable;
}

mod error;
pub use error::Error;

mod memory_mapping;
pub use memory_mapping::{
  MappableVec,
  MemoryLayout,
  MemoryMappable,
  SlicerMut,
};

mod four_cc;
pub use four_cc::FourCC;

mod shm;
pub use shm::{
  delete_shm,
  Shm,
};
