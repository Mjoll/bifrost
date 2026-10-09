mod error;
pub use error::Error;

mod memory_mapping;
pub use memory_mapping::MemoryMappable;

mod shm;
pub use shm::{
  delete_shm,
  Shm,
};
