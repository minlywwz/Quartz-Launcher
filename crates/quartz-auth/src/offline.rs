use md5::{Digest, Md5};
use uuid::Uuid;

pub fn offline_uuid(username: &str) -> Uuid {
    let input = format!("OfflinePlayer:{username}");
    let digest = Md5::digest(input.as_bytes());

    let mut bytes = digest.to_vec();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    Uuid::from_bytes(bytes.try_into().expect("MD5 digest is 16 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_deterministic() {
        let a = offline_uuid("Notch");
        let b = offline_uuid("Notch");
        assert_eq!(a, b);
    }
}
