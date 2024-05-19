use std::collections::HashMap;
use std::fs::File;
use std::ops::ControlFlow;
use std::str::FromStr;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use thiserror::Error;

#[derive(Debug)]
pub struct Form {
    parts: HashMap<String, Part>,
}

#[derive(Debug, Default)]
pub struct Part {
    pub headers: Option<HashMap<String, String>>,
    pub filename: Option<String>,
    pub reader: Option<File>,
    pub bytes: Option<Vec<u8>>,
    pub text: Option<String>,
}

impl Part {
    pub fn text(value: String) -> Self {
        Self {
            text: Some(value),
            ..Self::default()
        }
    }

    pub fn bytes(value: Vec<u8>) -> Self {
        Self {
            bytes: Some(value),
            ..Self::default()
        }
    }

    pub fn reader(r: File) -> Self {
        Self {
            reader: Some(r),
            ..Self::default()
        }
    }

    pub fn file_name(self, value: String) -> Self {
        Self {
            filename: Some(value),
            ..self
        }
    }

    pub fn headers(self, value: HashMap<String, String>) -> Self {
        Self {
            headers: Some(value),
            ..self
        }
    }
}

impl TryInto<reqwest::blocking::multipart::Part> for Part {
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

impl Form {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
        }
    }

    pub fn part(&mut self, name: String, part: Part) {
        self.parts.insert(name, part);
    }
}

impl TryInto<reqwest::blocking::multipart::Form> for Form {
    type Error = Error;

    fn try_into(self) -> Result<reqwest::blocking::multipart::Form, Self::Error> {
        let mut form = reqwest::blocking::multipart::Form::new();
        for (name, part) in self.parts {
            form = form.part(name, part.try_into()?);
        }

        Ok(form)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("part has no body")]
    EmptyBody,
}
