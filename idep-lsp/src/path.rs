use lsp_types::Url;
use std::path::PathBuf;
use url::ParseError;

/// Convert an idep-internal file URL to a WSL-friendly URL for LSP server consumption.
///
/// - `file:///C:/Users/foo` -> `file:///mnt/c/Users/foo`
/// - `file:///mnt/c/Users/foo` stays the same
/// - non-file URLs are returned unchanged
pub fn to_server_uri(uri: &Url) -> Url {
    if uri.scheme() != "file" {
        return uri.clone();
    }

    let path = uri.path();

    if let Some((drive, rest)) = windows_drive_prefix(path) {
        let server_path = format!("/mnt/{}/{}", drive.to_ascii_lowercase(), rest);
        return pathbuf_to_file_url(PathBuf::from(server_path)).unwrap_or_else(|_| uri.clone());
    }

    // Already a /mnt/<drive>/... path — normalize drive to lowercase for consistency
    if let Some((drive, rest)) = mnt_drive_prefix(path) {
        let server_path = format!("/mnt/{}/{}", drive.to_ascii_lowercase(), rest);
        return pathbuf_to_file_url(PathBuf::from(server_path)).unwrap_or_else(|_| uri.clone());
    }

    uri.clone()
}

/// Convert an LSP server file URL back to idep-internal form (Windows-style if from /mnt/<drive>).
///
/// - `file:///mnt/c/Users/foo` -> `file:///C:/Users/foo`
/// - `file:///home/user/project` stays the same
/// - non-file URLs are returned unchanged
pub fn from_server_uri(uri: &Url) -> Url {
    if uri.scheme() != "file" {
        return uri.clone();
    }

    let path = uri.path();

    if let Some((drive, rest)) = mnt_drive_prefix(path) {
        let rebuilt = format!("file:///{}:/{}", drive.to_ascii_uppercase(), rest);
        return Url::parse(&rebuilt).unwrap_or_else(|_| uri.clone());
    }

    uri.clone()
}

fn windows_drive_prefix(path: &str) -> Option<(char, &str)> {
    // Matches /C:/foo or /c:/foo
    let bytes = path.as_bytes();
    if bytes.len() >= 4 && bytes[0] == b'/' && bytes[2] == b':' && bytes[3] == b'/' {
        let drive = bytes[1] as char;
        if drive.is_ascii_alphabetic() {
            let rest = &path[4..];
            return Some((drive, rest));
        }
    }
    None
}

fn mnt_drive_prefix(path: &str) -> Option<(char, &str)> {
    // Matches /mnt/<drive>/<rest>
    let prefix = "/mnt/";
    if path.starts_with(prefix) && path.len() > prefix.len() + 2 {
        let drive_char = path.as_bytes()[prefix.len()] as char;
        if drive_char.is_ascii_alphabetic() && path.as_bytes()[prefix.len() + 1] == b'/' {
            let rest = &path[prefix.len() + 2..];
            return Some((drive_char, rest));
        }
    }
    None
}

fn pathbuf_to_file_url(path: PathBuf) -> Result<Url, ParseError> {
    Url::from_file_path(path).map_err(|_| ParseError::IdnaError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_uri_to_server_and_back_roundtrip() {
        let uri = Url::parse("file:///C:/Users/alice/project/main.rs").unwrap();
        let server = to_server_uri(&uri);
        assert_eq!(server.as_str(), "file:///mnt/c/Users/alice/project/main.rs");

        let back = from_server_uri(&server);
        assert_eq!(back.as_str(), uri.as_str());
    }

    #[test]
    fn linux_uri_passthrough() {
        let uri = Url::parse("file:///home/alice/project/main.rs").unwrap();
        let server = to_server_uri(&uri);
        assert_eq!(server, uri);

        let back = from_server_uri(&server);
        assert_eq!(back, uri);
    }

    #[test]
    fn mnt_drive_to_windows_roundtrip() {
        let server = Url::parse("file:///mnt/d/dev/code.rs").unwrap();
        let back = from_server_uri(&server);
        assert_eq!(back.as_str(), "file:///D:/dev/code.rs");

        let server_again = to_server_uri(&back);
        assert_eq!(server_again.as_str(), "file:///mnt/d/dev/code.rs");
    }
}
