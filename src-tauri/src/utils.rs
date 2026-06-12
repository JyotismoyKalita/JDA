use std::collections::HashMap;

use reqwest::RequestBuilder;

const SKIPPED_REQUEST_HEADERS: &[&str] = &[
    "host",
    "connection",
    "content-length",
    "accept-encoding",
    "range",
];

pub fn apply_browser_headers(
    mut req: RequestBuilder,
    headers: &HashMap<String, String>,
) -> RequestBuilder {
    for (name, value) in headers {
        let lower_name = name.to_ascii_lowercase();
        if value.is_empty() || SKIPPED_REQUEST_HEADERS.contains(&lower_name.as_str()) {
            continue;
        }

        let Ok(header_name) = reqwest::header::HeaderName::from_bytes(lower_name.as_bytes()) else {
            continue;
        };
        let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) else {
            continue;
        };

        req = req.header(header_name, header_value);
    }

    req
}

pub fn parse_filename(header: &str) -> Option<String> {
    if let Some(name) = header.split(';').find_map(|p| p.trim().strip_prefix("filename*=")) {
        if let Some(encoded) = name.split("''").nth(1) {
            if let Ok(decoded) = urlencoding::decode(encoded) {
                return Some(decoded.into_owned());
            }
        }
    }
    if let Some(name) = header.split(';').find_map(|p| p.trim().strip_prefix("filename=")) {
        return Some(name.trim_matches('"').to_string());
    }
    None
}

pub fn extension_from_mime(mime: &str) -> &str {
    match mime {
        "application/pdf" => ".pdf",
        "application/zip" => ".zip",
        "application/x-rar-compressed" => ".rar",
        "application/json" => ".json",
        "text/html" => ".html",
        "text/plain" => ".txt",
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "video/mp4" => ".mp4",
        "audio/mpeg" => ".mp3",
        "application/octet-stream" => ".bin",
        "application/x-msdownload" => ".exe",
        _ => "",
    }
}
