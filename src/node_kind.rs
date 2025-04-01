#[repr(u64)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Directory = 0,
    File = 1,
}

impl NodeKind {
    pub fn to_le_bytes(self) -> [u8; 8] {
        (self as u64).to_le_bytes()
    }

    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        match u64::from_le_bytes(bytes) {
            0 => NodeKind::Directory,
            1 => NodeKind::File,
            _ => panic!("Invalid bytes for NodeKind"),
        }
    }
}
