use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

pub fn unique_file_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    let extension = path.extension().and_then(|s| s.to_str());

    for i in 1.. {
        let file_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{}({}).{}", stem, i, ext),
            _ => format!("{}({})", stem, i),
        };
        let candidate = parent.join(file_name);

        if !candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

pub fn unique_file_name_in_dir(dir: &Path, desired_name: &str, reserved_names: &[String]) -> String {
    let desired_path = dir.join(desired_name);
    if !desired_path.exists()
        && !dir.join(format!("{}.jdm", desired_name)).exists()
        && !reserved_names.iter().any(|name| name == desired_name)
    {
        return desired_name.to_string();
    }

    let path = Path::new(desired_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    let extension = path.extension().and_then(|s| s.to_str());

    for i in 1.. {
        let file_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{}({}).{}", stem, i, ext),
            _ => format!("{}({})", stem, i),
        };

        if !dir.join(&file_name).exists()
            && !dir.join(format!("{}.jdm", file_name)).exists()
            && !reserved_names.iter().any(|name| name == &file_name)
        {
            return file_name;
        }
    }

    desired_name.to_string()
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
