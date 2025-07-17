mod file;
mod page;

pub use file::FileEntry;
pub use page::{Page, RECORD_COUNT as PAGE_RECORD_COUNT};

use anyhow::{bail, Result};
use std::io::{Read, Seek, SeekFrom, Write};

use dhfarm_engine::db::table::traits::TableTrait;
use dhfarm_engine::Segment;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::engine::{header::{IsTypeTrait, PaxHeader, PaxTypeFlag, TarHeader, UsedBlocksTrait, UstarTypeFlag}, index::file::FileMeta};

pub const PAGE_SIZE: u64 = 1024 * 1024;

pub(crate) struct Index {
    pub first_page: usize,
    pub pages: Vec<Page>,

    /// Files in the page.
    entries: IndexMap<String, FileEntry>,

    /// Modified entries.
    modified: HashMap<usize, PhantomData<()>>,
}

impl Index {
    /// Creates an index instance with a single page.
    /// 
    /// # Returns
    /// 
    /// * `Self`: The created index instance.
    pub fn new() -> Self {
        let mut entries = IndexMap::new();
        entries.shift_insert(0, "".to_string(), FileEntry::default());
        Self {
            first_page: 0,
            pages: Vec::new(),
            entries,
            modified: HashMap::new()
        }
    }

    pub fn read_headers(stream: impl Read + Seek) -> Result<()> {
        unimplemented!()
    }

    /// Opens an index file and loads all pages into memory.
    ///
    /// # Arguments
    ///
    /// * `stream`: The stream to read the index file from.
    ///
    /// # Returns
    ///
    /// * `IoResult<Self>`: The result of the open operation.
    pub fn open(stream: &mut (impl Read + Seek + Write)) -> Result<Self> {
        let mut offset;
        let mut pages = Vec::new();
        let mut entries = IndexMap::new();
        entries.insert(String::default(), FileEntry::default());

        // read pages
        loop {
            // read page header
            let mut header = TarHeader::load(stream)?;
            if !header.is_regular_file() {
                bail!("expected regular file");
            }
            let size = header.get_content_size();
            if size != PAGE_SIZE {
                bail!("invalid index page size");
            }

            // read page data
            offset = stream.stream_position()?;
            let mut segment = Segment::new_unsafe(stream, offset, size)?;
            match Page::load(&mut segment) {
                Ok(mut page) => {
                    // validate table
                    if page.table.header.meta.record_count != PAGE_RECORD_COUNT {
                        bail!("invalid index page record count");
                    }

                    // record page offsets
                    page.offset = offset;
                    page.table_offset = offset + 512 * header.get_used_blocks() as u64;

                    // first record is always the offset of the next page unless 0
                    let record = match page.table.record_from(&mut segment, 0)? {
                        Some(record) => record,
                        None => bail!("expected record 0 to exists")
                    };
                    

                    // add page records to the index
                    let iter = page.iter(&mut segment)?;
                    let mut is_first = true;
                    for record in iter {
                        // handle the first entry, this one contains the offset of the next page
                        if is_first {
                            offset = match record.get("offset") {
                                Some(v) => v.try_into()?,
                                None => bail!("expected record 0 to contain 'offset' field")
                            };
                            continue;
                        }

                        // handle the other entries
                        let entry = FileEntry::from_record(&record)?;
                        if entry.meta.offset < 1 {
                            // exit whenever the offset is 0, this will mark us the first empty record
                            break;
                        }
                        entries.insert(entry.meta.path.clone(), entry);
                    }

                    // save table as page
                    pages.push(page);

                    // exit when offset is 0
                    if offset < 1 {
                        break;
                    }
                }
                Err(_) => {
                    // exit as error when the index positions are corrupted
                    bail!("page not found, the index is corrupted, please fallback to scan mode");
                },
            }
        }
        Ok(Self{
            first_page: 0,
            pages,
            entries,
            modified: HashMap::new()
        })
    }

    /// Adds a new page to the index.
    /// 
    /// # Arguments
    /// 
    /// * `segment` - Segment to write the page into.
    /// * `offset` - Offset of the new page.
    /// * `path` - Path of the new page.
    pub fn add_page(&mut self, stream: &mut (impl Read + Seek + Write), path: &str) -> Result<&mut Page> {
        // seek up to the end of the TAR
        stream.seek(SeekFrom::End(1024))?;
        let page_offset = stream.stream_position()?;

        // save new page
        let mut header = PaxHeader::new(PaxTypeFlag::Ustar(UstarTypeFlag::RegularFile));
        header.set_attr_path(path);
        header.set_attr_size(PAGE_SIZE);
        header.save(stream)?;
        let table_offset = page_offset + 512 * header.get_used_blocks() as u64;
        let mut segment = Segment::new_unsafe(stream, table_offset, PAGE_SIZE)?;
        let mut page = Page::new(&mut segment)?;
        page.offset = page_offset;
        page.table_offset = table_offset;

        // write TAR end
        stream.write(&[0u8; 1024])?;
        stream.flush()?;

        // update the last page to point to the new page
        let page_count = self.pages.len();
        if page_count > 0 {
            let last_page = &mut self.pages[page_count - 1];
            let mut record = last_page.table.header.record.new_record()?;
            record.set("offset", page_offset.into());
            record.set("path", path.into());
            let mut last_segment = Segment::new_unsafe(stream, last_page.table_offset, PAGE_SIZE)?;
            last_page.table.save_record_into(&mut last_segment, 0, &record)?;
        }

        // save new page into the page array
        self.pages.push(page);
        Ok(self.pages.last_mut().unwrap())
    }

    /// Gets the number of entries in the page.
    /// 
    /// # Returns
    /// 
    /// * `usize` - The number of entries in the page.
    pub fn len(&self) -> usize {
        self.entries.len() - 1
    }

    /// Remove an entry from the page.
    /// 
    /// # Arguments
    /// 
    /// * `writer` - The writer to use for writing the page.
    /// * `index` - The index of the entry to remove.
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - The result of the remove operation.
    pub fn remove(&mut self, index: usize) -> Result<()> {
        // validate index
        let index = index + 1;
        let len = self.entries.len();
        if index > len - 1 {
            return Err(anyhow::anyhow!("index out of bounds"));
        }

        // rearrange when entry to be removed is not the last one
        let last = len - 1;
        if index < last {
            // move last entry to the removed entry index and rearrange the last entry references
            let last_next_part = self.entries[last].next_part;
            let last_prev_part = self.entries[last].prev_part;
            if last_next_part > 0 {
                self.entries[last_next_part].prev_part = index;
                self.modified.insert(last_next_part, PhantomData::default());
            }
            if last_prev_part > 0 {
                self.entries[last_prev_part].next_part = index;
                self.modified.insert(last_prev_part, PhantomData::default());
            }
            self.entries.swap_indices(index, last);
            self.modified.insert(index, PhantomData::default());
        }

        // rearrange references of the entry to be removed
        let removed_entry = self.entries.pop().unwrap().1;
        let removed_next_part = removed_entry.next_part;
        let removed_prev_part = removed_entry.prev_part;
        if removed_next_part > 0 {
            if removed_next_part > last {
                bail!("entry to be removed has a next_part out of bounds")
            }
            self.entries[removed_next_part].prev_part = removed_prev_part;
            self.modified.insert(removed_next_part, PhantomData::default());
        }
        if removed_prev_part > 0 {
            if removed_prev_part > last {
                bail!("entry to be removed has a prev_part out of bounds")
            }
            self.entries[removed_prev_part].next_part = removed_next_part;
            self.modified.insert(removed_prev_part, PhantomData::default());
        }

        Ok(())
    }

    /// Flushes the modified entries to the writer.
    /// 
    /// # Arguments
    /// 
    /// * `writer` - The writer to use for writing the page.
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - The result of the flush operation.
    pub fn flush(&mut self, writer: &mut (impl Read + Seek + Write)) -> Result<()> {
        // TODO: finish the entries logic movement from src/engine/index/page.rs to src/engine/index.rs

        // update modified entry records
        let length = self.entries.len();
        for index in self.modified.keys() {
            let index = *index;
            if index < length {
                let record = match self.entries.get_index(index) {
                    Some((_, entry)) => entry.as_record(&self.table)?,
                    None => continue
                };
                self.table.save_record_into(writer, index as u64, &record)?;
            }
        }

        // soft delete empty records
        if length < self.max_index {
            let empty_record = self.table.header.record.new_record()?;
            for index in length..self.max_index {
                self.table.save_record_into(writer, index as u64, &empty_record)?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    /// Appends an entry to the page.
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The entry to append.
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - The result of the append operation.
    pub fn append(&mut self, entry: FileMeta, prev_part: usize, next_part: usize) -> Result<()> {
        let length = self.entries.len();
        if self.entries.contains_key(&entry.path) {
            return Err(anyhow::anyhow!("entry already exists"));
        }
        self.entries.insert(entry.path.clone(), FileEntry {
            meta: entry,
            next_part: next_part,
            prev_part: prev_part
        });
        self.modified.insert(length, PhantomData::default());
        self.max_index = length;
        Ok(())
    }

    /// Gets an entry by path.
    /// 
    /// # Arguments
    /// 
    /// * `path` - The path of the entry to get.
    /// 
    /// # Returns
    /// 
    /// * `Option<&FileEntry>` - The entry if found, otherwise None.
    pub fn get(&self, path: &str) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    /// Gets a mutable entry by path.
    /// 
    /// # Arguments
    /// 
    /// * `path` - The path of the entry to get.
    /// 
    /// # Returns
    /// 
    /// * `Option<&mut FileEntry>` - The entry if found, otherwise None.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut FileEntry> {
        self.entries.get_mut(path)
    }

    /// Gets an entry by index.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index of the entry to get.
    /// 
    /// # Returns
    /// 
    /// * `Option<&FileEntry>` - The entry if found, otherwise None.
    pub fn get_index(&self, index: usize) -> Option<&FileEntry> {
        match self.entries.get_index(index + 1) {
            Some((_, entry)) => Some(entry),
            None => None
        }
    }

    /// Gets a mutable entry by index.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The index of the entry to get.
    /// 
    /// # Returns
    /// 
    /// * `Option<&mut FileEntry>` - The entry if found, otherwise None.
    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut FileEntry> {
        match self.entries.get_index_mut(index + 1) {
            Some((_, entry)) => Some(entry),
            None => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}