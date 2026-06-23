/// Constant-time comparison so a wrong password's response time leaks no
/// information about how many characters were correct.
pub fn verify_password(expected: &str, provided: &str) -> bool {
    let a = expected.as_bytes();
    let b = provided.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_correct_password() {
        assert!(verify_password("hunter2", "hunter2"));
    }

    #[test]
    fn rejects_wrong_password() {
        assert!(!verify_password("hunter2", "hunter3"));
    }

    #[test]
    fn rejects_different_length() {
        assert!(!verify_password("hunter2", "hunter22"));
        assert!(!verify_password("hunter2", "hunter"));
    }
}
