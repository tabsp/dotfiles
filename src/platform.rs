use std::fs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    pub platform: Platform,
    pub arch: Arch,
    pub distro: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Mac,
    Linux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arch {
    Arm64,
    X86_64,
}

impl Platform {
    pub fn key(self) -> &'static str {
        match self {
            Self::Mac => "mac",
            Self::Linux => "linux",
        }
    }
}

impl Arch {
    pub fn key(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64",
            Self::X86_64 => "x86_64",
        }
    }
}

pub fn detect_host() -> Result<Host, String> {
    let platform = match std::env::consts::OS {
        "macos" => Platform::Mac,
        "linux" => Platform::Linux,
        other => return Err(format!("unsupported operating system: {other}")),
    };

    let arch = match std::env::consts::ARCH {
        "aarch64" => Arch::Arm64,
        "x86_64" => Arch::X86_64,
        other => return Err(format!("unsupported architecture: {other}")),
    };

    let distro = if platform == Platform::Linux {
        Some(read_linux_distro().unwrap_or_else(|| "unknown".to_string()))
    } else {
        None
    };

    Ok(Host {
        platform,
        arch,
        distro,
    })
}

pub fn distro_supported(host: &Host) -> bool {
    match host.platform {
        Platform::Mac => true,
        Platform::Linux => matches!(host.distro.as_deref(), Some("ubuntu" | "debian")),
    }
}

fn read_linux_distro() -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            return Some(value.trim_matches('"').to_ascii_lowercase());
        }
    }
    None
}
