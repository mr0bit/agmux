//! Release discovery, checksum verification, and binary replacement for the CLI.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;

use crate::remote_bootstrapper::RemoteBootstrapper;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn target() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => bail!("unsupported architecture: {other}"),
    };
    let os = if cfg!(all(target_os = "linux", target_env = "gnu")) {
        "unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_env = "musl")) {
        "unknown-linux-musl"
    } else if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(all(target_os = "windows", target_env = "msvc")) {
        "pc-windows-msvc"
    } else {
        bail!("unsupported operating system or target environment");
    };
    Ok(format!("{arch}-{os}"))
}

fn asset<'a>(release: &'a Release, name: &str) -> Result<&'a str> {
    release
        .assets
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.browser_download_url.as_str())
        .with_context(|| format!("release does not contain {name}"))
}

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>> {
    Ok(client
        .get(url)
        .send()?
        .error_for_status()?
        .bytes()?
        .to_vec())
}

/// Check for a release and, unless `check` is set, install its verified binaries.
///
/// Returns an error if discovery, verification, extraction, or replacement fails.
pub fn update(check: bool, force: bool, version: Option<&str>) -> Result<()> {
    let base = std::env::var("AGMUX_UPDATE_URL")
        .unwrap_or_else(|_| "https://api.github.com/repos/mr0bit/agmux/releases/latest".into());
    let url = if let Some(requested) = version {
        let v =
            Version::parse(requested.trim_start_matches('v')).context("invalid target version")?;
        let releases = base
            .strip_suffix("/latest")
            .unwrap_or(base.trim_end_matches('/'));
        format!("{releases}/tags/v{v}")
    } else {
        base
    };
    let client = reqwest::blocking::Client::builder()
        .user_agent("agmux-updater")
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let release: Release =
        serde_json::from_slice(&get(&client, &url).context("failed to fetch release metadata")?)?;
    let latest =
        Version::parse(release.tag_name.trim_start_matches('v')).context("invalid release tag")?;
    if let Some(requested) = version {
        if latest != Version::parse(requested.trim_start_matches('v'))? {
            bail!("release tag does not match requested version");
        }
    }
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let available = latest > current;
    println!(
        "Current: v{current}, Latest: v{latest} - {}",
        if available {
            "Update available!"
        } else {
            "Up to date."
        }
    );
    if check || (!available && !force) {
        return Ok(());
    }
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    let name = format!("agmux-v{latest}-{}.{extension}", target()?);
    let checksum_url =
        asset(&release, "SHA256SUMS").or_else(|_| asset(&release, "SHA256SUMS.txt"))?;
    let sums =
        String::from_utf8(get(&client, checksum_url)?).context("checksum manifest is not UTF-8")?;
    let mut matches = sums.lines().filter_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then_some(hash)
    });
    let expected = matches.next().context("archive checksum missing")?;
    if matches.next().is_some()
        || expected.len() != 64
        || !expected.bytes().all(|c| c.is_ascii_hexdigit())
    {
        bail!("invalid or duplicate archive checksum");
    }
    let temp = tempfile::tempdir()?;
    let archive_path = temp.path().join(&name);
    let bytes = get(&client, asset(&release, &name)?).context("failed to download archive")?;
    fs::write(&archive_path, &bytes)?;
    if !RemoteBootstrapper::sha256_hex(&bytes).eq_ignore_ascii_case(expected) {
        bail!("SHA-256 checksum mismatch for {name}");
    }
    let binaries = extract(&archive_path)?;
    let executable = std::env::current_exe().context("cannot locate current executable")?;
    let bin_dir = executable
        .parent()
        .context("executable has no parent directory")?;
    let daemon_name = if cfg!(windows) {
        "agmuxd.exe"
    } else {
        "agmuxd"
    };
    let sibling = bin_dir.join(daemon_name);
    let daemon = if sibling.exists() {
        sibling
    } else {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .context("cannot locate agmuxd or home directory")?;
        PathBuf::from(home).join(".local/bin").join(daemon_name)
    };
    install(&[(executable, binaries.0), (daemon, binaries.1)])?;
    println!("Successfully updated agmux and agmuxd to v{latest}.");
    Ok(())
}

fn extract(path: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut cli = None;
    let mut daemon = None;
    let mut take = |name: &Path, reader: &mut dyn Read| -> Result<()> {
        let basename = name.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let slot = match basename {
            "agmux" if !cfg!(windows) => &mut cli,
            "agmuxd" if !cfg!(windows) => &mut daemon,
            "agmux.exe" if cfg!(windows) => &mut cli,
            "agmuxd.exe" if cfg!(windows) => &mut daemon,
            _ => return Ok(()),
        };
        if slot.is_some() {
            bail!("duplicate binary in archive: {basename}");
        }
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        *slot = Some(data);
        Ok(())
    };
    if cfg!(windows) {
        let mut archive = zip::ZipArchive::new(File::open(path)?)?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            if !entry.is_file() {
                continue;
            }
            let name = entry.enclosed_name().context("invalid zip path")?;
            take(&name, &mut entry)?;
        }
    } else {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(File::open(path)?));
        for entry in archive.entries()? {
            let mut entry = entry?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let name = entry.path()?.into_owned();
            take(&name, &mut entry)?;
        }
    }
    Ok((
        cli.context("archive missing agmux")?,
        daemon.context("archive missing agmuxd")?,
    ))
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn install(binaries: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    let suffix = format!(".tmp.{}", std::process::id());
    let mut staged = Vec::new();
    let result = (|| -> Result<()> {
        for (dest, contents) in binaries {
            if !dest.is_file() {
                bail!("installed binary not found: {}", dest.display());
            }
            let tmp = suffixed(dest, &suffix);
            let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            staged.push(tmp);
            file.write_all(contents)?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o755))?;
            }
        }
        #[cfg(not(windows))]
        for (dest, _) in binaries {
            fs::copy(dest, suffixed(dest, ".old"))?;
        }
        for (index, ((dest, _), tmp)) in binaries.iter().zip(&staged).enumerate() {
            #[allow(unused_mut)]
            let mut touched = false;
            let replacement = (|| -> io::Result<()> {
                #[cfg(windows)]
                {
                    let backup = suffixed(dest, ".old");
                    if backup.exists() {
                        fs::remove_file(&backup)?;
                    }
                    fs::rename(dest, backup)?;
                    touched = true;
                }
                fs::rename(tmp, dest)?;
                Ok(())
            })();
            if let Err(error) = replacement {
                let mut failures = Vec::new();
                for (old_dest, _) in binaries.iter().take(index + usize::from(touched)) {
                    if let Err(e) = restore(old_dest) {
                        failures.push(format!("{}: {e}", old_dest.display()));
                    }
                }
                if !failures.is_empty() {
                    bail!(
                        "replacement failed: {error}; rollback failed: {}",
                        failures.join(", ")
                    );
                }
                return Err(error).context("replacement failed; binaries restored");
            }
        }
        Ok(())
    })();
    for path in staged {
        if let Err(e) = fs::remove_file(&path) {
            if e.kind() != io::ErrorKind::NotFound {
                eprintln!("failed to remove {}: {e}", path.display());
            }
        }
    }
    result
}

fn restore(dest: &Path) -> io::Result<()> {
    let backup = suffixed(dest, ".old");
    #[cfg(windows)]
    if dest.exists() {
        fs::remove_file(dest)?;
    }
    fs::rename(backup, dest)
}
