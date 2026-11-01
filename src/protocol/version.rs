#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinecraftVersionInfo {
    pub name: &'static str,
    pub protocol: i32,
}

pub const DEFAULT_TARGET_VERSION: &str = "1.21.11";
pub const DEFAULT_TARGET_PROTOCOL: i32 = 767;

pub const SUPPORTED_VERSIONS: &[MinecraftVersionInfo] = &[
    MinecraftVersionInfo { name: "1.21.11", protocol: 767 },
    MinecraftVersionInfo { name: "1.21.4", protocol: 769 },
    MinecraftVersionInfo { name: "1.21.3", protocol: 768 },
    MinecraftVersionInfo { name: "1.21.2", protocol: 768 },
    MinecraftVersionInfo { name: "1.21.1", protocol: 767 },
    MinecraftVersionInfo { name: "1.21", protocol: 767 },
    MinecraftVersionInfo { name: "1.20.6", protocol: 766 },
    MinecraftVersionInfo { name: "1.20.5", protocol: 766 },
    MinecraftVersionInfo { name: "1.20.4", protocol: 765 },
    MinecraftVersionInfo { name: "1.20.3", protocol: 765 },
    MinecraftVersionInfo { name: "1.20.2", protocol: 764 },
    MinecraftVersionInfo { name: "1.20.1", protocol: 763 },
    MinecraftVersionInfo { name: "1.20", protocol: 763 },
    MinecraftVersionInfo { name: "1.19.4", protocol: 762 },
    MinecraftVersionInfo { name: "1.19.2", protocol: 760 },
    MinecraftVersionInfo { name: "1.18.2", protocol: 758 },
    MinecraftVersionInfo { name: "1.16.5", protocol: 754 },
    MinecraftVersionInfo { name: "1.12.2", protocol: 340 },
    MinecraftVersionInfo { name: "1.8.9", protocol: 47 },
];

pub fn find_version_by_protocol(protocol: i32) -> Option<&'static str> {
    for ver in SUPPORTED_VERSIONS {
        if ver.protocol == protocol {
            return Some(ver.name);
        }
    }
    None
}

pub fn find_protocol_by_name(name: &str) -> Option<i32> {
    for ver in SUPPORTED_VERSIONS {
        if ver.name.eq_ignore_ascii_case(name) {
            return Some(ver.protocol);
        }
    }
    None
}

pub fn resolve_version_for_client(
    client_protocol: i32,
    target_version_str: &str,
    multi_version_enabled: bool,
) -> (String, i32) {
    if multi_version_enabled {
        if let Some(name) = find_version_by_protocol(client_protocol) {
            // Client is using a known supported version -> match their protocol so they see green ping!
            return (format!("CrabMC {}", name), client_protocol);
        }
    }

    // Default target
    let protocol = find_protocol_by_name(target_version_str).unwrap_or(DEFAULT_TARGET_PROTOCOL);
    (format!("CrabMC {}", target_version_str), protocol)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_resolution() {
        // Multi-version enabled with 1.21.11 target
        let (name0, proto0) = resolve_version_for_client(767, "1.21.11", true);
        assert_eq!(name0, "CrabMC 1.21.11");
        assert_eq!(proto0, 767);

        // Client 1.20.4 (765)
        let (name2, proto2) = resolve_version_for_client(765, "1.21.11", true);
        assert_eq!(name2, "CrabMC 1.20.4");
        assert_eq!(proto2, 765);

        // Multi-version disabled
        let (name3, proto3) = resolve_version_for_client(765, "1.21.11", false);
        assert_eq!(name3, "CrabMC 1.21.11");
        assert_eq!(proto3, 767);
    }
}
