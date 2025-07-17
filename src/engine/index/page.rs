use anyhow::{bail, Result};
use indexmap::IndexMap;
use std::{collections::HashMap, io::{Read, Seek, Write}, marker::PhantomData};
use dhfarm_engine::{db::{field::FieldType, table::{traits::TableTrait, IterRecord, Table}}, traits::ByteSized, uuid::Uuid};
use crate::engine::index::{file::FileMeta, FileEntry};

pub const RECORD_COUNT: u64 = 51;

/// Represents a page of the index.
pub struct Page {
    /// Page offset.
    pub offset: u64,

    /// Page table offset.
    pub table_offset: u64,
    
    /// Table used to store the file entries.
    pub table: Table,

    /// Maximum index registered table records. We will use it to know how many
    /// records to soft remove from the table.
    max_index: usize
}

impl Page {
    /// Creates a new page and initializes the files array with a single empty entry
    /// to avoid the use of Option for next_part and prev_part so whenever it has a
    /// value of 0, it means it is empty.
    /// 
    /// # Arguments:
    /// 
    /// * `segment` - The segment to use for creating the page.
    /// 
    /// # Returns:
    /// 
    /// * `Result<Self>` - The created page.
    pub fn new(segment: &mut (impl Read + Seek + Write)) -> Result<Self> {
        let mut table = Table::new("page", Some(Uuid::from_bytes([0u8; Uuid::BYTES])))?;
        table.header_mut().record.add("offset", FieldType::U64).unwrap();
        table.header_mut().record.add("path", FieldType::Str(100)).unwrap();
        table.header_mut().record.add("parted", FieldType::Bool).unwrap();
        table.header_mut().record.add("size", FieldType::U64).unwrap();
        table.header_mut().record.add("next_part", FieldType::U8).unwrap();
        table.header_mut().record.add("prev_part", FieldType::U8).unwrap();
        table.save_headers_into(segment)?;
        table.fill_records_into(segment, RECORD_COUNT)?;
        Ok(Self {
            table,
            max_index: 0,
            offset: 0,
            table_offset: 0
        })
    }

    /// Loads a page from a reader.
    /// 
    /// # Arguments:
    /// 
    /// * `reader` - The reader to use for loading the page.
    /// 
    /// # Returns:
    /// 
    /// * `Result<Self>` - The loaded page.
    pub fn load(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let table = Table::load(reader)?;
        let mut entries = IndexMap::new();
        let iter = table.iter(reader, None, None)?;
        
        let max_index = entries.len();
        Ok(Self {
            table,
            max_index,
            offset: 0,
            table_offset: 0
        })
    }

    /// Return the table record iterator.
    /// 
    /// # Arguments
    /// 
    /// * `reader` - Byte reader.
    /// 
    /// # Returns
    /// 
    /// * `Result<IterRecord<'reader, 'table, impl Read + Seek>>` - The iterator of the table records.
    pub fn iter<'reader, 'table>(&'table self, reader: &'reader mut (impl Read + Seek)) -> Result<IterRecord<'reader, 'table, impl Read + Seek>> {
        self.table.iter(reader, None, None)
    }
}

#[cfg(test)]
mod test_helper {
    use dhfarm_engine::db::field::{Record, Value};

    use crate::engine::index::PAGE_SIZE;

    use super::*;

    /// Adds records to the table.
    /// 
    /// # Arguments
    /// 
    /// * `table` - The table to add records to.
    /// * `writer` - The writer to use for writing the records.
    /// 
    /// # Returns
    /// 
    /// * `Result<(Vec<Record>, Vec<FileEntry>)>` - The result of the add operation.
    pub fn add_records(table: &mut Table, writer: &mut (impl Read + Write + Seek)) -> Result<(Vec<Record>, Vec<FileEntry>)> {
        let mut records = Vec::new();
        let mut entries = Vec::new();
        let mut offset = PAGE_SIZE;

        // add first record
        let mut  record = table.header.record.new_record()?;
        record.set("offset", Value::U64(offset));
        record.set("path", Value::Str("/path/to/recordA.0".to_string()));
        record.set("parted", Value::Bool(true));
        record.set("size", Value::U64(10));
        record.set("next_part", Value::U8(3));
        record.set("prev_part", Value::U8(0));
        table.append_record_into(writer, &record, false)?;
        entries.push(FileEntry::from_record(&record)?);
        records.push(record);
        offset += 512 + 10;

        // add second record
        let mut record = table.header.record.new_record()?;
        record.set("offset", Value::U64(offset));
        record.set("path", Value::Str("/path/to/recordB".to_string()));
        record.set("parted", Value::Bool(false));
        record.set("size", Value::U64(5));
        record.set("next_part", Value::U8(0));
        record.set("prev_part", Value::U8(0));
        table.append_record_into(writer, &record, false)?;
        entries.push(FileEntry::from_record(&record)?);
        records.push(record);
        offset += 512 + 5;

        // add third record
        let mut record = table.header.record.new_record()?;
        record.set("offset", Value::U64(offset));
        record.set("path", Value::Str("/path/to/recordA.1".to_string()));
        record.set("parted", Value::Bool(true));
        record.set("size", Value::U64(5));
        record.set("next_part", Value::U8(0));
        record.set("prev_part", Value::U8(1));
        table.append_record_into(writer, &record, true)?;
        entries.push(FileEntry::from_record(&record)?);
        records.push(record);

        Ok((records, entries))
    }

    pub fn create_fake_table(writer: &mut (impl Read + Write + Seek), record_count: u64) -> Result<Table> {
        let mut table = Table::new("page", Some(Uuid::from_bytes([0u8; Uuid::BYTES])))?;
        table.header.record.add("offset", FieldType::U64)?;
        table.header.record.add("path", FieldType::Str(100))?;
        table.header.record.add("parted", FieldType::Bool)?;
        table.header.record.add("size", FieldType::U64)?;
        table.header.record.add("next_part", FieldType::U8)?;
        table.header.record.add("prev_part", FieldType::U8)?;
        table.save_headers_into(writer)?;
        table.fill_records_into(writer, record_count)?;
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dhfarm_engine::db::field::Value;
    use dhfarm_engine::traits::DataTrait;
    use dhfarm_engine::Data;
    use std::io::Cursor;

    #[test]
    fn new() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let page = match Page::new(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to create page: {}", e);
                return;
            }
        };
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0], FileEntry::default());
    }

    #[test]
    fn load() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        let (_, entries) = test_helper::add_records(&mut table, &mut data).unwrap();
        data.flush().unwrap();
        let page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(4, page.entries.len());
        assert_eq!(FileEntry::default(), page.entries[0]);
        assert_eq!(entries[0], page.entries[1]);
        assert_eq!(entries[1], page.entries[2]);
        assert_eq!(entries[2], page.entries[3]);
    }

    #[test]
    fn load_ignore_extra_records() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        let (_, entries) = test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(4, page.entries.len());
        assert_eq!(FileEntry::default(), page.entries[0]);
        assert_eq!(entries[0], page.entries[1]);
        assert_eq!(entries[1], page.entries[2]);
        assert_eq!(entries[2], page.entries[3]);
    }

    #[test]
    fn len() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut page = match Page::new(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(1, page.entries.len());
        assert_eq!(0, page.len());
        page.entries.insert("testA".to_string(), FileEntry::default());
        assert_eq!(2, page.entries.len());
        assert_eq!(1, page.len());
        page.entries.insert("testB".to_string(), FileEntry::default());
        assert_eq!(3, page.entries.len());
        assert_eq!(2, page.len());
    }

    #[test]
    fn remove() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(4, page.entries.len());
        assert_eq!(3, page.entries[1].next_part);
        assert_eq!(0, page.entries[1].prev_part);
        if let Err(e) = page.remove(2) {
            assert!(false, "Failed to remove entry: {}", e);
            return;
        }
        assert_eq!(3, page.entries.len());
        assert_eq!("", &page.entries[0].meta.path);
        assert_eq!("/path/to/recordA.0", &page.entries[1].meta.path);
        assert_eq!(0, page.entries[1].next_part);
        assert_eq!(0, page.entries[1].prev_part);
        assert_eq!("/path/to/recordB", &page.entries[2].meta.path);
        if let Err(e) = page.remove(0) {
            assert!(false, "Failed to remove entry: {}", e);
            return;
        }
        assert_eq!(2, page.entries.len());
        assert_eq!("", &page.entries[0].meta.path);
        assert_eq!("/path/to/recordB", &page.entries[1].meta.path);
        if let Err(e) = page.remove(0) {
            assert!(false, "Failed to remove entry: {}", e);
            return;
        }
        assert_eq!(1, page.entries.len());
        assert_eq!("", &page.entries[0].meta.path);
    }

    #[test]
    fn remove_rearrange() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(4, page.entries.len());
        assert_eq!(0, page.entries[3].next_part);
        assert_eq!(1, page.entries[3].prev_part);
        if let Err(e) = page.remove(0) {
            assert!(false, "Failed to remove entry: {}", e);
            return;
        }
        assert_eq!(3, page.entries.len());
        assert_eq!("", &page.entries[0].meta.path);
        assert_eq!("/path/to/recordA.1", &page.entries[1].meta.path);
        assert_eq!(0, page.entries[1].next_part);
        assert_eq!(0, page.entries[1].prev_part);
        assert_eq!("/path/to/recordB", &page.entries[2].meta.path);
    }

    #[test]
    fn remove_out_of_bounds() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut page = Page::new(&mut data).unwrap();
        match page.remove(99) {
            Ok(_) => assert!(false, "expected error but got success"),
            Err(e) => assert_eq!(e.to_string(), "index out of bounds")
        } match page.remove(0) {
            Ok(_) => assert!(false, "expected error but got success"),
            Err(e) => assert_eq!(e.to_string(), "index out of bounds")
        }
    }

    #[test]
    fn flush() {
        let mut binary = Vec::new();
        let mut data = Data::new(Cursor::new(&mut binary), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        let _ = test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut expected = binary.clone();
        let mut data = Data::new(Cursor::new(&mut binary), false);
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        assert_eq!(4, page.entries.len());
        if let Err(e) = page.remove(2) {
            assert!(false, "Failed to remove entry: {}", e);
            return;
        }
        assert_eq!(3, page.entries.len());
        assert_eq!("", &page.entries[0].meta.path);
        assert_eq!("/path/to/recordA.0", &page.entries[1].meta.path);
        assert_eq!("/path/to/recordB", &page.entries[2].meta.path);
        assert_eq!(expected, binary);
        let mut data = Data::new(Cursor::new(&mut binary), false);
        match page.flush(&mut data) {
            Ok(_) => {},
            Err(e) => {
                assert!(false, "Failed to flush page: {}", e);
                return;
            }
        }
        let mut expected_data = Data::new(Cursor::new(&mut expected), false);
        let empty_record = table.header_ref().record.new_record().unwrap();
        let mut record = page.table.record_from(&mut expected_data, 1).unwrap().unwrap();
        record.set("next_part", Value::U8(0));
        page.table.save_record_into(&mut expected_data, 1, &record).unwrap();
        page.table.save_record_into(&mut expected_data, 3, &empty_record).unwrap();
        page.table.save_headers_into(&mut expected_data).unwrap();
        expected_data.flush().unwrap();
        assert_eq!(expected, binary);
    }

    #[test]
    fn append() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        let entry_meta = FileMeta {
            path: "/path/to/recordC".to_string(),
            offset: 0,
            size: 0,
            parted: false
        };
        let expected = FileEntry {
            meta: entry_meta.clone(),
            next_part: 3,
            prev_part: 2
        };
        if let Err(e) = page.append(entry_meta, 2, 3) {
            assert!(false, "Failed to append entry: {}", e);
            return;
        }
        assert_eq!(5, page.entries.len());
        assert_eq!(expected, page.entries[4]);

    }

    #[test]
    fn get() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        match page.get("/path/to/recordA.0") {
            Some(v) => assert_eq!(&page.entries[1], v),
            None => assert!(false, "expected entry recordA.0 but got not found")
        }
        match page.get("/path/to/recordB") {
            Some(v) => assert_eq!(&page.entries[2], v),
            None => assert!(false, "expected entry recordB but got not found")
        }
        match page.get("/path/to/recordA.1") {
            Some(v) => assert_eq!(&page.entries[3], v),
            None => assert!(false, "expected entry recordA.1 but got not found")
        }
    }

    #[test]
    fn get_mut() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        let mut expected = page.entries[1].clone();
        let entry = match page.get_mut("/path/to/recordA.0") {
            Some(v) => v,
            None => {
                assert!(false, "expected entry recordA.0 but got not found");
                return;
            }
        };
        assert_eq!(&mut expected, entry);
        entry.next_part = 99;
        assert_eq!(page.entries[1].next_part, 99);
        let mut expected = page.entries[2].clone();
        match page.get_mut("/path/to/recordB") {
            Some(v) => assert_eq!(&mut expected, v),
            None => assert!(false, "expected entry recordB but got not found")
        }
        let mut expected = page.entries[3].clone();
        match page.get_mut("/path/to/recordA.1") {
            Some(v) => assert_eq!(&mut expected, v),
            None => assert!(false, "expected entry recordA.1 but got not found")
        }
    }

    #[test]
    fn get_index() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        match page.get_index(0) {
            Some(v) => assert_eq!(&page.entries[1], v),
            None => assert!(false, "expected entry recordA.0 but got not found")
        }
        match page.get_index(1) {
            Some(v) => assert_eq!(&page.entries[2], v),
            None => assert!(false, "expected entry recordB but got not found")
        }
        match page.get_index(2) {
            Some(v) => assert_eq!(&page.entries[3], v),
            None => assert!(false, "expected entry recordA.1 but got not found")
        }
    }

    #[test]
    fn get_index_mut() {
        let mut data = Data::new(Cursor::new(Vec::new()), false);
        let mut table = test_helper::create_fake_table(&mut data, 1).unwrap();
        test_helper::add_records(&mut table, &mut data).unwrap();
        table.fill_records_into(&mut data, 8).unwrap();
        data.flush().unwrap();
        let mut page = match Page::load(&mut data) {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to load page: {}", e);
                return;
            }
        };
        let mut expected = page.entries[1].clone();
        let entry = match page.get_index_mut(0) {
            Some(v) => v,
            None => {
                assert!(false, "expected entry recordA.0 but got not found");
                return;
            }
        };
        assert_eq!(&mut expected, entry);
        entry.next_part = 99;
        assert_eq!(page.entries[1].next_part, 99);
        let mut expected = page.entries[2].clone();
        match page.get_index_mut(1) {
            Some(v) => assert_eq!(&mut expected, v),
            None => assert!(false, "expected entry recordB but got not found")
        }
        let mut expected = page.entries[3].clone();
        match page.get_index_mut(2) {
            Some(v) => assert_eq!(&mut expected, v),
            None => assert!(false, "expected entry recordA.1 but got not found")
        }
    }
}