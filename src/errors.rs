use std::time;
use redis;

pub type RedlockResult<T> = Result<T, RedlockError>;

quick_error!{
  #[derive(Debug)]
  pub enum RedlockError {
    RedisError(err: redis::RedisError) { from(err: redis::RedisError) -> (err) }
    TimeError(err: time::SystemTimeError) { from(err: time::SystemTimeError) -> (err) }
    NoServerError { description("Redlock must be initialized with at least one redis server") }
    TimeoutError { description("Redlock request timeout") }
    DelayJitterError { description("Retry jitter must be smaller than retry delay") }
    LockExpired { description("The lock has already expired") }
    UnableToLock { description("Unable to lock the resource") }
    UnableToUnlock { description("Unable to unlock the resource") }
    UnableToExtend { description("Unable to extend the resource") }
  }
}
