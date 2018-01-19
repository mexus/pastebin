//! Request helpers.

use iron::{self, Request};
use std::borrow::Cow;

/// Convenience functions for a `Request`.
pub trait RequestExt {
    /// Checks if a request has been made from a known browser as opposed to a command line client
    /// (like wget or curl).
    fn is_browser(&self) -> bool;

    /// Tries to obtain an `n`-th segment of the URI.
    fn url_segment_n(&self, n: usize) -> Option<&str>;

    /// Extracts value of an argument (a URI part after `?`).
    fn get_arg(&self, arg: &str) -> Option<Cow<str>>;
}

impl<'a, 'b> RequestExt for Request<'a, 'b> {
    fn is_browser(&self) -> bool {
        lazy_static! {
            static ref BROWSERS: Vec<&'static str> =
                vec!["Gecko/", "AppleWebKit/", "Opera/", "Trident/", "Chrome/"];
        }
        self.headers.get::<iron::headers::UserAgent>()
            .map(|agent| {
                     debug!("User agent: [{}]", agent);
                     BROWSERS.iter().any(|browser| agent.contains(browser))
                 })
            .unwrap_or(false)
    }

    fn url_segment_n(&self, n: usize) -> Option<&str> {
        self.url.as_ref()
            .path_segments()
            .and_then(|mut it| it.nth(n))
            .and_then(|s| {
                          if s.is_empty() {
                              None
                          } else {
                              Some(s)
                          }
                      })
    }

    fn get_arg(&self, arg: &str) -> Option<Cow<str>> {
        self.url.as_ref()
            .query_pairs()
            .find(|&(ref name, _)| name == arg)
            .map(|(_, value)| value)
    }
}
