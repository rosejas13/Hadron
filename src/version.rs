// Hadron — Goldleaf's USB client (native Rust port of Quark)
// Ported from the Java Quark by XorTroll (GPL-3.0-or-later).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
}

impl Version {
    pub const CURRENT: Version = Version { major: 1, minor: 1, micro: 0 };

    pub fn new(major: u8, minor: u8, micro: u8) -> Self {
        Version { major, minor, micro }
    }

    pub fn older_than(self, other: Version) -> bool {
        if self.major != other.major {
            return self.major < other.major;
        }
        if self.minor != other.minor {
            return self.minor < other.minor;
        }
        self.micro < other.micro
    }

    pub fn same(self, other: Version) -> bool {
        self == other
    }

    pub fn newer_than(self, other: Version) -> bool {
        !(self.older_than(other) || self == other)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.micro)
    }
}