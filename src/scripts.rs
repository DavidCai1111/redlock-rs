pub const LOCK_SCRIPT: &'static str = r#""
  return redis.call("set", KEYS[1], ARGV[1], "NX", "PX", ARGV[2])
""#;

pub const UNLOCK_SCRIPT: &'static str = r#""
  if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
  else
    return 0
  end
""#;
