# redlock-rs
[![Build Status](https://travis-ci.org/DavidCai1993/redlock-rs.svg?branch=master)](https://travis-ci.org/DavidCai1993/redlock-rs)

A Rust [Redlock](https://redis.io/topics/distlock) implementation for distributed, highly-available redis locks.

## Installation

```toml
[dependencies]
rust_redlock = "0.4.0"
```

## Documentation

See: https://docs.rs/rust_redlock/0.4.0/rust_redlock

## Usage

```rust
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
let lock = redlock.lock("resource_name",
                        time::Duration::from_millis(1000))?;
// Release the lock of the resource when you are done.
lock.unlock()?;
```
