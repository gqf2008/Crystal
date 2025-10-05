// File Helper - File operations and information
// Corresponds to: Client/Utils/FileHelper.cs

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Information about a file for download/patch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInformation {
    /// Relative file name
    pub file_name: String,
    
    /// Uncompressed file length in bytes
    pub length: i32,
    
    /// Compressed file length in bytes
    pub compressed: i32,
    
    /// File creation time
    pub creation: DateTime<Utc>,
}

impl FileInformation {
    /// Create a new FileInformation
    pub fn new(file_name: String, length: i32, compressed: i32, creation: DateTime<Utc>) -> Self {
        Self {
            file_name,
            length,
            compressed,
            creation,
        }
    }
    
    /// Read FileInformation from a binary reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        
        // Read file name (length-prefixed string in .NET format)
        let name_len = reader.read_u8()? as usize;
        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes)?;
        let file_name = String::from_utf8(name_bytes)?;
        
        // Read file lengths
        let length = reader.read_i32::<LittleEndian>()?;
        let compressed = reader.read_i32::<LittleEndian>()?;
        
        // Read creation time (as .NET DateTime binary format - 64-bit ticks)
        let ticks = reader.read_i64::<LittleEndian>()?;
        let creation = dotnet_datetime_to_chrono(ticks);
        
        Ok(Self {
            file_name,
            length,
            compressed,
            creation,
        })
    }
    
    /// Write FileInformation to a binary writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        
        // Write file name (length-prefixed string)
        let name_bytes = self.file_name.as_bytes();
        writer.write_u8(name_bytes.len() as u8)?;
        writer.write_all(name_bytes)?;
        
        // Write file lengths
        writer.write_i32::<LittleEndian>(self.length)?;
        writer.write_i32::<LittleEndian>(self.compressed)?;
        
        // Write creation time (as .NET DateTime binary format)
        let ticks = chrono_to_dotnet_datetime(&self.creation);
        writer.write_i64::<LittleEndian>(ticks)?;
        
        Ok(())
    }
}

/// Download progress tracker
#[derive(Debug, Clone)]
pub struct Download {
    /// File information
    pub info: FileInformation,
    
    /// Current bytes downloaded
    pub current_bytes: i64,
    
    /// Whether download is completed
    pub completed: bool,
}

impl Download {
    /// Create a new Download tracker
    pub fn new(info: FileInformation) -> Self {
        Self {
            info,
            current_bytes: 0,
            completed: false,
        }
    }
    
    /// Update download progress
    pub fn update_progress(&mut self, bytes: i64) {
        self.current_bytes = bytes;
        self.completed = self.current_bytes >= self.info.length as i64;
    }
    
    /// Get download progress percentage (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        if self.info.length == 0 {
            return 1.0;
        }
        (self.current_bytes as f64 / self.info.length as f64).min(1.0)
    }
    
    /// Get download progress percentage (0 - 100)
    pub fn progress_percent(&self) -> u8 {
        (self.progress() * 100.0) as u8
    }
}

/// Convert .NET DateTime binary format (ticks since 0001-01-01) to chrono DateTime
/// .NET DateTime.ToBinary() format: 63 bits for ticks + 1 bit for kind
fn dotnet_datetime_to_chrono(binary: i64) -> DateTime<Utc> {
    // Extract ticks (ignore the kind bit)
    let ticks = binary & 0x3FFFFFFFFFFFFFFF;
    
    // .NET DateTime epoch: 0001-01-01 00:00:00
    // Unix epoch: 1970-01-01 00:00:00
    // Difference: 621355968000000000 ticks (100-nanosecond units)
    const TICKS_TO_UNIX_EPOCH: i64 = 621355968000000000;
    const TICKS_PER_SECOND: i64 = 10000000;
    
    let unix_ticks = ticks - TICKS_TO_UNIX_EPOCH;
    let seconds = unix_ticks / TICKS_PER_SECOND;
    let nanos = ((unix_ticks % TICKS_PER_SECOND) * 100) as u32;
    
    DateTime::from_timestamp(seconds, nanos).unwrap_or_else(|| Utc::now())
}

/// Convert chrono DateTime to .NET DateTime binary format
fn chrono_to_dotnet_datetime(dt: &DateTime<Utc>) -> i64 {
    const TICKS_TO_UNIX_EPOCH: i64 = 621355968000000000;
    const TICKS_PER_SECOND: i64 = 10000000;
    
    let seconds = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();
    
    let unix_ticks = seconds * TICKS_PER_SECOND + (nanos as i64 / 100);
    let ticks = unix_ticks + TICKS_TO_UNIX_EPOCH;
    
    // Set kind bit to UTC (bit 62 = 0 for Unspecified, 1 for UTC, 2 for Local)
    ticks | (1i64 << 62)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[test]
    fn test_file_information_serialization() {
        let now = Utc::now();
        let info = FileInformation::new(
            "test.dat".to_string(),
            1024,
            512,
            now,
        );
        
        // Serialize
        let mut buffer = Vec::new();
        info.write_to(&mut buffer).unwrap();
        
        // Deserialize
        let mut cursor = Cursor::new(buffer);
        let restored = FileInformation::read_from(&mut cursor).unwrap();
        
        assert_eq!(restored.file_name, info.file_name);
        assert_eq!(restored.length, info.length);
        assert_eq!(restored.compressed, info.compressed);
        // DateTime comparison might have slight differences due to precision
        assert!((restored.creation.timestamp() - info.creation.timestamp()).abs() <= 1);
    }
    
    #[test]
    fn test_download_progress() {
        let info = FileInformation::new(
            "bigfile.dat".to_string(),
            10000,
            5000,
            Utc::now(),
        );
        
        let mut download = Download::new(info);
        assert_eq!(download.progress_percent(), 0);
        assert!(!download.completed);
        
        download.update_progress(5000);
        assert_eq!(download.progress_percent(), 50);
        assert!(!download.completed);
        
        download.update_progress(10000);
        assert_eq!(download.progress_percent(), 100);
        assert!(download.completed);
    }
    
    #[test]
    fn test_dotnet_datetime_conversion() {
        // Test with a known .NET DateTime binary value
        // 2024-01-01 00:00:00 UTC
        let dotnet_binary = 638395584000000000i64 | (1i64 << 62); // UTC kind
        let dt = dotnet_datetime_to_chrono(dotnet_binary);
        
        // Convert back
        let restored = chrono_to_dotnet_datetime(&dt);
        
        // Should be close (within a second due to precision differences)
        assert!((restored - dotnet_binary).abs() < 10000000); // Within 1 second
    }
}
