use std::{
  marker::PhantomData,
  time::Duration,
};

use super::{
  Error,
  EventTrait,
  MemoryLayout,
  MemoryMappable,
  PlatformTrait,
  SlicerMut,
};

/**
 *  An event used for synchronization between processes sharing memory. It has
 *  two states: signaled and cleared. If the event is signaled wait will
 *  return immediately. If not it will wait until the event transitions into
 *  the signaled state OR timeout is reached. Signaling wakes every waiter,
 *  not just one.
 *
 *  When using this with data the idea is to put the events outside of the
 *  mutex that contains the data which is shared. For instance:
 *
 *  ```text
 *  MessageBus {
 *    message_available: Event<Platform>,
 *    message_queue: Mutex<Platform, MessageQueue>,
 *  }
 *  ```
 *
 *  The usage for reading messages would be:
 *
 *  ```text
 *  Consumer:
 *    loop {
 *      {
 *        let mut lock = message_bus.message_queue.lock(TIMEOUT).unwrap();
 *        if let Some(message) = lock.as_mut().pop() {
 *          if lock.as_mut().is_empty() {
 *            message_bus.message_available.clear().unwrap();
 *          }
 *          return message;
 *        }
 *      }
 *      message_bus.message_available.wait(TIMEOUT).unwrap();
 *    }
 *
 *  Producer:
 *    {
 *      let mut lock = message_bus.message_queue.lock(TIMEOUT).unwrap();
 *      lock.as_mut().send(message).unwrap();
 *      message_bus.message_available.signal().unwrap();
 *    }
 *  ```
 *
 *  Three rules make this work. None of them can be checked by the compiler:
 *
 *  1. `signal` and `clear` are always called while the data lock is held.
 *     This is what keeps signals from getting lost. Because the state is
 *     sticky, a producer that signals in the window between a consumer
 *     releasing the lock and calling `wait` is still seen by that `wait`.
 *  2. `clear` is only called after observing, under that same lock, that
 *     there is nothing left to consume. Clearing while data remains strands
 *     it until the next signal.
 *  3. `wait` is called WITHOUT the data lock held. Waiting inside the
 *     critical section stops any producer from taking the lock to signal,
 *     so the wait can only end in a timeout.
 *
 *  If a process dies while holding the event's internal lock the event is
 *  permanently poisoned and every operation on it fails from then on. The
 *  only way to recover is to recreate the shared memory.
 */
pub struct Event<P: PlatformTrait> {
  _marker: PhantomData<P>,
}

impl<P: PlatformTrait> Event<P> {
  pub fn new() -> Event<P> {
    Event { _marker: Default::default() }
  }
}

/**
 *  Mapped handle to an [Event]. See [Event] for the locking rules that
 *  `signal`, `clear` and `wait` require of the caller.
 */
pub struct EventMut<'e, P: PlatformTrait> {
  event: P::Event<'e>,
}

impl<'e, P: PlatformTrait> EventMut<'e, P> {
  pub fn signal(&mut self) -> Result<(), Error> {
    self.event.signal()
  }

  pub fn clear(&mut self) -> Result<(), Error> {
    self.event.clear()
  }

  pub fn wait(
    &mut self,
    timeout: Duration,
    interrupt: Option<&dyn Fn() -> bool>,
  ) -> Result<(), Error> {
    self.event.wait(timeout, interrupt)
  }
}

impl<P: PlatformTrait> MemoryMappable for Event<P> {
  type Ref<'e> = EventMut<'e, P>;
  type Mut<'e> = EventMut<'e, P>;

  fn memory_layout(&self) -> MemoryLayout {
    <P::Event<'static> as EventTrait<'static>>::memory_layout()
  }

  fn initialize<'t>(&self, slicer: SlicerMut<'t>) -> Result<SlicerMut<'t>, Error> {
    let slicer = <P::Event<'static> as EventTrait<'static>>::initialize(slicer)?;
    Ok(slicer)
  }

  fn map_mut<'t>(slicer: SlicerMut<'t>) -> Result<(SlicerMut<'t>, Self::Mut<'t>), Error> {
    let (slicer, event) = <P::Event<'t> as EventTrait<'t>>::from_existing_mut(slicer)?;

    let event = EventMut { event };

    Ok((slicer, event))
  }

  fn to_immutable<'t>(mutable: Self::Mut<'t>) -> Self::Ref<'t> { mutable }

  fn reborrow_mut<'b, 'm>(mutable: &'b mut Self::Mut<'m>) -> Self::Mut<'b> {
    EventMut {
      event: P::reborrow_event_mut(&mut mutable.event)
    }
  }
}





