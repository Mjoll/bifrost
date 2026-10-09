pub mod derive {
  pub use memory_mappable_derive::MemoryMappable;
}

mod error;
pub use error::Error;

mod memory_mapping;
pub use memory_mapping::{
  MemoryLayout,
  MemoryMappable,
  SlicerMut,
};

mod shm;
pub use shm::{
  delete_shm,
  Shm,
};
