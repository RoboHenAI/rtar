use std::io::Write;


/// Represents a USTAR TAR header.
use super::helper::*;
use super::{UsedBlocksTrait, IsTypeTrait};

/// USTAR header type flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UstarTypeFlag {
    RegularFile,
    HardLink,
    SymbolicLink,
    CharacterSpecial,
    BlockSpecial,
    Directory,
    FIFO,
    ContiguousFile,
    Unknown(u8)
}

impl From<u8> for UstarTypeFlag {
    fn from(value: u8) -> Self {
        match value {
            b'0' => UstarTypeFlag::RegularFile,
            b'1' => UstarTypeFlag::HardLink,
            b'2' => UstarTypeFlag::SymbolicLink,
            b'3' => UstarTypeFlag::CharacterSpecial,
            b'4' => UstarTypeFlag::BlockSpecial,
            b'5' => UstarTypeFlag::Directory,
            b'6' => UstarTypeFlag::FIFO,
            b'7' => UstarTypeFlag::ContiguousFile,
            v => UstarTypeFlag::Unknown(v),
        }
    }
}

impl From<UstarTypeFlag> for u8 {
    fn from(value: UstarTypeFlag) -> Self {
        match value {
            UstarTypeFlag::RegularFile => b'0',
            UstarTypeFlag::HardLink => b'1',
            UstarTypeFlag::SymbolicLink => b'2',
            UstarTypeFlag::CharacterSpecial => b'3',
            UstarTypeFlag::BlockSpecial => b'4',
            UstarTypeFlag::Directory => b'5',
            UstarTypeFlag::FIFO => b'6',
            UstarTypeFlag::ContiguousFile => b'7',
            UstarTypeFlag::Unknown(v) => v,
        }
    }
}

impl IsTypeTrait for UstarTypeFlag {
    fn is_regular_file(&self) -> bool {
        if let UstarTypeFlag::RegularFile = self { return true }
        false
    }
    
    fn is_hard_link(&self) -> bool {
        if let UstarTypeFlag::HardLink = self { return true }
        false
    }

    fn is_symbolic_link(&self) -> bool {
        if let UstarTypeFlag::SymbolicLink = self { return true }
        false
    }

    fn is_character_special(&self) -> bool {
        if let UstarTypeFlag::CharacterSpecial = self { return true }
        false
    }

    fn is_block_special(&self) -> bool {
        if let UstarTypeFlag::BlockSpecial = self { return true }
        false
    }

    fn is_directory(&self) -> bool {
        if let UstarTypeFlag::Directory = self { return true }
        false
    }

    fn is_fifo(&self) -> bool {
        if let UstarTypeFlag::FIFO = self { return true }
        false
    }

    fn is_contiguous_file(&self) -> bool {
        if let UstarTypeFlag::ContiguousFile = self { return true }
        false
    }
}

/// Represents a USTAR TAR header (POSIX)
#[derive(Debug, Clone, PartialEq)]
pub struct UstarHeader {
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
    pub typeflag: UstarTypeFlag,
    /// Name of linked file (null-terminated)
    pub linkname: String,
    /// USTAR indicator "ustar"
    pub magic: String,
    /// USTAR version "00"
    pub version: String,
    /// Owner user name (null-terminated)
    pub uname: String,
    /// Owner group name (null-terminated)
    pub gname: String,
    /// Device major number (octal string)
    pub devmajor: u32,
    /// Device minor number (octal string)
    pub devminor: u32,
    /// Filename prefix (null-terminated)
    pub prefix: String,
    /// The used blocks saved.
    saved_blocks: usize,
}

impl UstarHeader {
    pub fn new(typeflag: UstarTypeFlag) -> Self {
        UstarHeader {
            name: String::default(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: typeflag,
            linkname: String::default(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: String::default(),
            gname: String::default(),
            devmajor: 0,
            devminor: 0,
            prefix: String::default(),
            saved_blocks: 0,
        }
    }

    /// Loads a USTAR header from the buffer.
    ///
    /// # Arguments
    /// * `buf` - Byte buffer.
    ///
    /// # Returns
    /// * `Ok(Self)` - The loaded USTAR header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load(buf: &[u8; 512]) -> anyhow::Result<Option<Self>> {
        // validate headers
        if &buf[257..262] != b"ustar" || (buf[262] != b' ' && buf[262] != b'\0') || &buf[263..265] != b"00" {
            return Ok(None)
        }
        let typeflag = buf[156].into();
        if let UstarTypeFlag::Unknown(_) = typeflag {
            return Ok(None);
        }

        // read data
        let name = get_str(&buf[0..100])?;
        let mode = parse_octal::<u32>(&buf[100..108])?;
        let uid = parse_octal::<u32>(&buf[108..116])?;
        let gid = parse_octal::<u32>(&buf[116..124])?;
        let size = parse_octal::<u64>(&buf[124..136])?;
        let mtime = parse_octal::<u64>(&buf[136..148])?;
        let chksum = parse_octal::<u32>(&buf[148..156])?;
        let linkname = get_str(&buf[157..257])?;
        let magic = get_str_with_min_size(&buf[257..263], 6)?;
        let version = get_str_with_min_size(&buf[263..265], 2)?;
        let uname = get_str(&buf[265..297])?;
        let gname = get_str(&buf[297..329])?;
        let devmajor = parse_octal::<u32>(&buf[329..337])?;
        let devminor = parse_octal::<u32>(&buf[337..345])?;
        let prefix = get_str(&buf[345..500])?;

        // TODO: calculate and validate checksum

        Ok(Some(UstarHeader {
            name,
            mode,
            uid,
            gid,
            size,
            mtime,
            chksum,
            typeflag,
            linkname,
            magic,
            version,
            uname,
            gname,
            devmajor,
            devminor,
            prefix,
            saved_blocks: 1,
        }))
    }

    /// Saves the USTAR header to the writer.
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
        buf[156] = self.typeflag.into();
        put_str(&mut buf[157..257], &self.linkname);
        put_str(&mut buf[257..263], &self.magic);
        put_str(&mut buf[263..265], &self.version);
        put_str(&mut buf[265..297], &self.uname);
        put_str(&mut buf[297..329], &self.gname);
        put_octal(&mut buf[329..337], self.devmajor);
        put_octal(&mut buf[337..345], self.devminor);
        put_str(&mut buf[345..500], &self.prefix);

        // Set checksum field to spaces before computing checksum (TAR spec)
        for b in &mut buf[148..156] { *b = b' '; }

        // Compute and write checksum
        let mut chksum: u32 = 0;
        for i in 0..512 { chksum = chksum.wrapping_add(buf[i] as u32); }
        let chksum_str = format!("{:06o}\0 ", chksum);
        let chksum_bytes = chksum_str.as_bytes();
        buf[148..148+chksum_bytes.len()].copy_from_slice(chksum_bytes);
        writer.write_all(&buf)?;
        self.saved_blocks = 1;
        Ok(())
    }
}

impl UsedBlocksTrait for UstarHeader {
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

    fn sample_header() -> UstarHeader {
        UstarHeader {
            name: "testfile.txt".to_string(),
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 1234,
            mtime: 1_600_000_000,
            chksum: 0, // will be computed
            typeflag: UstarTypeFlag::RegularFile,
            linkname: "".to_string(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: "user".to_string(),
            gname: "group".to_string(),
            devmajor: 0,
            devminor: 0,
            prefix: "".to_string(),
            saved_blocks: 0,
        }
    }

    #[test]
    fn round_trip_save_load() {
        let mut header = sample_header();
        let mut buf = [0u8; 512];
        assert_eq!(0, header.saved_blocks);
        match header.save(&mut (&mut buf as &mut [u8])) {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "Failed to save header: {}", e)
        }
        let loaded = match UstarHeader::load(&buf) {
            Ok(opt) => match opt {
                Some(h) => h,
                None => {
                    assert!(false, "Invalida magic/version");
                    return;
                },
            },
            Err(e) => {
                assert!(false, "Failed to load header: {}", e);
                return;
            },
        };
        assert_eq!(header.saved_blocks, 1);
        assert_eq!(header.name, loaded.name);
        assert_eq!(header.mode, loaded.mode);
        assert_eq!(header.uid, loaded.uid);
        assert_eq!(header.gid, loaded.gid);
        assert_eq!(header.size, loaded.size);
        assert_eq!(header.mtime, loaded.mtime);
        assert_eq!(header.typeflag, loaded.typeflag);
        assert_eq!(header.linkname, loaded.linkname);
        assert_eq!(header.magic, loaded.magic);
        assert_eq!(header.version, loaded.version);
        assert_eq!(header.uname, loaded.uname);
        assert_eq!(header.gname, loaded.gname);
        assert_eq!(header.devmajor, loaded.devmajor);
        assert_eq!(header.devminor, loaded.devminor);
        assert_eq!(header.prefix, loaded.prefix);
        // chksum is not round-tripped, ignore for comparison
    }

    #[test]
    fn minimal_header() {
        let mut header = UstarHeader {
            name: "".to_string(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: UstarTypeFlag::Unknown(0),
            linkname: "".to_string(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: "".to_string(),
            gname: "".to_string(),
            devmajor: 0,
            devminor: 0,
            prefix: "".to_string(),
            saved_blocks: 0
        };
        let mut buf = [0u8; 512];
        match header.save(&mut (&mut buf as &mut [u8])) {
            Ok(_) => assert!(true),
            Err(e) => {
                assert!(false, "Failed to save header: {}", e);
                return;
            },
        }
        let loaded = match UstarHeader::load(&mut buf) {
            Ok(opt) => match opt {
                Some(h) => h,
                None => {
                    assert!(false, "Invalid magic/version");
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