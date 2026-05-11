use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use std::time::Duration;

pub struct DownloadedFile {
    pub bytes: Vec<u8>,
}

pub fn download_https(url: &str, allow_redirects: bool) -> Result<DownloadedFile, String> {
    validate_manifest_https(url)?;
    let client = Client::builder()
        .redirect(if allow_redirects {
            Policy::limited(10)
        } else {
            Policy::none()
        })
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|err| format!("failed to build http client: {err}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("download failed: {err}"))?;
    validate_final_https(response.url().as_str())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("download returned {status}"));
    }
    let bytes = response
        .bytes()
        .map_err(|err| format!("failed to read response body: {err}"))?;
    Ok(DownloadedFile { bytes: bytes.to_vec() })
}

pub fn validate_manifest_https(url: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        return Ok(());
    }
    Err(format!("url must use https: {url}"))
}

pub fn validate_final_https(url: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        return Ok(());
    }
    Err(format!("final url must use https: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_manifest_https_accepts_https() {
        assert!(validate_manifest_https("https://example.com/file").is_ok());
    }

    #[test]
    fn validate_manifest_https_rejects_non_https() {
        let err = validate_manifest_https("http://example.com/file").expect_err("must fail");
        assert!(err.contains("must use https"));
    }

    #[test]
    fn validate_final_https_rejects_non_https() {
        let err = validate_final_https("ftp://example.com/file").expect_err("must fail");
        assert!(err.contains("final url must use https"));
    }
}
