use std::{
  time::Duration,
  marker::PhantomData,
  thread::{
    self,
    JoinHandle,
  },
  sync::{
    Arc,
    Mutex,
    mpsc::{
      self,
      Sender,
      Receiver,
      RecvTimeoutError,
    }
  },
};

use super::{
  Name,
  NameQueue,
  PlatformTrait,
};

pub struct InterruptCallback<P: PlatformTrait> {
  join_handle: Option<JoinHandle<()>>,
  abort_thread: Sender<()>,
  aborted: Arc<Mutex<bool>>,
  _marker: PhantomData<P>,
}


impl<P: PlatformTrait> InterruptCallback<P> {
  pub(super) fn new<F: FnMut() -> bool + Send + 'static>(
    name: Name,
    callback: F,
    check_interval: Duration,
  ) -> InterruptCallback<P> {
    let boxed_f = Box::new(callback);
    let (abort_thread, abort_recv) = mpsc::channel();
    let aborted = Arc::new(Mutex::new(false));
    let aborted2 = aborted.clone();
    let join_handle = thread::spawn(move || {
      interrupt_check_thread::<P>(name, boxed_f, abort_recv, aborted2, check_interval);
    });

    InterruptCallback {
      abort_thread,
      aborted,
      join_handle: Some(join_handle),
      _marker: Default::default(),
    }
  }

  pub fn is_aborted(&self) -> bool {
    match self.aborted.lock() {
      Ok(value) => *value,
      Err(_) => false,
    }
  }
}

impl<P: PlatformTrait> Drop for InterruptCallback<P> {
  fn drop(&mut self) {
    if let Some(join_handle) = self.join_handle.take() {
      if let Ok(()) = self.abort_thread.send(()) {
        let _ = join_handle.join();
      }
    }
  }
}

fn interrupt_check_thread<P: PlatformTrait>(
  name: Name,
  mut callback: Box<dyn FnMut() -> bool>,
  abort_thread: Receiver<()>,
  aborted: Arc<Mutex<bool>>,
  interval: Duration,
) -> bool {
  let Ok(mut name_queue) = NameQueue::<P>::open(name) else {
    return false;
  };

  loop {
    match abort_thread.recv_timeout(interval) {
      Err(RecvTimeoutError::Timeout) => { },
      Ok(()) |
      Err(RecvTimeoutError::Disconnected) => {
        return true;
      },
    }
    if callback() {
      if let Ok(mut aborted) = aborted.lock() {
        *aborted = true;
      }
      return name_queue.interrupt().is_ok();
    }
  }
}
