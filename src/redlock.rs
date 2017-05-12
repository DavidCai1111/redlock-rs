use std::time;
use redis;
use scripts;
use errors::RedlockResult;

lazy_static! {
  static ref LOCK: redis::Script = redis::Script::new(scripts::LOCK_SCRIPT);
  static ref UNLOCK: redis::Script = redis::Script::new(scripts::UNLOCK_SCRIPT);
}

#[derive(Debug)]
pub struct Redlock {
    servers: Vec<redis::Client>,
    retry_count: u32,
    retry_delay: time::Duration,
}

impl Redlock {
    pub fn new<T: redis::IntoConnectionInfo>(addrs: Vec<T>) -> RedlockResult<Redlock> {
        if addrs.is_empty() {} // TODO: throw empty addrs error
        let mut servers = Vec::with_capacity(addrs.len());
        for addr in addrs {
            servers.push(redis::Client::open(addr)?)
        }

        Ok(Redlock {
               servers: servers,
               retry_count: 10,
               retry_delay: time::Duration::from_millis(400),
           })
    }
}
