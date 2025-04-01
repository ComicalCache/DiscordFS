## DFS
DFS or DiscordFS is a blazingly slow™️ ⚡️ primitive filesystem implementation in Discord. It features files of arbitrary (8.8TB) size and folders, concipated as a command line tool.

```bash
# for a list of commands and help
dfs --help
dfs upload --help
```

#### Requirenments
Requires a Discord bot that has permissions to edit a channel and create, edit, and delete messages in that channel, as well as see the message history. Add the Discord bot token and channel ID in the `.env` file.

#### Performance
This is generally pretty slow since there is no caching of the directory tree (yet?). It is not viable (or recommended) to actually be used and was just an excuse to implement a simple filesystem.

#### Legal
Please don't sue me Discord, I can't afford that.
