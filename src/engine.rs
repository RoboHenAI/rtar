pub mod header;
pub mod index;
pub mod tar;

use std::io::{Read, Write};
use std::io::Result as IoResult;

/// Default to 4k bytes
pub const DEFAULT_BUFFER_SIZE: usize = 4096;

/// Read a u8 value from the reader.
pub fn read_u8(reader: &mut impl Read) -> IoResult<u8> {
    let mut buf = [0u8;  (u8::BITS / 8) as usize];
    reader.read_exact(&mut buf)?;
    Ok(u8::from_be_bytes(buf))
}

/// Write a u8 value from the writer.
pub fn write_u8(writer: &mut impl Write, value: u8) -> IoResult<()> {
    let buf = value.to_be_bytes();
    writer.write_all(&buf)?;
    Ok(())
}

/// Read a u64 value from the reader.
pub fn read_u64(reader: &mut impl Read) -> IoResult<u64> {
    let mut buf = [0u8;  (u64::BITS / 8) as usize];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

/// Write a u64 value from the writer.
pub fn write_u64(writer: &mut impl Write, value: u64) -> IoResult<()> {
    let buf = value.to_be_bytes();
    writer.write_all(&buf)?;
    Ok(())
}