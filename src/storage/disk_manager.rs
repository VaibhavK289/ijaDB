use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::storage::page::Page;
use crate::{PAGE_SIZE, PageId};

/// The DiskManager is responsible for reading and writing pages to disk.
pub struct DiskManager {
    file: File,
    next_page_id: PageId,
}

impl DiskManager {
    /// Create a new DiskManager that writes to the given file path.
    pub fn new(db_file_path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(db_file_path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        // Calculate the next available page ID based on the file size.
        let next_page_id = (file_size / PAGE_SIZE as u64) as PageId;

        Ok(Self { file, next_page_id })
    }

    /// Read a specific page from disk.
    pub fn read_page(&mut self, page_id: PageId) -> io::Result<Page> {
        let offset = (page_id as usize * PAGE_SIZE) as u64;
        self.file.seek(SeekFrom::Start(offset))?;

        let mut page = Page::new();
        let _bytes_read = self.file.read(&mut page.data)?;

        // If we read less than a full page but not 0 (EOF), it's a partial read.
        // We return what we read, padded with zeros (handled by Page::new).

        Ok(page)
    }

    /// Write a specific page to disk.
    pub fn write_page(&mut self, page_id: PageId, page: &Page) -> io::Result<()> {
        let offset = (page_id as usize * PAGE_SIZE) as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&page.data)?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Allocate a new page ID.
    pub fn allocate_page(&mut self) -> PageId {
        let page_id = self.next_page_id;
        self.next_page_id += 1;
        page_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{}_{}.db",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_disk_manager_read_write_page() {
        let path = get_temp_path("test_read_write");

        let mut disk_manager = DiskManager::new(&path).expect("Failed to create DiskManager");

        // 1. Allocate a page
        let page_id = disk_manager.allocate_page();
        assert_eq!(page_id, 0);

        // 2. Create a page with some data
        let mut page = Page::new();
        let test_data = b"ijaDB test data";
        page.data[..test_data.len()].copy_from_slice(test_data);

        // 3. Write the page to disk
        disk_manager
            .write_page(page_id, &page)
            .expect("Failed to write page");

        // 4. Read the page back from disk
        let read_page = disk_manager
            .read_page(page_id)
            .expect("Failed to read page");

        // 5. Verify the data matches
        assert_eq!(&read_page.data[..test_data.len()], test_data);

        // Clean up
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_disk_manager_allocate_pages() {
        let path = get_temp_path("test_allocate");

        let mut disk_manager = DiskManager::new(&path).expect("Failed to create DiskManager");

        let page0 = disk_manager.allocate_page();
        let page1 = disk_manager.allocate_page();
        let page2 = disk_manager.allocate_page();

        assert_eq!(page0, 0);
        assert_eq!(page1, 1);
        assert_eq!(page2, 2);

        // Clean up
        let _ = std::fs::remove_file(path);
    }
}
