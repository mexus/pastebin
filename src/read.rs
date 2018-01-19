//! Reading from stream helper.

use Error;
use std::io::Read;

/// Read a portion of data.
fn read_data_portion<R: Read>(stream: &mut R,
                              buffer: &mut Vec<u8>,
                              portion_size: usize,
                              limit: usize)
                              -> Result<bool, Error> {
    let mut portion = vec![0u8; portion_size];
    let size = stream.read(&mut portion)?;
    if size == 0 {
        return Ok(false);
    }
    if buffer.len() + size > limit {
        return Err(Error::TooBig);
    }
    portion.resize(size, 0u8);
    buffer.append(&mut portion);
    Ok(true)
}

/// Loads data from stream in portions of 512 bytes until an end of data or the limit is reached.
/// If a limit is reached Error::TooBig is returned.
pub fn load_data<R: Read>(stream: &mut R, limit: usize) -> Result<Vec<u8>, Error> {
    const PORTION_SIZE: usize = 1024;
    let mut result = Vec::with_capacity(limit);
    while read_data_portion(stream, &mut result, PORTION_SIZE, limit)? {}
    Ok(result)
}
