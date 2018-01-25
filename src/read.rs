//! Reading from stream helper.

use Error;
use std::io::Read;

/// Loads data from stream either in portions of 1024 bytes until an end of data or the limit is
/// reached or an exact amount of bytes if `data_length` is not `None`.
///
/// If a limit is reached Error::TooBig is returned.
pub fn load_data<R: Read>(stream: &mut R,
                          data_length: u64)
                          -> Result<Vec<u8>, Error> {
    let mut data = vec![0u8; data_length as usize];
    stream.read_exact(&mut data)?;
    Ok(data)
}
