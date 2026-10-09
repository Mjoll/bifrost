use bytemuck::{
  Pod,
  Zeroable,
};

use crate::derive::*;

use super::{
  MemoryMappable,
  MappableVec,
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

#[derive(MemoryMappable)]
struct Bar {
  one: u32,
  two: u64,
  foo: Foo,
}

#[derive(MemoryMappable)]
struct Foo {
  elements: MappableVec<u32>,
  one: u32,
  two: u64,
}

#[test]
fn test_memory_mapping_2() {
  let bar = Bar {
    one: 42,
    two: 69,
    foo: Foo {
      one: 3,
      two: 8,
      elements: vec![0u32; 40].into(),
    }
  };

  let memory_layout = Bar::memory_layout(&bar);
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
    let (_slicer, mapped_bar) = Bar::map_mut(slicer_mut).unwrap();

    assert_eq!(*mapped_bar.one, 42);
    assert_eq!(*mapped_bar.two, 69);

    {
      assert_eq!(*mapped_bar.foo.one, 3);
      assert_eq!(*mapped_bar.foo.two, 8);
      assert_eq!(mapped_bar.foo.elements.len(), 40);

      *mapped_bar.foo.one = 1;
      *mapped_bar.foo.two = 2;

      for (index, element) in &mut mapped_bar.foo.elements.iter_mut().enumerate() {
        *element = index as u32;
      }
    }
  }

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    let (_slicer, mapped_bar) = Bar::map_mut(slicer_mut).unwrap();
    {
      assert_eq!(*mapped_bar.foo.one, 1);
      assert_eq!(*mapped_bar.foo.two, 2);
      assert_eq!(mapped_bar.foo.elements.len(), 40);

      for (index, element) in &mut mapped_bar.foo.elements.iter().enumerate() {
        assert_eq!(*element, index as u32)
      }
    }
  }
}


#[derive(MemoryMappable)]
struct Wide {
  wide: MappableVec<u64>,
  trailer: u32,
}

#[test]
fn test_mappable_vec_mixed_alignment() {
  let wide = Wide {
    wide: vec![0u64; 7].into(),
    trailer: 0xBBBB,
  };

  let memory_layout = wide.memory_layout();
  let mut data = vec![0u8; memory_layout.size + memory_layout.alignment];
  let offset = data.as_mut_ptr().addr();
  let pad = memory_layout.calculate_padding(offset);
  let aligned_data = &mut data[pad..];

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    wide.initialize(slicer_mut).unwrap();
  }

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    let (_slicer, mapped_wide) = Wide::map_mut(slicer_mut).unwrap();

    assert_eq!(mapped_wide.wide.len(), 7);
    // A vector that consumed the wrong number of bytes shifts every following
    // field, so the trailer is what catches a padding mismatch.
    assert_eq!(*mapped_wide.trailer, 0xBBBB);

    for (index, element) in mapped_wide.wide.iter_mut().enumerate() {
      *element = ((index as u64) << 40) | 0xFEED;
    }
  }

  {
    let slicer_mut = SlicerMut::new(aligned_data);
    let (_slicer, mapped_wide) = Wide::map_mut(slicer_mut).unwrap();

    assert_eq!(*mapped_wide.trailer, 0xBBBB);

    for (index, element) in mapped_wide.wide.iter().enumerate() {
      assert_eq!(*element, ((index as u64) << 40) | 0xFEED);
    }
  }
}
