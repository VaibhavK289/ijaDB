use crate::PAGE_SIZE;

/// A Page is simply a fixed-size byte array.
pub struct Page {
    pub data: [u8; PAGE_SIZE],
}

impl Page {
    /// Create a new empty page, filled with zeros.
    pub fn new() -> Self {
        Self {
            data: [0; PAGE_SIZE],
        }
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}
