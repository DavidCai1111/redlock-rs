use redis;
use time;

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
    OutOfRangeError (err: time::OutOfRangeError) {
      from(err: time::OutOfRangeError) -> (err)
    }
    TimeoutError {
      description("Redlock request timeout")
    }
    UnableToLock {
      description("Unable to lock the resource")
    }
  }
}
