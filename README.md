## DFS
DFS or DiscordFS is a blazingly slow™️ ⚡️ primitive filesystem implementation in Discord. It features uploading and downloading military-grade™️ encrypted files of arbitrary (8.8TB) size and folders, concipated as a command line tool.

```bash
# for a list of commands and help
dfs --help
dfs upload --help
```

https://github.com/user-attachments/assets/9351f76b-5448-4a64-8cb1-04c0e2c9299f

#### Tech
It features nodes and data blocks and is (in spirit) similar to a filesystem like the UNIX filesystem. This makes it, unlike other implementations of data storage on Discord I've seen, unique by being self-contained, meaning that all file information is also stored on Discord itself and accessible if the root node of the filesystem is known. 

#### Requirenments
Requires a Discord bot that has permissions to edit a channel and create, edit, and delete messages in that channel, as well as see the message history. Add the Discord bot token, channel ID and AES key in the `.env` file.

#### Performance
This is generally pretty slow since there is no caching of the directory tree (yet?). It is not viable (or recommended) to actually be used and was just an excuse to implement a simple filesystem.

#### Legal
Please don't sue me Discord, I can't afford that.
