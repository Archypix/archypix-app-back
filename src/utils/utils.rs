use rand::rngs::OsRng;
use rand::RngCore;

/// Generates a random hex token of the given length (one byte = two characters)
pub fn random_token(bytes: usize) -> Vec<u8> {
    let mut auth_token = vec![0u8; bytes];
    OsRng.fill_bytes(&mut auth_token);
    auth_token
}

/// Generates a random integer code of the given number of digits
pub fn random_code(digits: u32) -> u32 {
    OsRng.next_u32() % 10u32.pow(digits)
}

/// Pads a string with a character on the left to reach the target length
pub fn left_pad(string: &str, char: char, target_length: usize) -> String {
    let mut res = String::new();
    for _ in 0..target_length - string.len() {
        res.push(char);
    }
    res.push_str(string);
    res
}

/// Gets the frontend host from the environment variable `FRONTEND_HOST`
pub fn get_frontend_host() -> String {
    std::env::var("FRONTEND_HOST").expect("FRONTEND_HOST must be set")
}
/// Gets the backend host from the environment variables `BACKEND_HOST`
pub fn get_backend_host() -> String {
    std::env::var("BACKEND_HOST").expect("BACKEND_HOST must be set")
}
