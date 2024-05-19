use std::str::FromStr;

use crate::error::Error::FormDataBoundaryMissing;

pub(crate) fn parse_form_data(
    content_type: String,
    content: String,
    directory: &str,
) -> anyhow::Result<reqwest::blocking::multipart::Form> {
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

        // let (head, body) = part.split_once("\n\n").unwrap_or((part.as_str(), ""));

        let lines = part.lines().map(String::from).collect::<Vec<_>>();

        let mut is_body = false;
        let mut body_raw = vec![];
        let mut headers_raw = vec![];

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

                body_raw.push(line);
            } else {
                headers_raw.push(line);
            }
        }

        let (headers, name, filename) = extract_part_headers(headers_raw);
        if name.is_none() && headers.is_empty() {
            continue;
        }

        let part = extract_form_part(body_raw, directory)?
            .file_name(filename.unwrap_or_default())
            .headers(headers);

        form = form.part(name.unwrap_or_default(), part);
    }

    Ok(form)
}

fn extract_form_part(
    content: Vec<String>,
    directory: &str,
) -> anyhow::Result<reqwest::blocking::multipart::Part> {
    let first_char = if content.is_empty() {
        None
    } else {
        Some(content[0].chars().next().unwrap_or_default())
    };

    if first_char == Some('<') {
        let filename = content[0].trim_start_matches('<').trim();
        // todo could be nicer
        let filepath = format!("{}/{}", directory, filename);
        let reader = std::fs::File::open(filepath)?;
        Ok(reqwest::blocking::multipart::Part::reader(reader))
    } else if first_char.is_some() {
        Ok(reqwest::blocking::multipart::Part::bytes(
            content.join("\n").into_bytes(),
        ))
    } else {
        Ok(reqwest::blocking::multipart::Part::text(content.join("\n")))
    }
}

fn extract_part_headers(
    headers: Vec<String>,
) -> (reqwest::header::HeaderMap, Option<String>, Option<String>) {
    let mut name = None;
    let mut filename = None;
    let mut header_map = reqwest::header::HeaderMap::new();

    for line in headers {
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
            header_map.insert(
                reqwest::header::HeaderName::from_str(key.trim()).unwrap(),
                reqwest::header::HeaderValue::from_str(value.trim()).unwrap(),
            );
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
        let form = parse_form_data(
            " multipart/form-data; boundary=foo".to_string(),
            CONTENT.to_string(),
            "../testdata",
        );

        assert!(form.is_ok());
    }

    #[test]
    fn test_extract_body_part() {
        let body = r#"Content-Disposition: form-data; name="image"; filename="Cargo.lock"
Content-Type: application/octet-stream

< ../Cargo.lock"#;

        let body_content = body.lines().map(String::from).collect::<Vec<String>>();

        let maybe_part = extract_form_part(body_content, "..");

        assert!(maybe_part.is_ok());
    }
}
