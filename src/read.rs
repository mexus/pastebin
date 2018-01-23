//! Reading from stream helper.

use Error;
use std::io::Read;

/// Loads data from stream in portions of 1024 bytes until an end of data or the limit is reached.
/// If a limit is reached Error::TooBig is returned.
fn load_data_portions<R: Read>(stream: &mut R, limit: usize) -> Result<Vec<u8>, Error> {
    const PORTION_SIZE: usize = 1024;
    let mut buf = vec![0u8; limit];
    let mut effective_size = 0usize;
    while effective_size + PORTION_SIZE < limit {
        let read_bytes = stream.read(&mut buf[effective_size..(effective_size + PORTION_SIZE)])?;
        if read_bytes == 0 {
            // EOF is reached.
            buf.truncate(effective_size);
            return Ok(buf);
        }
        effective_size += read_bytes;
    }
    Err(Error::TooBig)
}

/// Loads data from stream either in portions of 1024 bytes until an end of data or the limit is
/// reached or an exact amount of bytes if `data_length` is not `None`.
///
/// If a limit is reached Error::TooBig is returned.
pub fn load_data<R: Read>(stream: &mut R,
                          limit: usize,
                          data_length: Option<u64>)
                          -> Result<Vec<u8>, Error> {
    match data_length {
        None => load_data_portions(stream, limit),
        Some(len) if (len as usize) > limit => Err(Error::TooBig),
        Some(len) => {
            let mut data = vec![0u8; len as usize];
            stream.read_exact(&mut data)?;
            Ok(data)
        }
    }
}
