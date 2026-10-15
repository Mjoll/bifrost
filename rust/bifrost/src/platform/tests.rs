use std::{
  time::Duration,
  slice,
  thread,
};

use super::{
  Event,
  Mutex,
  Platform,
  Error,
};

use crate::{
  derive::*,
  MemoryMappable,
  MappableVec,
  SlicerMut,
};

#[test]
pub fn test_locking() {

  #[derive(MemoryMappable)]
  struct Queue {
    slot_available: Event<Platform>,
    value_available: Event<Platform>,
    inner: Mutex<Platform, QueueMutable>,
  }

  const TIMEOUT : Duration = Duration::from_secs(5);

  impl<'t> QueueRef<'t> {
    fn is_closed(&mut self) -> bool {
      let lock = self.inner.lock(TIMEOUT).unwrap();
      lock.is_closed()
    }

    fn pop(&mut self) -> Option<u32> {
      loop {
        {
          let mut lock = self.inner.lock_mut(TIMEOUT).unwrap();
          let was_full = lock.full();
          if let Some(value) = lock.pop() {
            if was_full {
              self.slot_available.signal().unwrap();
            }
            return Some(value);
          }
          if lock.is_closed() {
            return None;
          }
          self.value_available.clear().unwrap();
        }
        self.value_available.wait(TIMEOUT, None).unwrap();
      }
    }

    fn push(&mut self, value: u32) {
      loop {
        {
          let mut lock = self.inner.lock_mut(TIMEOUT).unwrap();
          let was_empty = lock.empty();
          if lock.push(value) {
            if was_empty {
              self.value_available.signal().unwrap();
            }
            break;
          }
          self.slot_available.clear().unwrap();
        }
        self.slot_available.wait(TIMEOUT, None).unwrap();
      }
    }

    fn close(&mut self) {
      let mut lock = self.inner.lock_mut(TIMEOUT).unwrap();
      lock.close();
      // Signal consumers to wake up and see the is_closed value
      self.value_available.signal().unwrap();
    }
  }

  #[derive(MemoryMappable)]
  struct QueueMutable {
    closed: u32,
    read_head: u32,
    write_head: u32,
    queue: MappableVec<u32>,
  }

  impl<'t> QueueMutableRef<'t> {
    fn is_closed(&self) -> bool {
      *self.closed == 1
    }
  }

  impl<'t> QueueMutableMut<'t> {
    fn is_closed(&self) -> bool {
      *self.closed == 1
    }

    fn close(&mut self) {
      *self.closed = 1;
    }

    fn pop(&mut self) -> Option<u32> {
      if self.empty() {
        None
      } else {
        // Increasing the read head out of order to ensure that failure to lock
        // will result in incorrect behavior
        let read_head = *self.read_head;
        *self.read_head += 1;
        let value = self.queue[read_head as usize % self.queue.len()];
        Some(value)
      }
    }

    fn full(&self) -> bool {
      let count = *self.write_head - *self.read_head;
      count as usize >= self.queue.len()
    }

    fn empty(&self) -> bool {
      self.write_head == self.read_head
    }

    fn push(&mut self, value: u32) -> bool {
      if self.full() {
        false
      } else {
        // Increasing the read head out of order to ensure that failure to lock
        // will result in incorrect behavior
        let write_head = *self.write_head;
        *self.write_head += 1;
        // In order to increase chance of bad reads happening we add a short
        // sleep between updating the value and incrementing the write head
        thread::sleep(std::time::Duration::from_millis(1));
        self.queue[write_head as usize % self.queue.len()] = value;
        true
      }
    }
  }

  let queue = Queue {
    slot_available: Event::new(),
    value_available: Event::new(),
    inner: Mutex::new(QueueMutable {
      closed: 0,
      read_head: 0,
      write_head: 0,
      queue: vec![0u32; 5].into(),
    })
  };

  let memory_layout = queue.memory_layout();
  let mut data = vec![0u8; memory_layout.size + memory_layout.alignment];
  let offset = data.as_ptr().addr();
  let pad = memory_layout.calculate_padding(offset);
  let aligned_data = &mut data[pad..];
  {
    let slicer = SlicerMut::new(aligned_data);
    queue.initialize(slicer).unwrap();
  }

  let raw_bytes = RawBytes::from_slice(aligned_data);

  fn consumer_thread_function(raw_bytes: RawBytes) -> Vec<u32> {
    let queue = raw_bytes.map_mut::<Queue>();
    // Since Shm only returns a Ref we need to simulate that so that we don't
    // accidentally rely on direct mutable access
    let mut queue : QueueRef<'_> = Queue::to_immutable(queue);

    let mut result = Vec::new();
    loop {
      match queue.pop() {
        Some(value) => {
          result.push(value)
        },
        None => {
          break;
        }
      }
    }
    result
  }

  let consumer1 = thread::spawn(move || {
    consumer_thread_function(raw_bytes)
  });

  let consumer2 = thread::spawn(move || {
    consumer_thread_function(raw_bytes)
  });

  let slicer_mut = SlicerMut::new(aligned_data);
  let (_slicer, queue) = Queue::map_mut(slicer_mut).unwrap();
  let mut queue = Queue::to_immutable(queue);

  let element_count = 100;
  for i in 0..element_count {
    println!("Pushing {}", i);
    queue.push(i);
  }
  queue.close();

  assert!(queue.is_closed());

  let result1 = consumer1.join().unwrap();
  let result2 = consumer2.join().unwrap();

  let mut all_result = Vec::new();
  all_result.extend_from_slice(&result1);
  all_result.extend_from_slice(&result2);
  all_result.sort();

  println!("Result1: {:?}", result1);
  println!("Result2: {:?}", result2);
  println!("Result: {:?}", all_result);

  assert_eq!(all_result.len(), element_count as usize);

  for (i, value) in all_result.into_iter().enumerate() {
    assert_eq!(i as u32, value);
  }
}

#[test]
fn test_mutex_timeout() {
  #[derive(MemoryMappable)]
  struct Foo {
    inner: Mutex<Platform, u32>,
  }


  let foo = Foo { inner: Mutex::new(0) };

  let memory_layout = foo.memory_layout();
  let mut data = vec![0u8; memory_layout.size + memory_layout.alignment];
  let offset = data.as_ptr().addr();
  let pad = memory_layout.calculate_padding(offset);
  let aligned_data = &mut data[pad..];

  let raw_bytes = RawBytes::from_slice(aligned_data);

  fn thread_function(raw_bytes: RawBytes) -> Result<(), Error> {
    let foo_mut = raw_bytes.map_mut::<Foo>();
    let mut foo = Foo::to_immutable(foo_mut);

    foo.inner.lock_mut(Duration::from_secs(1))?;
    Ok(())
  }

  let foo_mut = raw_bytes.map_mut::<Foo>();
  let mut foo = Foo::to_immutable(foo_mut);

  let lock = foo.inner.lock_mut(Duration::from_secs(1)).unwrap();

  let thread1 = thread::spawn(move || {
    thread_function(raw_bytes)
  });

  thread::sleep(Duration::from_secs(2));

  let thread_result = thread1.join().unwrap();

  drop(lock);

  match thread_result {
    Ok(()) => {
      panic!("Expected thread to fail");
    }
    Err(Error::Timeout) => { },
    Err(error) => {
      panic!("Expected thread to fail with timeout error. Instead it failed with: {:?}", error);
    }
  }
}

// This is a shared memory library which means that the mutex provides no
// mechanism for sharing data within one process. So in order to test the
// mutex within one process we need unsafe code to make multiple mutable
// references to the same data.
#[derive(Copy, Clone)]
struct RawBytes {
  ptr: *mut u8,
  size: usize,
}

unsafe impl Send for RawBytes { }

impl RawBytes {
  fn from_slice(slice: &mut [u8]) -> RawBytes {
    RawBytes {
      ptr: slice.as_mut_ptr(),
      size: slice.len(),
    }
  }

  fn map_mut<T: MemoryMappable>(&self) -> T::Mut<'_> {
    let slice_mut = unsafe {
      slice::from_raw_parts_mut(self.ptr, self.size)
    };
    let slicer = SlicerMut::new(slice_mut);
    let (_slicer, mapped) = T::map_mut(slicer).unwrap();
    mapped
  }
}
