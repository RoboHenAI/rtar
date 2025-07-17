pub mod helper;
pub mod ustar;
pub mod gnu;
pub mod pax;
pub mod v7;
mod traits;

pub use traits::{UsedBlocksTrait, IsTypeTrait};
pub use ustar::{UstarHeader, UstarTypeFlag};
pub use gnu::{GnuHeader, GnuTypeFlag};
pub use pax::{Attribute as PaxAttribute, PaxHeader, PaxTypeFlag};
pub use v7::{V7Header, V7TypeFlag};

use anyhow::Result;
use std::io::{Read, Write};

/// Represents any supported TAR header.
pub enum TarHeader {
    Ustar(UstarHeader),
    Gnu(GnuHeader),
    Pax(PaxHeader),
    V7(V7Header),
    Unknown([u8; 512], usize),
}

impl TarHeader {
    /// Loads a TAR header from the reader.
    ///
    /// # Arguments
    /// * `reader` - Byte reader.
    ///
    /// # Returns
    /// * `Ok(Self)` - The loaded header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load(reader: &mut impl Read) -> Result<Self> {
        let mut buf = [0u8; 512];
        let readed = reader.read(&mut buf)?;
        if readed != 512 {
            // Return read bytes as Unknown
            return Ok(TarHeader::Unknown(buf, readed));
        }

        // load header from buffer based on its magic and version
        if let Some(header) = GnuHeader::load(&buf, reader)? {
            return Ok(TarHeader::Gnu(header));
        }
        if let Some(header) = PaxHeader::load(&buf, reader)? {
            return Ok(TarHeader::Pax(header));
        }
        if let Some(header) = UstarHeader::load(&buf)? {
            return Ok(TarHeader::Ustar(header));
        }
        if let Some(header) = V7Header::load(&buf)? {
            return Ok(TarHeader::V7(header));
        }

        // Return read bytes as Unknown
        Ok(TarHeader::Unknown(buf, 512))
    }

    /// Saves the TAR header to the writer.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    ///
    /// # Returns
    /// * `Ok(())` - On success.
    /// * `Err(e)` - If write fails.
    pub fn save(&mut self, writer: &mut impl Write) -> Result<()> {
        match self {
            TarHeader::Ustar(h) => h.save(writer),
            TarHeader::Gnu(h) => h.save(writer),
            TarHeader::Pax(h) => h.save(writer),
            TarHeader::V7(h) => h.save(writer),
            TarHeader::Unknown(bytes, size) => {
                if *size > 0 {
                    writer.write_all(&bytes[0..*size])?;
                }
                Ok(())
            },
        }
    }

    /// Returns the size of the content in bytes.
    pub fn get_content_size(&self) -> u64 {
        match self {
            TarHeader::Ustar(h) => h.size,
            TarHeader::Gnu(h) => h.size,
            TarHeader::Pax(h) => h.size,
            TarHeader::V7(h) => h.size,
            TarHeader::Unknown(_, _) => 0,
        }
    }
}

impl UsedBlocksTrait for TarHeader {
    fn get_used_blocks(&mut self) -> usize {
        match self {
            Self::Ustar(h) => h.get_used_blocks(),
            Self::Gnu(h) => h.get_used_blocks(),
            Self::Pax(h) => h.get_used_blocks(),
            Self::V7(h) => h.get_used_blocks(),
            Self::Unknown(_, _) => 0,
        }
    }

    fn get_saved_blocks(&self) -> usize {
        match self {
            Self::Ustar(h) => h.get_saved_blocks(),
            Self::Gnu(h) => h.get_saved_blocks(),
            Self::Pax(h) => h.get_saved_blocks(),
            Self::V7(h) => h.get_saved_blocks(),
            Self::Unknown(_, _) => 0,
        }
    }

    fn calc_used_blocks(&self) -> usize {
        match self {
            Self::Ustar(h) => h.calc_used_blocks(),
            Self::Gnu(h) => h.calc_used_blocks(),
            Self::Pax(h) => h.calc_used_blocks(),
            Self::V7(h) => h.calc_used_blocks(),
            Self::Unknown(_, _) => 0,
        }
    }
}

impl IsTypeTrait for TarHeader {
    fn is_regular_file(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_regular_file(),
            Self::Gnu(h) => h.typeflag.is_regular_file(),
            Self::Pax(h) => h.typeflag.is_regular_file(),
            Self::V7(h) => h.typeflag.is_regular_file(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_hard_link(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_hard_link(),
            Self::Gnu(h) => h.typeflag.is_hard_link(),
            Self::Pax(h) => h.typeflag.is_hard_link(),
            Self::V7(h) => h.typeflag.is_hard_link(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_symbolic_link(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_symbolic_link(),
            Self::Gnu(h) => h.typeflag.is_symbolic_link(),
            Self::Pax(h) => h.typeflag.is_symbolic_link(),
            Self::V7(h) => h.typeflag.is_symbolic_link(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_character_special(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_character_special(),
            Self::Gnu(h) => h.typeflag.is_character_special(),
            Self::Pax(h) => h.typeflag.is_character_special(),
            Self::V7(h) => h.typeflag.is_character_special(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_block_special(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_block_special(),
            Self::Gnu(h) => h.typeflag.is_block_special(),
            Self::Pax(h) => h.typeflag.is_block_special(),
            Self::V7(h) => h.typeflag.is_block_special(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_directory(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_directory(),
            Self::Gnu(h) => h.typeflag.is_directory(),
            Self::Pax(h) => h.typeflag.is_directory(),
            Self::V7(h) => h.typeflag.is_directory(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_fifo(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_fifo(),
            Self::Gnu(h) => h.typeflag.is_fifo(),
            Self::Pax(h) => h.typeflag.is_fifo(),
            Self::V7(h) => h.typeflag.is_fifo(),
            Self::Unknown(_, _) => false,
        }
    }

    fn is_contiguous_file(&self) -> bool {
        match self {
            Self::Ustar(h) => h.typeflag.is_contiguous_file(),
            Self::Gnu(h) => h.typeflag.is_contiguous_file(),
            Self::Pax(h) => h.typeflag.is_contiguous_file(),
            Self::V7(h) => h.typeflag.is_contiguous_file(),
            Self::Unknown(_, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn make_header_buf(magic: &[u8], version: &[u8], typeflag: u8) -> [u8; 512] {
        let mut buf = [0u8; 512];
        buf[257..257+magic.len()].copy_from_slice(magic);
        buf[263..263+version.len()].copy_from_slice(version);
        buf[156] = typeflag;
        buf
    }

    #[test]
    fn detects_ustar() {
        let buf = make_header_buf(b"ustar\0", b"00", b'0');
        let mut stream = Cursor::new(buf);
        match TarHeader::load(&mut stream) {
            Ok(h) => match h {
                TarHeader::Ustar(_) => {},
                _ => assert!(false, "Did not detect USTAR header"),
            },
            Err(e) => assert!(false, "Failed to load header: {}", e),
        }
    }

    #[test]
    fn detects_gnu() {
        let buf = make_header_buf(b"ustar ", b" \0", b'0');
        let mut stream = Cursor::new(buf);
        match TarHeader::load(&mut stream) {
            Ok(h) => match h {
                TarHeader::Gnu(_) => {},
                _ => assert!(false, "Did not detect GNU header"),
            },
            Err(e) => assert!(false, "Failed to load header: {}", e),
        }
    }

    #[test]
    fn detects_pax() {
        let buf = make_header_buf(b"ustar\0", b"00", b'x');
        let mut stream = Cursor::new(buf);
        match TarHeader::load(&mut stream) {
            Ok(h) => match h {
                TarHeader::Pax(_) => {},
                _ => assert!(false, "Did not detect PAX header"),
            },
            Err(e) => assert!(false, "Failed to load header: {}", e),
        };
    }

    #[test]
    fn detects_v7() {
        // No magic
        let mut buf = [0u8; 512];
        buf[156] = b'0';
        let mut stream = Cursor::new(buf);
        match TarHeader::load(&mut stream) {
            Ok(h) => match h {
                TarHeader::V7(_) => {},
                _ => assert!(false, "Did not detect V7 header"),
            },
            Err(e) => assert!(false, "Failed to load header: {}", e),
        };
    }

    #[test]
    fn detects_unknown() {
        use std::io::Cursor;
        // Corrupted or non-TAR header (bad magic, bad structure)
        let mut buf = [0xFFu8; 512];
        // Place something invalid in magic and typeflag
        buf[257..263].copy_from_slice(b"bogus!");
        buf[156] = 0xFF;
        let mut stream = Cursor::new(buf);
        match TarHeader::load(&mut stream) {
            Ok(h) => match h {
                TarHeader::Unknown(raw, size) => {
                    assert_eq!(&raw[257..263], b"bogus!");
                    assert_eq!(raw[156], 0xFF);
                    assert_eq!(size, 512);
                },
                _ => assert!(false, "Did not detect Unknown header"),
            },
            Err(e) => assert!(false, "Failed to load header: {}", e),
        }
    }

    #[test]
    fn round_trip_unknown() {
        use std::io::{Cursor, Seek, SeekFrom};
        let mut bytes = [0xABu8; 512];
        bytes[257..263].copy_from_slice(b"custom"); // 6 bytes
        bytes[156] = 0x42;
        let mut header = TarHeader::Unknown(bytes, 512);
        let mut buf = Cursor::new(vec![0u8; 512]);
        header.save(&mut buf).unwrap();
        buf.seek(SeekFrom::Start(0)).unwrap();
        let loaded = TarHeader::load(&mut buf).unwrap();
        match loaded {
            TarHeader::Unknown(raw, size) => {
                assert_eq!(&raw[257..263], b"custom");
                assert_eq!(raw[156], 0x42);
                assert_eq!(size, 512);
                assert_eq!(raw, bytes);
            },
            _ => panic!("Did not round-trip Unknown header"),
        }
    }
}