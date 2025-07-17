use dhfarm_engine::db::table::Table;
use dhfarm_engine::db::field::Record;
use anyhow::{bail, Result};

#[derive(Clone, PartialEq, Debug)]
pub struct FileMeta {
    pub offset: u64,
    pub path: String,
    pub parted: bool,
    pub size: u64,
}

impl FileMeta {
    /// Copies the values from another file meta into this one.
    /// 
    /// # Arguments
    /// 
    /// * `meta`: The file meta to copy from.
    pub fn copy_from(&mut self, meta: &FileMeta) {
        self.offset = meta.offset;
        self.path = meta.path.clone();
        self.parted = meta.parted;
        self.size = meta.size;
    }
}

impl Default for FileMeta {
    fn default() -> Self {
        Self {
            offset: 0,
            path: String::new(),
            parted: false,
            size: 0
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct FileEntry {
    pub meta: FileMeta,
    pub next_part: usize,
    pub prev_part: usize
}

impl FileEntry {
    /// Copies the values from another file entry into this one.
    /// 
    /// # Arguments
    /// 
    /// * `entry`: The file entry to copy from.
    pub fn copy_from(&mut self, entry: &FileEntry) {
        self.meta.copy_from(&entry.meta);
        self.next_part = entry.next_part;
        self.prev_part = entry.prev_part;
    }

    pub fn as_record(&self, table: &Table) -> Result<Record> {
        let mut record = table.header.record.new_record()?;
        record.set("offset", self.meta.offset.into());
        record.set("path", self.meta.path.as_str().into());
        record.set("parted", self.meta.parted.into());
        record.set("size", self.meta.size.into());
        record.set("next_part", (self.next_part as u8).into());
        record.set("prev_part", (self.prev_part as u8).into());
        Ok(record)
    }

    pub fn from_record(record: &Record) -> Result<Self> {
        let meta = FileMeta {
            offset: match record.get("offset") {
                Some(v) => v.try_into()?,
                None => bail!("expected 'offset' field")
            },
            path: match record.get("path") {
                Some(v) => v.try_into()?,
                None => bail!("expected 'path' field")
            },
            parted: match record.get("parted") {
                Some(v) => v.try_into()?,
                None => bail!("expected 'parted' field")
            },
            size: match record.get("size") {
                Some(v) => v.try_into()?,
                None => bail!("expected 'size' field")
            }
        };
        let next_part: u8 = match record.get("next_part") {
            Some(v) => v.try_into()?,
            None => bail!("expected 'next_part' field")
        };
        let prev_part: u8 = match record.get("prev_part") {
            Some(v) => v.try_into()?,
            None => bail!("expected 'prev_part' field")
        };
        Ok(Self {
            meta,
            next_part: next_part.into(),
            prev_part: prev_part.into()
        })
    }
}

impl Default for FileEntry {
    fn default() -> Self {
        Self {
            meta: FileMeta::default(),
            next_part: 0,
            prev_part: 0
        }
    }
}