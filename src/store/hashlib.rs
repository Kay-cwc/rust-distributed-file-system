use crypto::{md5, sha1, digest::Digest};

const CAS_BLOCK_SIZE: usize = 5;

pub fn cas_path_transform(s: String) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.input_str(&s);
    let hash_str = hasher.result_str();

    let slice_len = hash_str.len() / CAS_BLOCK_SIZE;
    let mut path = vec![String::new(); slice_len];

    for i in 0..slice_len {
        let start = i * CAS_BLOCK_SIZE;
        let end = start + CAS_BLOCK_SIZE;
        path[i] = hash_str[start..end].to_string();
    }

    path.join("/")
}

pub fn filename_transform(s: String) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.input_str(&s);
    
    hasher.result_str()
}

pub fn get_file_hash(buf: &[u8]) -> String {
    println!("buf: {:?}", buf);
    let mut hasher = md5::Md5::new();
    hasher.input(buf);

    hasher.result_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cas_path_transform() {
        let key = String::from("test");
        let actual_pathname = cas_path_transform(key);
        let expected_pathname = "a94a8/fe5cc/b19ba/61c4c/0873d/391e9/87982/fbbd3".to_string();
        assert_eq!(actual_pathname, expected_pathname);
    }

    #[test]
    fn test_get_file_hash() {
        let buf = vec![1, 2, 3, 4];
        let actual_hash = get_file_hash(&buf);
        println!("actual_hash: {}", actual_hash);
        let expected_hash = "08d6c05a21512a79a1dfeb9d2a8f262f".to_string();
        assert_eq!(actual_hash, expected_hash);
    }
}