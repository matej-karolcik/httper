use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use reqwest::blocking::RequestBuilder;

use crate::error::Error;
use crate::error::Error::{
    EmptyRequest, FormDataBoundaryMissing, InvalidHeader, InvalidMethod, InvalidUrl, NoRequestLine,
    NotEnoughParts, RequestBodyError, RequestError,
};

pub fn parse_request(
    content: &str,
    client: reqwest::blocking::Client,
    directory: &str,
) -> Result<reqwest::blocking::Request, Error> {
    let mut lines = content.lines();

    if lines.clone().count() < 1 {
        return Err(EmptyRequest);
    }

    let first_line = lines
        .find(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .ok_or(NoRequestLine)?;

    let parts = first_line
        .split(' ')
        .map(String::from)
        .collect::<Vec<String>>();

    if parts.len() < 2 {
        return Err(NotEnoughParts(first_line.to_string()));
    }

    let method =
        reqwest::Method::from_str(&parts[0]).map_err(|_| InvalidMethod(parts[0].to_string()))?;

    let url = parts[1]
        .parse::<reqwest::Url>()
        .map_err(|e| InvalidUrl(parts[1].to_string(), e))?;

    let version = if let Some(v) = parts.get(2) {
        v
    } else if url.scheme() == "https" {
        "HTTP/2.0"
    } else {
        "HTTP/1.1"
    };

    let mut builder = client.request(method, url).version(map_version(version));

    let mut is_body = false;
    let mut body = String::new();
    let mut content_type = None;

    for line in lines {
        if line.is_empty() {
            is_body = true;
        }

        if is_body {
            body.push_str(line);
            body.push('\n');
            continue;
        }

        if !line.contains(':') {
            return Err(InvalidHeader(line.to_string()));
        }

        let (key, value) = line.split_once(':').unwrap();

        if key.to_lowercase().trim() == reqwest::header::CONTENT_TYPE {
            content_type = Some(value.trim().to_string());
            continue;
        }

        if key.to_lowercase().trim() == reqwest::header::AUTHORIZATION {
            let value = value.trim();
            if value.starts_with("Bearer") {
                builder = builder.bearer_auth(value.trim_start_matches("Bearer").trim());
                continue;
            } else if value.starts_with("Basic") {
                let value = value.trim_start_matches("Basic").trim();
                let (username, password) = value
                    .split_once(' ')
                    .ok_or(InvalidHeader(value.to_string()))?;
                builder = builder.basic_auth(username, Some(password));
                continue;
            }
        }

        builder = builder.header(key.trim(), value.trim());
    }

    if let Some(content_type) = content_type {
        builder = attach_body(builder, content_type, body, directory).map_err(RequestBodyError)?;
    }

    let request = builder.build().map_err(RequestError)?;

    Ok(request)
}

fn map_version(v: &str) -> reqwest::Version {
    match v.to_uppercase().trim() {
        "HTTP/0.9" => reqwest::Version::HTTP_09,
        "HTTP/1.0" => reqwest::Version::HTTP_10,
        "HTTP/2.0" | "HTTP/2" => reqwest::Version::HTTP_2,
        "HTTP/3.0" | "HTTP/3" => reqwest::Version::HTTP_3,
        _ => reqwest::Version::HTTP_11,
    }
}

fn attach_body(
    builder: RequestBuilder,
    content_type: String,
    content: String,
    directory: &str,
) -> Result<RequestBuilder> {
    let trimmed = content_type
        .split_once(';')
        .unwrap_or((content_type.as_str(), ""))
        .0
        .trim();

    let builder = match trimmed {
        "application/json" => builder.json(content.as_str()),
        // todo who knows if this works
        "application/x-www-form-urlencoded" => builder.form(content.as_str()),
        "multipart/form-data" => {
            let form = parse_form_data(content_type, content, directory)?;

            builder.multipart(form)
        }
        _ => builder.body(content),
    };

    Ok(builder)
}

fn parse_form_data(
    content_type: String,
    content: String,
    directory: &str,
) -> Result<reqwest::blocking::multipart::Form> {
    let boundary = content_type
        .split(';')
        .map(|s| s.trim())
        .find(|part| part.starts_with("boundary="))
        .map(|part| part.trim_start_matches("boundary="))
        .ok_or(FormDataBoundaryMissing(content_type.clone()))?;

    let mut form = reqwest::blocking::multipart::Form::new();
    let delimiter = format!("--{}", boundary);

    let content_parts = content
        .split(delimiter.as_str())
        .map(String::from)
        .collect::<Vec<String>>();

    for part in content_parts {
        if part.trim().is_empty() || part == "\n" || part.starts_with("--") {
            continue;
        }

        let lines = part.lines().map(String::from).collect::<Vec<_>>();

        let mut is_body = false;
        let mut body_raw = vec![];
        let mut headers_raw = String::new();

        for line in lines {
            if line.is_empty() {
                if headers_raw.is_empty() {
                    continue;
                } else if body_raw.is_empty() {
                    is_body = true;
                    continue;
                }
            }

            if is_body {
                if line.is_empty() && body_raw.is_empty() {
                    continue;
                }

                body_raw.extend(line.as_bytes());
                body_raw.push(b'\n');
            } else {
                // todo vector of strings
                headers_raw.push_str(line.as_str());
                headers_raw.push('\n');
            }
        }

        let (headers, name, filename) = extract_form_headers(&headers_raw);
        if name.is_none() && headers.is_empty() {
            continue;
        }

        let part = extract_form_body(body_raw, directory)?
            .file_name(filename.unwrap_or_default())
            .headers(headers);

        form = form.part(name.unwrap_or_default(), part);
    }

    Ok(form)
}

fn extract_form_headers(
    headers: &String,
) -> (reqwest::header::HeaderMap, Option<String>, Option<String>) {
    let lines = headers.lines();

    let mut name = None;
    let mut filename = None;
    let mut headers_raw = HashMap::new();

    for line in lines {
        if line
            .to_lowercase()
            .trim()
            .starts_with("content-disposition")
        {
            let disposition = line.split_once(':').unwrap().1;
            let parts = disposition.split(';').collect::<Vec<&str>>();

            parts.iter().for_each(|part| {
                if !part.contains('=') {
                    return;
                }

                let (key, value) = part.split_once('=').unwrap();
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                if key == "name" {
                    name = Some(value.to_string());
                } else if key == "filename" {
                    filename = Some(value.to_string());
                }
            });
        }

        if line.contains(':') {
            let (key, value) = line.split_once(':').unwrap();
            headers_raw.insert(key.trim(), value.trim());
        }
    }

    let mut headers = reqwest::header::HeaderMap::new();

    for (key, value) in headers_raw {
        headers.insert(
            reqwest::header::HeaderName::from_str(key).unwrap(),
            reqwest::header::HeaderValue::from_str(value).unwrap(),
        );
    }

    (headers, name, filename)
}

fn extract_form_body(
    content: Vec<u8>,
    directory: &str,
) -> Result<reqwest::blocking::multipart::Part> {
    let first_char = if content.is_empty() {
        None
    } else {
        Some(content[0] as char)
    };

    if first_char == Some('<') {
        let filename = String::from_utf8(content)?;
        let filename = filename.trim_start_matches('<').trim();
        let filepath = format!("{}/{}", directory, filename);
        let reader = std::fs::File::open(filepath)?;
        Ok(reqwest::blocking::multipart::Part::reader(reader))
    } else if first_char.is_some() {
        Ok(reqwest::blocking::multipart::Part::bytes(content))
    } else {
        Ok(reqwest::blocking::multipart::Part::text(""))
    }
}
