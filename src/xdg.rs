use std::{fs::File, io::BufRead};

pub fn home() -> String {
    if let Ok(s) = std::env::var("HOME") {
        return s
    }

    let uid = Some(rustix::process::geteuid().as_raw().to_string());
    let passwd = File::open("/etc/passwd").unwrap();

    for l in std::io::BufReader::new(passwd).lines() {
        let l = l.unwrap();
        let mut parts = l.split(':');

        if uid.as_deref() == parts.nth(2) {
            return parts.nth(2).map(str::to_owned).unwrap();
        }
    }

    unreachable!()
}

pub fn runtime() -> String {
    if let Ok(s) = std::env::var("XDG_RUNTIME_HOME") {
        return s
    }

    let uid = rustix::process::geteuid();

    if uid.is_root() {
        return "/run".to_string()
    }

    format!("/run/user/{}", uid.as_raw())
}

pub fn state() -> String {
    if let Ok(s) = std::env::var("XDG_STATE_HOME") {
        return s
    }

    let uid = rustix::process::geteuid();

    if uid.is_root() {
        return "/var/lib".to_string();
    }

    format!("{}{}", home(), ".local/state")
}
