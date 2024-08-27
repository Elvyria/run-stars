use std::{fs::File, io::BufRead, sync::OnceLock};

pub fn home() -> &'static str {
    static HOME: OnceLock<String> = OnceLock::new();

    HOME.get_or_init(|| {
        if let Ok(s) = std::env::var("HOME") {
            return s
        }

        let uid = Some(rustix::process::geteuid().as_raw().to_string());
        let passwd = File::open("/etc/passwd").expect("/etc/passwd should be accessable if $HOME is not set");

        for l in std::io::BufReader::new(passwd).lines() {
            let l = l.expect("reading /etc/passwd should not cause issues");
            let mut parts = l.split(':');

            if uid.as_deref() == parts.nth(2) {
                return parts.nth(2).map(str::to_owned).expect("/etc/passwd should not be malformed");
            }
        }

        unreachable!()
    }).as_ref()
}

pub fn runtime() -> &'static str {
    static RUNTIME: OnceLock<String> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        if let Ok(s) = std::env::var("XDG_RUNTIME_HOME") {
            return s
        }

        let uid = rustix::process::geteuid();

        if uid.is_root() {
            return "/run".to_string()
        }

        format!("/run/user/{}", uid.as_raw())
    })
}

pub fn state() -> &'static str {
    static STATE: OnceLock<String> = OnceLock::new();

    STATE.get_or_init(|| {
        if let Ok(s) = std::env::var("XDG_STATE_HOME") {
            return s
        }

        let uid = rustix::process::geteuid();

        if uid.is_root() {
            return "/var/lib".to_string();
        }

        format!("{}{}", home(), ".local/state")
    }).as_ref()
}
