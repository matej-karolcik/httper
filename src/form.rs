use std::collections::HashMap;

use crate::error::Error::{FormDataBoundaryMissing, FormPartNameMissing};
use crate::model;

pub(crate) fn parse_form_data(
    content_type: String,
    content: String,
    directory: &str,
) -> anyhow::Result<model::Form> {
    let boundary = content_type
        .split(';')
        .map(|s| s.trim())
        .find(|part| part.starts_with("boundary="))
        .map(|part| part.trim_start_matches("boundary="))
        .ok_or(FormDataBoundaryMissing(content_type.clone()))?;

    let mut form = model::Form::new();
    let delimiter = format!("--{}", boundary);

    let content_parts = content
        .split(delimiter.as_str())
        .map(String::from)
        .collect::<Vec<String>>();

    for part in content_parts {
        if part.trim().is_empty() || part == "\n" || part.starts_with("--") {
            continue;
        }

        let (name, part) = extract_form_part(part, directory)?;

        form.part(name, part);
    }

    Ok(form)
}

fn extract_form_part(part: String, directory: &str) -> anyhow::Result<(String, model::Part)> {
    let (head, body) = part.split_once("\n\n").unwrap_or((part.as_str(), ""));

    let head = head.trim().to_string();
    let body = body.trim().to_string();

    let (headers, name, filename) = extract_part_headers(head);

    if name.is_none() {
        return Err(FormPartNameMissing.into());
    }

    let first_char = if body.is_empty() {
        None
    } else {
        Some(body.chars().next().unwrap_or_default())
    };

    let mut part = if first_char == Some('<') {
        let filename = body.trim_start_matches('<').trim();
        // todo could be nicer
        let filepath = format!("{}/{}", directory, filename);
        let reader = std::fs::File::open(filepath)?;
        model::Part::reader(reader)
    } else if first_char.is_some() {
        model::Part::bytes(body.into_bytes())
    } else {
        model::Part::text(body)
    };

    if let Some(filename) = filename {
        part = part.file_name(filename);
    }

    part = part.headers(headers);

    Ok((name.unwrap(), part))
}

fn extract_part_headers(
    headers_raw: String,
) -> (HashMap<String, String>, Option<String>, Option<String>) {
    let mut name = None;
    let mut filename = None;
    let mut header_map = HashMap::new();
    let headers = headers_raw.lines();

    for line in headers {
        let canonized = line.to_lowercase();
        let canonized = canonized.trim();

        if canonized.starts_with("content-type") {
            continue;
        }

        if canonized.starts_with("content-disposition") {
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
            header_map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    (header_map, name, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &str = r#"--foo
    Content-Disposition: form-data; name="image"; filename="Cargo.lock"
    Content-Type: application/octet-stream

    < ../Cargo.lock
    --foo
    content-Disposition: form-data; name="title"
    Content-Type: text/plain

    test text

    foobar
    --foo--"#;

    #[test]
    fn test_extract_form_body() {
        parse_form_data(
            " multipart/form-data; boundary=foo".to_string(),
            CONTENT.to_string(),
            "testdata",
        )
        .unwrap();
    }

    #[test]
    fn test_extract_body_part() {
        let body = r#"Content-Disposition: form-data; name="image"; filename="Cargo.lock"
Content-Type: application/octet-stream

< ../Cargo.lock"#
            .to_string();

        let (name, part) = extract_form_part(body, "testdata").unwrap();

        assert_eq!(name, "image");

        assert!(part.headers.is_some());

        let headers = part.headers.unwrap();

        assert_eq!(headers.len(), 1);
        assert!(part.reader.is_some());
    }
}
