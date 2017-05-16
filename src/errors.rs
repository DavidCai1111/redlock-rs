use std::time;
use redis;

pub type RedlockResult<T> = Result<T, RedlockError>;

quick_error!{
  #[derive(Debug)]
  pub enum RedlockError {
    RedisError(err: redis::RedisError) {
      from(err: redis::RedisError) -> (err)
    }
    NoServerError {
      description("Redlock must be initialized with at least one redis server")
    }
    TimeoutError {
      description("Redlock request timeout")
    }
    UnableToLock {
      description("Unable to lock the resource")
    }
    TimeError(err: time::SystemTimeError) {
      from(err: time::SystemTimeError) -> (err)
    }
  }
}
