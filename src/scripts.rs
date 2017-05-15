use redis;

const LOCK_SCRIPT: &'static str = r#""
  return redis.call("set", KEYS[1], ARGV[1], "NX", "PX", ARGV[2])
""#;

const UNLOCK_SCRIPT: &'static str = r#""
  if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
  else
    return 0
  end
""#;

lazy_static! {
  pub static ref LOCK: redis::Script = redis::Script::new(LOCK_SCRIPT);
  pub static ref UNLOCK: redis::Script = redis::Script::new(UNLOCK_SCRIPT);
}
