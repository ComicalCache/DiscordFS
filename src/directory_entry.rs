const NAME_LEN_SIZE: usize = std::mem::size_of::<NameLen>();
const DIRECTORY_ENTRY_SIZE: usize = NAME_LEN + BLOCK_INDEX_SIZE + NAME_LEN_SIZE;

pub const BLOCK_INDEX_SIZE: usize = std::mem::size_of::<BlockIndex>();
pub const NAME_LEN: usize = (1 << 10) - BLOCK_INDEX_SIZE - NAME_LEN_SIZE;

pub type BlockIndex = u64;
type NameLen = u64;

pub struct DirectoryEntry {
    // max (2^10 - 8 - 8 =) 1008 byte names
    name_len: u64,
    name: String,

    // data block
    block: BlockIndex,
}

impl DirectoryEntry {
    pub fn new<S: AsRef<str>>(name: S, block: BlockIndex) -> Self {
        let name = name.as_ref();
        DirectoryEntry {
            name_len: name.len() as u64,
            name: name.to_string(),
            block,
        }
    }

    pub fn block_id(&self) -> BlockIndex {
        self.block
    }

    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        let name = name.as_ref().to_string();
        assert!(
            name.len() <= NAME_LEN,
            "Name exceeds directory entry name size of {NAME_LEN}: `{}`",
            name.len()
        );

        self.name = name;
        self.name_len = self.name.len() as u64;
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }
}

impl DirectoryEntry {
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let bytes = self
            .name_len
            .to_le_bytes()
            .iter()
            .chain(self.name.as_bytes())
            .chain(&self.block.to_le_bytes())
            .copied()
            .collect::<Vec<u8>>();

        assert!(
            bytes.len() <= DIRECTORY_ENTRY_SIZE,
            "Converting DirectoryEntry to bytes has unexpected size: `{}`",
            bytes.len()
        );

        bytes
    }

    pub fn from_le_bytes(bytes: &[u8]) -> Vec<Self> {
        let mut entries = Vec::new();

        let mut bytes = bytes.iter();
        while bytes.len() > 0 {
            let mut name_len = [0; NAME_LEN_SIZE];
            for name_len_byte in name_len.iter_mut().take(NAME_LEN_SIZE) {
                *name_len_byte = *bytes
                    .next()
                    .expect("Malformed input doesn't contain full name size");
            }

            let name_len = u64::from_le_bytes(name_len);
            assert!(
                name_len <= NAME_LEN as u64,
                "Name length exceeds maximum directory entry name length"
            );
            let mut name = String::with_capacity(name_len as usize);
            for _ in 0..name_len {
                name.push(
                    *bytes
                        .next()
                        .expect("Malformed input doesn't contain full name")
                        as char,
                );
            }
            assert!(
                name_len == name.len() as u64,
                "Corrupted directory entry has mismatched name length and stored name length"
            );

            let mut block = [0; BLOCK_INDEX_SIZE];
            for block_byte in block.iter_mut().take(BLOCK_INDEX_SIZE) {
                *block_byte = *bytes
                    .next()
                    .expect("Malformed input doesn't contain full block id");
            }
            let block = u64::from_le_bytes(block);

            entries.push(DirectoryEntry {
                name_len,
                name,
                block,
            });
        }

        entries
    }
}
