// HTTP Download Example - Demonstrates downloader module usage

use client_rust::downloader::{Downloader, DownloadProgress, format_bytes, format_speed};
use tokio::sync::mpsc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    println!("=== HTTP Downloader Example ===\n");
    
    // Example 1: Download single file
    println!("Example 1: Single file download");
    download_single_file().await?;
    
    println!("\n");
    
    // Example 2: Download multiple files concurrently
    println!("Example 2: Concurrent downloads");
    download_multiple_files().await?;
    
    println!("\nAll examples completed!");
    Ok(())
}

/// Download a single file with progress tracking
async fn download_single_file() -> anyhow::Result<()> {
    let downloader = Downloader::new(1);
    
    // Create progress channel
    let (tx, mut rx) = mpsc::unbounded_channel::<DownloadProgress>();
    
    // Spawn progress monitor
    let monitor = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.completed {
                println!(
                    "✓ {} - Complete ({})",
                    progress.file_name,
                    format_bytes(progress.downloaded)
                );
            } else {
                println!(
                    "  {} - {}% ({} at {})",
                    progress.file_name,
                    progress.percent(),
                    format_bytes(progress.downloaded),
                    format_speed(progress.speed)
                );
            }
            
            if let Some(error) = progress.error {
                println!("✗ Error: {}", error);
            }
        }
    });
    
    // Download a test file (use a small public file)
    let url = "https://httpbin.org/bytes/1024"; // 1KB test file
    let dest = PathBuf::from("target/test_download.bin");
    
    println!("Downloading from: {}", url);
    downloader.download_file(&url, &dest, Some(tx)).await?;
    
    // Wait for progress monitor to finish
    monitor.await?;
    
    // Verify file exists
    if dest.exists() {
        let size = std::fs::metadata(&dest)?.len();
        println!("File downloaded successfully: {} bytes", size);
        // Clean up
        std::fs::remove_file(&dest)?;
    }
    
    Ok(())
}

/// Download multiple files concurrently
async fn download_multiple_files() -> anyhow::Result<()> {
    let downloader = Downloader::new(3); // 3 concurrent downloads
    
    // Create progress channel
    let (tx, mut rx) = mpsc::unbounded_channel::<DownloadProgress>();
    
    // Spawn progress monitor
    let monitor = tokio::spawn(async move {
        let mut completed_count = 0;
        let total_count = 3;
        
        while let Some(progress) = rx.recv().await {
            if progress.completed {
                completed_count += 1;
                println!(
                    "✓ [{}/{}] {} - Complete ({})",
                    completed_count,
                    total_count,
                    progress.file_name,
                    format_bytes(progress.downloaded)
                );
            } else {
                println!(
                    "  {} - {}% ({} at {})",
                    progress.file_name,
                    progress.percent(),
                    format_bytes(progress.downloaded),
                    format_speed(progress.speed)
                );
            }
            
            if let Some(error) = progress.error {
                println!("✗ Error downloading {}: {}", progress.file_name, error);
            }
        }
    });
    
    // Prepare download list (use different sizes to show concurrency)
    let downloads = vec![
        (
            "https://httpbin.org/bytes/512".to_string(),
            PathBuf::from("target/test_file1.bin")
        ),
        (
            "https://httpbin.org/bytes/1024".to_string(),
            PathBuf::from("target/test_file2.bin")
        ),
        (
            "https://httpbin.org/bytes/2048".to_string(),
            PathBuf::from("target/test_file3.bin")
        ),
    ];
    
    println!("Starting {} concurrent downloads...", downloads.len());
    downloader.download_files(downloads.clone(), tx).await?;
    
    // Wait for progress monitor
    monitor.await?;
    
    // Clean up
    for (_, path) in downloads {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    
    Ok(())
}
