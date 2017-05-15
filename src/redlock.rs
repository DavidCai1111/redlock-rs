use std::time;
use std::default::Default;
use redis;
use scripts;
use futures_cpupool::CpuPool;
use errors::{RedlockResult, RedlockError};
use util;

lazy_static! {
  static ref LOCK: redis::Script = redis::Script::new(scripts::LOCK_SCRIPT);
  static ref UNLOCK: redis::Script = redis::Script::new(scripts::UNLOCK_SCRIPT);
}

pub struct Redlock {
    clients: Vec<redis::Client>,
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
        let mut clients = Vec::with_capacity(config.addrs.len());
        for addr in config.addrs {
            clients.push(redis::Client::open(addr)?)
        }

        Ok(Redlock {
               clients,
               retry_count: config.retry_count,
               retry_delay: config.retry_delay,
           })
    }

    pub fn lock(&self, resource_name: &str, ttl: time::Duration) -> RedlockResult<()> {
        let clients_clen = self.clients.len();
        let mut waitings = clients_clen;
        let mut votes = 0;
        let quorum = (clients_clen as f64 / 2_f64).floor() as usize + 1;

        let pool = CpuPool::new(clients_clen);

        for i in 0..clients_clen {
            let conn = &self.clients[i].get_connection()?;

            match LOCK.arg(resource_name)
                      .arg(util::get_random_string(32))
                      .invoke::<()>(conn) {
                Ok(_) => {
                    votes += 1;
                    waitings -= 1;
                    if waitings > 1 {
                        continue;
                    }

                    if votes > quorum {
                        return Ok(());
                    }
                }
                Err(_) => unimplemented!(),
            }
        }

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
        assert_eq!(redlock.clients.len(), 1);
        assert_eq!(redlock.retry_count, 10);
        assert_eq!(redlock.retry_delay, time::Duration::from_millis(400));
    }
}
