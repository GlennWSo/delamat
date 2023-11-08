use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        SaltString,
    },
    Argon2, PasswordHash, PasswordHasher,
};

const SALT_LENGTH: usize = 16;
pub fn create_hash(word: &str) -> String {
    let salt = {
        let mut salt_bytes = [0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt_bytes);
        SaltString::encode_b64(&salt_bytes).expect("salt string invariant violated")
    };
    let argon2 = Argon2::default();
    argon2
        .hash_password(word.as_bytes(), &salt)
        .unwrap() // TODO remove unwrap
        .to_string()
}
