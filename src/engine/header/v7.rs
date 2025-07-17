use anyhow::Result;
use std::io::Write;

use super::helper::*;
use super::{UsedBlocksTrait, IsTypeTrait};

/// V7 header type flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum V7TypeFlag {
    RegularFile,
    HardLink,
    SymbolicLink,
    Directory,
    FIFO,
    Unknown(u8)
}

impl From<u8> for V7TypeFlag {
    fn from(value: u8) -> Self {
        match value {
            b'0' => Self::RegularFile,
            b'1' => Self::HardLink,
            b'2' => Self::SymbolicLink,
            b'5' => Self::Directory,
            b'6' => Self::FIFO,
            v => Self::Unknown(v),
        }
    }
}

impl From<V7TypeFlag> for u8 {
    fn from(value: V7TypeFlag) -> Self {
        match value {
            V7TypeFlag::RegularFile => b'0',
            V7TypeFlag::HardLink => b'1',
            V7TypeFlag::SymbolicLink => b'2',
            V7TypeFlag::Directory => b'5',
            V7TypeFlag::FIFO => b'6',
            V7TypeFlag::Unknown(v) => v,
        }
    }
}

impl IsTypeTrait for V7TypeFlag {
    fn is_regular_file(&self) -> bool {
        if let Self::RegularFile = self { return true }
        false
    }
    
    fn is_hard_link(&self) -> bool {
        if let Self::HardLink = self { return true }
        false
    }

    fn is_symbolic_link(&self) -> bool {
        if let Self::SymbolicLink = self { return true }
        false
    }

    fn is_character_special(&self) -> bool {
        false
    }

    fn is_block_special(&self) -> bool {
        false
    }

    fn is_directory(&self) -> bool {
        if let Self::Directory = self { return true }
        false
    }

    fn is_fifo(&self) -> bool {
        if let Self::FIFO = self { return true }
        false
    }

    fn is_contiguous_file(&self) -> bool {
        false
    }
}

/// Represents a V7 TAR header (original UNIX)
#[derive(Debug, Clone, PartialEq)]
pub struct V7Header {
    /// File name (null-terminated)
    pub name: String,
    /// File mode (octal string)
    pub mode: u32,
    /// Owner user ID (octal string)
    pub uid: u32,
    /// Owner group ID (octal string)
    pub gid: u32,
    /// File size in bytes (octal string)
    pub size: u64,
    /// Modification time (octal string)
    pub mtime: u64,
    /// Header checksum (octal string)
    pub chksum: u32,
    /// Type flag
    pub typeflag: V7TypeFlag,
    /// Name of linked file (null-terminated)
    pub linkname: String,
    /// The used blocks saved.
    saved_blocks: usize,
}


impl V7Header {
    /// Creates a new V7 header.
    /// 
    /// # Arguments
    /// * `typeflag` - The type flag of the header.
    /// 
    /// # Returns
    /// * `Self` - The created V7 header.
    pub fn new(typeflag: V7TypeFlag) -> Self {
        V7Header {
            name: String::default(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag,
            linkname: String::default(),
            saved_blocks: 0,
        }
    }

    /// Loads a V7 header from the buffer.
    ///
    /// # Arguments
    /// * `buf` - Byte buffer.
    ///
    /// # Returns
    /// * `Ok(Self)` - The loaded V7 header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load(buf: &[u8; 512]) -> Result<Option<Self>> {
        let typeflag = buf[156].into();
        if let V7TypeFlag::Unknown(_) = typeflag {
            return Ok(None);
        }
        let name = get_str(&buf[0..100])?;
        let mode = parse_octal::<u32>(&buf[100..108])?;
        let uid = parse_octal::<u32>(&buf[108..116])?;
        let gid = parse_octal::<u32>(&buf[116..124])?;
        let size = parse_octal::<u64>(&buf[124..136])?;
        let mtime = parse_octal::<u64>(&buf[136..148])?;
        let chksum = parse_octal::<u32>(&buf[148..156])?;
        let linkname = get_str(&buf[157..257])?;

        Ok(Some(V7Header {
            name,
            mode,
            uid,
            gid,
            size,
            mtime,
            chksum,
            typeflag,
            linkname,
            saved_blocks: 1,
        }))
    }

    /// Saves the V7 header to the writer.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    ///
    /// # Returns
    /// * `Ok(())` - On success.
    /// * `Err(e)` - If write fails.
    pub fn save(&mut self, writer: &mut impl Write) -> anyhow::Result<()> {
        let mut buf = [0u8; 512];
        put_str(&mut buf[0..100], &self.name);
        put_octal(&mut buf[100..108], self.mode);
        put_octal(&mut buf[108..116], self.uid);
        put_octal(&mut buf[116..124], self.gid);
        put_octal(&mut buf[124..136], self.size);
        put_octal(&mut buf[136..148], self.mtime);

        // chksum is written after calculating
        buf[156] = self.typeflag.into();
        put_str(&mut buf[157..257], &self.linkname);
        
        // Set checksum field to spaces before computing checksum (TAR spec)
        buf[148..156].fill(b' ');
        
        // Compute and write checksum
        let mut chksum: u32 = 0;
        for i in 0..512 { chksum = chksum.wrapping_add(buf[i] as u32); }
        let chksum_str = format!("{:06o}\0 ", chksum);
        let chksum_bytes = chksum_str.as_bytes();
        buf[148..148+chksum_bytes.len()].copy_from_slice(chksum_bytes);
        writer.write_all(&buf)?;
        self.chksum = chksum;

        self.saved_blocks = 1;
        Ok(())
    }

}

impl UsedBlocksTrait for V7Header {
    fn get_used_blocks(&mut self) -> usize {
        1
    }

    fn get_saved_blocks(&self) -> usize {
        self.saved_blocks
    }

    fn calc_used_blocks(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header() -> V7Header {
        V7Header {
            name: "test.txt".to_string(),
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 1234,
            mtime: 1_600_000_000,
            chksum: 0, // will be computed
            typeflag: V7TypeFlag::RegularFile,
            linkname: "".to_string(),
            saved_blocks: 0,
        }
    }

    #[test]
    fn round_trip_save_load() {
        let mut header = sample_header();
        let mut buf = [0u8; 512];
        match header.save(&mut (&mut buf as &mut [u8])) {
            Ok(_) => assert!(true),
            Err(e) => {
                assert!(false, "Failed to save header: {}", e);
                return;
            }
        }
        let loaded = match V7Header::load(&mut buf) {
            Ok(opt) => match opt {
                Some(h) => h,
                None => {
                    assert!(false, "Invalid typeflag");
                    return;
                },
            },
            Err(e) => {
                assert!(false, "Failed to load header: {}", e);
                return;
            },
        };
        assert_eq!(header.name, loaded.name);
        assert_eq!(header.mode, loaded.mode);
        assert_eq!(header.uid, loaded.uid);
        assert_eq!(header.gid, loaded.gid);
        assert_eq!(header.size, loaded.size);
        assert_eq!(header.mtime, loaded.mtime);
        assert_eq!(header.typeflag, loaded.typeflag);
        assert_eq!(header.linkname, loaded.linkname);
        // chksum is not round-tripped, ignore for comparison
        // TODO: compare checksum
    }

    #[test]
    fn minimal_header() {
        let mut header = V7Header {
            name: "".to_string(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: V7TypeFlag::Unknown(0),
            linkname: "".to_string(),
            saved_blocks: 0,
        };
        let mut buf = [0u8; 512];
        match header.save(&mut (&mut buf as &mut [u8])) {
            Ok(_) => assert!(true),
            Err(e) => {
                assert!(false, "Failed to save header: {}", e);
                return;
            }
        }
        let loaded = match V7Header::load(&mut buf) {
            Ok(opt) => match opt {
                Some(h) => h,
                None => {
                    assert!(false, "Invalid typeflag");
                    return;
                },
            },
            Err(e) => {
                assert!(false, "Failed to load header: {}", e);
                return;
            },
        };
        assert_eq!(header.name, loaded.name);
        assert_eq!(header.size, loaded.size);
    }
}