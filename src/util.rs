use std::time::Duration;
use rand::{self, Rng};

pub fn get_random_string(len: usize) -> String {
    rand::thread_rng()
        .gen_ascii_chars()
        .take(len)
        .collect::<String>()
}

pub fn num_milliseconds(duration: Duration) -> u64 {
    let secs_part = duration.as_secs() * 1000;
    let nano_part = duration.subsec_nanos() / 1000_000;

    secs_part + nano_part as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_string() {
        assert_eq!(get_random_string(32).len(), 32);
    }

    #[test]
    fn test_num_milliseconds() {
        let duration = Duration::from_millis(5010);
        assert_eq!(num_milliseconds(duration), 5010);
    }
}
