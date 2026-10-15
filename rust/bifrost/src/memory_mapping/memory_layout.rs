use std::mem;

use bytemuck::Pod;

#[derive(Copy, Clone, Debug)]
pub struct MemoryLayout {
  pub alignment: usize,
  pub size: usize,
}

impl MemoryLayout {
  pub fn new() -> MemoryLayout {
    MemoryLayout {
      alignment: 1,
      size: 0,
    }
  }

  pub fn from_pod<T: Pod>() -> MemoryLayout {
    MemoryLayout {
      alignment: mem::align_of::<T>(),
      size: mem::size_of::<T>(),
    }
  }

  pub fn from_pod_slice<T: Pod>(count: usize) -> MemoryLayout {
    MemoryLayout {
      alignment: mem::align_of::<T>(),
      size: mem::size_of::<T>() * count,
    }
  }

  pub fn with<T: Pod>(self) -> MemoryLayout {
    self.with_memory_layout(MemoryLayout::from_pod::<T>())
  }

  pub fn with_slice<T: Pod>(self, count: usize) -> MemoryLayout {
    self.with_memory_layout(MemoryLayout::from_pod_slice::<T>(count))
  }

  pub fn with_memory_layout(mut self, memory_layout: MemoryLayout) -> MemoryLayout {
    self.alignment = lcm(self.alignment, memory_layout.alignment);
    self.size += calculate_padding(self.size, memory_layout.alignment);
    self.size += memory_layout.size;
    self
  }

  pub fn calculate_padding(&self, offset: usize) -> usize {
    calculate_padding(offset, self.alignment)
  }

  pub fn as_slice(self, count: usize) -> MemoryLayout {
    MemoryLayout {
      alignment: self.alignment,
      size: self.size * count,
    }
  }

  pub fn of<T>() -> MemoryLayout {
    MemoryLayout {
      alignment: mem::align_of::<T>(),
      size: mem::size_of::<T>(),
    }
  }
}

fn calculate_padding(offset: usize, alignment: usize) -> usize {
  (alignment - (offset % alignment)) % alignment
}

fn lcm(a: usize, b: usize) -> usize {
    a / gcd(a, b) * b
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

