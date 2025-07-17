use anyhow::Result;
use std::io::{Read, Write};

/// Represents a PAX TAR header.
use indexmap::IndexMap;
use dhfarm_engine::db::field::Value;
use super::helper::*;
use super::{UsedBlocksTrait, IsTypeTrait, UstarTypeFlag};

/// PAX header type flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaxTypeFlag {
    Extended,
    Global,
    Ustar(UstarTypeFlag)
}

impl From<u8> for PaxTypeFlag {
    fn from(value: u8) -> Self {
        match value {
            b'x' => Self::Extended,
            b'g' => Self::Global,
            v => Self::Ustar(UstarTypeFlag::from(v)),
        }
    }
}

impl From<PaxTypeFlag> for u8 {
    fn from(value: PaxTypeFlag) -> Self {
        match value {
            PaxTypeFlag::Extended => b'x',
            PaxTypeFlag::Global => b'g',
            PaxTypeFlag::Ustar(v) => u8::from(v),
        }
    }
}

impl IsTypeTrait for PaxTypeFlag {
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

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The value of the attribute unless it is a string then it will be Value::Default
    pub value: Value,

    /// The raw value of the attribute
    pub raw: String
}

impl Attribute {
    pub fn from_str(s: String) -> Self {
        Self {
            value: Value::Default,
            raw: s
        }
    }

    pub fn from_u64(s: String) -> Self {
        Self {
            value: Value::U64(s.parse::<u64>().unwrap()),
            raw: s
        }
    }

    pub fn from_f64(s: String) -> Self {
        Self {
            value: Value::F64(s.parse::<f64>().unwrap()),
            raw: s
        }
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Represents a PAX TAR header (extended attributes)
#[derive(Debug, Clone, PartialEq)]
pub struct PaxHeader {
    /// File name (null-terminated) (max 100 bytes for standard)
    pub name: String,
    /// File mode (octal string)
    pub mode: u32,
    /// Owner user ID (octal string) (max 8 bytes for standard)
    pub uid: u32,
    /// Owner group ID (octal string) (max 8 bytes for standard)
    pub gid: u32,
    /// File size in bytes (octal string) (max 12 bytes for standard)
    pub size: u64,
    /// Modification time (octal string) (max 12 bytes for standard)
    pub mtime: u64,
    /// Header checksum (octal string)
    pub chksum: u32,
    /// Type flag
    pub typeflag: PaxTypeFlag,
    /// Name of linked file (null-terminated) (max 100 bytes for standard)
    pub linkname: String,
    /// USTAR indicator "ustar" or PAX magic
    pub magic: String,
    /// USTAR version "00"
    pub version: String,
    /// Owner user name (null-terminated) (max 32 bytes for standard)
    pub uname: String,
    /// Owner group name (null-terminated) (max 32 bytes for standard)
    pub gname: String,
    /// Device major number (octal string)
    pub devmajor: u32,
    /// Device minor number (octal string)
    pub devminor: u32,
    /// Filename prefix (null-terminated) (max 155 bytes for standard)
    pub prefix: String,
    /// PAX attributes (key-value pairs, preserves order)
    attributes: IndexMap<String, Attribute>,
    /// The used blocks so far not saved yet.
    used_blocks: usize,
    /// The used blocks saved.
    saved_blocks: usize,
    /// Should calculate used blocks.
    updated_used_blocks: bool
}

impl PaxHeader {
    /// Returns the PAX path attribute if present.
    pub fn get_attr_path(&self) -> Option<&str> {
        match self.attributes.get("path") {
            Some(attr) => Some(&attr.raw),
            None => None
        }
    }

    /// Sets the PAX path attribute.
    /// 
    /// # Arguments
    /// 
    /// * `path` - The path to set.
    pub fn set_attr_path(&mut self, path: &str) {
        self.set_attr("path", Attribute::from_str(path.to_string()));
    }

    /// Returns the PAX linkpath attribute if present.
    pub fn get_attr_linkpath(&self) -> Option<&str> {
        match self.attributes.get("linkpath") {
            Some(attr) => Some(&attr.raw),
            None => None
        }
    }

    /// Sets the PAX linkpath attribute.
    /// 
    /// # Arguments
    /// 
    /// * `linkpath` - The linkpath to set.
    pub fn set_attr_linkpath(&mut self, linkpath: &str) {
        self.set_attr("linkpath", Attribute::from_str(linkpath.to_string()));
    }

    /// Returns the PAX uname attribute if present.
    pub fn get_attr_uname(&self) -> Option<&str> {
        match self.attributes.get("uname") {
            Some(attr) => Some(&attr.raw),
            None => None
        }
    }

    /// Sets the PAX uname attribute.
    /// 
    /// # Arguments
    /// 
    /// * `uname` - The uname to set.
    pub fn set_attr_uname(&mut self, uname: &str) {
        self.set_attr("uname", Attribute::from_str(uname.to_string()));
    }

    /// Returns the PAX gname attribute if present.
    pub fn get_attr_gname(&self) -> Option<&str> {
        match self.attributes.get("gname") {
            Some(attr) => Some(&attr.raw),
            None => None
        }
    }

    /// Sets the PAX gname attribute.
    /// 
    /// # Arguments
    /// 
    /// * `gname` - The gname to set.
    pub fn set_attr_gname(&mut self, gname: &str) {
        self.set_attr("gname", Attribute::from_str(gname.to_string()));
    }

    /// Returns the PAX uid attribute if present, parsed as u64.
    pub fn get_attr_uid(&self) -> Option<u64> {
        match self.attributes.get("uid") {
            Some(attr) => match attr.value {
                Value::U64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX uid attribute.
    /// 
    /// # Arguments
    /// 
    /// * `uid` - The uid to set.
    pub fn set_attr_uid(&mut self, uid: u64) {
        self.set_attr("uid", Attribute{value: Value::U64(uid), raw: uid.to_string()});
    }

    /// Returns the PAX gid attribute if present, parsed as u64.
    pub fn get_attr_gid(&self) -> Option<u64> {
        match self.attributes.get("gid") {
            Some(attr) => match attr.value {
                Value::U64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX gid attribute.
    /// 
    /// # Arguments
    /// 
    /// * `gid` - The gid to set.
    pub fn set_attr_gid(&mut self, gid: u64) {
        self.set_attr("gid", Attribute{value: Value::U64(gid), raw: gid.to_string()});
    }

    /// Returns the PAX size attribute if present, parsed as u64.
    pub fn get_attr_size(&self) -> Option<u64> {
        match self.attributes.get("size") {
            Some(attr) => match attr.value {
                Value::U64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX uid attribute.
    /// 
    /// # Arguments
    /// 
    /// * `uid` - The uid to set.
    pub fn set_attr_size(&mut self, size: u64) {
        self.set_attr("size", Attribute{value: Value::U64(size), raw: size.to_string()});
    }

    /// Returns the PAX mtime attribute if present, parsed as f64.
    pub fn get_attr_mtime(&self) -> Option<f64> {
        match self.attributes.get("mtime") {
            Some(attr) => match attr.value {
                Value::F64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX mtime attribute.
    pub fn set_attr_mtime(&mut self, mtime: f64) {
        self.set_attr("mtime", Attribute{value: Value::F64(mtime), raw: mtime.to_string()});
    }

    /// Returns the PAX atime attribute if present, parsed as f64.
    pub fn get_attr_atime(&self) -> Option<f64> {
        match self.attributes.get("atime") {
            Some(attr) => match attr.value {
                Value::F64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX atime attribute.
    /// 
    /// # Arguments
    /// 
    /// * `atime` - The atime to set.
    pub fn set_attr_atime(&mut self, atime: f64) {
        self.set_attr("atime", Attribute{value: Value::F64(atime), raw: atime.to_string()});
    }

    /// Returns the PAX ctime attribute if present, parsed as f64.
    pub fn get_attr_ctime(&self) -> Option<f64> {
        match self.attributes.get("ctime") {
            Some(attr) => match attr.value {
                Value::F64(v) => Some(v),
                _ => None
            },
            None => None
        }
    }

    /// Sets the PAX ctime attribute.
    /// 
    /// # Arguments
    /// 
    /// * `ctime` - The ctime to set.
    pub fn set_attr_ctime(&mut self, ctime: f64) {
        self.set_attr("ctime", Attribute{value: Value::F64(ctime), raw: ctime.to_string()});
    }

    /// Returns the PAX attribute if present.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute.
    /// 
    /// # Returns
    /// * `Option<&Attribute>` - The attribute if present.
    pub fn get_attr(&self, key: &str) -> Option<&Attribute> {
        self.attributes.get(key)
    }

    /// Inserts the PAX attribute at the specified index.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute.
    /// * `value` - The value of the attribute.
    pub fn insert_attr(&mut self, key: &str, value: Attribute) -> Option<Attribute> {
        self.updated_used_blocks = false;
        self.attributes.insert(key.to_string(), value)
    }

    /// Sets the PAX attribute.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute.
    /// * `value` - The value of the attribute.
    pub fn set_attr(&mut self, key: &str, value: Attribute) {
       self.insert_attr(key, value);
    }

    /// Inserts the PAX attribute at the specified index.
    /// 
    /// # Arguments
    /// * `index` - The index at which to insert the attribute.
    /// * `key` - The key of the attribute.
    /// * `value` - The value of the attribute.
    pub fn insert_attr_at(&mut self, index: usize, key: &str, value: Attribute) {
        self.updated_used_blocks = false;
        self.attributes.shift_insert(index, key.to_string(), value);
    }

    /// Returns the index of the PAX attribute with the specified key.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute.
    pub fn get_attr_index(&self, key: &str) -> Option<usize> {
        self.attributes.get_index_of(key)
    }

    /// Removes the PAX attribute with the specified key.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute to remove.
    pub fn remove_attr(&mut self, key: &str) -> Option<Attribute> {
        self.updated_used_blocks = false;
        self.attributes.shift_remove(key)
    }

    /// Removes the PAX attribute at the specified index.
    /// 
    /// # Arguments
    /// * `index` - The index at which to remove the attribute.
    pub fn remove_attr_at(&mut self, index: usize) -> Option<(String, Attribute)> {
        self.updated_used_blocks = false;
        self.attributes.shift_remove_index(index)
    }

    /// Pushes the PAX attribute to the end of the attributes.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute.
    /// * `value` - The value of the attribute.
    pub fn push_attr(&mut self, key: &str, value: Attribute) {
        self.updated_used_blocks = false;
        self.attributes.insert(key.to_string(), value);
    }

    /// Pops the PAX attribute from the end of the attributes.
    /// 
    /// # Arguments
    /// * `key` - The key of the attribute to pop.
    pub fn pop_attr(&mut self) -> Option<(String, Attribute)> {
        self.updated_used_blocks = false;
        self.attributes.pop()
    }

    /// Clears all PAX attributes.
    pub fn clear_attr(&mut self) {
        self.updated_used_blocks = false;
        self.attributes.clear();
    }
    
    /// Returns an iterator over the PAX attributes.
    pub fn iter_attr(&self) -> indexmap::map::Iter<'_, String, Attribute> {
        self.attributes.iter()
    }
    
    /// Returns a mutable iterator over the PAX attributes.
    pub fn iter_attr_mut(&mut self) -> indexmap::map::IterMut<'_, String, Attribute> {
        self.attributes.iter_mut()
    }

    /// Creates a new PAX header.
    pub fn new(typeflag: PaxTypeFlag) -> Self {
        Self {
            name: String::default(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag,
            linkname: String::default(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: String::default(),
            gname: String::default(),
            devmajor: 0,
            devminor: 0,
            prefix: String::default(),
            attributes: IndexMap::new(),
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false
        }
    }

    /// Loads a PAX header from the buffer and update the saved_blocks property.
    ///
    /// # Arguments
    /// * `buf` - Byte buffer.
    /// * `reader` - Reader positioned at the start of a header block. Supports reading long name/link records.
    ///
    /// # Returns
    /// * `Ok(Self)` - The loaded PAX header.
    /// * `Err(e)` - If header could not be read or parsed.
    pub fn load(buf: &[u8; 512], reader: &mut impl Read) -> Result<Option<Self>> {
        // validate headers
        if &buf[257..262] != b"ustar"
            || (buf[262] != b' ' && buf[262] != b'\0')
            || (&buf[263..265] != b"00" && &buf[263..265] != b" \0")
            || (buf[156] != b'x' && buf[156] != b'g') {
            return Ok(None);
        }
        let typeflag = buf[156].into();
        if let PaxTypeFlag::Ustar(UstarTypeFlag::Unknown(_)) = typeflag {
            return Ok(None);
        }

        // load standard header data
        let mut header = PaxHeader::new(typeflag);
        header.name = get_str(&buf[0..100])?;
        header.mode = parse_octal::<u32>(&buf[100..108])?;
        header.uid = parse_octal::<u32>(&buf[108..116])?;
        header.gid = parse_octal::<u32>(&buf[116..124])?;
        header.size = parse_octal::<u64>(&buf[124..136])?;
        header.mtime = parse_octal::<u64>(&buf[136..148])?;
        header.chksum = parse_octal::<u32>(&buf[148..156])?;
        header.linkname = get_str(&buf[157..257])?;
        header.magic = get_str_with_min_size(&buf[257..263], 6)?;
        header.version = get_str_with_min_size(&buf[263..265], 2)?;
        header.uname = get_str(&buf[265..297])?;
        header.gname = get_str(&buf[297..329])?;
        header.devmajor = parse_octal::<u32>(&buf[329..337])?;
        header.devminor = parse_octal::<u32>(&buf[337..345])?;
        header.prefix = get_str(&buf[345..500])?;
        // TODO: calculate and validate checksum

        // Read PAX attribute data block from reader in 512-byte chunks, streaming parse with Vec<u8>
        let size = header.size;
        if size > 0 {
            let mut total_read = 0u64;
            let mut data_buf = [0u8; 512];
            let mut line_size = 0usize;
            let mut line_buf: Vec<u8> = Vec::new();
            let mut virtual_buf: &[u8];
            let mut virtual_last_index: usize;
            let lookup  = [b' ', b'=', b'\n'];
            let mut lookup_index = 0usize;
            let mut key: String = String::default();
            let mut value: Attribute;
            let mut value_raw: String;
            let mut index: usize;
            let mut char: u8;
            let mut start: usize;
            while total_read < size {
                // read a more bytes from reader into the data buffer
                index = 0;
                start = 0;
                reader.read_exact(&mut data_buf)?;
                total_read += 512;
                virtual_buf = if total_read > size {
                    virtual_last_index = (512 + size - total_read - 1) as usize;
                    &data_buf[0..virtual_last_index + 1]
                } else {
                    virtual_last_index = 511;
                    &data_buf
                };

                // Parse PAX data into lines
                loop {
                    // exit loop when we reach the end of the buffer to load more data
                    if index > virtual_last_index {
                        if index > start {
                            line_buf.extend_from_slice(&virtual_buf[start..index]);
                        }
                        break;
                    }

                    // grab next char
                    char = virtual_buf[index];
                    index += 1;

                    // check current lookup
                    if char != lookup[lookup_index] {
                        continue;
                    }
                    match lookup_index {
                        // handle key start
                        0 => {
                            line_buf.extend_from_slice(&virtual_buf[start..index - 1]);
                            line_size = usize::from_str_radix(std::str::from_utf8(&line_buf)?, 10)?;
                            line_buf = Vec::with_capacity(line_size);
                            lookup_index = 1;
                            start = index;
                        },
                        // handle '='
                        1 => {
                            line_buf.extend_from_slice(&virtual_buf[start..index - 1]);
                            key = std::str::from_utf8(&line_buf)?.to_string();
                            line_buf = Vec::with_capacity(line_size + (index - start - 1));
                            start = index;
                            lookup_index = 2;
                        },
                        // handle '\n'
                        _ => {
                            line_buf.extend_from_slice(&virtual_buf[start..index - 1]);
                            value_raw = std::str::from_utf8(&line_buf)?.to_string();
                            value = match &key as &str {
                                "uid" => Attribute::from_u64(value_raw),
                                "gid" => Attribute::from_u64(value_raw),
                                "mtime" => Attribute::from_f64(value_raw),
                                "atime" => Attribute::from_f64(value_raw),
                                "ctime" => Attribute::from_f64(value_raw),
                                "size" => Attribute::from_u64(value_raw),
                                _ => Attribute::from_str(value_raw)
                            };
                            line_buf = Vec::new();
                            lookup_index = 0;
                            header.attributes.insert(key, value);
                            key = String::default();
                            start = index;
                        }
                    }
                }
            }
        }

        header.saved_blocks = header.get_used_blocks();
        Ok(Some(header))
    }

    /// Calculates the size of a PAX data line.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key of the attribute.
    /// * `value` - The value of the attribute.
    /// 
    /// # Returns
    /// 
    /// * `u64` - The size of the attribute.
    fn calc_line_size(key: &str, value: &Attribute) -> u64 {
        // first we calc the line without the line size prefix, basically: " key=value\n"
        let line_size = (key.as_bytes().len() + value.raw.as_bytes().len() + 3) as u64;

        // now we calc the line size digits so we can use it later for a correction
        let line_digits = (line_size.checked_ilog10().unwrap_or(0) + 1) as u64;

        // we calc the first iteration of the prefix using the expected prefix size + line size
        let mut prefix = line_digits + line_size;

        // we now calculate the estimated prefix digits
        let mut prefix_digits = (prefix.checked_ilog10().unwrap_or(0) + 1) as u64;

        // we adjust the prefix digits until they match the previous prefix digits
        let mut old_prefix_digits;
        loop {
            old_prefix_digits = prefix_digits;
            prefix = prefix + prefix_digits - line_digits;
            prefix_digits = (prefix.checked_ilog10().unwrap_or(0) + 1) as u64;
            if prefix_digits == old_prefix_digits {
                break;
            }
        }

        // provide the total line size including the prefix digits
        prefix
    }

    /// Saves the PAX header to the writer updating the saved blocks.
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

        // Calculate PAX attribute data block size
        let mut pax_size = 0u64;
        for (k, v) in &self.attributes {
            pax_size += Self::calc_line_size(k, v);
        }
        put_octal(&mut buf[124..136], pax_size);
        put_octal(&mut buf[136..148], self.mtime);
        buf[156] = self.typeflag.into();
        put_str(&mut buf[157..257], &self.linkname);
        put_str(&mut buf[257..263], &self.magic);
        put_str(&mut buf[263..265], &self.version);
        put_str(&mut buf[265..297], &self.uname);
        put_str(&mut buf[297..329], &self.gname);
        put_octal(&mut buf[329..337], self.devmajor);
        put_octal(&mut buf[337..345], self.devminor);

        // Only write the prefix field (filename prefix)
        put_str(&mut buf[345..500], &self.prefix);

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

        // Write PAX attributes as key=value\n lines in insertion order (IndexMap)
        for (k, v) in &self.attributes {
            let line_size = Self::calc_line_size(k, v);
            let prefix = format!("{} ", line_size);
            writer.write_all(prefix.as_bytes())?;
            writer.write_all(k.as_bytes())?;
            writer.write_all(b"=")?;
            writer.write_all(v.raw.as_bytes())?;
            writer.write_all(b"\n")?;
        }

        self.saved_blocks = self.get_used_blocks();
        Ok(())
    }

    /// Returns true if this PAX header is a global header (applies to all subsequent files).
    pub fn is_global(&self) -> bool {
        self.typeflag == PaxTypeFlag::Global
    }
}

impl UsedBlocksTrait for PaxHeader {
    fn calc_used_blocks(&self) -> usize {
        let mut used_blocks = 1;
        let mut total = 0;
        if self.attributes.len() > 0 {
            for (key, value) in &self.attributes {
                total += Self::calc_line_size(key, value);
            }
            used_blocks += (total / 512 + if total % 512 > 0 {1} else {0}) as usize;
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
    use super::*;
    use std::io::{Cursor, Seek};

    fn sample_header() -> PaxHeader {
        let mut attributes = IndexMap::new();
        attributes.insert("path".to_string(), Attribute::from_str("test.txt".to_string()));
        attributes.insert("size".to_string(), Attribute::from_u64("1234".to_string()));
        PaxHeader {
            name: "test.txt".to_string(),
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 30,
            mtime: 1_600_000_000,
            chksum: 0,
            typeflag: PaxTypeFlag::Extended,
            linkname: "abc".to_string(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: "user".to_string(),
            gname: "group".to_string(),
            devmajor: 2,
            devminor: 1,
            prefix: "bcd".to_string(),
            attributes,
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false,
        }
    }

    #[test]
    fn attribute_from_str() {
        let attr = Attribute::from_str("hello".to_string());
        assert_eq!(attr.value, Value::Default);
        assert_eq!(attr.raw, "hello");
    }

    #[test]
    fn attribute_from_u64() {
        let attr = Attribute::from_u64("1234".to_string());
        assert_eq!(attr.value, Value::U64(1234));
        assert_eq!(attr.raw, "1234");
    }

    #[test]
    fn attribute_from_f64() {
        let attr = Attribute::from_f64("1234.56".to_string());
        assert_eq!(attr.value, Value::F64(1234.56));
        assert_eq!(attr.raw, "1234.56");
    }

    #[test]
    fn calc_line_size() {
        let key = "hello";
        let value = "world";
        let size = PaxHeader::calc_line_size(key, &Attribute::from_str(value.to_string()));
        // 15 hello=world\n
        assert_eq!(size, 15);
    }

    #[test]
    fn calc_line_size_extra_digit() {
        let key = "a";
        let value = "world";
        let size = PaxHeader::calc_line_size(key, &Attribute::from_str(value.to_string()));
        // 15 hello=world\n
        assert_eq!(size, 11);
    }

    #[test]
    fn test_is_global() {
        let mut h = PaxHeader {
            name: "./PaxHeaders.X/global".to_string(),
            mode: 0o644,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: PaxTypeFlag::Global,
            linkname: String::new(),
            magic: "ustar".to_string(),
            version: "00".to_string(),
            uname: "user".to_string(),
            gname: "group".to_string(),
            devmajor: 0,
            devminor: 0,
            prefix: String::new(),
            attributes: IndexMap::new(),
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false
        };
        assert!(h.is_global());
        h.typeflag = PaxTypeFlag::Extended;
        assert!(!h.is_global());
    }

    #[test]
    fn save_sets_name_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 100];
        put_str(&mut expected, &header.name);
        assert_eq!(&buf[0..100], &expected);
    }

    #[test]
    fn save_sets_mode_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 8];
        put_octal(&mut expected, header.mode);
        assert_eq!(&buf[100..108], &expected);
    }

    #[test]
    fn save_sets_uid_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 8];
        put_octal(&mut expected, header.uid);
        assert_eq!(&buf[108..116], &expected);
    }

    #[test]
    fn save_sets_gid_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 8];
        put_octal(&mut expected, header.gid);
        assert_eq!(&buf[116..124], &expected);
    }

    #[test]
    fn save_sets_size_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        // Calculate expected size using the iterative approach (real-world PAX line calculation)
        let mut pax_size = 0u64;
        for (k, v) in &header.attributes {
            let line = format!(" {}={}\n", k, v);
            let mut len = line.len();
            loop {
                let prefix = format!("{}", len);
                let full_line = format!("{}{}", prefix, &line);
                if full_line.len() == len {
                    pax_size += len as u64;
                    break;
                }
                len = full_line.len();
            }
        }
        let mut expected = [0u8; 12];
        put_octal(&mut expected, pax_size);
        assert_eq!(&buf[124..136], &expected);
    }

    #[test]
    fn save_sets_mtime_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 12];
        put_octal(&mut expected, header.mtime);
        assert_eq!(&buf[136..148], &expected);
    }

    #[test]
    fn save_sets_typeflag_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        assert_eq!(buf[156], u8::from(header.typeflag));
    }

    #[test]
    fn save_sets_linkname_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 100];
        put_str(&mut expected, &header.linkname);
        assert_eq!(&buf[157..257], &expected);
    }

    #[test]
    fn save_sets_magic_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 6];
        put_str(&mut expected, &header.magic);
        assert_eq!(&buf[257..263], &expected);
    }

    #[test]
    fn save_sets_version_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 2];
        put_str(&mut expected, &header.version);
        assert_eq!(&buf[263..265], &expected);
    }

    #[test]
    fn save_sets_uname_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 32];
        put_str(&mut expected, &header.uname);
        assert_eq!(&buf[265..297], &expected);
    }

    #[test]
    fn save_sets_gname_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 32];
        put_str(&mut expected, &header.gname);
        assert_eq!(&buf[297..329], &expected);
    }

    #[test]
    fn save_sets_devmajor_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 8];
        put_octal(&mut expected, header.devmajor);
        assert_eq!(&buf[329..337], &expected);
    }

    #[test]
    fn save_sets_devminor_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 8];
        put_octal(&mut expected, header.devminor);
        assert_eq!(&buf[337..345], &expected);
    }

    #[test]
    fn save_sets_prefix_field() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        let mut expected = [0u8; 155];
        put_str(&mut expected, &header.prefix);
        assert_eq!(&buf[345..500], &expected);
    }

    #[test]
    fn save_sets_first_pax_attribute_line() {
        let mut header = sample_header();
        let mut buf = [0u8; 2048];
        header.save(&mut (&mut buf as &mut [u8])).expect("save");
        // PAX data starts at offset 514
        // Build expected first line
        let mut pos = 512;
        for (k, v) in header.attributes.iter() {
            let line = format!(" {}={}\n", k, v);
            let mut len = line.len();
            loop {
                let prefix = format!("{}", len);
                let full_line = format!("{}{}", prefix, &line);
                if full_line.len() == len {
                    let bytes = full_line.as_bytes();
                    assert_eq!(&buf[pos..pos + bytes.len()], bytes);
                    pos += bytes.len();
                    break;
                }
                len = full_line.len();
            }
        }
    }

    #[test]
    fn test_get_uid_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("uid".to_string(), Attribute::from_u64("12345".to_string()));
        assert_eq!(header.get_attr_uid(), Some(12345));
        header.attributes.insert("uid".to_string(), Attribute::from_str("notanumber".to_string()));
        assert_eq!(header.get_attr_uid(), None);
    }
    #[test]
    fn test_get_gid_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("gid".to_string(), Attribute::from_u64("54321".to_string()));
        assert_eq!(header.get_attr_gid(), Some(54321));
        header.attributes.insert("gid".to_string(), Attribute::from_str("notanumber".to_string()));
        assert_eq!(header.get_attr_gid(), None);
    }
    #[test]
    fn test_get_size_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        assert_eq!(header.get_attr_size(), Some(99999));
        header.attributes.insert("size".to_string(), Attribute::from_str("notanumber".to_string()));
        assert_eq!(header.get_attr_size(), None);
    }
    #[test]
    fn test_get_mtime_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("mtime".to_string(), Attribute::from_f64("1625097600.123".to_string()));
        assert_eq!(header.get_attr_mtime(), Some(1625097600.123));
        header.attributes.insert("mtime".to_string(), Attribute::from_str("notafloat".to_string()));
        assert_eq!(header.get_attr_mtime(), None);
    }
    #[test]
    fn test_get_atime_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("atime".to_string(), Attribute::from_f64("1625097601.456".to_string()));
        assert_eq!(header.get_attr_atime(), Some(1625097601.456));
        header.attributes.insert("atime".to_string(), Attribute::from_str("notafloat".to_string()));
        assert_eq!(header.get_attr_atime(), None);
    }
    #[test]
    fn test_get_ctime_from_attribute() {
        let mut header = sample_header();
        header.attributes.insert("ctime".to_string(), Attribute::from_f64("1625097602.789".to_string()));
        assert_eq!(header.get_attr_ctime(), Some(1625097602.789));
        header.attributes.insert("ctime".to_string(), Attribute::from_str("notafloat".to_string()));
        assert_eq!(header.get_attr_ctime(), None);
    }
    #[test]
    fn round_trip_save_load() {
        let mut header = sample_header();
        let mut stream = Cursor::new([0u8; 2048]);
        assert!(!header.updated_used_blocks, "expected updated_used_blocks to be false");
        assert_eq!(header.used_blocks, 0);
        assert_eq!(header.saved_blocks, 0);
        match header.save(&mut stream) {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "Failed to save header: {}", e),
        }
        assert!(header.updated_used_blocks, "expected updated_used_blocks to be true");
        assert_eq!(header.used_blocks, 2);
        assert_eq!(header.saved_blocks, 2);
        stream.flush().unwrap();
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match PaxHeader::load(&buf, &mut stream) {
            Ok(opt) => match opt {
                Some(h) => h,
                None => {
                    assert!(false, "Invalid magic/version");
                    return;
                }
            },
            Err(e) => {
                assert!(false, "Failed to load header: {}", e);
                return;
            }
        };
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
        assert_eq!(header.chksum, loaded.chksum);
        assert_eq!(header.used_blocks, loaded.used_blocks);
        assert_eq!(header.saved_blocks, loaded.saved_blocks);
        assert_eq!(header.updated_used_blocks, loaded.updated_used_blocks);
        assert_eq!(header.attributes.len(), loaded.attributes.len());
        // prefix is used for attribute serialization
        for (k, v) in &header.attributes {
            assert_eq!(loaded.attributes.get(k), Some(v));
        }
    }

    #[test]
    fn minimal_header() {
        let mut header = PaxHeader {
            name: "".to_string(),
            mode: 0,
            uid: 0,
            gid: 0,
            size: 0,
            mtime: 0,
            chksum: 0,
            typeflag: PaxTypeFlag::Extended,
            linkname: "".to_string(),
            magic: "ustar\0".to_string(),
            version: "00".to_string(),
            uname: "".to_string(),
            gname: "".to_string(),
            devmajor: 0,
            devminor: 0,
            prefix: "".to_string(),
            attributes: IndexMap::new(),
            used_blocks: 0,
            saved_blocks: 0,
            updated_used_blocks: false
        };
        let mut stream = Cursor::new([0u8; 1024]);
        match header.save(&mut stream) {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, "Failed to save header: {}", e),
        }
        stream.rewind().unwrap();
        let mut buf = [0u8; 512];
        stream.read_exact(&mut buf).unwrap();
        let loaded = match PaxHeader::load(&buf, &mut stream) {
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
        assert!(loaded.attributes.is_empty());
    }

    #[test]
    fn calc_used_blocks() {
        let mut header = sample_header();
        header.attributes.clear();
        assert_eq!(header.calc_used_blocks(), 1);
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        assert_eq!(header.calc_used_blocks(), 2);
        header.attributes.clear();
        let massive_text = "a".repeat(506);
        header.attributes.insert("a".to_string(), Attribute::from_str(massive_text));
        assert_eq!(header.calc_used_blocks(), 3);
        header.attributes.clear();
        let massive_text = "a".repeat(1018);
        header.attributes.insert("a".to_string(), Attribute::from_str(massive_text));
        assert_eq!(header.calc_used_blocks(), 4);
    }

    #[test]
    fn set_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 0);
        header.set_attr("size", Attribute::from_u64("99999".to_string()));
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 1);
    }

    #[test]
    fn insert_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 0);
        header.insert_attr("size", Attribute::from_u64("99999".to_string()));
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 1);
    }

    #[test]
    fn insert_attr_at() {
        let mut header = sample_header();
        header.attributes.clear();
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 0);
        header.insert_attr_at(0, "size", Attribute::from_u64("99999".to_string()));
        header.insert_attr_at(0, "foo", Attribute::from_u64("99999".to_string()));
        header.insert_attr_at(1, "bar", Attribute::from_u64("99999".to_string()));
        assert_eq!(header.attributes.get_index_of("size"), Some(2));
        assert_eq!(header.attributes.get_index_of("foo"), Some(0));
        assert_eq!(header.attributes.get_index_of("bar"), Some(1));
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 3);
    }

    #[test]
    fn remove_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 1);
        header.remove_attr("size");
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 0);
    }

    #[test]
    fn remove_attr_at() {
        let mut header = sample_header();
        header.attributes.clear();
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        header.attributes.insert("test".to_string(), Attribute::from_u64("99999".to_string()));
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 2);
        header.remove_attr_at(0);
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 1);
    }

    #[test]
    fn clear_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        header.attributes.insert("test".to_string(), Attribute::from_u64("99999".to_string()));
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 2);
        header.clear_attr();
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 0);
    }

    #[test]
    fn push_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 0);
        header.push_attr("size", Attribute::from_u64("99999".to_string()));
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 1);
    }

    #[test]
    fn pop_attr() {
        let mut header = sample_header();
        header.attributes.clear();
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        header.updated_used_blocks = true;
        assert_eq!(header.updated_used_blocks, true);
        assert_eq!(header.attributes.len(), 1);
        let attr = header.pop_attr();
        assert_eq!(header.updated_used_blocks, false);
        assert_eq!(header.attributes.len(), 0);
        assert_eq!(attr, Some(("size".to_string(), Attribute::from_u64("99999".to_string()))));
    }

    #[test]
    fn get_used_blocks() {
        let mut header = sample_header();
        header.attributes.clear();
        assert!(!header.updated_used_blocks);
        assert_eq!(header.used_blocks, 0);
        assert_eq!(header.get_used_blocks(), 1);
        assert!(header.updated_used_blocks);
        header.attributes.insert("size".to_string(), Attribute::from_u64("99999".to_string()));
        header.updated_used_blocks = false;
        assert_eq!(header.used_blocks, 1);
        assert_eq!(header.get_used_blocks(), 2);
        assert!(header.updated_used_blocks);
        header.attributes.insert("path".to_string(), Attribute::from_u64("99999".to_string()));
        assert!(header.updated_used_blocks);
        assert_eq!(header.used_blocks, 2);
        assert_eq!(header.get_used_blocks(), 2);
        assert!(header.updated_used_blocks);
        header.updated_used_blocks = false;
        header.used_blocks = 0;
        assert_eq!(header.get_used_blocks(), 2);
        assert!(header.updated_used_blocks);
    }
}