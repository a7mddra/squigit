use anyhow::{Context, Result};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::{fs, io::Write, path::PathBuf};

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn entry_path() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    let path = dirs::config_dir()
        .context("Could not find the startup directory")?
        .join("autostart/squigit.desktop");
    #[cfg(target_os = "macos")]
    let path = dirs::home_dir()
        .context("Could not find the startup directory")?
        .join("Library/LaunchAgents/io.github.a7mddra.squigit.plist");
    Ok(path)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn enabled() -> Result<bool> {
    let contents = match fs::read_to_string(entry_path()?) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("Could not read login startup settings"),
    };
    #[cfg(target_os = "linux")]
    return Ok(contents.lines().any(|line| line.starts_with("Exec="))
        && !contents.lines().any(|line| {
            matches!(
                line.trim(),
                "Hidden=true" | "X-GNOME-Autostart-enabled=false"
            )
        }));
    #[cfg(target_os = "macos")]
    Ok(!contents.is_empty())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn set_enabled(enabled: bool) -> Result<()> {
    let path = entry_path()?;
    if !enabled {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("Could not disable login startup"),
        };
    }
    let executable = std::env::current_exe()?.canonicalize()?;
    let executable = executable
        .to_str()
        .context("The application path is not valid Unicode")?;
    #[cfg(target_os = "linux")]
    let contents = {
        anyhow::ensure!(
            !executable.contains(['\n', '\r']),
            "The application path contains a line break"
        );
        let executable = executable
            .replace('\\', "\\\\\\\\")
            .replace('"', "\\\\\"")
            .replace('`', "\\\\`")
            .replace('$', "\\\\$")
            .replace('%', "%%");
        format!("[Desktop Entry]\nType=Application\nName=Squigit\nExec=\"{executable}\"\nIcon=squigit\nTerminal=false\nX-GNOME-Autostart-enabled=true\n")
    };
    #[cfg(target_os = "macos")]
    let contents = {
        let executable = executable
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>io.github.a7mddra.squigit</string><key>ProgramArguments</key><array><string>{executable}</string></array><key>RunAtLoad</key><true/><key>LimitLoadToSessionType</key><string>Aqua</string></dict></plist>\n")
    };
    let directory = path
        .parent()
        .context("Could not find the startup directory")?;
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(contents.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|error| error.error)
        .context("Could not enable login startup")?;
    Ok(())
}

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
pub fn enabled() -> Result<bool> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_KEY) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("Could not read login startup settings"),
    };
    match key.get_value::<String, _>("Squigit") {
        Ok(value) => Ok(!value.is_empty()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("Could not read login startup settings"),
    }
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if enabled {
        let executable = std::env::current_exe()?;
        let command = format!("\"{}\"", executable.display());
        anyhow::ensure!(
            command.encode_utf16().count() <= 260,
            "The application path is too long for login startup"
        );
        key.set_value("Squigit", &command)
            .context("Could not enable login startup")?;
    } else if let Err(error) = key.delete_value("Squigit") {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error).context("Could not disable login startup");
        }
    }
    Ok(())
}
