use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    about = "Use Discord like a File System.\n\n> Directories always end with a '/', if you want to address a directory it's mandatory to put a trailing '/'!", long_about = None
)]
pub struct Command {
    /// What operation to execute
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Clone, Subcommand)]
pub enum Operation {
    #[command(about = "List filesystem contents", long_about = None)]
    Ls {
        /// Start directory (default is '/')
        path: Option<String>,
    },
    #[command(about = "Upload data", long_about = None)]
    Upload {
        /// Source path to file
        source: String,

        /// Destination path (including file name)
        destination: String,
    },
    #[command(about = "Download files", long_about = None)]
    Download {
        /// Source path (only files)
        source: String,

        /// Destination path
        destination: String,
    },
    #[command(about = "Delete files", long_about = None)]
    Rm {
        /// Only delete directory entry but not data
        #[arg(short, long)]
        quick: bool,

        /// Delete a directory
        #[arg(short, long)]
        recursive: bool,

        /// Path
        path: String,
    },
    #[command(about = "Move files or directories", long_about = None)]
    Mv {
        // Source path
        source: String,
        // Destination path (must not include file/directory name that is being moved)
        destination: String,
    },
    #[command(about = "Rename files and directories", long_about = None)]
    Rename {
        /// Old name (must include path)
        old: String,

        /// New name (must not include path)
        new: String,
    },
    #[command(about = "Create directories", long_about = None)]
    Mkdir {
        /// Path
        path: String,
    },
}
