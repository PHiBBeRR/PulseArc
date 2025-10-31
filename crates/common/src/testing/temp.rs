//! Temporary file and directory helpers
//!
//! Provides RAII wrappers for temporary files and directories that
//! automatically clean up when dropped.

// Allow missing error/panic docs for temp file utilities - IO errors are self-explanatory
// and these are simple wrappers around standard library functionality
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::{fs, io};

/// Temporary directory that is automatically deleted when dropped
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::temp::TempDir;
///
/// let temp_dir = TempDir::new("test-dir").unwrap();
/// let path = temp_dir.path();
/// // Use the directory...
/// // Automatically cleaned up when temp_dir goes out of scope
/// ```
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a new temporary directory with a prefix
    pub fn new(prefix: &str) -> io::Result<Self> {
        let temp_base = std::env::temp_dir();
        let dir_name = format!("{}-{}", prefix, uuid::Uuid::new_v4());
        let path = temp_base.join(dir_name);

        fs::create_dir_all(&path)?;

        Ok(Self { path })
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create a file in the temporary directory
    pub fn create_file(&self, name: &str, contents: &str) -> io::Result<PathBuf> {
        let file_path = self.path.join(name);
        fs::write(&file_path, contents)?;
        Ok(file_path)
    }

    /// Create a subdirectory
    pub fn create_dir(&self, name: &str) -> io::Result<PathBuf> {
        let dir_path = self.path.join(name);
        fs::create_dir_all(&dir_path)?;
        Ok(dir_path)
    }

    /// Keep the directory (don't delete on drop) and return its path
    ///
    /// This consumes the `TempDir` and returns the path, preventing automatic
    /// cleanup.
    pub fn keep(mut self) -> PathBuf {
        let path = self.path.clone();
        self.path = PathBuf::new(); // Set to empty path so Drop won't delete
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Temporary file that is automatically deleted when dropped
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::temp::TempFile;
///
/// let temp_file = TempFile::new("test-file", "txt").unwrap();
/// let path = temp_file.path();
/// // Use the file...
/// // Automatically cleaned up when temp_file goes out of scope
/// ```
#[derive(Debug)]
pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// Create a new temporary file with a prefix and extension
    pub fn new(prefix: &str, extension: &str) -> io::Result<Self> {
        let temp_base = std::env::temp_dir();
        let file_name = format!("{}-{}.{}", prefix, uuid::Uuid::new_v4(), extension);
        let path = temp_base.join(file_name);

        // Create empty file
        fs::write(&path, "")?;

        Ok(Self { path })
    }

    /// Create a new temporary file with initial contents
    pub fn with_contents(prefix: &str, extension: &str, contents: &str) -> io::Result<Self> {
        let temp_base = std::env::temp_dir();
        let file_name = format!("{}-{}.{}", prefix, uuid::Uuid::new_v4(), extension);
        let path = temp_base.join(file_name);

        fs::write(&path, contents)?;

        Ok(Self { path })
    }

    /// Get the path to the temporary file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write contents to the file
    pub fn write(&self, contents: &str) -> io::Result<()> {
        fs::write(&self.path, contents)
    }

    /// Read contents from the file
    pub fn read(&self) -> io::Result<String> {
        fs::read_to_string(&self.path)
    }

    /// Keep the file (don't delete on drop) and return its path
    ///
    /// This consumes the `TempFile` and returns the path, preventing automatic
    /// cleanup.
    pub fn keep(mut self) -> PathBuf {
        let path = self.path.clone();
        self.path = PathBuf::new(); // Set to empty path so Drop won't delete
        path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() && self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::temp.
    use super::*;

    /// Validates `TempDir::new` behavior for the temp dir creation scenario.
    ///
    /// Assertions:
    /// - Ensures `temp_dir.path().exists()` evaluates to true.
    /// - Ensures `!path.exists()` evaluates to true.
    #[test]
    fn test_temp_dir_creation() {
        let temp_dir = TempDir::new("test").unwrap();
        assert!(temp_dir.path().exists());
        let path = temp_dir.path().to_path_buf();

        drop(temp_dir);
        assert!(!path.exists());
    }

    /// Validates `TempDir::new` behavior for the temp dir create file scenario.
    ///
    /// Assertions:
    /// - Ensures `file_path.exists()` evaluates to true.
    /// - Confirms `contents` equals `"hello"`.
    #[test]
    fn test_temp_dir_create_file() {
        let temp_dir = TempDir::new("test").unwrap();
        let file_path = temp_dir.create_file("test.txt", "hello").unwrap();
        assert!(file_path.exists());

        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "hello");
    }

    /// Validates `TempDir::new` behavior for the temp dir create subdir
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `subdir.exists()` evaluates to true.
    /// - Ensures `subdir.is_dir()` evaluates to true.
    #[test]
    fn test_temp_dir_create_subdir() {
        let temp_dir = TempDir::new("test").unwrap();
        let subdir = temp_dir.create_dir("subdir").unwrap();
        assert!(subdir.exists());
        assert!(subdir.is_dir());
    }

    /// Validates `TempFile::new` behavior for the temp file creation scenario.
    ///
    /// Assertions:
    /// - Ensures `temp_file.path().exists()` evaluates to true.
    /// - Ensures `!path.exists()` evaluates to true.
    #[test]
    fn test_temp_file_creation() {
        let temp_file = TempFile::new("test", "txt").unwrap();
        assert!(temp_file.path().exists());
        let path = temp_file.path().to_path_buf();

        drop(temp_file);
        assert!(!path.exists());
    }

    /// Validates `TempFile::with_contents` behavior for the temp file with
    /// contents scenario.
    ///
    /// Assertions:
    /// - Confirms `contents` equals `"hello world"`.
    #[test]
    fn test_temp_file_with_contents() {
        let temp_file = TempFile::with_contents("test", "txt", "hello world").unwrap();
        let contents = temp_file.read().unwrap();
        assert_eq!(contents, "hello world");
    }

    /// Validates `TempFile::new` behavior for the temp file write read
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `contents` equals `"test data"`.
    #[test]
    fn test_temp_file_write_read() {
        let temp_file = TempFile::new("test", "txt").unwrap();
        temp_file.write("test data").unwrap();

        let contents = temp_file.read().unwrap();
        assert_eq!(contents, "test data");
    }
}
