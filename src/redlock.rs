use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};
use std::default::Default;
use std::thread;
use redis;
use rand::{thread_rng, Rng};
use scripts::{LOCK, UNLOCK};
use errors::{RedlockResult, RedlockError};
use util;

#[derive(Debug)]
pub struct Lock<'a> {
    redlock: &'a Redlock,
    resource_name: String,
    value: String,
    ttl: Duration,
}

impl<'a> Lock<'a> {
    pub fn unlock(&self) -> RedlockResult<()> {
        self.redlock.unlock(&self.resource_name, &self.value)
    }
}

#[derive(Debug)]
pub struct Redlock {
    clients: Vec<redis::Client>,
    retry_count: u32,
    retry_delay: Duration,
    retry_jitter: u32,
    drift_factor: f32,
}

pub struct Config<T: redis::IntoConnectionInfo> {
    pub addrs: Vec<T>,
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub retry_jitter: u32,
    pub drift_factor: f32,
}

impl Default for Config<&'static str> {
    fn default() -> Self {
        Config {
            addrs: vec!["redis://127.0.0.1"],
            retry_count: 10,
            retry_delay: Duration::from_millis(400),
            retry_jitter: 400,
            drift_factor: 0.01,
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
               retry_jitter: config.retry_jitter,
               drift_factor: config.drift_factor,
           })
    }

    pub fn unlock(&self, resource_name: &str, value: &str) -> RedlockResult<()> {
        let clients_len = self.clients.len();
        let quorum = (clients_len as f64 / 2_f64).floor() as usize + 1;

        let mut waitings = clients_len;
        let mut votes = 0;
        let mut attempts = 0;

        'attempts: while attempts < self.retry_count {
            attempts += 1;
            for (_, client) in self.clients.iter().enumerate() {
                match unlock(client, resource_name, value) {
                    Ok(_) => {
                        waitings -= 1;
                        if waitings > 0 {
                            continue;
                        }
                        votes += 1;
                        if votes >= quorum {
                            return Ok(());
                        }
                    }
                    Err(_) => continue 'attempts,
                }
            }
        }

        Err(RedlockError::UnableToUnlock)
    }

    pub fn lock(&self, resource_name: &str, ttl: Duration) -> RedlockResult<Lock> {
        let clients_len = self.clients.len();
        let quorum = (clients_len as f64 / 2_f64).floor() as usize + 1;

        let mut waitings = clients_len;
        let mut votes = 0;
        let mut attempts = 0;

        let start = SystemTime::now();

        'attempts: while attempts < self.retry_count {
            attempts += 1;
            for (_, client) in self.clients.iter().enumerate() {
                let value: &str = &util::get_random_string(32);
                match lock(client, resource_name, value, ttl) {
                    Ok(_) => {
                        waitings -= 1;
                        if waitings > 0 {
                            continue;
                        }

                        let time_elapsed = start.elapsed()?;
                        let lock = Lock {
                            redlock: self,
                            resource_name: String::from(resource_name),
                            value: String::from(value),
                            ttl: ttl - time_elapsed,
                        };

                        votes += 1;
                        if votes > quorum && time_elapsed < ttl {
                            return Ok(lock);
                        }

                        match lock.unlock() {
                            _ => {
                                thread::sleep(self.get_retry_timeout());
                                continue 'attempts;
                            }
                        };
                    }
                    Err(_) => {
                        thread::sleep(self.get_retry_timeout());
                        continue 'attempts;
                    }
                }
            }
        }


        Err(RedlockError::UnableToLock)
    }

    fn get_retry_timeout(&self) -> Duration {
        let jitter = self.retry_jitter as i32 * thread_rng().gen_range(-1, 1);
        if jitter >= 0 {
            self.retry_delay.add(Duration::from_millis(jitter as u64))
        } else {
            self.retry_delay.sub(Duration::from_millis(-jitter as u64))
        }
    }
}

fn lock(client: &redis::Client,
        resource_name: &str,
        value: &str,
        ttl: Duration)
        -> RedlockResult<()> {
    LOCK.arg(resource_name)
        .arg(value)
        .arg(util::num_milliseconds(ttl))
        .invoke::<()>(&client.get_connection()?)?;

    Ok(())
}

fn unlock(client: &redis::Client, resource_name: &str, value: &str) -> RedlockResult<()> {
    UNLOCK
        .arg(resource_name)
        .arg(value)
        .invoke::<()>(&client.get_connection()?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let default_config = Config::default();
        assert_eq!(default_config.addrs, vec!["redis://127.0.0.1"]);
        assert_eq!(default_config.retry_count, 10);
        assert_eq!(default_config.retry_delay, Duration::from_millis(400));
    }

    #[test]
    #[should_panic]
    fn test_new_with_no_server() {
        Redlock::new::<&str>(Config {
                                 addrs: vec![],
                                 retry_count: 10,
                                 retry_delay: Duration::from_millis(400),
                                 retry_jitter: 400,
                                 drift_factor: 0.01,
                             })
                .unwrap();
    }

    #[test]
    fn test_new() {
        let redlock = Redlock::new(Config::default()).unwrap();
        assert_eq!(redlock.clients.len(), 1);
        assert_eq!(redlock.retry_count, 10);
        assert_eq!(redlock.retry_delay, Duration::from_millis(400));
    }
}
