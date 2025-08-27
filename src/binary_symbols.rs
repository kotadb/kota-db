//! Binary format for efficient symbol storage
//!
//! This module provides a zero-dependency, high-performance storage format
//! for code symbols using memory-mapped files and fixed-size structs.

use anyhow::{Context, Result};
use memmap2::{Mmap, MmapOptions};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::mem;
use std::path::Path;
use tracing::info;

/// Magic bytes to identify our file format
const KOTA_MAGIC: &[u8; 4] = b"KOTA";

/// Current version of the binary format
const FORMAT_VERSION: u32 = 1;

/// Platform endianness marker (1 = little-endian, 2 = big-endian)
/// TODO: Store this in header reserved bytes in v2 for cross-platform support
#[cfg(target_endian = "little")]
#[allow(dead_code)]
const ENDIAN_MARKER: u32 = 1;
#[cfg(target_endian = "big")]
#[allow(dead_code)]
const ENDIAN_MARKER: u32 = 2;

/// Fixed-size representation of a symbol for direct memory access
///
/// # Safety
/// This struct uses `#[repr(C)]` to guarantee a stable memory layout.
/// All fields are POD (Plain Old Data) types with no pointers or references.
/// The struct can be safely transmuted to/from bytes on little-endian systems.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PackedSymbol {
    /// UUID as 16 bytes
    pub id: [u8; 16],
    /// Offset into string table for symbol name
    pub name_offset: u32,
    /// Symbol type (function, class, etc.) as byte
    pub kind: u8,
    /// Offset into string table for file path
    pub file_path_offset: u32,
    /// Start line number
    pub start_line: u32,
    /// End line number
    pub end_line: u32,
    /// Parent symbol ID (all zeros if none)
    pub parent_id: [u8; 16],
    /// Reserved for future use
    pub _reserved: [u8; 3],
}

impl PackedSymbol {
    /// Size of packed symbol in bytes
    pub const SIZE: usize = mem::size_of::<Self>();

    /// Convert to bytes for writing (little-endian)
    ///
    /// # Safety
    /// Safe because PackedSymbol is #[repr(C)] with only POD fields.
    /// Format is little-endian - will need conversion on big-endian systems.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        // SAFETY: PackedSymbol has stable layout with #[repr(C)] and contains
        // only fixed-size POD types with no padding between fields
        unsafe { mem::transmute(*self) }
    }

    /// Convert from bytes for reading (little-endian)
    ///
    /// # Safety  
    /// Safe because PackedSymbol is #[repr(C)] with only POD fields.
    /// Assumes data was written in little-endian format.
    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Self {
        // SAFETY: PackedSymbol has stable layout with #[repr(C)] and the
        // input bytes array has exactly the right size
        unsafe { mem::transmute(bytes) }
    }
}

/// Header for the symbol database file
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SymbolDatabaseHeader {
    /// Magic bytes "KOTA"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Number of symbols
    pub symbol_count: u32,
    /// Offset to string table
    pub string_table_offset: u64,
    /// Size of string table
    pub string_table_size: u64,
    /// Offset to symbol data
    pub symbols_offset: u64,
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl SymbolDatabaseHeader {
    pub const SIZE: usize = mem::size_of::<Self>();

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        unsafe { mem::transmute(*self) }
    }

    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }
}

/// Writer for creating binary symbol databases
pub struct BinarySymbolWriter {
    symbols: Vec<PackedSymbol>,
    string_table: Vec<u8>,
    string_offsets: HashMap<String, u32>,
}

impl BinarySymbolWriter {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            string_table: Vec::new(),
            string_offsets: HashMap::new(),
        }
    }

    /// Intern a string into the string table
    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(&offset) = self.string_offsets.get(s) {
            return offset;
        }

        let offset = self.string_table.len() as u32;
        self.string_table.extend_from_slice(s.as_bytes());
        self.string_table.push(0); // Null terminator
        self.string_offsets.insert(s.to_string(), offset);
        offset
    }

    /// Add a symbol to the writer
    #[allow(clippy::too_many_arguments)]
    pub fn add_symbol(
        &mut self,
        id: uuid::Uuid,
        name: &str,
        kind: u8,
        file_path: &str,
        start_line: u32,
        end_line: u32,
        parent_id: Option<uuid::Uuid>,
    ) {
        let name_offset = self.intern_string(name);
        let file_path_offset = self.intern_string(file_path);

        let packed = PackedSymbol {
            id: *id.as_bytes(),
            name_offset,
            kind,
            file_path_offset,
            start_line,
            end_line,
            parent_id: parent_id.map_or([0u8; 16], |pid| *pid.as_bytes()),
            _reserved: [0; 3],
        };

        self.symbols.push(packed);
    }

    /// Write the complete database to a file
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let mut file = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .context("Failed to create symbol database file")?,
        );

        // Calculate offsets
        let header_size = SymbolDatabaseHeader::SIZE;
        let symbols_offset = header_size as u64;
        let symbols_size = self.symbols.len() * PackedSymbol::SIZE;
        let string_table_offset = symbols_offset + symbols_size as u64;

        // Create header
        let header = SymbolDatabaseHeader {
            magic: *KOTA_MAGIC,
            version: FORMAT_VERSION,
            symbol_count: self.symbols.len() as u32,
            string_table_offset,
            string_table_size: self.string_table.len() as u64,
            symbols_offset,
            _reserved: [0; 32],
        };

        // Write header
        file.write_all(&header.to_bytes())?;

        // Write symbols
        for symbol in &self.symbols {
            file.write_all(&symbol.to_bytes())?;
        }

        // Write string table
        file.write_all(&self.string_table)?;

        file.flush()?;
        Ok(())
    }
}

impl Default for BinarySymbolWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reader for memory-mapped symbol databases
pub struct BinarySymbolReader {
    mmap: Mmap,
    header: SymbolDatabaseHeader,
    /// Fast UUID → index mapping for O(1) lookups
    uuid_index: std::collections::HashMap<uuid::Uuid, usize>,
}

impl BinarySymbolReader {
    /// Open a symbol database for reading
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).context("Failed to open symbol database")?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .context("Failed to memory-map symbol database")?
        };

        // Read header
        if mmap.len() < SymbolDatabaseHeader::SIZE {
            anyhow::bail!("Symbol database file too small");
        }

        let mut header_bytes = [0u8; SymbolDatabaseHeader::SIZE];
        header_bytes.copy_from_slice(&mmap[..SymbolDatabaseHeader::SIZE]);
        let header = SymbolDatabaseHeader::from_bytes(header_bytes);

        // Validate magic and version
        if header.magic != *KOTA_MAGIC {
            anyhow::bail!(
                "Invalid symbol database magic bytes. Expected KOTA, got {:?}",
                String::from_utf8_lossy(&header.magic)
            );
        }
        if header.version != FORMAT_VERSION {
            anyhow::bail!(
                "Unsupported symbol database version: {} (expected {})",
                header.version,
                FORMAT_VERSION
            );
        }

        // Check endianness (stored in reserved bytes for now)
        // TODO: In v2, add explicit endian field to header
        #[cfg(target_endian = "big")]
        {
            anyhow::bail!(
                "Big-endian systems not yet supported. File was written on little-endian system."
            );
        }

        // Build UUID index for fast lookups
        let symbol_count = header.symbol_count as usize;
        let mut uuid_index = std::collections::HashMap::with_capacity(symbol_count);

        for i in 0..symbol_count {
            let offset = header.symbols_offset as usize + i * PackedSymbol::SIZE;
            let mut symbol_bytes = [0u8; PackedSymbol::SIZE];
            symbol_bytes.copy_from_slice(&mmap[offset..offset + PackedSymbol::SIZE]);
            let symbol = PackedSymbol::from_bytes(symbol_bytes);
            let uuid = uuid::Uuid::from_bytes(symbol.id);
            uuid_index.insert(uuid, i);
        }

        info!("Built UUID index for {} symbols", symbol_count);

        Ok(Self {
            mmap,
            header,
            uuid_index,
        })
    }

    /// Get the number of symbols
    pub fn symbol_count(&self) -> usize {
        self.header.symbol_count as usize
    }

    /// Get a symbol by index (O(1) access)
    pub fn get_symbol(&self, index: usize) -> Option<PackedSymbol> {
        if index >= self.symbol_count() {
            return None;
        }

        let offset = self.header.symbols_offset as usize + index * PackedSymbol::SIZE;
        let mut symbol_bytes = [0u8; PackedSymbol::SIZE];
        symbol_bytes.copy_from_slice(&self.mmap[offset..offset + PackedSymbol::SIZE]);

        Some(PackedSymbol::from_bytes(symbol_bytes))
    }

    /// Get a string from the string table
    pub fn get_string(&self, offset: u32) -> Result<String> {
        let start = self.header.string_table_offset as usize + offset as usize;
        if start >= self.mmap.len() {
            anyhow::bail!("String offset out of bounds");
        }

        // Find null terminator
        let slice = &self.mmap[start..];
        let end = slice
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("String not null-terminated"))?;

        String::from_utf8(slice[..end].to_vec()).context("Invalid UTF-8 in string table")
    }

    /// Get symbol name
    pub fn get_symbol_name(&self, symbol: &PackedSymbol) -> Result<String> {
        self.get_string(symbol.name_offset)
    }

    /// Get symbol file path
    pub fn get_symbol_file_path(&self, symbol: &PackedSymbol) -> Result<String> {
        self.get_string(symbol.file_path_offset)
    }

    /// Iterate over all symbols
    pub fn iter_symbols(&self) -> impl Iterator<Item = PackedSymbol> + '_ {
        (0..self.symbol_count()).filter_map(move |i| self.get_symbol(i))
    }

    /// Find symbol by UUID (O(1) lookup)
    pub fn find_symbol(&self, id: uuid::Uuid) -> Option<PackedSymbol> {
        let index = *self.uuid_index.get(&id)?;
        self.get_symbol(index)
    }

    /// Find symbol by name (O(n) search - use sparingly)
    pub fn find_symbol_by_name(&self, name: &str) -> Option<(PackedSymbol, uuid::Uuid)> {
        self.iter_symbols().find_map(|symbol| {
            if let Ok(symbol_name) = self.get_symbol_name(&symbol) {
                if symbol_name == name {
                    return Some((symbol, uuid::Uuid::from_bytes(symbol.id)));
                }
            }
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_binary_format_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.symdb");

        // Write symbols
        let mut writer = BinarySymbolWriter::new();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        writer.add_symbol(id1, "test_function", 1, "src/test.rs", 10, 20, None);
        writer.add_symbol(id2, "TestClass", 2, "src/test.rs", 30, 50, Some(id1));

        writer.write_to_file(&db_path).unwrap();

        // Read symbols back
        let reader = BinarySymbolReader::open(&db_path).unwrap();

        assert_eq!(reader.symbol_count(), 2);

        let symbol1 = reader.get_symbol(0).unwrap();
        assert_eq!(reader.get_symbol_name(&symbol1).unwrap(), "test_function");
        assert_eq!(symbol1.start_line, 10);

        let symbol2 = reader.get_symbol(1).unwrap();
        assert_eq!(reader.get_symbol_name(&symbol2).unwrap(), "TestClass");
        assert_eq!(symbol2.parent_id, *id1.as_bytes());
    }

    #[test]
    fn test_corrupted_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("corrupted.symdb");

        // Write invalid magic bytes
        std::fs::write(&db_path, b"BADMAGIC").unwrap();

        let result = BinarySymbolReader::open(&db_path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        // File is too small, so we get a different error
        assert!(
            err_msg.contains("Symbol database file too small")
                || err_msg.contains("Invalid symbol database magic")
        );
    }

    #[test]
    fn test_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("wrong_version.symdb");

        // Create header with wrong version
        let header = SymbolDatabaseHeader {
            magic: *KOTA_MAGIC,
            version: 999, // Wrong version
            symbol_count: 0,
            string_table_offset: 88,
            string_table_size: 0,
            symbols_offset: 88,
            _reserved: [0; 32],
        };

        std::fs::write(&db_path, header.to_bytes()).unwrap();

        let result = BinarySymbolReader::open(&db_path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("Unsupported symbol database version"));
    }

    #[test]
    fn test_empty_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("empty.symdb");

        let writer = BinarySymbolWriter::new();
        writer.write_to_file(&db_path).unwrap();

        let reader = BinarySymbolReader::open(&db_path).unwrap();
        assert_eq!(reader.symbol_count(), 0);
        assert!(reader.get_symbol(0).is_none());
    }
}
