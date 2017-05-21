use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};
use std::default::Default;
use std::thread;
use redis;
use rand::{thread_rng, Rng};
use scripts::{LOCK, UNLOCK, EXTEND};
use errors::{RedlockResult, RedlockError};
use util;

#[derive(Debug)]
enum RequestInfo<'a> {
    Lock,
    Extend { resource_value: &'a str },
}

// Lock represents a acquired lock for specified resource.
#[derive(Debug)]
pub struct Lock<'a> {
    redlock: &'a Redlock,
    resource_name: String,
    value: String,
    expiration: SystemTime,
}

impl<'a> Lock<'a> {
    // Release the acquired lock.
    pub fn unlock(&self) -> RedlockResult<()> {
        self.redlock.unlock(&self.resource_name, &self.value)
    }

    // Extend the TTL of acquired lock.
    pub fn extend(&self, ttl: Duration) -> RedlockResult<Lock> {
        if self.expiration < SystemTime::now() {
            return Err(RedlockError::LockExpired);
        }

        Ok(self.redlock.extend(&self.resource_name, &self.value, ttl)?)
    }
}

// Configuration of Redlock
pub struct Config<T>
    where T: redis::IntoConnectionInfo
{
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

#[derive(Debug)]
pub struct Redlock {
    clients: Vec<redis::Client>,
    retry_count: u32,
    retry_delay: Duration,
    retry_jitter: u32,
    drift_factor: f32,
    quorum: usize,
}

impl Redlock {
    // Create a new redlock instance.
    pub fn new<T: redis::IntoConnectionInfo>(config: Config<T>) -> RedlockResult<Redlock> {
        if config.addrs.is_empty() {
            return Err(RedlockError::NoServerError);
        }
        let mut clients = Vec::with_capacity(config.addrs.len());
        for addr in config.addrs {
            clients.push(redis::Client::open(addr)?)
        }

        let quorum = (clients.len() as f64 / 2_f64).floor() as usize + 1;

        Ok(Redlock {
               clients: clients,
               retry_count: config.retry_count,
               retry_delay: config.retry_delay,
               retry_jitter: config.retry_jitter,
               drift_factor: config.drift_factor,
               quorum: quorum,
           })
    }

    // Locks the given resource using the Redlock algorithm.
    pub fn lock(&self, resource_name: &str, ttl: Duration) -> RedlockResult<Lock> {
        self.request(RequestInfo::Lock, resource_name, ttl)
    }

    fn extend(&self, resource_name: &str, value: &str, ttl: Duration) -> RedlockResult<Lock> {
        self.request(RequestInfo::Extend { resource_value: value },
                     resource_name,
                     ttl)
    }

    fn request(&self,
               info: RequestInfo,
               resource_name: &str,
               ttl: Duration)
               -> RedlockResult<(Lock)> {
        let mut attempts = 0;
        let drift = Duration::from_millis((self.drift_factor as f64 *
                                           util::num_milliseconds(ttl) as f64)
                                                  .round() as
                                          u64 + 2);

        'attempts: while attempts < self.retry_count {
            attempts += 1;

            // Start time of this attempt
            let start = SystemTime::now();

            let mut waitings = self.clients.len();
            let mut votes = 0;
            let mut errors = 0;

            let value: String = match info {
                RequestInfo::Lock => util::get_random_string(32),
                RequestInfo::Extend { resource_value } => String::from(resource_value),
            };

            for client in &self.clients {
                let request_result = match info {
                    RequestInfo::Lock => lock(client, resource_name, &value, ttl),
                    RequestInfo::Extend { .. } => extend(client, resource_name, &value, ttl),
                };

                let lock = Lock {
                    redlock: self,
                    resource_name: String::from(resource_name),
                    value: value.clone(),
                    expiration: start + ttl - drift,
                };

                match request_result {
                    Ok(_) => {
                        waitings -= 1;
                        votes += 1;

                        if waitings > 0 {
                            continue;
                        }
                        // suceess: aquire the lock
                        if votes >= self.quorum && lock.expiration > SystemTime::now() {
                            return Ok(lock);
                        }

                        // fail: releases all aquired locks and retry
                        lock.unlock().is_ok(); // Just ingore the result
                        thread::sleep(self.get_retry_timeout());
                        continue 'attempts;
                    }
                    Err(_) => {
                        errors += 1;
                        // This attempt is doomed to fail, will retry after
                        // the timeout
                        if errors > self.quorum {
                            lock.unlock().is_ok(); // Just ingore the result
                            thread::sleep(self.get_retry_timeout());
                            continue 'attempts;
                        }
                    }
                }
            }
        }

        // Exceed the retry count, return the error
        match info {
            RequestInfo::Lock => Err(RedlockError::UnableToLock),
            RequestInfo::Extend { .. } => Err(RedlockError::UnableToExtend),
        }
    }

    fn unlock(&self, resource_name: &str, value: &str) -> RedlockResult<()> {
        let mut attempts = 0;

        'attempts: while attempts < self.retry_count {
            attempts += 1;

            let mut waitings = self.clients.len();
            let mut votes = 0;
            let mut errors = 0;

            for client in &self.clients {
                match unlock(client, resource_name, value) {
                    Ok(_) => {
                        waitings -= 1;
                        votes += 1;

                        if waitings > 0 {
                            continue;
                        }
                        if votes >= self.quorum {
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        errors += 1;
                        // This attempt is doomed to fail, will retry after
                        // the timeout
                        if errors >= self.quorum {
                            thread::sleep(self.get_retry_timeout());
                            continue 'attempts;
                        }
                    }
                }
            }
        }

        // Exceed the retry count, return the error
        Err(RedlockError::UnableToUnlock)
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
    LOCK.key(String::from(resource_name))
        .arg(String::from(value))
        .arg(util::num_milliseconds(ttl))
        .invoke::<()>(&client.get_connection()?)?;

    Ok(())
}

fn unlock(client: &redis::Client, resource_name: &str, value: &str) -> RedlockResult<()> {
    UNLOCK
        .key(resource_name)
        .arg(value)
        .invoke::<()>(&client.get_connection()?)?;

    Ok(())
}

fn extend(client: &redis::Client,
          resource_name: &str,
          value: &str,
          ttl: Duration)
          -> RedlockResult<()> {
    EXTEND
        .key(resource_name)
        .arg(value)
        .arg(util::num_milliseconds(ttl))
        .invoke::<()>(&client.get_connection()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::Commands;

    fn get_local_redis_conn() -> redis::Connection {
        redis::Client::open("redis://127.0.0.1")
            .unwrap()
            .get_connection()
            .unwrap()
    }

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

    #[test]
    fn test_lock() {
        let redlock = Redlock::new(Config::default()).unwrap();
        let resource_name = "test_lock";
        let lock = redlock
            .lock(resource_name, Duration::from_millis(2000))
            .unwrap();
        assert!(lock.expiration < SystemTime::now().add(Duration::from_millis(2000)));
    }

    #[test]
    fn test_lock_five() {
        let redlock = Redlock::new::<&str>(Config {
                                               addrs: vec!["redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1"],
                                               retry_count: 10,
                                               retry_delay: Duration::from_millis(400),
                                               retry_jitter: 400,
                                               drift_factor: 0.01,
                                           })
                .unwrap();
        let resource_name = "test_lock_five";
        let lock = redlock
            .lock(resource_name, Duration::from_millis(2000))
            .unwrap();
        assert!(lock.expiration < SystemTime::now().add(Duration::from_millis(2000)));
    }

    #[test]
    fn test_unlock() {
        let redlock = Redlock::new::<&str>(Config {
                                               addrs: vec!["redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1"],
                                               retry_count: 10,
                                               retry_delay: Duration::from_millis(400),
                                               retry_jitter: 400,
                                               drift_factor: 0.01,
                                           })
                .unwrap();
        let resource_name = "test_unlock";
        let lock = redlock
            .lock(resource_name, Duration::from_millis(2000))
            .unwrap();

        let conn = get_local_redis_conn();
        let value: String = conn.get(resource_name).unwrap();
        assert_eq!(value.len(), 32);

        lock.unlock().unwrap();
        let res: Option<String> = conn.get(resource_name).unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn test_extend() {
        let redlock = Redlock::new::<&str>(Config {
                                               addrs: vec!["redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1",
                                                           "redis://127.0.0.1"],
                                               retry_count: 10,
                                               retry_delay: Duration::from_millis(400),
                                               retry_jitter: 400,
                                               drift_factor: 0.01,
                                           })
                .unwrap();
        let resource_name = "test_extend";
        let lock = redlock
            .lock(resource_name, Duration::from_millis(2000))
            .unwrap();
        let lock_extended = lock.extend(Duration::from_millis(2000)).unwrap();

        assert!(lock_extended.expiration < SystemTime::now().add(Duration::from_millis(2000)));
    }
}
