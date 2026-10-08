//! # mtk-package
//!
//! Unified binary package container codec for MoziToolKit (`.mtkcache`, `.mtkscene`).

pub mod error;
pub mod header;
pub mod reader;
pub mod toc;
pub mod writer;

pub use error::PackageError;
pub use header::{
    ChecksumType, CompressionType, HeaderFlags, PackageHeader, PackageProfile,
    CURRENT_VERSION_MAJOR, CURRENT_VERSION_MINOR, HEADER_SIZE, MAGIC_MTKP,
};
pub use reader::MtkPackageReader;
pub use toc::{ChunkEntry, TableOfContents};
pub use writer::{ChunkWriteOptions, MtkPackageWriter};
