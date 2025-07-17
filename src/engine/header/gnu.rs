use anyhow::{bail, Result};
use std::io::{Read, Write};

use super::helper::*;
use super::{UsedBlocksTrait, UstarTypeFlag, IsTypeTrait};

/// PAX header type flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GnuTypeFlag {
    LongName,
    LongLinkName,
    DirectoryDump,
    MultiVolume,
    NextFile,
    Sparse,
    Ustar(UstarTypeFlag)
}

impl From<u8> for GnuTypeFlag {
    fn from(value: u8) -> Self {
        match value {
            b'L' => Self::LongName,
            b'K' => Self::LongLinkName,
            b'D' => Self::DirectoryDump,
            b'M' => Self::MultiVolume,
            b'N' => Self::NextFile,
            b'S' => Self::Sparse,
            v => Self::Ustar(UstarTypeFlag::from(v)),
        }
    }
}

impl From<GnuTypeFlag> for u8 {
    fn from(value: GnuTypeFlag) -> Self {
        match value {
            GnuTypeFlag::LongName => b'x',
            GnuTypeFlag::LongLinkName => b'g',
            GnuTypeFlag::DirectoryDump => b'D',
            GnuTypeFlag::MultiVolume => b'M',
            GnuTypeFlag::NextFile => b'N',
            GnuTypeFlag::Sparse => b'S',
            GnuTypeFlag::Ustar(v) => u8::from(v),
        }
    }
}

impl IsTypeTrait for GnuTypeFlag {
    fn is_regular_file(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_regular_file(),
            _ => false,
        }
    }
    
    fn is_hard_link(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_hard_link(),
            _ => false,
        }
    }

    fn is_symbolic_link(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_symbolic_link(),
            _ => false,
        }
    }

    fn is_character_special(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_character_special(),
            _ => false,
        }
    }

    fn is_block_special(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_block_special(),
            _ => false,
        }
    }

    fn is_directory(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_directory(),
            _ => false,
        }
    }

    fn is_fifo(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_fifo(),
            _ => false,
        }
    }

    fn is_contiguous_file(&self) -> bool {
        match self {
            Self::Ustar(v) => v.is_contiguous_file(),
            _ => false,
        }
    }
}

/// Represents a GNU sparse entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseEntry {
    /// Offset in the file (as bytes from start).
    pub offset: u64,
    /// Number of bytes in the sparse segment.
    pub numbytes: u64,
}

/// Represents a GNU TAR header, including GNU extensions.
#[derive(Debug, Clone, PartialEq)]
pub struct GnuHeader {
    /// File name (null-terminated).
    name: String,
    /// File mode (octal string).
    pub mode: u32,
    /// Owner user ID (octal string).
    pub uid: u32,
    /// Owner group ID (octal string).
    pub gid: u32,
    /// File size in bytes (octal string).
    pub size: u64,
    /// Modification time (octal string).
    pub mtime: u64,
    /// Header checksum (octal string).
    chksum: u32,
    /// Type flag.
    pub typeflag: GnuTypeFlag,
    /// Name of linked file (null-terminated).
    linkname: String,
    /// USTAR indicator "ustar" or GNU magic.
    pub magic: String,
    /// USTAR version "00".
    pub version: String,
    /// Owner user name (null-terminated).
    pub uname: String,
    /// Owner group name (null-terminated).
    pub gname: String,
    /// Device major number (octal string).
    pub devmajor: u32,
    /// Device minor number (octal string).
    pub devminor: u32,
    /// Array of up to 4 sparse entries in the header.
    sparse: Vec<SparseEntry>,
    /// True if there are extended sparse headers.
    pub isextended: bool,
    /// Actual file size (for sparse files).
    pub realsize: Option<u64>,
    /// Access time (seconds since epoch).
    pub atime: Option<u64>,
    /// Change time (seconds since epoch).
    pub ctime: Option<u64>,
    /// Optional incremental dump fields (not always used).
    pub incremental: Option<String>,
    /// Additional GNU fields not parsed.
    pub gnu_extra: [u8; 12],
    /// The used blocks so far not saved yet.
    used_blocks: usize,
    /// The used blocks saved.
    saved_blocks: usize,
    /// Should calculate used blocks.
    updated_used_blocks: bool
}

impl GnuHeader {
    /// Returns the name of the file.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Sets the name of the file.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The name of the file.
    pub fn set_name(&mut self, name: String) {
        self.updated_used_blocks = false;
        self.name = name;
    }

    /// Returns the link name of the file.
    pub fn get_linkname(&self) -> &str {
        &self.linkname
    }

    /// Sets the link name of the file.
    /// 
    /// # Arguments
    /// 
    /// * `linkname` - The link name of the file.
    pub fn set_linkname(&mut self, linkname: String) {
        self.updated_used_blocks = false;
        self.linkname = linkname;
    }

    /// Pushes a sparse entry to the header.
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The sparse entry to push.
    pub fn push_sparse(&mut self, entry: SparseEntry) {
        self.updated_used_blocks = false;
        self.sparse.push(entry);
    }

    /// Pops a sparse entry from the header.
    /// 
    /// # Returns
    /// 
    /// * `Option<SparseEntry>` - The sparse entry that was popped.
    pub fn pop_sparse(&mut self) -> Option<SparseEntry> {
        self.updated_used_blocks = false;
        self.sparse.pop()
    }

    /// Inserts a sparse entry at the specified index.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index at which to insert the sparse entry.
    /// * `entry` - The sparse entry to insert.
    pub fn insert_sparse(&mut self, index: usize, entry: SparseEntry) {
        self.updated_used_blocks = false;
        self.sparse.insert(index, entry);
    }

    /// Removes a sparse entry from the header.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index of the sparse entry to remove.
    pub fn remove_sparse(&mut self, index: usize) {
        self.updated_used_blocks = false;
        self.sparse.remove(index);
    }

    /// Clears all sparse entries from the header.
    pub fn clear_sparse(&mut self) {
        self.updated_used_blocks = false;
        self.sparse.clear();
    }

    /// Returns an iterator over the sparse entries.
    pub fn iter_sparse(&self) -> std::slice::Iter<'_, SparseEntry> {
        self.sparse.iter()
    }

    /// Returns a mutable iterator over the sparse entries.
    pub fn iter_sparse_mut(&mut self) -> std::slice::IterMut<'_, SparseEntry> {
        self.sparse.iter_mut()
    }

    /// Creates a new GNU header.
    pub fn new(typeflag: GnuTypeFlag) -> Self {
        Self {
            name: String::new(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag,
            linkname: String::new(),
            magic: "ustar ".to_string(),
            version: " \0".to_string(),
            uname: String::new(),
            gname: String::new(),
            devmajor: 0,
            devminor: 0,
            sparse: Vec::new(),
            isextended: false,
            realsize: None,
            atime: None,
            ctime: None,
            incremental: None,
            gnu_extra: [0u8; 12],
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false
        }
    }

    /// Reads a GNU long header.
    ///
    /// # Arguments
    ///
    /// * `buf` - 512-byte buffer containing the GNU header.
    /// * `reader` - Reader positioned at the start of a header block. Supports reading long name/link records.
    pub fn read_long_header(buf: &[u8; 512], reader: &mut impl Read) -> Result<String> {
        // Validate checksum
        let chksum = parse_octal::<u32>(&buf[148..156])?;
        let mut chksum_bytes = buf.clone();
        let mut new_chksum: u32 = 0;
        chksum_bytes[148..156].fill(b' ');
        for i in 0..512 { new_chksum = new_chksum.wrapping_add(chksum_bytes[i] as u32); }
        if chksum != new_chksum {
            bail!("Invalid long name checksum: expected {}, got {}", chksum, new_chksum);
        }
        
        // Read long linkname
        let mut size = parse_octal::<u64>(&buf[124..136])?;
        let mut data = Vec::with_capacity(size as usize);
        let mut block: [u8; 512];
        while size > 0 {
            block = [0u8; 512];
            reader.read_exact(&mut block)?;
            let n = std::cmp::min(size, 512);
            data.extend_from_slice(&block[..n as usize]);
            size -= n;
        }
        Ok(std::str::from_utf8(&data)?.trim_end_matches('\0').to_string())
    }

    /// Loads a GNU long name records.
    ///
    /// # Arguments
    ///
    /// * `buf` - 512-byte buffer containing the GNU header.
    /// * `reader` - Reader positioned at the start of a header block. Supports reading long name/link records.
    pub fn load_long_name(&mut self, buf: &[u8; 512], reader: &mut impl Read) -> Result<()> {
        self.name = Self::read_long_header(buf, reader)?;
        Ok(())
    }

    /// Loads a GNU long linkname header.
    ///
    /// # Arguments
    ///
    /// * `buf` - 512-byte buffer containing the GNU header.
    /// * `reader` - Reader positioned at the start of a header block. Supports reading long name/link records.
    pub fn load_long_link(&mut self, buf: &[u8; 512], reader: &mut impl Read) -> Result<()> {
        self.linkname = Self::read_long_header(buf, reader)?;
        Ok(())
    }

    /// Loads a standard GNU header from the buffer, including GNU extensions.
    ///
    /// # Arguments
    /// * `buf` - 512-byte buffer containing the GNU header.
    ///
    /// # Returns
    /// * `Ok(Self)` - The loaded GNU header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load_standard(&mut self, buf: &[u8; 512], reader: &mut impl Read, skip_name: bool, skip_linkname: bool) -> Result<()> {
        if !skip_name {
            self.name = get_str(&buf[0..100])?;
        }
        self.mode = parse_octal::<u32>(&buf[100..108])?;
        self.uid = parse_octal::<u32>(&buf[108..116])?;
        self.gid = parse_octal::<u32>(&buf[116..124])?;
        self.size = parse_octal::<u64>(&buf[124..136])?;
        self.mtime = parse_octal::<u64>(&buf[136..148])?;
        self.chksum = parse_octal::<u32>(&buf[148..156])?;
        self.typeflag = buf[156].into();
        if !skip_linkname {
            self.linkname = get_str(&buf[157..257])?;
        }
        self.magic = get_str_with_min_size(&buf[257..263], 6)?;
        self.version = get_str_with_min_size(&buf[263..265], 2)?;
        self.uname = get_str(&buf[265..297])?;
        self.gname = get_str(&buf[297..329])?;
        self.devmajor = parse_octal::<u32>(&buf[329..337])?;
        self.devminor = parse_octal::<u32>(&buf[337..345])?;
        self.atime = if &buf[345..357] != [0u8; 12] {
            match parse_octal::<u64>(&buf[345..357]) {
                Ok(val) => Some(val),
                Err(_) => None,
            }
        } else {
            None
        };
        self.ctime = if &buf[357..369] != [0u8; 12] {
            match parse_octal::<u64>(&buf[357..369]) {
                Ok(val) => Some(val),
                Err(_) => None,
            }
        } else {
            None
        };
        self.isextended = buf[482] == b'1';
        let buf_temp = &buf[483..495];
        self.realsize = if buf_temp != &[0u8; 12] {
            match parse_octal::<u64>(&buf_temp) {
                Ok(val) => Some(val),
                Err(_) => None,
            }
        } else {
            None
        };
        // TODO: calculate and validate checksum

        // GNU extensions:
        // Sparse entries: 4 x (offset: 12, numbytes: 12) = 96 bytes (500..596)
        for i in 0..4 {
            let offset = 386 + i * 24;
            let offset_buff = &buf[offset..offset+12];
            let numbytes_buff = &buf[offset+12..offset+24];
            if offset_buff != &[0u8; 12] && numbytes_buff != &[0u8; 12] {
                let offset = parse_octal::<u64>(offset_buff)?;
                let numbytes = parse_octal::<u64>(numbytes_buff)?;
                self.sparse.push(SparseEntry { offset, numbytes });
            }
        }
    
        // Incremental dump fields (not always present, e.g. 369..500)
        self.incremental = if self.sparse.len() < 1 && &buf[369..500] != &[0u8; 131] {
            Some(std::str::from_utf8(&buf[369..500])?.trim_end_matches(char::from(0)).to_string())
        } else {
            None
        };

        // gnu_extra: any remaining bytes
        self.gnu_extra.copy_from_slice(&buf[500..512]);

        // keep reading sparse fields when needed
        let mut next = self.isextended;
        while next {
            let mut buf = [0u8; 512];
            reader.read_exact(&mut buf)?;
            let mut offset = 0;
            while offset < 504 {
                let offset_buff = &buf[offset..offset+12];
                let numbytes_buff = &buf[offset+12..offset+24];
                if offset_buff != &[0u8; 12] && numbytes_buff != &[0u8; 12] {
                    let offset = parse_octal::<u64>(offset_buff)?;
                    let numbytes = parse_octal::<u64>(numbytes_buff)?;
                    self.sparse.push(SparseEntry { offset, numbytes });
                }
                offset += 24;
            }
            next = buf[504] == b'1';
        }
        Ok(())
    }
    
    /// Loads a GNU header including GNU extensions from the buffer and update the saved_blocks property.
    ///
    /// # Arguments
    /// * `buf` - 512-byte buffer containing the GNU header.
    /// * `reader` - Reader positioned at the start of a header block. Supports reading long name/link records.
    ///
    /// # Returns
    /// * `Ok(Option(Self))` - The loaded GNU header.
    /// * `Ok(None)` - If header is not a GNU header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load(buf: &[u8; 512], reader: &mut impl Read) -> Result<Option<Self>> {
        // validate headers
        if &buf[257..263] != b"ustar " || &buf[263..265] != b" \0" {
            return Ok(None);
        }
        let typeflag = buf[156].into();
        if let GnuTypeFlag::Ustar(UstarTypeFlag::Unknown(_)) = typeflag {
            return Ok(None);
        }

        // load each header in it's order
        let mut skip_name = false;
        let mut skip_linkname = false;
        let mut buffer = buf;
        let mut buf: [u8; 512] = [0u8; 512];
        let mut header = GnuHeader::new(typeflag);
        if typeflag == GnuTypeFlag::LongName {
            header.load_long_name(buffer, reader)?;
            reader.read_exact(&mut buf)?;
            buffer = &buf;
            skip_name = true;
        }
        if typeflag == GnuTypeFlag::LongLinkName {
            header.load_long_link(buffer, reader)?;
            reader.read_exact(&mut buf)?;
            buffer = &buf;
            skip_linkname = true;
        }
        header.load_standard(buffer, reader, skip_name, skip_linkname)?;
        header.saved_blocks = header.get_used_blocks();
        Ok(Some(header))
    }

    /// Saves a long header to the writer without updating the saved_blocks property.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    /// * `typeflag` - Type flag for the long header.
    /// * `value` - Value of the long header.
    ///
    /// # Returns
    /// * `Ok(bool)` - Whether the header was saved.
    /// * `Err(e)` - If write fails.
    pub fn save_long_header(&self, writer: &mut impl Write, typeflag: u8, value: &str) -> Result<bool> {
        // validate value size to be lower than 100 bytes
        let value_bytes = value.as_bytes();
        let value_bytes_len = value_bytes.len();
        if value_bytes_len < 101 {
            return Ok(false)
        }

        // save long header data
        let mut buf = [0u8; 512];
        buf[0..13].copy_from_slice(b"././@LongLink"); // name
        buf[100..108].copy_from_slice(b"0000000\0"); // mode
        buf[108..116].copy_from_slice(b"0000000\0"); // uid
        buf[116..124].copy_from_slice(b"0000000\0"); // gid
        put_octal(&mut buf[124..136], value_bytes_len as u64); // size
        buf[136..148].copy_from_slice(b"00000000000\0"); // mtime
        buf[148..156].fill(b' '); // chksum
        buf[156] = typeflag; // typeflag
        buf[257..263].copy_from_slice(b"ustar "); // magic
        buf[263..265].copy_from_slice(b" \0"); // version

        // compute checksum
        let mut chksum: u32 = 0;
        for i in 0..512 { chksum = chksum.wrapping_add(buf[i] as u32); }
        let chksum_str = format!("{:06o}\0 ", chksum);
        let chksum_bytes = chksum_str.as_bytes();
        buf[148..148+chksum_bytes.len()].copy_from_slice(chksum_bytes);
        writer.write_all(&buf)?;
        let value_bytes = value.as_bytes();
        writer.write_all(value_bytes)?;
        writer.write_all(&vec![0u8; 512 - value_bytes_len])?;
        Ok(true)
    }

    /// Saves a long name to the writer without updating the saved_blocks property.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    ///
    /// # Returns
    /// * `Ok(&str)` - An empty string if not long, else name.
    /// * `Err(e)` - If write fails.
    pub fn save_long_name(&self, writer: &mut impl Write) -> Result<bool> {
        self.save_long_header(writer, b'L', &self.name)
    }

    /// Saves a long link to the writer without updating the saved_blocks property.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    ///
    /// # Returns
    /// * `Ok(&str)` - An empty string if not long, else name.
    /// * `Err(e)` - If write fails.
    pub fn save_long_link(&self, writer: &mut impl Write) -> Result<bool> {
        self.save_long_header(writer, b'K', &self.linkname)
    }

    /// Saves a GNU header to the writer updating the saved blocks.
    ///
    /// # Arguments
    /// * `writer` - Byte writer.
    ///
    /// # Returns
    /// * `Ok(())` - On success.
    /// * `Err(e)` - If write fails.
    pub fn save(&mut self, writer: &mut impl Write) -> Result<()> {
        // write the possible GNU long headers when needed
        let skip_name = self.save_long_name(writer)?;
        let skip_linkname = self.save_long_link(writer)?;

        // Set buffer default bytes to spaces so the checksum field is correct before computing checksum (TAR spec)
        let mut buf = [0u8; 512];
        if !skip_name {
            put_str(&mut buf[0..100], &self.name);
        }
        put_octal(&mut buf[100..108], self.mode);
        put_octal(&mut buf[108..116], self.uid);
        put_octal(&mut buf[116..124], self.gid);
        put_octal(&mut buf[124..136], self.size);
        put_octal(&mut buf[136..148], self.mtime);
        // chksum is written after calculating
        buf[156] = self.typeflag.into();
        if !skip_linkname {
            put_str(&mut buf[157..257], &self.linkname);
        }
        put_str(&mut buf[257..263], &self.magic);
        put_str(&mut buf[263..265], &self.version);
        put_str(&mut buf[265..297], &self.uname);
        put_str(&mut buf[297..329], &self.gname);
        put_octal(&mut buf[329..337], self.devmajor);
        put_octal(&mut buf[337..345], self.devminor);

        // GNU extra
        buf[500..512].copy_from_slice(&self.gnu_extra);

        // GNU sparse entries
        let sparse_count = self.sparse.len();
        let mut isextended = false;

        // Write first 4 sparse entries into header
        let mut off = 386;
        for entry in self.sparse.iter().take(4) {
            put_octal(&mut buf[off..off+12], entry.offset);
            put_octal(&mut buf[off+12..off+24], entry.numbytes);
            off += 24;
        }

        // Set isextended flag in header if more than 4 entries
        buf[482] = if sparse_count > 4 {
            isextended = true;
            b'1'
        } else {
            b'0'
        };

        // Write realsize if present
        match self.realsize {
            Some(realsize) => put_octal(&mut buf[483..495], realsize),
            None => buf[483..495].fill(0)
        }

        // Write atime/ctime if present
        match self.atime {
            Some(atime) => put_octal(&mut buf[345..357], atime),
            None => buf[345..357].fill(0)
        }
        match self.ctime {
            Some(ctime) => put_octal(&mut buf[357..369], ctime),
            None => buf[357..369].fill(0)
        }

        // Write incremental dump fields if present and not a sparse file
        if self.sparse.is_empty() {
            match self.incremental {
                Some(ref inc) => {
                    let bytes = inc.as_bytes();
                    let n = std::cmp::min(bytes.len(), 131);
                    buf[369..369+n].copy_from_slice(&bytes[..n]);
                    buf[369+n..500].fill(0);
                },
                None => buf[369..500].fill(0)
            }
        }

        // Write checksum
        buf[148..156].fill(b' ');
        let mut chksum: u32 = 0;
        for i in 0..512 { chksum = chksum.wrapping_add(buf[i] as u32); }
        let chksum_str = format!("{:06o}\0 ", chksum);
        let chksum_bytes = chksum_str.as_bytes();
        buf[148..148+chksum_bytes.len()].copy_from_slice(chksum_bytes);
        self.chksum = chksum;

        // Write standard header
        writer.write_all(&buf)?;

        // Write extended sparse headers if needed
        if isextended {
            let total = self.sparse.len();
            let mut processed = 4;
            let mut offset;
            let mut block: [u8; 512];
            while processed < total {
                // write entries up to 21 per block
                block = [0u8; 512];
                offset = 0;
                while offset < 504 {
                    if !(processed < total) {
                        break;
                    }
                    let entry = &self.sparse[processed];
                    put_octal(&mut block[offset..offset+12], entry.offset);
                    put_octal(&mut block[offset+12..offset+24], entry.numbytes);
                    offset += 24;
                    processed += 1;
                }

                // Set isextended flag for this block
                block[504] = if processed < total { b'1' } else { b'0' };
                writer.write_all(&block)?;
            }
        }

        // update the saved blocks
        self.saved_blocks = self.get_used_blocks();
        Ok(())
    }
}

impl UsedBlocksTrait for GnuHeader {
    fn calc_used_blocks(&self) -> usize {
        let mut used_blocks = 1;
        let name_length = self.name.len();
        let linkname_length = self.linkname.len();
        if name_length > 100 {
            used_blocks += 1 + (name_length - 100) / 512 + if (name_length - 100) % 512 > 0 {1} else {0};
        }
        if linkname_length > 100 {
            used_blocks += 1 + (linkname_length - 100) / 512 + if (linkname_length - 100) % 512 > 0 {1} else {0};
        }
        let sparse_length = self.sparse.len();
        if sparse_length > 4 {
            used_blocks += (sparse_length - 4) / 21 + if (sparse_length - 4) % 21 > 0 {1} else {0};
        }
        used_blocks
    }

    fn get_used_blocks(&mut self) -> usize {
        if !self.updated_used_blocks {
            self.used_blocks = self.calc_used_blocks();
            self.updated_used_blocks = true;
        }
        self.used_blocks
    }

    fn get_saved_blocks(&self) -> usize {
        self.saved_blocks
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek};

    use super::*;

    fn sample_header() -> GnuHeader {
        GnuHeader {
            name: "testfile.txt".to_string(),
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 1234,
            mtime: 1_600_000_000,
            chksum: 0, // will be computed
            typeflag: GnuTypeFlag::Ustar(UstarTypeFlag::RegularFile),
            linkname: "".to_string(),
            magic: "ustar ".to_string(),
            version: " \0".to_string(),
            uname: "user".to_string(),
            gname: "group".to_string(),
            devmajor: 0,
            devminor: 0,
            sparse: vec![],
            isextended: false,
            realsize: None,
            atime: None,
            ctime: None,
            incremental: None,
            gnu_extra: [0u8; 12],
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false,
        }
    }

    #[test]
    fn sparse_header_round_trip_basic() {
        // 1–4 sparse entries (no extended header)
        let mut header = sample_header();
        for n in 0..4 {
            header.sparse.push(SparseEntry { offset: n as u64 * 100, numbytes: 50 + n as u64 });
        }
        let mut stream = Cursor::new([0u8; 4096]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse, header.sparse);
        assert_eq!(loaded.isextended, false);
    }

    #[test]
    fn sparse_header_round_trip_extended() {
        // 25 entries (4 in main, 21 in one extended block)
        let mut header = sample_header();
        header.sparse = (0..25).map(|i| SparseEntry { offset: i as u64 * 1000, numbytes: 500 + i as u64 }).collect();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse, header.sparse);
        assert_eq!(loaded.isextended, true);
    }

    #[test]
    fn sparse_header_round_trip_extended_2_headers() {
        // 25 entries (4 in main, 21 in one extended block)
        let mut header = sample_header();
        header.sparse = (0..34).map(|i| SparseEntry { offset: i as u64 * 1000, numbytes: 500 + i as u64 }).collect();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse, header.sparse);
        assert_eq!(loaded.isextended, true);
    }

    #[test]
    fn sparse_header_edge_cases() {
        // 0 entries
        let mut header = sample_header();
        header.sparse.clear();
        let mut stream = Cursor::new([0u8; 4096]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse.len(), 0);
        assert_eq!(loaded.isextended, false);

        // 4 entries (no isextended)
        let mut header = sample_header();
        header.sparse = (0..4).map(|i| SparseEntry { offset: i as u64 * 10, numbytes: 10 }).collect();
        let mut stream = Cursor::new([0u8; 4096]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse, header.sparse);
        assert_eq!(loaded.isextended, false);

        // 5 entries (should trigger extended header)
        let mut header = sample_header();
        header.sparse = (0..5).map(|i| SparseEntry { offset: i as u64 * 10, numbytes: 10 }).collect();
        let mut stream = Cursor::new([0u8; 4096]);
        header.save(&mut stream).expect("save");
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(loaded.sparse, header.sparse);
        assert_eq!(loaded.isextended, true);
    }

    #[test]
    fn gnu_field_name_round_trip() {
        let mut header = sample_header();
        header.name = "testfile.txt".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
    }

    #[test]
    fn gnu_field_mode_round_trip() {
        let mut header = sample_header();
        header.mode = 0o755;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.mode, loaded.mode);
    }

    #[test]
    fn gnu_field_uid_round_trip() {
        let mut header = sample_header();
        header.uid = 1234;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.uid, loaded.uid);
    }

    #[test]
    fn gnu_field_gid_round_trip() {
        let mut header = sample_header();
        header.gid = 5678;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.gid, loaded.gid);
    }

    #[test]
    fn gnu_field_size_round_trip() {
        let mut header = sample_header();
        header.size = 987654321;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.size, loaded.size);
    }

    #[test]
    fn gnu_field_mtime_round_trip() {
        let mut header = sample_header();
        header.mtime = 123456789;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.mtime, loaded.mtime);
    }

    #[test]
    fn gnu_field_typeflag_round_trip() {
        let mut header = sample_header();
        header.typeflag = GnuTypeFlag::Ustar(UstarTypeFlag::RegularFile);
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.typeflag, loaded.typeflag);
    }

    #[test]
    fn gnu_field_linkname_round_trip() {
        let mut header = sample_header();
        header.linkname = "symlink.txt".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.linkname, loaded.linkname);
    }

    #[test]
    fn gnu_field_magic_round_trip() {
        let mut header = sample_header();
        header.magic = "ustar ".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.magic, loaded.magic);
    }

    #[test]
    fn gnu_field_version_round_trip() {
        let mut header = sample_header();
        header.version = " \0".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.version, loaded.version);
    }

    #[test]
    fn gnu_field_uname_round_trip() {
        let mut header = sample_header();
        header.uname = "user1".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.uname, loaded.uname);
    }

    #[test]
    fn gnu_field_gname_round_trip() {
        let mut header = sample_header();
        header.gname = "group1".to_string();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.gname, loaded.gname);
    }

    #[test]
    fn gnu_field_devmajor_round_trip() {
        let mut header = sample_header();
        header.devmajor = 8;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.devmajor, loaded.devmajor);
    }

    #[test]
    fn gnu_field_devminor_round_trip() {
        let mut header = sample_header();
        header.devminor = 9;
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.devminor, loaded.devminor);
    }

    #[test]
    fn gnu_field_realsize_sparse_round_trip() {
        let mut header = sample_header();
        header.sparse = vec![SparseEntry { offset: 1234567, numbytes: 1234567 }];
        header.realsize = Some(1234567);
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.realsize, loaded.realsize);
    }

    #[test]
    fn gnu_field_realsize_incremental_round_trip() {
        // Don't add any sparse records to the header to trigger incremental
        let mut header = sample_header();
        header.realsize = Some(1234567);
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(None, loaded.realsize);
    }

    #[test]
    fn gnu_field_atime_round_trip() {
        let mut header = sample_header();
        header.atime = Some(1234);
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.atime, loaded.atime);
    }

    #[test]
    fn gnu_field_ctime_round_trip() {
        let mut header = sample_header();
        header.ctime = Some(5678);
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.ctime, loaded.ctime);
    }

    #[test]
    fn gnu_field_incremental_round_trip() {
        let mut header = sample_header();
        header.sparse.clear(); // incremental only written if sparse is empty
        header.incremental = Some("incdumpdata".to_string());
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.incremental, loaded.incremental);
    }

    #[test]
    fn gnu_field_gnu_extra_round_trip() {
        let mut header = sample_header();
        header.gnu_extra = *b"extrafield12";
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
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
        assert_eq!(header.gnu_extra, loaded.gnu_extra);
    }

    #[test]
    fn round_trip_save_load() {
        let mut header = sample_header();
        let mut stream = Cursor::new([0u8; 2048]);
        match header.save(&mut stream) {
            Ok(_) => assert!(true),
            Err(e) => {
                assert!(false, "failed to save header: {}", e);
                return;
            }
        };
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&mut buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
                None => {
                    assert!(false, "Invalid magic/version");
                    return;
                },
            },
            Err(e) => {
                assert!(false, "failed to load header: {}", e);
                return;
            }
        };
        // chksum is not round-tripped, ignore for comparison
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
        assert_eq!(header.isextended, loaded.isextended);
        assert_eq!(header.realsize, loaded.realsize);
        assert_eq!(header.atime, loaded.atime);
        assert_eq!(header.ctime, loaded.ctime);
        assert_eq!(header.incremental, loaded.incremental);
        assert_eq!(header.gnu_extra, loaded.gnu_extra);
        assert_eq!(header.used_blocks, loaded.used_blocks);
        assert_eq!(header.saved_blocks, loaded.saved_blocks);
        assert_eq!(header.updated_used_blocks, loaded.updated_used_blocks);
        // Do not compare chksum field; it is recalculated on save/load
    }

    #[test]
    fn minimal_header() {
        let mut header = GnuHeader {
            name: "".to_string(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: GnuTypeFlag::Ustar(UstarTypeFlag::Unknown(0)),
            linkname: "".to_string(),
            magic: "ustar ".to_string(),
            version: " \0".to_string(),
            uname: "".to_string(),
            gname: "".to_string(),
            devmajor: 0,
            devminor: 0,
            sparse: vec![],
            isextended: false,
            realsize: None,
            atime: None,
            ctime: None,
            incremental: None,
            gnu_extra: [0u8; 12],
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false,
        };
        let mut stream = Cursor::new([0u8; 2048]);
        assert!(!header.updated_used_blocks, "expected updated_used_blocks to be false");
        assert_eq!(0, header.used_blocks);
        assert_eq!(0, header.saved_blocks);
        match header.save(&mut stream) {
            Ok(_) => {},
            Err(e) => {
                assert!(false, "failed to save header: {}", e);
                return;
            }
        }
        assert!(header.updated_used_blocks, "expected updated_used_blocks to be true");
        assert_eq!(1, header.used_blocks);
        assert_eq!(1, header.saved_blocks);
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match GnuHeader::load(&mut buf, &mut stream) {
            Ok(opt) => match opt {
                Some(header) => header,
                None => {
                    assert!(false, "expected header");
                    return;
                }
            },
            Err(e) => {
                assert!(false, "failed to load header: {}", e);
                return;
            }
        };
        assert_eq!(header, loaded);
        // Do not compare chksum field; it is recalculated on save/load
    }

    #[test]
    fn invalid_data() {
        // Buffer too short
        let mut stream = Cursor::new([0u8; 2048]);
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        match GnuHeader::load(&mut buf, &mut stream) {
            Ok(opt) => match opt {
                Some(_) => assert!(false, "expected invalid magic/version"),
                None => assert!(true),
            },
            Err(e) => assert!(false, "expected invalid magic/version but got error: {}", e),
        }
    }

    #[test]
    fn gnu_long_name_block_layout() {
        let mut header = sample_header();
        header.typeflag = GnuTypeFlag::Ustar(UstarTypeFlag::RegularFile);
        let long_name = std::str::from_utf8(&[42u8; 101] as &[u8]).unwrap().to_string();
        header.name = long_name.clone();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 2048];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(buf[156], b'L'); // typeflag 'L' for long name
        assert_eq!(&buf[512..512+long_name.len()], long_name.as_bytes());
        assert_eq!(buf[1024+156], b'0'); // next header is standard header
    }

    #[test]
    fn gnu_long_linkname_block_layout() {
        let mut header = sample_header();
        header.typeflag = GnuTypeFlag::Ustar(UstarTypeFlag::RegularFile);
        let long_linkname = std::str::from_utf8(&[42u8; 101] as &[u8]).unwrap().to_string();
        header.linkname = long_linkname.clone();
        let mut stream = Cursor::new([0u8; 2048]);
        header.save(&mut stream).unwrap();
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 2048];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(buf[156], b'K'); // typeflag 'K' for long linkname
        assert_eq!(&buf[512..512+long_linkname.len()], long_linkname.as_bytes());
        assert_eq!(buf[1024+156], b'0'); // next header is standard header
    }

    #[test]
    fn calc_used_blocks_default() {
        let header = sample_header();
        assert_eq!(1, header.calc_used_blocks());
    }
    #[test]
    fn calc_used_blocks_long_name() {
        let mut header = sample_header();
        assert_eq!(1, header.calc_used_blocks());
        header.name = std::str::from_utf8(&[42u8; 101] as &[u8]).unwrap().to_string();
        assert_eq!(3, header.calc_used_blocks());
        header.name = std::str::from_utf8(&[42u8; 612] as &[u8]).unwrap().to_string();
        assert_eq!(3, header.calc_used_blocks());
        header.name = std::str::from_utf8(&[42u8; 613] as &[u8]).unwrap().to_string();
        assert_eq!(4, header.calc_used_blocks());
        header.name = std::str::from_utf8(&[42u8; 1124] as &[u8]).unwrap().to_string();
        assert_eq!(4, header.calc_used_blocks());
        header.name = std::str::from_utf8(&[42u8; 1125] as &[u8]).unwrap().to_string();
        assert_eq!(5, header.calc_used_blocks());
    }

    #[test]
    fn calc_used_blocks_long_linkname() {
        let mut header = sample_header();
        assert_eq!(1, header.calc_used_blocks());
        header.linkname = std::str::from_utf8(&[42u8; 101] as &[u8]).unwrap().to_string();
        assert_eq!(3, header.calc_used_blocks());
        header.linkname = std::str::from_utf8(&[42u8; 612] as &[u8]).unwrap().to_string();
        assert_eq!(3, header.calc_used_blocks());
        header.linkname = std::str::from_utf8(&[42u8; 613] as &[u8]).unwrap().to_string();
        assert_eq!(4, header.calc_used_blocks());
        header.linkname = std::str::from_utf8(&[42u8; 1124] as &[u8]).unwrap().to_string();
        assert_eq!(4, header.calc_used_blocks());
        header.linkname = std::str::from_utf8(&[42u8; 1125] as &[u8]).unwrap().to_string();
        assert_eq!(5, header.calc_used_blocks());
    }

    #[test]
    fn calc_used_blocks_sparse() {
        let mut header = sample_header();
        assert_eq!(1, header.calc_used_blocks());
        for _ in 0..4 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(1, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..5 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(2, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..25 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(2, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..26 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(3, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..46 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(3, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..47 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(4, header.calc_used_blocks());
        header.sparse.clear();
        for _ in 0..50 {
            header.sparse.push(SparseEntry {
                offset: 0,
                numbytes: 0
            });
        }
        assert_eq!(4, header.calc_used_blocks());
    }

    #[test]
    fn set_name() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.set_name("my name".to_string());
        assert_eq!(&header.name, "my name");
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn set_linkname() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.set_linkname("my linkname".to_string());
        assert_eq!(&header.linkname, "my linkname");
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn push_sparse() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.push_sparse(SparseEntry {
            offset: 0,
            numbytes: 0
        });
        assert_eq!(header.sparse.len(), 1);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn pop_sparse() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.sparse.push(SparseEntry {
            offset: 0,
            numbytes: 0
        });
        assert_eq!(header.sparse.len(), 1);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.pop_sparse();
        assert_eq!(header.sparse.len(), 0);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn insert_sparse() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.insert_sparse(0, SparseEntry {
            offset: 0,
            numbytes: 0
        });
        assert_eq!(header.sparse.len(), 1);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn remove_sparse() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.sparse.push(SparseEntry {
            offset: 0,
            numbytes: 0
        });
        assert_eq!(header.sparse.len(), 1);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.remove_sparse(0);
        assert_eq!(header.sparse.len(), 0);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn clear_sparse() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.sparse.push(SparseEntry {
            offset: 0,
            numbytes: 0
        });
        assert_eq!(header.sparse.len(), 1);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.clear_sparse();
        assert_eq!(header.sparse.len(), 0);
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
    }

    #[test]
    fn get_used_blocks() {
        let mut header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        assert_eq!(header.get_used_blocks(), 1);
        header = sample_header();
        assert!(!header.updated_used_blocks, "used_blocks should not be updated");
        header.name = std::str::from_utf8(&[42u8; 101] as &[u8]).unwrap().to_string();
        assert_eq!(header.get_used_blocks(), 3);
        assert!(header.updated_used_blocks, "used_blocks should be updated");
    }
}
