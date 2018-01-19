//! A helper module for MIME and ContentType related stuff.

use iron::headers::ContentType;
use mime_guess;
use std::path::Path;
use tree_magic;

/// Checks whether a given mime type represents some text.
pub fn is_text(mime_type: &str) -> bool {
    match mime_type {
        "application/x-sh" => true,
        s if s.starts_with("text/") => true,
        _ => false,
    }
}

/// Converts a given mime type into a content type.
pub fn to_content_type(mime_type: String) -> ContentType {
    match mime_type.parse() {
        Ok(mime) => ContentType(mime),
        Err(()) => ContentType::plaintext(),
    }
}

/// Guesses mime type of a file.
fn mime_from_file_name<P: AsRef<Path>>(name: P) -> Option<&'static str> {
    name.as_ref().extension()
        .and_then(|s| s.to_str())
        .and_then(mime_guess::get_mime_type_str)
}

/// Guesses a file's content type.
pub fn file_content_type<P: AsRef<Path>>(p: P) -> ContentType {
    let mime_type =
        mime_from_file_name(&p).map(Into::into)
                               .unwrap_or_else(|| tree_magic::from_filepath(p.as_ref()));
    to_content_type(mime_type)
}

/// Guesses a content type of given data and its file name (if any).
pub fn data_mime_type<P: AsRef<Path>>(file_name: Option<P>, data: &[u8]) -> String {
    file_name.as_ref()
             .and_then(mime_from_file_name)
             .map(Into::into)
             .unwrap_or_else(|| tree_magic::from_u8(data))
}
