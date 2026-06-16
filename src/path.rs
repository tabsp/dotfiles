use std::path::PathBuf;

pub fn expand_home(path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}
