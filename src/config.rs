//! Which parts show in the menu bar; persisted as key=value lines under Application Support.

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub icon: bool,
    pub cpu: bool,
    pub mem: bool,
    pub disk: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            icon: true,
            cpu: true,
            mem: true,
            disk: true,
        }
    }
}

/// Keys as written to disk, in the order the toggles appear.
pub const KEYS: [&str; 4] = ["icon", "cpu", "mem", "disk"];

impl Config {
    pub fn get(&self, key: &str) -> bool {
        match key {
            "icon" => self.icon,
            "cpu" => self.cpu,
            "mem" => self.mem,
            "disk" => self.disk,
            _ => false,
        }
    }

    pub fn set(&mut self, key: &str, on: bool) {
        match key {
            "icon" => self.icon = on,
            "cpu" => self.cpu = on,
            "mem" => self.mem = on,
            "disk" => self.disk = on,
            _ => {}
        }
    }

    /// Never let the status item go blank: with everything off, CPU % comes back.
    pub fn effective(mut self) -> Self {
        if !(self.icon || self.cpu || self.mem || self.disk) {
            self.cpu = true;
        }
        self
    }

    /// Config file wins; otherwise MATRIXSTAT_TITLE (cpu,mem,disk|none) seeds the text parts.
    pub fn load() -> Self {
        if let Some(cfg) = path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|s| Self::parse(&s))
        {
            return cfg;
        }
        let mut cfg = Self::default();
        if let Ok(mode) = std::env::var("MATRIXSTAT_TITLE") {
            cfg.cpu = false;
            cfg.mem = false;
            cfg.disk = false;
            for key in mode.split(',') {
                cfg.set(key.trim(), true);
            }
        }
        cfg
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(p, self.serialize());
    }

    fn parse(s: &str) -> Self {
        let mut cfg = Self::default();
        for line in s.lines() {
            if let Some((k, v)) = line.split_once('=') {
                cfg.set(k.trim(), v.trim() == "true");
            }
        }
        cfg
    }

    fn serialize(&self) -> String {
        KEYS.iter()
            .map(|k| format!("{k}={}\n", self.get(k)))
            .collect()
    }
}

fn path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/matrixstat/config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_effective() {
        let mut c = Config::default();
        c.set("mem", false);
        c.set("disk", false);
        let back = Config::parse(&c.serialize());
        assert_eq!(back, c);
        let all_off = Config {
            icon: false,
            cpu: false,
            mem: false,
            disk: false,
        };
        assert!(all_off.effective().cpu);
    }
}
