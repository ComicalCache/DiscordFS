use indicatif::{HumanBytes, HumanCount};

use crate::{
    directory_entry::{BLOCK_INDEX_SIZE, BlockIndex, DirectoryEntry, NAME_LEN},
    node_kind::NodeKind::{self, Directory, File},
};

const SIZE_SIZE: usize = std::mem::size_of::<Size>();
const KIND_SIZE: usize = std::mem::size_of::<NodeKind>();

const BLOCK_COUNT: usize =
    (BLOCK_SIZE - KIND_SIZE - SIZE_SIZE - BLOCK_INDEX_SIZE) / BLOCK_INDEX_SIZE;

pub const MAX_FILE_SIZE: usize = BLOCK_SIZE * BLOCK_COUNT;
pub const ENTRY_COUNT: usize =
    (BLOCK_SIZE - KIND_SIZE - SIZE_SIZE - BLOCK_INDEX_SIZE) / (NAME_LEN + BLOCK_INDEX_SIZE);
pub const BLOCK_SIZE: usize = 1 << 23;

pub type Size = u64;

pub struct Node {
    // if it's a file or directory
    pub kind: NodeKind,

    // if file, file size in bytes, if directory, directory entry count
    size: Size,

    // parent directory, if 0 => root node
    pub parent_block_id: BlockIndex,

    // single level block indices
    // => a file can be 8796067856384B ≈ 8.8TB in size
    blocks: Vec<BlockIndex>,
    entries: Vec<DirectoryEntry>,
}

impl Node {
    pub fn new(kind: NodeKind, parent_block_id: BlockIndex) -> Self {
        Node {
            kind,
            size: 0,
            parent_block_id,
            blocks: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> &Vec<DirectoryEntry> {
        assert!(self.kind == Directory, "Node is not a directory");

        &self.entries
    }

    pub fn contains_entry<S: AsRef<str>>(&self, entry_name: S) -> bool {
        assert!(self.kind == Directory, "Node is not a directory");

        self.entries
            .iter()
            .any(|entry| entry.get_name() == entry_name.as_ref())
    }

    pub fn blocks(&self) -> &Vec<BlockIndex> {
        assert!(self.kind == File, "Node is not a file");

        &self.blocks
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn is_full(&self) -> bool {
        assert!(self.kind == Directory, "Node is not a directory");

        self.size == ENTRY_COUNT as u64
    }

    pub fn push_data_block(&mut self, block: BlockIndex, size: Size) {
        assert!(self.kind == File, "Node is not a file");
        assert!(
            self.blocks.len() < BLOCK_COUNT,
            "File will exceed the maximum block count of {}",
            HumanCount(BLOCK_COUNT as u64)
        );
        assert!(
            self.size <= MAX_FILE_SIZE as u64,
            "File reported larger than maximum possible filesize of {} ({MAX_FILE_SIZE}): {}",
            HumanBytes(MAX_FILE_SIZE as u64),
            self.size
        );

        self.blocks.push(block);
        self.size += size;
    }

    pub fn push_directory_entry<S: AsRef<str>>(&mut self, name: S, block: BlockIndex) {
        assert!(self.kind == Directory, "Node is not a directory");
        assert!(
            self.size < ENTRY_COUNT as u64,
            "Directory will exceed the maximum entry count of {}",
            HumanCount(ENTRY_COUNT as u64)
        );

        self.entries.push(DirectoryEntry::new(name, block));
        self.size += 1;
    }

    pub fn rename_directory_entry<S1: AsRef<str>, S2: AsRef<str>>(&mut self, old: S1, new: S2) {
        assert!(self.kind == Directory, "Node is not a directory");

        self.entries
            .iter_mut()
            .find(|entry| entry.get_name() == old.as_ref())
            .expect("Directory entry doesn't exist")
            .set_name(new);
    }

    pub fn get_directory_entry<S: AsRef<str>>(&mut self, name: S) -> &DirectoryEntry {
        assert!(self.kind == Directory, "Node is not a directory");

        self.entries
            .iter()
            .find(|entry| entry.get_name() == name.as_ref())
            .expect("Directory entry doesn't exist")
    }

    pub fn delete_directory_entry<S: AsRef<str>>(&mut self, name: S) {
        assert!(self.kind == Directory, "Node is not a directory");

        self.entries.remove(
            self.entries
                .iter()
                .position(|entry| entry.get_name() == name.as_ref())
                .expect("Directory entry doesn't exist"),
        );
        self.size -= 1;
    }
}

impl Node {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();

        res.extend(self.kind.to_le_bytes().iter());
        res.extend(self.size.to_le_bytes().iter());
        res.extend(self.parent_block_id.to_le_bytes().iter());

        match self.kind {
            Directory => res.extend(self.entries.iter().flat_map(DirectoryEntry::to_le_bytes)),
            File => res.extend(self.blocks.iter().flat_map(|entry| entry.to_le_bytes())),
        }

        assert!(
            res.len() <= BLOCK_SIZE,
            "Converting Node to bytes has unexpected size: {}",
            HumanCount(res.len() as u64)
        );

        res
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        assert!(
            bytes.len() <= BLOCK_SIZE,
            "Data exceeds maximum block size of {}: {}",
            HumanCount(BLOCK_SIZE as u64),
            HumanCount(bytes.len() as u64)
        );
        assert!(
            bytes.len() >= KIND_SIZE + SIZE_SIZE + BLOCK_INDEX_SIZE,
            "Too little data supplied to build a Node: {}",
            bytes.len()
        );

        const KIND_POS: usize = 0;
        const SIZE_POS: usize = KIND_SIZE;
        const PARENT_BLOCK_ID_POS: usize = SIZE_POS + SIZE_SIZE;
        const CONTENT_POS: usize = PARENT_BLOCK_ID_POS + BLOCK_INDEX_SIZE;

        let mut res = Node::new(Directory, 0);
        let mut u64_bytes = [0; 8];

        u64_bytes.copy_from_slice(&bytes[KIND_POS..SIZE_POS]);
        res.kind = NodeKind::from_le_bytes(u64_bytes);
        u64_bytes.copy_from_slice(&bytes[SIZE_POS..PARENT_BLOCK_ID_POS]);
        res.size = u64::from_le_bytes(u64_bytes);
        u64_bytes.copy_from_slice(&bytes[PARENT_BLOCK_ID_POS..CONTENT_POS]);
        res.parent_block_id = u64::from_le_bytes(u64_bytes);

        match res.kind {
            Directory => {
                res.entries = DirectoryEntry::from_le_bytes(&bytes[CONTENT_POS..]);

                assert!(
                    res.entries.len() as u64 == res.size,
                    "Malformed input data has inconsistent amount of entries: {} != {}",
                    HumanCount(res.entries.len() as u64),
                    HumanCount(res.size)
                );
            }
            File => {
                assert!(
                    res.size <= MAX_FILE_SIZE as u64,
                    "Malformed input data reports file sizes larger than the maximum of {} ({}): {} ({})",
                    HumanBytes(MAX_FILE_SIZE as u64),
                    HumanCount(MAX_FILE_SIZE as u64),
                    HumanBytes(res.size),
                    HumanCount(res.size)
                );

                res.blocks = bytes[CONTENT_POS..]
                    .as_chunks::<BLOCK_INDEX_SIZE>()
                    .0
                    .iter()
                    .map(|idx| u64::from_le_bytes(*idx))
                    .collect()
            }
        }

        res
    }
}
