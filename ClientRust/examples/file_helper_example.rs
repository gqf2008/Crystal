// Example: File helper utilities
// Demonstrates FileInformation and Download tracking
// Run with: cargo run --example file_helper_example

use mir2_client::utils::{FileInformation, Download};
use chrono::Utc;
use std::io::Cursor;

fn main() -> anyhow::Result<()> {
    println!("=== File Helper Example ===\n");
    
    // Create file information
    let file_info = FileInformation::new(
        "GameData.pak".to_string(),
        10485760,  // 10 MB uncompressed
        5242880,   // 5 MB compressed
        Utc::now(),
    );
    
    println!("📦 File Information:");
    println!("  Name: {}", file_info.file_name);
    println!("  Uncompressed: {} bytes ({:.2} MB)", 
             file_info.length, 
             file_info.length as f64 / 1024.0 / 1024.0);
    println!("  Compressed: {} bytes ({:.2} MB)", 
             file_info.compressed, 
             file_info.compressed as f64 / 1024.0 / 1024.0);
    println!("  Compression ratio: {:.1}%", 
             100.0 - (file_info.compressed as f64 / file_info.length as f64 * 100.0));
    println!("  Created: {}", file_info.creation);
    
    // Test serialization
    println!("\n🔄 Testing Serialization...");
    let mut buffer = Vec::new();
    file_info.write_to(&mut buffer)?;
    println!("  Serialized to {} bytes", buffer.len());
    
    let mut cursor = Cursor::new(buffer);
    let restored = FileInformation::read_from(&mut cursor)?;
    println!("  Deserialized successfully");
    assert_eq!(restored.file_name, file_info.file_name);
    println!("  ✅ Data integrity verified");
    
    // Download tracking
    println!("\n⬇️  Download Tracking:");
    let mut download = Download::new(file_info);
    
    // Simulate download progress
    let chunk_size = 1048576; // 1 MB chunks
    for i in 1..=10 {
        download.update_progress(i * chunk_size);
        println!("  [{:3}%] Downloaded {} / {} MB", 
                 download.progress_percent(),
                 download.current_bytes / 1048576,
                 download.info.length / 1048576);
        
        if i == 5 {
            println!("       ... continuing ...");
        }
    }
    
    println!("  ✅ Download completed: {}", download.completed);
    
    // Multiple files example
    println!("\n📂 Multiple Files Download:");
    let files = vec![
        ("Maps.dat", 20971520, 10485760),
        ("Textures.pak", 52428800, 26214400),
        ("Sounds.pak", 31457280, 15728640),
    ];
    
    let mut downloads: Vec<Download> = files.iter().map(|(name, len, comp)| {
        let info = FileInformation::new(
            name.to_string(),
            *len,
            *comp,
            Utc::now(),
        );
        Download::new(info)
    }).collect();
    
    // Simulate downloads at different progress
    downloads[0].update_progress(20971520);  // 100%
    downloads[1].update_progress(26214400);  // 50%
    downloads[2].update_progress(0);          // 0%
    
    for (i, dl) in downloads.iter().enumerate() {
        let status = if dl.completed { "✅" } else { "⏳" };
        println!("  {} [{}%] {} ({:.1} MB)", 
                 status,
                 dl.progress_percent(),
                 dl.info.file_name,
                 dl.info.length as f64 / 1048576.0);
    }
    
    let total_completed = downloads.iter().filter(|d| d.completed).count();
    println!("\n📊 Overall: {} / {} files completed", total_completed, downloads.len());
    
    println!("\n✅ Example completed successfully!");
    
    Ok(())
}
