use bytemuck::{
  Pod,
  Zeroable,
};

use crate::derive::*;

use super::{
  MemoryMappable,
  SlicerMut,
};

#[derive(Pod, Copy, Clone, Zeroable, Debug)]
#[repr(C)]
struct PodData {
  three: u32,
  // bytemuck Pod derive fails if this is not present. Ensuring that Pod
  // structs are mappable to pure memory
  _pad: u32,
  four: u64,
}

#[derive(MemoryMappable)]
struct NonPodData {
  one: u32,
  two: u64,
  pod: PodData,
}

#[test]
fn test_memory_mapping_1() {
  let bar = NonPodData {
    one: 42,
    two: 69,
    pod: PodData {
      three: 3,
      _pad: 0,
      four: 4,
    }
  };

  let memory_layout = NonPodData::memory_layout(&bar);
  let mut data = vec![0u8; memory_layout.size + memory_layout.alignment];
  let offset = data.as_mut_ptr().addr();
  let pad = memory_layout.calculate_padding(offset);
  let aligned_data = &mut data[pad..];

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    bar.initialize(slicer_mut).unwrap();
  }

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    let (_slicer, mapped_bar) = NonPodData::map_mut(slicer_mut).unwrap();

    assert_eq!(*mapped_bar.one, 42);
    assert_eq!(*mapped_bar.two, 69);
    assert_eq!(mapped_bar.pod.three, 3);
    assert_eq!(mapped_bar.pod.four, 4);
    *mapped_bar.one = 67;
    *mapped_bar.two = 420;
    mapped_bar.pod.four = 80085;
  }

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    let (_slicer, mapped_bar) = NonPodData::map_mut(slicer_mut).unwrap();
    assert_eq!(*mapped_bar.one, 67);
    assert_eq!(*mapped_bar.two, 420);
    assert_eq!(mapped_bar.pod.four, 80085);
  }
}

