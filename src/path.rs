use std::path::PathBuf;

pub fn expand_home(path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_home_preserves_absolute_path() {
        let result = expand_home("/usr/local/bin").unwrap();
        assert_eq!(result, PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn expand_home_replaces_tilde() {
        let result = expand_home("~/test").unwrap();
        let home = std::env::var_os("HOME").unwrap();
        assert_eq!(result, PathBuf::from(home).join("test"));
    }

    #[test]
    fn expand_home_preserves_dot() {
        let result = expand_home("./config").unwrap();
        assert_eq!(result, PathBuf::from("./config"));
    }

    #[test]
    fn expand_home_accepts_just_tilde_slash() {
        let result = expand_home("~/").unwrap();
        let home = std::env::var_os("HOME").unwrap();
        assert_eq!(result, PathBuf::from(home));
    }
}
