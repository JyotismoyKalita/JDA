pub fn range_for(total: u64, parts: usize, i: usize) -> (u64, u64) {
    if total == 0 {
        return (0, 0);
    }

    let chunk = total / parts as u64;
    let start = i as u64 * chunk;
    let end = if i == parts - 1 {
        total - 1
    } else {
        start + chunk - 1
    };

    (start, end)
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
