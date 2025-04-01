use serenity::{
    Client,
    all::{ChannelId, CreateAttachment, CreateMessage, EditMessage, MessageId},
};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    directory_entry::BlockIndex,
    node::{self, Node},
    node_kind::NodeKind::{Directory, File},
    util,
};

pub struct NodeFS {
    root_node_id: BlockIndex,
    data_channel: ChannelId,

    client: Client,
}

impl NodeFS {
    pub fn new(data_channel_id: u64, client: serenity::Client) -> Self {
        NodeFS {
            root_node_id: 0,
            data_channel: ChannelId::new(data_channel_id),
            client,
        }
    }

    pub async fn setup(&mut self) {
        if let Some(topic) = util::get_guild_channel(&self.client, self.data_channel)
            .await
            .expect("Data channel should be guild channel")
            .topic
        {
            let block_id = topic.parse::<u64>().expect(
                "Only the root message ID should be in the channel topic and be a valid u64",
            );
            self.root_node_id = block_id;
        } else {
            // root node has parent of 0
            let (_, root_node_block_id) = self.create_directory_node(0).await;

            // store root node id in discord topic
            util::edit_channel_topic(
                &self.client,
                self.data_channel,
                root_node_block_id.to_string(),
            )
            .await
            .expect("Failed to save root node block id in channel topic");

            self.root_node_id = root_node_block_id;
        }
    }

    pub async fn list(&self, path: Option<String>) {
        if let Some(path) = path {
            let (_, name) = NodeFS::split_path(path.as_str(), true, true);
            let (path_node, _) = self.traverse_path(path.as_str()).await;
            self.__list(0, name, path_node).await;
        } else {
            self.__list(0, "/", self.get_directory_node(self.root_node_id).await)
                .await;
        }
    }

    pub async fn upload(&self, source: String, destination: String) {
        // Open source file
        let mut file = fs::File::open(source).await.expect("Failed to open file");
        let filesize = file
            .metadata()
            .await
            .expect("Failed to fetch source file size")
            .len() as usize;
        assert!(
            filesize <= node::MAX_FILE_SIZE,
            "File exceeds maximum file size of `{}`",
            node::MAX_FILE_SIZE
        );

        let (file_path, file_name) = NodeFS::split_path(destination.as_str(), false, false);

        // get target directory
        let (mut dir_node, dir_node_id) = self.traverse_path(file_path).await;
        assert!(!dir_node.is_full(), "The directory is full");
        assert!(
            !dir_node.contains_entry(file_name),
            "The file already exists"
        );

        // create file node
        let (mut file_node, file_node_id) = self.create_file_node(dir_node_id).await;

        // upload file in at most block sized chunks
        let mut read_bytes = 0;
        while read_bytes != filesize {
            let chunk_size = std::cmp::min(filesize - read_bytes, node::BLOCK_SIZE);
            let mut chunk = vec![0; chunk_size];
            file.read_exact(&mut chunk)
                .await
                .expect("Error reading from file");
            read_bytes += chunk_size;

            let block_id = self.create_data_block(chunk).await;
            file_node.push_data_block(block_id, chunk_size as u64);
        }

        // update nodes
        dir_node.push_directory_entry(file_name, file_node_id);
        self.edit_directory_node(dir_node_id, dir_node).await;
        self.edit_file_node(file_node_id, file_node).await;
    }

    pub async fn download(&self, source: String, destination: String) {
        // open destination file
        let mut file = fs::File::create(destination)
            .await
            .expect("Failed to create file");

        // get source file
        let (source_node, _) = self.traverse_path(source).await;
        assert!(source_node.kind != Directory, "Can't download directories");

        // read all data blocks and write them to the destination
        for block_id in source_node.blocks() {
            let block = self.get_data_block(*block_id).await;

            file.write_all(&block)
                .await
                .expect("Failed to write downloaded data");
        }
    }

    pub async fn rm(&self, path: String, quick: bool, recursive: bool) {
        // would be caught later but can give a nicer error here
        assert!(path != "/", "Cannot delete root directory");

        let (_, file_name) = NodeFS::split_path(path.as_str(), true, false);

        // get target directory
        let (target_node, target_node_id) = self.traverse_path(path.as_str()).await;
        let dir_node_id = target_node.parent_block_id;
        let mut dir_node = self.get_directory_node(dir_node_id).await;

        match target_node.kind {
            Directory if !recursive => panic!("Directories must be deleted recursively"),
            File if recursive => panic!("Files cannot be deleted recursively"),
            _ => {}
        }

        // delete nodes and data blocks
        if !quick {
            if recursive {
                self.delete_directory(target_node, target_node_id).await;
            } else {
                self.delete_file(target_node, target_node_id).await;
            }
        }

        // delete file directory entry
        dir_node.delete_directory_entry(file_name);
        self.edit_directory_node(dir_node_id, dir_node).await;
    }

    pub async fn mv(&self, source: String, destination: String) {
        if source == destination {
            return;
        }
        assert!(source != "/", "Cannot move root directory");

        let (_, source_name) = NodeFS::split_path(source.as_str(), true, false);
        let (source_node, source_node_id) = self.traverse_path(source.as_str()).await;
        let mut source_parent_node = self.get_directory_node(source_node.parent_block_id).await;
        let (mut target_node, target_node_id) = self.traverse_path(destination).await;
        assert!(target_node.kind == Directory, "Must move into a directory");
        assert!(!target_node.is_full(), "The directory is full");

        // move entry and save
        source_parent_node.delete_directory_entry(source_name);
        target_node.push_directory_entry(source_name, source_node_id);
        self.edit_directory_node(source_node.parent_block_id, source_parent_node)
            .await;
        self.edit_directory_node(target_node_id, target_node).await;
    }

    pub async fn replace(&self, source: String, destination: String, quick: bool) {
        self.rm(destination.clone(), quick, false).await;
        self.upload(source, destination).await;
    }

    pub async fn rename(&self, old: String, new: String) {
        assert!(new != "/", "New name must not only be a '/'");

        let slash_pos = new.chars().position(|ch| ch == '/');
        if old.ends_with('/') {
            assert!(
                slash_pos.unwrap() == new.len() - 1,
                "New directory name must only have `/` at the end"
            );
        } else {
            assert!(slash_pos.is_none(), "New file name must not end with `/`");
        }

        let (target_path, target_name) = NodeFS::split_path(old.as_str(), true, false);

        // get target directory
        let (mut dir_node, dir_node_id) = self.traverse_path(target_path).await;

        // rename entry and save
        dir_node.rename_directory_entry(target_name, new);
        self.edit_directory_node(dir_node_id, dir_node).await;
    }

    pub async fn mkdir(&self, path: String) {
        let (target_path, target_path_name) = NodeFS::split_path(path.as_str(), true, true);

        // get target directory
        let (mut dir_node, dir_node_id) = self.traverse_path(target_path).await;
        assert!(!dir_node.is_full(), "The directory is full");
        assert!(
            !dir_node.contains_entry(target_path_name),
            "The file already exists"
        );

        let (_, new_dir_node_id) = self.create_directory_node(dir_node_id).await;

        // add new directory
        dir_node.push_directory_entry(target_path_name, new_dir_node_id);
        self.edit_directory_node(dir_node_id, dir_node).await;
    }
}

impl NodeFS {
    async fn __list(&self, ident: usize, curr_name: &str, curr_dir: Node) {
        let unit = match curr_dir.kind {
            Directory => "entries",
            File => "bytes",
        };

        println!(
            "{:ident$}{curr_name} - - - - - - - {} ({unit})",
            "",
            curr_dir.size()
        );

        if curr_dir.kind == File {
            return;
        }

        // recursively list directory hierarchy
        for entry in curr_dir.entries() {
            let entry_node = self.get_node(entry.block_id()).await;
            Box::pin(self.__list(ident + 4, entry.get_name().as_str(), entry_node)).await;
        }
    }

    async fn delete_file(&self, node: Node, node_id: BlockIndex) {
        assert!(
            node.kind == File,
            "Attempt to delete non file node as file node"
        );

        // delete file data blocks
        for block_id in node.blocks() {
            self.delete_block(*block_id).await;
        }

        // delete file node
        self.delete_block(node_id).await;
    }

    async fn delete_directory(&self, node: Node, node_id: BlockIndex) {
        assert!(
            node.kind == Directory,
            "Attempt to delete non directory node as directory node"
        );

        // delete all directory contents (recursively)
        for directory_entry in node.entries() {
            let entry_node_id = directory_entry.block_id();
            let entry_node = self.get_node(entry_node_id).await;

            match entry_node.kind {
                Directory => Box::pin(self.delete_directory(entry_node, entry_node_id)).await,
                File => self.delete_file(entry_node, entry_node_id).await,
            }
        }

        // delete directory node
        self.delete_block(node_id).await;
    }

    fn split_path(path: &str, allow_dirs: bool, require_dir: bool) -> (&str, &str) {
        if require_dir {
            assert!(allow_dirs, "Directories required but not allowed");
        }
        if !allow_dirs {
            assert!(!path.ends_with('/'), "Directories not allowed");
        }
        if require_dir {
            assert!(path.ends_with('/'), "Directories are required");
        }

        // ignore trailing '/' for dirs to find parent folder
        let bound = if require_dir || (allow_dirs && path.ends_with('/')) {
            path.len() - 1
        } else {
            path.len()
        };

        let trailing_slash_pos = path[..bound]
            .rfind('/')
            .expect("Target path must have trailing filename");

        path.split_at(trailing_slash_pos + 1)
    }

    async fn traverse_path<S: AsRef<str>>(&self, path: S) -> (Node, BlockIndex) {
        assert!(
            path.as_ref().starts_with('/'),
            "Paths must start with a `/`"
        );

        // edge case of '/'
        if path.as_ref() == "/" {
            return (self.get_root_directory_node().await, self.root_node_id);
        }

        let path_segments: Vec<&str> = path.as_ref().split_inclusive('/').collect();

        // if the path ends with a '/' it points to a directory
        let path_to_dir = path_segments.last().unwrap().ends_with('/');

        let mut dir = self.get_root_directory_node().await;
        // traverse path
        // exclude first segment of leading '/' and last of filename
        for segment in path_segments[..path_segments.len() - 1].iter().skip(1) {
            assert!(!segment.is_empty(), "Consecutive `/` are not permitted");

            // this panics if a path segment in the middle is not a directory as it's supposed to
            dir = self
                .get_directory_node(dir.get_directory_entry(segment).block_id())
                .await;
        }

        // get destination directory or file
        if path_to_dir {
            let dir_node_block_id = dir
                .get_directory_entry(path_segments.last().unwrap())
                .block_id();
            (
                self.get_directory_node(dir_node_block_id).await,
                dir_node_block_id,
            )
        } else {
            let file_node_block_id = dir
                .get_directory_entry(path_segments.last().unwrap())
                .block_id();
            (
                self.get_file_node(file_node_block_id).await,
                file_node_block_id,
            )
        }
    }

    async fn create_directory_node(&self, parent_node_id: BlockIndex) -> (Node, BlockIndex) {
        let node = Node::new(Directory, parent_node_id);
        let attachment = CreateAttachment::bytes(node.to_bytes(), "node");

        let block_id = util::send_message(
            &self.client,
            self.data_channel,
            CreateMessage::new().content("").add_file(attachment),
        )
        .await
        .expect("Failed to create directory node");

        (node, block_id.get())
    }

    async fn edit_directory_node(&self, node_id: BlockIndex, node: Node) {
        assert!(
            node.kind == Directory,
            "Tried to update non directory node as directory node"
        );

        let attachment = CreateAttachment::bytes(node.to_bytes(), "node");
        util::edit_message(
            &self.client,
            self.data_channel,
            MessageId::new(node_id),
            EditMessage::new().new_attachment(attachment),
        )
        .await
        .expect("Failed to edit directory node");
    }

    async fn get_directory_node(&self, node_id: BlockIndex) -> Node {
        let node = Node::from_bytes(
            util::read_attachment(&self.client, self.data_channel, MessageId::new(node_id))
                .await
                .expect("Failed to get directory node"),
        );

        assert!(
            node.kind == Directory,
            "Tried to get non directory node as directory node"
        );

        node
    }

    async fn get_root_directory_node(&self) -> Node {
        let node = Node::from_bytes(
            util::read_attachment(
                &self.client,
                self.data_channel,
                MessageId::new(self.root_node_id),
            )
            .await
            .expect("Failed to get root node"),
        );

        assert!(node.kind == Directory, "Root node is corrupted");

        node
    }

    async fn create_file_node(&self, parent_node_id: BlockIndex) -> (Node, BlockIndex) {
        let node = Node::new(File, parent_node_id);
        let attachment = CreateAttachment::bytes(node.to_bytes(), "node");

        let block_id = util::send_message(
            &self.client,
            self.data_channel,
            CreateMessage::new().content("").add_file(attachment),
        )
        .await
        .expect("Failed to create file node");

        (node, block_id.get())
    }

    async fn edit_file_node(&self, node_id: BlockIndex, node: Node) {
        assert!(
            node.kind == File,
            "Tried to update non file node as file node"
        );

        let attachment = CreateAttachment::bytes(node.to_bytes(), "node");
        util::edit_message(
            &self.client,
            self.data_channel,
            MessageId::new(node_id),
            EditMessage::new().new_attachment(attachment),
        )
        .await
        .expect("Failed to edit file node");
    }

    async fn get_file_node(&self, node_id: BlockIndex) -> Node {
        let node = Node::from_bytes(
            util::read_attachment(&self.client, self.data_channel, MessageId::new(node_id))
                .await
                .expect("Failed to get file node"),
        );

        assert!(node.kind == File, "Tried to get non file node as file node");

        node
    }

    async fn create_data_block(&self, data: Vec<u8>) -> BlockIndex {
        let attachment = CreateAttachment::bytes(data, "data");
        util::send_message(
            &self.client,
            self.data_channel,
            CreateMessage::new().content("").add_file(attachment),
        )
        .await
        .expect("Failed to create data block")
        .get()
    }

    async fn get_data_block(&self, block_id: u64) -> Vec<u8> {
        util::read_attachment(&self.client, self.data_channel, MessageId::new(block_id))
            .await
            .expect("Failed to get data block")
    }

    async fn delete_block(&self, block_id: u64) {
        util::delete_message(&self.client, self.data_channel, MessageId::new(block_id))
            .await
            .expect("Failed to delete block");
    }

    async fn get_node(&self, node_id: BlockIndex) -> Node {
        Node::from_bytes(
            util::read_attachment(&self.client, self.data_channel, MessageId::new(node_id))
                .await
                .expect("Failed to get node"),
        )
    }
}
