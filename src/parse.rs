use std::str::FromStr;

use anyhow::Result;
use reqwest::blocking::RequestBuilder;

use crate::error::Error;
use crate::error::Error::{
    EmptyRequest, InvalidMethod, InvalidUrl, NoRequestLine, NotEnoughParts, RequestBody,
    SendRequest,
};
use crate::form::parse_form_data;

pub fn parse_request(
    content: &str,
    client: reqwest::blocking::Client,
    directory: &str,
) -> Result<reqwest::blocking::Request, Error> {
    if content.trim().is_empty() {
        return Err(EmptyRequest(content.to_string()));
    }

    let content = content
        .lines()
        .filter(|line| !line.starts_with("//") && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let (head, body) = content.split_once("\n\n").unwrap_or((content.as_str(), ""));
    let (first_line, headers) = head
        .split_once('\n')
        .ok_or(NoRequestLine(body.to_string()))?;

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

    let builder = client.request(method, url).version(map_version(version));

    let (content_type, mut builder) = extract_headers(headers.to_string(), builder)?;

    if let Some(content_type) = content_type {
        builder =
            attach_body(builder, content_type, body.to_string(), directory).map_err(RequestBody)?;
    }

    let request = builder.build().map_err(SendRequest)?;

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
        "application/json" => builder
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(content),
        // todo who knows if this works
        "application/x-www-form-urlencoded" => builder.form(content.as_str()),
        "multipart/form-data" => {
            let form = parse_form_data(content_type, content, directory)?;

            builder.multipart(form.try_into()?)
        }
        _ => builder.body(content),
    };

    Ok(builder)
}

fn extract_headers(
    headers_raw: String,
    mut builder: RequestBuilder,
) -> Result<(Option<String>, RequestBuilder), crate::error::Error> {
    let mut content_type = None;

    let lines = headers_raw.lines();

    for line in lines {
        if !line.contains(':') {
            return Err(Error::InvalidHeader(line.to_string()));
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
            } else if value.starts_with("Basic") {
                let value = value.trim_start_matches("Basic").trim();

                let creds = value.split_once(' ');
                if creds.is_none() {
                    return Err(Error::InvalidHeader(line.to_string()));
                }

                let (username, password) = creds.unwrap();
                builder = builder.basic_auth(username.trim(), Some(password.trim()));
            }

            continue;
        }
        builder = builder.header(key.trim(), value.trim());
    }

    Ok((content_type, builder))
}
