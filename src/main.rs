use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use mime_guess::mime;

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("Usage: {} <path to .http file>", args[0]);
        std::process::exit(1);
    }

    let filepath = &args[1];

    let content = std::fs::read_to_string(filepath)?;

    let client = reqwest::Client::new();
    let request = parse(content.as_str(), client)?;

    let start = std::time::Instant::now();
    let response = reqwest::Client::new()
        .execute(request)
        .await
        .context("Failed to send request")?;

    let content_type = response
        .headers()
        .iter()
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

            let filename = format!(
                "response-{}.{}",
                Utc::now().to_rfc3339_opts(SecondsFormat::Secs, false),
                extension
            );
            std::fs::write(&filename, response.bytes().await?.as_ref())?;
        }
    }

    // todo
    // Response code: 200 (OK); Time: 3148ms (3 s 148 ms); Content length: 5183823 bytes (5,18 MB)

    println!("done in {:?}", start.elapsed());

    Ok(())
}

fn parse(content: &str, client: reqwest::Client) -> Result<reqwest::Request> {
    let mut lines = content.lines();

    if lines.clone().count() < 1 {
        // todo error enum
        return Err(anyhow!("Empty file"));
    }

    let first_line = lines.next().unwrap();
    let parts = first_line
        .split(' ')
        .map(String::from)
        .collect::<Vec<String>>();

    if parts.len() < 2 {
        return Err(anyhow!("Invalid request line"));
    }

    let method = reqwest::Method::from_str(&parts[0]).context("Invalid method")?;
    let url = parts[1].parse::<reqwest::Url>().context("Invalid url")?;
    // todo this is ugly
    let version = if let Some(v) = parts.get(2) {
        v
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
            return Err(anyhow!(format!("Invalid header: {}", line)));
        }

        let (key, value) = line.split_once(':').unwrap();

        builder = builder.header(key.trim(), value.trim());
    }

    let request = builder
        .body(body)
        .build()
        .context("Failed to build request")?;

    Ok(request)
}

fn map_version(v: &str) -> reqwest::Version {
    match v.to_uppercase().trim() {
        "HTTP/0.9" => reqwest::Version::HTTP_09,
        "HTTP/1.0" => reqwest::Version::HTTP_10,
        "HTTP/2.0" => reqwest::Version::HTTP_2,
        "HTTP/3.0" => reqwest::Version::HTTP_3,
        _ => reqwest::Version::HTTP_11,
    }
}
