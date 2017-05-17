use redis;

lazy_static! {
  pub static ref LOCK: redis::Script = redis::Script::new("
    return redis.call(\"set\", KEYS[1], ARGV[1], \"NX\", \"PX\", ARGV[2])
  ");

  pub static ref UNLOCK: redis::Script = redis::Script::new("
    if redis.call(\"get\", KEYS[1]) == ARGV[1] then
      return redis.call(\"del\", KEYS[1])
    else
      return 0
    end
  ");

  pub static ref EXTEND: redis::Script = redis::Script::new("
    if redis.call(\"get\", KEYS[1]) == ARGV[1] then
      return redis.call(\"pexpire\", KEYS[1], ARGV[2])
    else
      return 0
    end
  ");
}
