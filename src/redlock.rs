use std::time;
use std::default::Default;
use redis;
use rand;
use scripts;
use errors::{RedlockResult, RedlockError};

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

pub struct Config<T: redis::IntoConnectionInfo> {
    pub addrs: Vec<T>,
    pub retry_count: u32,
    pub retry_delay: time::Duration,
}

impl Default for Config<&'static str> {
    fn default() -> Self {
        Config {
            addrs: vec!["redis://127.0.0.1/"],
            retry_count: 10,
            retry_delay: time::Duration::from_millis(400),
        }
    }
}

impl Redlock {
    pub fn new<T: redis::IntoConnectionInfo>(config: Config<T>) -> RedlockResult<Redlock> {
        if config.addrs.is_empty() {
            return Err(RedlockError::NoServerError);
        }
        let mut servers = Vec::with_capacity(config.addrs.len());
        for addr in config.addrs {
            servers.push(redis::Client::open(addr)?)
        }

        Ok(Redlock {
               servers: servers,
               retry_count: config.retry_count,
               retry_delay: config.retry_delay,
           })
    }

    pub fn lock(&self, resource_name: &str, ttl: time::Duration) -> RedlockResult<()> {
        let mut waitings = self.servers.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let default_config = Config::default();
        assert_eq!(default_config.addrs, vec!["redis://127.0.0.1/"]);
        assert_eq!(default_config.retry_count, 10);
        assert_eq!(default_config.retry_delay, time::Duration::from_millis(400));
    }

    #[test]
    #[should_panic]
    fn test_new_with_no_server() {
        let redlock = Redlock::new::<&str>(Config {
                                               addrs: vec![],
                                               retry_count: 10,
                                               retry_delay: time::Duration::from_millis(400),
                                           })
                .unwrap();
    }

    #[test]
    fn test_new() {
        let redlock = Redlock::new(Config::default()).unwrap();
        assert_eq!(1, redlock.servers.len());
        assert_eq!(redlock.retry_count, 10);
        assert_eq!(redlock.retry_delay, time::Duration::from_millis(400));
    }
}
