use anyhow::{bail, Result as AnyResult};
use std::string::FromUtf8Error;

// Helper to extract and trim null-terminated strings
pub(crate) fn get_str(buf: &[u8]) -> Result<String, FromUtf8Error> {
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..nul].to_vec())
}

// Helper to extract and trim null-terminated strings
pub(crate) fn get_str_with_min_size(buf: &[u8], min_size: usize) -> Result<String, FromUtf8Error> {
    let nul = buf.iter().enumerate().position(|(i, &b)| !(i < min_size) && b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..nul].to_vec())
}

// Helper to parse octal strings
pub(crate) fn parse_octal<T: std::str::FromStr>(buf: &[u8]) -> AnyResult<T>
where
    T: num_traits::Num + std::fmt::Debug,
{
    let binding = String::from_utf8(buf.to_vec())?;
    let s = binding.trim_matches(|c| c == char::from(0) || c == ' ').trim();
    if s.is_empty() {
        return Ok(T::zero());
    }
    match T::from_str_radix(s, 8) {
        Ok(v) => Ok(v),
        Err(_) => bail!("Invalid octal: {}", s)
    }
}

// Helper to write a string (null-terminated or space-padded)
pub(crate) fn put_str(dst: &mut [u8], value: &str) {
    let bytes = value.as_bytes();
    let len = bytes.len().min(dst.len());
    dst[..len].copy_from_slice(&bytes[..len]);
    if len < dst.len() {
        // Null-terminate if possible
        dst[len..].fill(0);
    }
}
// Helper to write octal numbers as space-padded strings
pub(crate) fn put_octal<T: itoa::Integer + std::fmt::Octal>(dst: &mut [u8], value: T) {
    let s = format!("{:0width$o}", value, width = dst.len() - 1); // leave space for null
    let bytes = s.as_bytes();
    let len = bytes.len().min(dst.len() - 1);
    dst[..len].copy_from_slice(&bytes[..len]);
    dst[len] = b'\0';
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_str_basic() {
        let data = b"hello\0world";
        match get_str(data) {
            Ok(v) => assert_eq!(v, "hello"),
            Err(e) => panic!("Failed to get string: {}", e),
        }
    }
    #[test]
    fn test_get_str_no_null() {
        let data = b"abcde";
        match get_str(data) {
            Ok(v) => assert_eq!(v, "abcde"),
            Err(e) => panic!("Failed to get string: {}", e),
        }
    }
    #[test]
    fn test_get_str_trailing_spaces() {
        let data = b"foo   \0";
        match get_str(data) {
            Ok(v) => assert_eq!(v, "foo"),
            Err(e) => panic!("Failed to get string: {}", e),
        }
    }

    #[test]
    fn test_parse_octal_u32() {
        let data = b"0000644\0";
        let val: u32 = parse_octal(data).unwrap();
        assert_eq!(val, 0o644);
    }
    #[test]
    fn test_parse_octal_u64() {
        let data = b"00001234\0";
        let val: u64 = parse_octal(data).unwrap();
        assert_eq!(val, 0o1234);
    }
    #[test]
    fn test_parse_octal_empty() {
        let data = b"\0";
        let val: u32 = parse_octal(data).unwrap();
        assert_eq!(val, 0);
    }
    #[test]
    fn test_parse_octal_invalid() {
        let data = b"notnum\0";
        let val: Result<u32, _> = parse_octal(data);
        assert!(val.is_err());
    }

    #[test]
    fn test_put_str_basic() {
        let mut buf = [0u8; 8];
        put_str(&mut buf, "abc");
        assert_eq!(&buf[..4], b"abc\0");
    }
    #[test]
    fn test_put_str_truncate() {
        let mut buf = [0u8; 4];
        put_str(&mut buf, "abcdef");
        assert_eq!(&buf, b"abcd");
    }
    #[test]
    fn test_put_str_exact_fit() {
        let mut buf = [0u8; 3];
        put_str(&mut buf, "xyz");
        assert_eq!(&buf, b"xyz");
    }

    #[test]
    fn test_put_octal_u32() {
        let mut buf = [0u8; 8];
        put_octal(&mut buf, 0o644u32);
        // Should be 0000644\0
        assert_eq!(&buf[..7], b"0000644");
        assert_eq!(buf[7], 0);
    }
    #[test]
    fn test_put_octal_u64() {
        let mut buf = [0u8; 12];
        put_octal(&mut buf, 0o1234u64);
        assert_eq!(&buf[..11], b"00000001234");
        assert_eq!(buf[11], 0);
    }
}