use std::time;
use std::default::Default;
use time as lib_time;
use redis;
use scripts::{LOCK, UNLOCK};
use errors::{RedlockResult, RedlockError};
use util;

#[derive(Debug)]
pub struct Lock<'a> {
    redlock: &'a Redlock,
    resource_name: String,
    value: String,
    ttl: time::Duration,
}

impl<'a> Lock<'a> {
    pub fn unlock(&self) -> RedlockResult<()> {
        unimplemented!()
    }
}

#[derive(Debug)]
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
            addrs: vec!["redis://127.0.0.1"],
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

    pub fn lock(&self, resource_name: &str, ttl: time::Duration) -> RedlockResult<Lock> {
        let clients_len = self.clients.len();
        let ttl = lib_time::Duration::from_std(ttl)?;
        let quorum = (clients_len as f64 / 2_f64).floor() as usize + 1;

        let mut waitings = clients_len;
        let mut votes = 0;
        let start = lib_time::now();

        for (_, client) in self.clients.iter().enumerate() {
            let value: &str = &util::get_random_string(32);

            let mut lock_job = || -> RedlockResult<Option<Lock>> {
                LOCK.arg(resource_name)
                    .arg(value)
                    .arg(ttl.num_milliseconds())
                    .invoke::<()>(&client.get_connection()?)?;

                let time_elapsed = lib_time::now() - start;
                if time_elapsed > ttl {
                    return Err(RedlockError::TimeoutError);
                }

                votes += 1;
                if votes > quorum {
                    Ok(Some(Lock {
                                redlock: self,
                                resource_name: String::from(resource_name),
                                value: String::from(value),
                                ttl: (ttl - time_elapsed).to_std()?,
                            }))
                } else {
                    Ok(None)
                }
            };

            match lock_job() {
                Ok(lock_opt) => {
                    if let Some(lock) = lock_opt {
                        return Ok(lock);
                    } else {
                        continue;
                    }
                }
                Err(_) => {
                    waitings -= 1;
                    if waitings == 0 {
                        return Err(RedlockError::UnableToLock);
                    }
                    continue;
                }
            }
        }

        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let default_config = Config::default();
        assert_eq!(default_config.addrs, vec!["redis://127.0.0.1"]);
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
