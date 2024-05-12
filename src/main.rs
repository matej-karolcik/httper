use std::str::FromStr;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use clap::ArgAction;
use mime_guess::mime;

use crate::error::Error;
use crate::error::Error::{
    EmptyRequest, InvalidHeader, InvalidMethod, InvalidUrl, NoRequestLine, NotEnoughParts,
    RequestError, ResponseBodyError,
};

mod error;

fn main() -> Result<()> {
    // todo attack mode ???
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
        println!("{:?}", request);
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
        // todo combine these
        .map(|(k, v)| (k, v.to_str().unwrap_or_default()))
        .filter(|(k, v)| {
            k.to_string() == reqwest::header::CONTENT_TYPE.to_string()
                && *v != "application/octet-stream"
        })
        .filter_map(|(_, v)| mime::Mime::from_str(v).ok())
        .collect::<Vec<_>>();

    // todo consider disposition maybe?

    if let Some(content_type) = content_type.first() {
        let extensions = mime_guess::get_mime_extensions(content_type);

        if extensions.is_some() {
            let extension = extensions.unwrap().first().unwrap();

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
    }

    println!(
        "Response code: {}; Time: {}ms ({:?}); Content length: {} bytes ({:.2} MB)",
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

        // todo authorization
        let (key, value) = line.split_once(':').unwrap();

        builder = builder.header(key.trim(), value.trim());
    }

    let request = builder.body(body).build().map_err(RequestError)?;

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
