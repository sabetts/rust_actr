//! ACT-R dispatcher address discovery.
//!
//! The Lisp dispatcher and `actr server` write the host and port they listen on
//! to `~/act-r-address.txt` and `~/act-r-port-num.txt`
//! (`framework/dispatcher.lisp:1414`). The Tcl Environment and Python client
//! read those files before connecting, so the native Environment does the same
//! by default.

use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_PORT: u16 = 2650;
pub(crate) const ADDRESS_FILE: &str = "act-r-address.txt";
pub(crate) const PORT_FILE: &str = "act-r-port-num.txt";

/// The dispatcher address read from the ACT-R home files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionAddress {
    pub host: String,
    pub port: u16,
}

impl ConnectionAddress {
    pub(crate) fn label(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Resolve the address from the current user's home directory.
pub(crate) fn discover() -> ConnectionAddress {
    discover_from(&home_dir())
}

/// Resolve the address from `home`, using ACT-R's defaults for missing or
/// unreadable files. Whitespace is trimmed because other clients read these
/// files with line-oriented parsers.
pub(crate) fn discover_from(home: &Path) -> ConnectionAddress {
    let host = read_trimmed(&home.join(ADDRESS_FILE))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_HOST.to_string());
    let port = read_trimmed(&home.join(PORT_FILE))
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|p| *p != 0)
        .unwrap_or(DEFAULT_PORT);
    ConnectionAddress { host, port }
}

/// The two files consulted by [`discover`], for diagnostics.
pub(crate) fn source_files() -> (PathBuf, PathBuf) {
    let home = home_dir();
    (home.join(ADDRESS_FILE), home.join(PORT_FILE))
}

/// Match the home directory used by `rpc::server::write_port_file` and the
/// language clients (`expanduser("~")`).
fn home_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path) = nonempty_var("USERPROFILE") {
            return PathBuf::from(path);
        }
        if let (Some(drive), Some(path)) = (nonempty_var("HOMEDRIVE"), nonempty_var("HOMEPATH")) {
            let mut home = std::ffi::OsString::from(drive);
            home.push(path);
            return PathBuf::from(home);
        }
    }
    nonempty_var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn nonempty_var(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|v| !v.is_empty())
}

fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_home(tag: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("actr-env-connection-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).expect("create temp home");
        home
    }

    fn write(home: &Path, name: &str, text: &str) {
        fs::write(home.join(name), text).expect("write temp file");
    }

    #[test]
    fn reads_host_and_port_from_home_files() {
        let home = test_home("both");
        write(&home, ADDRESS_FILE, " 10.0.0.11\r\n");
        write(&home, PORT_FILE, " 2651\n");
        assert_eq!(
            discover_from(&home),
            ConnectionAddress {
                host: "10.0.0.11".into(),
                port: 2651,
            }
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn missing_files_use_act_r_defaults() {
        let home = test_home("missing");
        assert_eq!(
            discover_from(&home),
            ConnectionAddress {
                host: DEFAULT_HOST.into(),
                port: DEFAULT_PORT,
            }
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn malformed_values_fall_back_independently() {
        let home = test_home("malformed");
        write(&home, ADDRESS_FILE, " \r\n");
        write(&home, PORT_FILE, "not-a-port");
        assert_eq!(
            discover_from(&home),
            ConnectionAddress {
                host: DEFAULT_HOST.into(),
                port: DEFAULT_PORT,
            }
        );

        write(&home, ADDRESS_FILE, "localhost\n");
        write(&home, PORT_FILE, "0\n");
        assert_eq!(
            discover_from(&home),
            ConnectionAddress {
                host: "localhost".into(),
                port: DEFAULT_PORT,
            }
        );
        let _ = fs::remove_dir_all(home);
    }
}
