use crate::config::DepsManifest;
use crate::installers;
use crate::platform::Host;

pub fn install_missing(deps: &DepsManifest, host: &Host) -> Result<(), String> {
    for (name, dep) in &deps.deps {
        let entries = dep.entries_for(host.platform.key(), host.arch.key());
        let Some(entry) = entries.first() else {
            continue;
        };

        println!("==> dependency {name}");
        installers::install_missing(&dep.command, entry)?;
    }

    Ok(())
}
