use std::sync::{
  Arc,
  Mutex,
};
use super::{
  Error,
  Interrupt,
  Name,
  NameQueue,
  PlatformTrait,
};

pub struct Controller<P: PlatformTrait> {
  is_aborted: Arc<Mutex<bool>>,
  queue: NameQueue<P>
}

impl<P: PlatformTrait> Controller<P> {
  pub fn abort(&mut self) -> Result<(), Error> {
    {
      if let Ok(mut lock) = self.is_aborted.lock() {
        *lock = true;
      }
    }
    self.queue.interrupt()
  }
}

pub struct Token {
  is_aborted: Arc<Mutex<bool>>,
}

impl Interrupt for Token {
  fn is_aborted(&self) -> bool {
    match self.is_aborted.lock() {
      Ok(value) => *value,
      Err(_) => false,
    }
  }
}

pub fn make_controller<P: PlatformTrait>(
  name: Name
) -> Result<(Controller<P>, Token), Error> {
  let queue = NameQueue::open(name)?;
  let is_aborted = Arc::new(Mutex::new(false));

  let token = Token { is_aborted: is_aborted.clone() };
  let controller = Controller { is_aborted, queue };
  Ok((controller, token))
}


