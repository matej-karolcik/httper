use std::str::FromStr;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use clap::ArgAction;
use reqwest::blocking::RequestBuilder;

use crate::error::Error;
use crate::error::Error::{
    EmptyRequest, FormDataBoundaryMissing, InvalidHeader, InvalidMethod, InvalidUrl, NoRequestLine,
    NotEnoughParts, RequestBodyError, RequestError, ResponseBodyError,
};

mod error;

fn main() -> Result<()> {
    let cmd = clap::Command::new("httper")
        .arg(
            clap::Arg::new("file")
                .help("File containing the HTTP request")
                .required(true),
        )
        .arg(
            clap::Arg::new("verbose")
                .action(ArgAction::SetTrue)
                .short('v')
                .long("verbose")
                .help("Print verbose output"),
        )
        .arg(
            clap::Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file for the response"),
        );

    let matches = cmd.get_matches();
    let filepath = matches.get_one::<String>("file").unwrap();
    let output = matches.get_one::<String>("output");
    let verbose = matches.get_flag("verbose");

    let content = std::fs::read_to_string(filepath)?;

    let client = reqwest::blocking::ClientBuilder::new()
        .connection_verbose(true)
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()?;

    let request = parse(content.as_str(), client.clone())?;

    if verbose {
        println!("\n{:?}", request);
        println!("{}", "-".repeat(80));
    }

    let start = std::time::Instant::now();
    let response = client.execute(request).map_err(RequestError)?;

    let duration = start.elapsed();

    let headers = response.headers().clone();
    let status_code = response.status();
    let content_length = response.content_length();
    let bytes = response.bytes().map_err(ResponseBodyError)?;

    let content_type = headers
        .iter()
        .filter_map(|(k, v)| {
            if k != reqwest::header::CONTENT_TYPE {
                return None;
            }

            let header_value = v.to_str().unwrap_or_default();
            if [
                mime::APPLICATION_OCTET_STREAM.as_ref(),
                mime::TEXT_PLAIN_UTF_8.as_ref(),
                mime::TEXT_PLAIN.as_ref(),
            ]
            .contains(&header_value)
            {
                return None;
            }

            mime::Mime::from_str(header_value).ok()
        })
        .collect::<Vec<_>>();

    // todo consider disposition header here maybe?

    if let Some(content_type) = content_type.first() {
        let extensions = mime_guess::get_mime_extensions(content_type);

        if extensions.is_some() {
            let extension = extensions.unwrap().first().unwrap();

            if verbose {
                println!("Content type: {:?}", content_type);
                println!("Extension: {:?}", extension);
            }

            let filename = if let Some(output) = output {
                output.to_string()
            } else {
                format!(
                    "response-{}.{}",
                    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                    extension
                )
            };

            if let Err(e) = std::fs::write(filename, bytes.clone()) {
                eprintln!("Failed to write response to file: {}", e);
            }
        }
    }

    let content_length = content_length.unwrap_or(bytes.len() as u64);

    if verbose {
        println!("Headers: {:?}", headers);
        if !bytes.is_empty() {
            println!("Content: {:?}", String::from_utf8_lossy(&bytes));
        }
    }

    println!(
        "\nResponse code: {}; Time: {}ms ({:?}); Content length: {} bytes ({:.2} MB)",
        status_code,
        duration.as_millis(),
        duration,
        content_length,
        content_length as f64 / 1_000_000.0,
    );

    Ok(())
}

fn parse(
    content: &str,
    client: reqwest::blocking::Client,
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
            continue;
        }

        if is_body {
            body.push_str(line);
            continue;
        }

        if !line.contains(':') {
            return Err(InvalidHeader(line.to_string()));
        }

        let (key, value) = line.split_once(':').unwrap();

        if key.to_lowercase().trim() == reqwest::header::CONTENT_TYPE {
            content_type = Some(value.trim().to_string());
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
        builder = attach_body(builder, content_type, body).map_err(RequestBodyError)?;
    }

    let request = builder.build().map_err(RequestError)?;

    Ok(request)
}

fn attach_body(
    builder: RequestBuilder,
    content_type: String,
    content: String,
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
            let boundary = content_type
                .split(';')
                .find(|part| part.trim().starts_with("boundary="))
                .map(|part| part.trim_start_matches("boundary="))
                .ok_or(FormDataBoundaryMissing(content_type.clone()))?;

            let mut form = reqwest::blocking::multipart::Form::new();
            let delimiter = format!("--{}", boundary);

            let mut part_headers = reqwest::header::HeaderMap::new();
            let content_parts = content
                .split(delimiter.as_str())
                .map(String::from)
                .collect::<Vec<String>>();

            for part in content_parts {
                let lines = part.lines().map(String::from).collect::<Vec<_>>();

                let mut part_name = None;
                let mut part_filename = None;
                let mut is_body = false;
                let mut body = vec![];
                let mut file = None;

                // todo split this into collect_headers and collect_body
                for line in lines {
                    if line.is_empty() && !is_body {
                        is_body = true;
                        continue;
                    }

                    if is_body {
                        if line.starts_with('<') {
                            let filename = line.trim_start_matches('<').trim();
                            let reader = std::fs::File::open(filename)?;
                            file = Some(reader);
                            break;
                        }

                        body.extend(line.as_bytes());
                        continue;
                    }

                    if line
                        .to_lowercase()
                        .trim()
                        .starts_with("content-disposition")
                    {
                        let disposition = line.split_once(':').unwrap().1;
                        let parts = disposition.split(';').collect::<Vec<&str>>();

                        parts.iter().for_each(|part| {
                            let (key, value) = part.split_once('=').unwrap();
                            let key = key.trim();
                            let value = value.trim().trim_matches('"');

                            if key == "name" {
                                part_name = Some(value);
                            } else if key == "filename" {
                                part_filename = Some(value);
                            }
                        });
                    }

                    if line.contains(':') {
                        let (key, value) = line.split_once(':').unwrap();
                        part_headers.insert(key.trim(), value.trim().parse().unwrap());
                    }
                }

                if let Some(filename) = part_filename {
                    let part = if let Some(filebody) = file {
                        reqwest::blocking::multipart::Part::reader(filebody).file_name(filename)
                    } else {
                        reqwest::blocking::multipart::Part::bytes(body).file_name(filename)
                    };
                    form = form.part(part_name.unwrap_or_default(), part);
                } else if let Some(name) = part_name {
                    let part = reqwest::blocking::multipart::Part::bytes(body).file_name(name);
                    form = form.part(part_name.unwrap_or_default(), part);
                }
            }

            builder.multipart(form)
        }
        _ => builder.body(content),
    };
    Ok(builder)
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
