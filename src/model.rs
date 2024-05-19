use std::collections::HashMap;
use std::io::Read;
use std::ops::ControlFlow;
use std::str::FromStr;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use thiserror::Error;

#[derive(Clone, Debug)]
pub(crate) struct Form<R>
where
    R: Read + Send + Clone + 'static,
{
    parts: HashMap<String, Part<R>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Part<R>
where
    R: Read + Send + Clone + 'static,
{
    headers: Option<HashMap<String, String>>,
    filename: Option<String>,
    reader: Option<R>,
    bytes: Option<Vec<u8>>,
    text: Option<String>,
}

impl<R> Part<R>
where
    R: Read + Send + Clone + 'static,
{
    fn text(value: String) -> Self {
        Self {
            text: Some(value),
            ..Self::default()
        }
    }

    fn bytes(value: Vec<u8>) -> Self {
        Self {
            bytes: Some(value),
            ..Self::default()
        }
    }

    fn reader(r: R) -> Self {
        Self {
            reader: Some(r),
            ..Self::default()
        }
    }

    fn with_filename(self, value: String) -> Self {
        let mut new = self.clone();
        new.filename = Some(value);
        new
    }

    fn with_headers(self, value: HashMap<String, String>) -> Self {
        let mut new = self.clone();
        new.headers = Some(value);
        new
    }
}

impl<R> Default for Part<R>
where
    R: Read + Send + Clone + 'static,
{
    fn default() -> Self {
        Self {
            headers: None,
            filename: None,
            reader: None,
            bytes: None,
            text: None,
        }
    }
}

impl<R> TryInto<reqwest::blocking::multipart::Part> for Part<R>
where
    R: Read + Send + Clone + 'static,
{
    type Error = Error;

    fn try_into(self) -> Result<reqwest::blocking::multipart::Part, Self::Error> {
        let mut part = if let Some(bytes) = self.bytes {
            Ok(reqwest::blocking::multipart::Part::bytes(bytes))
        } else if let Some(text) = self.text {
            Ok(reqwest::blocking::multipart::Part::text(text))
        } else if let Some(reader) = self.reader {
            Ok(reqwest::blocking::multipart::Part::reader(reader))
        } else {
            Err(Error::EmptyBody)
        }?;

        if let Some(filename) = self.filename {
            part = part.file_name(filename);
        }

        if let Some(headers) = self.headers {
            let mut header_map = HeaderMap::new();
            let _ = headers
                .iter()
                .filter_map(|(k, v)| {
                    let key = HeaderName::from_str(k.as_str()).ok()?;
                    let value = HeaderValue::from_str(v.as_str()).ok()?;

                    Some((key, value))
                })
                .try_for_each(|(k, v)| {
                    let value = header_map.insert(k, v);

                    if value.is_none() {
                        return ControlFlow::Break(value);
                    }

                    ControlFlow::Continue(())
                });

            part = part.headers(header_map);
        }

        Ok(part)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("part has no body")]
    EmptyBody,
}
