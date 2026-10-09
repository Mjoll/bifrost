use super::{
  Fraction,
  Resolution,
};

#[derive(Debug)]
pub enum Error {
  ShmError(bifrost::Error),
  BadEnumValue,
  NotFound,
  AlreadyExists,
  NotAFrame,
  BadResolution(Resolution),
  BadFrameRate(Fraction),
  BadSampleRate(u32),
  NotAFrameQueue,
  NotAStream,
  Timeout,
  QueueClosed,
  StreamFormatChange,

  /**
   *  If the consumer hold a reference to a large number of frames and later
   *  return them all to the Stream by calling release_frame this error may
   *  be returned meaning that the pool is full. This can be fixed by
   *  increasing pool size.
   */
  ReturnQueueFull,
}

impl From<bifrost::Error> for Error {
  fn from(value: bifrost::Error) -> Self {
    match value {
      bifrost::Error::NotFound => Error::NotFound,
      bifrost::Error::AlreadyExists => Error::AlreadyExists,
      bifrost::Error::Timeout => Error::Timeout,
      bifrost::Error::QueueClosed => Error::QueueClosed,
      other => Error::ShmError(other),
    }
  }
}


