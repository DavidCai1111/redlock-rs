use redis;

pub type RedlockResult<T> = Result<T, RedlockError>;

quick_error!{
  #[derive(Debug)]
  pub enum RedlockError {
    RedisError(err: redis::RedisError) {
      // display("[redlock] Redis error: {}", err)
      from(err: redis::RedisError) -> (err)
    }
  }
}
