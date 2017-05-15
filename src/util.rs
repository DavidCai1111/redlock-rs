use rand::{self, Rng};

pub fn get_random_string(len: usize) -> String {
    rand::thread_rng()
        .gen_ascii_chars()
        .take(len)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_string() {
        assert_eq!(get_random_string(32).len(), 32);
    }
}
