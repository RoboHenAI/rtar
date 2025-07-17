use dhfarm_engine::traits::DataTrait;
use dhfarm_engine::{Data, Segment};
use indexmap::IndexMap;
use tokio::sync::Mutex;
use std::default;
use std::fs::OsFile;
use std::io::{Read, Seek, SeekFrom, Write, Error as IoError};
use std::io::Result as IoResult;
use std::path::PathBuf;
use crate::engine::index::{Index, PAGE_SIZE};

const BLOCK_SIZE: u64 = 512;

struct Tar<T: Read + Write + Seek> {
    stream: Data<T>,
    index: Index,
    need_closing: bool,
    end_fake_id: usize
}

impl<'tar, T: Read + Write + Seek> Tar<T> {
    /// Creates a new tar instance.
    /// 
    /// # Arguments
    /// * `file`: The file to create the tar from.
    /// * `index`: The index to create the tar from.
    /// 
    /// # Returns
    /// * `Self`: The created tar instance.
    fn new(stream: T) -> Self {
        let index = Index::new();
        Self{
            stream: Data::new(stream, false),
            index,
            need_closing: false,
            end_fake_id: 0
        }
    }

    fn last_file(&'tar self) -> Option<&'tar SubFile> {
        let file = match self.files.get_index(self.end_fake_id) {
            Some((_, file)) => file,
            None => return None
        };
        Some(file)
    }

    /// Pads the stream with zeroes to the next block size.
    /// 
    /// # Arguments
    /// * `writer`: The writer to pad the stream with zeroes.
    /// * `len`: The length of the stream.
    /// 
    /// # Returns
    /// * `IoResult<()>`: The result of the padding operation.
    fn pad_zeroes(writer: &mut impl Write, len: u64) -> IoResult<()> {
        let buf = [0; BLOCK_SIZE as usize];
        let remaining = BLOCK_SIZE - (len % BLOCK_SIZE);
        if remaining < BLOCK_SIZE {
            writer.write_all(&buf[..remaining as usize])?;
        }
        Ok(())
    }

    /// Creates a new tar file.
    /// 
    /// # Arguments
    /// * `path`: The path to create the tar file at.
    /// 
    /// # Returns
    /// * `IoResult<Self>`: The result of the create operation.
    pub async fn create_new(path: PathBuf) -> IoResult<Self> {
        let file = match OsFile::create_new(path) {
            Ok(file) => file,
            Err(err) => Err(err)?
        };
        let index = Index::new();
        let mut myself = Self::new(file, index);

        // create index header file
        let stream = &mut myself.stream;
        let mut header = tar::Header::new_gnu();
        header.set_path(".0.rhindex")?;
        header.set_size(512);
        header.set_cksum();
        stream.write_all(header.as_bytes())?;

        // write index page
        let page = myself.index.add_page();
        page.write_all(stream)?;
        Self::pad_zeroes(stream, PAGE_SIZE as u64)?;
        drop(lock);
        Ok(myself)
    }

    /// Opens a tar file and loads the files.
    /// 
    /// # Arguments
    /// * `file`: The file to open the tar from.
    /// 
    /// # Returns
    /// * `IoResult<Self>`: The result of the open operation.
    pub async fn open(mut file: OsFile) -> IoResult<Self> {
        let index = Index::open(&mut file)?;
        let mut tar = Self::new(file, index);
        let lock = tar.mutex.lock().await;
        let stream = &mut tar.stream;

        for page in tar.index.pages.iter() {
            for entry in page.iter() {
                let entry = *entry;
                if entry == 1 {
                    break;
                }
            }
        }
        drop(lock);

        // TODO: Read all sub files
        Ok(tar)
    }

    // Flush any non flushed data into the tar.
    fn inner_flush(&mut self) -> IoResult<()> {
        if !self.need_flush {
            return Ok(());
        }
        self.stream.flush()?;
        self.need_flush = false;
        Ok(())
    }

    /// Write this tar's closing tag when needed.
    fn inner_close(&mut self) -> IoResult<()> {
        self.inner_flush()?;
        if !self.need_closing {
            return Ok(());
        }

        // look for the end of the file and write the tar end tag
        let pos = match self.last_file() {
            Some(file) => file.pos + file.entry.size,
            None => return Err(IoError::new(std::io::ErrorKind::NotFound, "last file index doesn't exists"))
        };
        self.stream.seek(SeekFrom::Start(pos))?;
        self.stream.write(&[0;1024])?;
        self.need_closing = false;
        Ok(())
    }

    /// Moves the stream position to the sub file position if different.
    pub(crate) async fn move_to(&mut self, file: &SubFile) -> IoResult<()> {
        let pos = self.stream.stream_position()?;
        if pos != file.pos {
            if self.need_flush {
                self.inner_flush();
            }
            self.stream.seek(SeekFrom::Start(file.pos))?;
        }
        Ok(())
    }

    pub(crate) async fn inner_read(&mut self, file: &mut SubFile, buf: &mut [u8]) -> IoResult<usize> {
        self.move_to(file).await?;
        let read = self.stream.read(buf)?;
        file.pos += read as u64;
        Ok(read)
    }


    pub(crate) async fn inner_write(&mut self, file: &mut SubFile, buf: &[u8]) -> IoResult<usize> {
        //self.ensure_index().await?;
        self.move_to(file).await?;
        let written = self.stream.write(buf)?;
        file.pos += written as u64;
        self.need_flush = true;
        Ok(written)
    }

    pub async fn flush(&'tar mut self) -> IoResult<()> {
        let _lock = self.mutex.get_mut();
        self.inner_flush();
        Ok(())
    }

    pub(crate) async fn auto_partition(&mut self, file: &mut SubFile, bytes_to_write: u64) -> IoResult<()> {
        // do nothing if the bytes to be written fits the file
        if file.pos + bytes_to_write < file.entry.size {
            return Ok(())
        }

        // partition when isn't the last file
        if self.end_fake_id != file.fake_id {
            // TODO: handle partitioning after header fixes

        }

        // handle file when last partition
        self.need_closing = true;
        Ok(())
    }
}

impl Drop for Tar {
    fn drop(&mut self) {
        self.inner_close().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tar() {
        let file = OsFile::create("test.tar").unwrap();
        let tar = Tar::new(file);
        assert!(tar.files.is_empty());
    }

    #[test]
    fn test_open_tar_standard() {}

    #[test]
    fn test_open_tar_good() {}

    #[test]
    fn test_open_tar_corrupted() {}

    #[test]
    fn test_ensure_index(){}

    #[test]
    fn test_auto_partition_fits() {
        
    }

    #[test]
    fn test_auto_partition_append() {}

    #[test]
    fn test_auto_partition_partition() {}
}