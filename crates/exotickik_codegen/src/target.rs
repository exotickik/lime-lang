#[derive(Clone, Debug)]
pub struct Target {
    pub name: &'static str,
    pub triple: &'static str,
    pub linker: &'static str,
    pub linker_is_cc: bool,
    pub exe_suffix: &'static str,
}

impl Target {
    pub fn host() -> Target {
        if cfg!(target_os = "windows") {
            Target::windows_x64()
        } else {
            Target::linux_x64()
        }
    }

    pub fn linux_x64() -> Target {
        Target {
            name: "linux-x64",
            triple: "x86_64-unknown-linux-gnu",
            linker: "cc",
            linker_is_cc: true,
            exe_suffix: "",
        }
    }

    pub fn windows_x64() -> Target {
        Target {
            name: "windows-x64",
            triple: "x86_64-pc-windows-gnu",
            linker: "lld-link",
            linker_is_cc: false,
            exe_suffix: ".exe",
        }
    }

    pub fn by_name(name: &str) -> Option<Target> {
        match name {
            "linux-x64" => Some(Target::linux_x64()),
            "windows-x64" => Some(Target::windows_x64()),
            "host" => Some(Target::host()),
            _ => None,
        }
    }
}
