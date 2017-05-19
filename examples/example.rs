extern crate rust_redlock;

use std::time;
use rust_redlock::*;

fn example() -> RedlockResult<()> {
    let redlock = Redlock::new(Config {
                                   addrs: vec!["redis1.example.com",
                                               "redis2.example.com",
                                               "redis3.example.com"],
                                   retry_count: 10,
                                   retry_delay: time::Duration::from_millis(400),
                                   retry_jitter: 400,
                                   drift_factor: 0.01,
                               })?;

    // Acquire the lock of the specified resource.
    let lock = redlock
        .lock("resource_name_to_lock", time::Duration::from_millis(1000))?;
    // Unlock the resource when you are done.
    lock.unlock()?;
    Ok(())
}

fn main() {
    example().unwrap();
}
