use crate::config::DepsManifest;
use crate::installers;
use crate::platform::{Host, Platform};

pub fn install_missing(deps: &DepsManifest, host: &Host) -> Result<(), String> {
    for (name, dep) in &deps.deps {
        let raw_entries = dep.entries_for(host.platform.key(), host.arch.key());
        let entries: Vec<_> = raw_entries
            .iter()
            .copied()
            .filter(|entry| entry.matches_distro(host))
            .collect();
        let Some(entry) = entries.first() else {
            let detail = if host.platform == Platform::Linux && !raw_entries.is_empty() {
                format!(
                    " for distro {}",
                    host.distro.as_deref().unwrap_or("unknown")
                )
            } else {
                String::new()
            };
            return Err(format!(
                "dependency {name} has no current-host entry{detail}"
            ));
        };

        println!("==> dependency {name}");
        installers::install_missing(&dep.command, entry, host)?;
    }

    Ok(())
}
