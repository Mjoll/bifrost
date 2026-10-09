use std::{
  sync::mpsc::{
    self,
    Sender,
    Receiver,
  },
  thread::{
    self,
    JoinHandle,
  },
  time::Duration,
};

use super::{
  Error,
  Frame,
  FrameInfo,
  Stream,
};

pub struct AsyncStream {
  internal: Option<Internal>,
}

impl AsyncStream {
  pub fn new(stream: Stream) -> AsyncStream {
    let internal = Internal::new(stream);

    AsyncStream { internal: Some(internal) }
  }

  pub fn allocate<F>(
    &mut self,
    frame_info: FrameInfo,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<Frame, Error>) -> () + Send + 'static
  {
    self.internal.as_mut()
      .expect("Internal should exist until Drop")
      .allocate(frame_info, callback)
  }

  pub fn pop<F>(&mut self, timeout: Duration, callback: F) -> Result<(), Error>
    where F: FnOnce(Result<Option<Frame>, Error>) -> () + Send + 'static
  {
    self.internal.as_mut()
      .expect("Internal should exist until Drop")
      .pop(timeout, callback)
  }

  pub fn push<F>(
    &mut self,
    frame: Frame,
    timeout: Duration,
    callback: F
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    self.internal.as_mut()
      .expect("Internal should exist until Drop")
      .push(frame, timeout, callback)
  }

  pub fn wait_until_drained<F>(
    &mut self,
    timeout: Duration,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    self.internal.as_mut()
      .expect("Internal should exist until Drop")
      .wait_until_drained(timeout, callback)
  }

  pub fn release_frame<F>(
    &mut self,
    frame: Frame,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    self.internal.as_mut()
      .expect("Internal should exist until Drop")
      .release_frame(frame, callback)
  }
}

impl Drop for AsyncStream {
  fn drop(&mut self) {
    if let Some(internal) = self.internal.take() {
      drop(internal.sender);
      let _ = internal.join_handle.join();
    }
  }
}

struct Internal {
  sender: Sender<Request>,
  join_handle: JoinHandle<()>,
}

impl Internal {
  pub fn new(stream: Stream) -> Internal {
    let (sender, receiver) = mpsc::channel();

    let join_handle = thread::spawn(move || {
      producer_thread_fn(stream, receiver)
    });

    Internal { sender, join_handle }
  }

  pub fn allocate<F>(
    &mut self,
    frame_info: FrameInfo,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<Frame, Error>) -> () + Send + 'static
  {
    let request = Request::Allocate {
      frame_info,
      callback: Box::new(callback),
    };

    self.sender
      .send(request)
      .map_err(|_| Error::AsyncStreamThreadDead)
  }

  pub fn pop<F>(&mut self, timeout: Duration, callback: F) -> Result<(), Error>
    where F: FnOnce(Result<Option<Frame>, Error>) -> () + Send + 'static
  {
    let request = Request::Pop {
      timeout,
      callback: Box::new(callback),
    };
    self.sender
      .send(request)
      .map_err(|_| Error::AsyncStreamThreadDead)
  }

  pub fn push<F>(
    &mut self,
    frame: Frame,
    timeout: Duration,
    callback: F
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    let request = Request::Push {
      frame,
      timeout,
      callback: Box::new(callback),
    };
    self.sender
      .send(request)
      .map_err(|_| Error::AsyncStreamThreadDead)
  }

  pub fn wait_until_drained<F>(
    &mut self,
    timeout: Duration,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    let request = Request::WaitUntilDrained {
      timeout,
      callback: Box::new(callback),
    };

    self.sender
      .send(request)
      .map_err(|_| Error::AsyncStreamThreadDead)
  }

  pub fn release_frame<F>(
    &mut self,
    frame: Frame,
    callback: F,
  ) -> Result<(), Error>
    where F: FnOnce(Result<(), Error>) -> () + Send + 'static
  {
    let request = Request::ReleaseFrame {
      frame,
      callback: Box::new(callback),
    };

    self.sender
      .send(request)
      .map_err(|_| Error::AsyncStreamThreadDead)
  }
}

enum Request {
  Pop {
    timeout: Duration,
    callback: Box<dyn FnOnce(Result<Option<Frame>, Error>) -> () + Send>
  },
  Push {
    frame: Frame,
    timeout: Duration,
    callback: Box<dyn FnOnce(Result<(), Error>) -> () + Send>
  },
  WaitUntilDrained {
    timeout: Duration,
    callback: Box<dyn FnOnce(Result<(), Error>) -> () + Send>
  },
  Allocate {
    frame_info: FrameInfo,
    callback: Box<dyn FnOnce(Result<Frame, Error>) -> () + Send>,
  },
  ReleaseFrame {
    frame: Frame,
    callback: Box<dyn FnOnce(Result<(), Error>) -> () + Send>,
  },
}

fn producer_thread_fn(mut stream: Stream, receiver: Receiver<Request>) {
  while let Ok(request) = receiver.recv() {
    match request {
      Request::Pop { timeout, callback } => {
        callback(stream.pop(timeout));
      }
      Request::Push { frame, timeout, callback } => {
        callback(stream.push(&mut Some(frame), timeout));
      },
      Request::WaitUntilDrained { timeout, callback } => {
        callback(stream.wait_until_drained(timeout))
      },
      Request::Allocate { frame_info, callback } => {
        callback(stream.allocate(frame_info))
      },
      Request::ReleaseFrame { frame, callback } => {
        callback(stream.release_frame(frame))
      }
    }
  }
}


