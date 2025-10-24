// HTTP Downloader - Async file download with progress tracking
// Used by LauncherWindow for patch file downloads

use anyhow::{Context, Result};
use reqwest::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Download progress update
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// File being downloaded
    pub file_name: String,
    
    /// Bytes downloaded so far
    pub downloaded: u64,
    
    /// Total bytes (if known)
    pub total: Option<u64>,
    
    /// Download speed (bytes per second)
    pub speed: u64,
    
    /// Whether download is complete
    pub completed: bool,
    
    /// Error message if failed
    pub error: Option<String>,
}

impl DownloadProgress {
    /// Get progress percentage (0-100)
    pub fn percent(&self) -> u8 {
        if let Some(total) = self.total {
            if total > 0 {
                return ((self.downloaded as f64 / total as f64) * 100.0).min(100.0) as u8;
            }
        }
        0
    }
}

/// HTTP downloader with progress tracking
pub struct Downloader {
    client: Client,
    concurrent_limit: usize,
}

impl Downloader {
    /// Create a new downloader
    /// 
    /// # Arguments
    /// * `concurrent_limit` - Maximum number of concurrent downloads (default: 1)
    pub fn new(concurrent_limit: usize) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            concurrent_limit: concurrent_limit.max(1),
        }
    }
    
    /// Download a single file
    /// 
    /// # Arguments
    /// * `url` - URL to download from
    /// * `dest_path` - Destination file path
    /// * `progress_tx` - Channel to send progress updates (optional)
    pub async fn download_file(
        &self,
        url: &str,
        dest_path: &Path,
        progress_tx: Option<mpsc::UnboundedSender<DownloadProgress>>,
    ) -> Result<()> {
        let file_name = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        tracing::debug!("Downloading {} to {:?}", url, dest_path);
        
        // Send initial progress
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress {
                file_name: file_name.clone(),
                downloaded: 0,
                total: None,
                speed: 0,
                completed: false,
                error: None,
            });
        }
        
        // Start download
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to send request")?;
        
        if !response.status().is_success() {
            let error_msg = format!("HTTP error: {}", response.status());
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress {
                    file_name: file_name.clone(),
                    downloaded: 0,
                    total: None,
                    speed: 0,
                    completed: false,
                    error: Some(error_msg.clone()),
                });
            }
            anyhow::bail!(error_msg);
        }
        
        let total_size = response.content_length();
        
        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Create output file
        let mut file = File::create(dest_path)
            .await
            .context("Failed to create output file")?;
        
        // Download with progress tracking
        let downloaded = Arc::new(AtomicU64::new(0));
        let start_time = std::time::Instant::now();
        
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;
            
            file.write_all(&chunk)
                .await
                .context("Failed to write to file")?;
            
            let current = downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;
            
            // Calculate speed
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (current as f64 / elapsed) as u64
            } else {
                0
            };
            
            // Send progress update
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress {
                    file_name: file_name.clone(),
                    downloaded: current,
                    total: total_size,
                    speed,
                    completed: false,
                    error: None,
                });
            }
        }
        
        file.flush().await?;
        
        let final_size = downloaded.load(Ordering::Relaxed);
        tracing::info!("Downloaded {} ({} bytes)", file_name, final_size);
        
        // Send completion
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress {
                file_name: file_name.clone(),
                downloaded: final_size,
                total: Some(final_size),
                speed: 0,
                completed: true,
                error: None,
            });
        }
        
        Ok(())
    }
    
    /// Download multiple files concurrently
    /// 
    /// # Arguments
    /// * `downloads` - List of (url, dest_path) pairs
    /// * `progress_tx` - Channel to send progress updates
    pub async fn download_files(
        &self,
        downloads: Vec<(String, std::path::PathBuf)>,
        progress_tx: mpsc::UnboundedSender<DownloadProgress>,
    ) -> Result<()> {
        use tokio::task::JoinSet;
        
        let mut tasks = JoinSet::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrent_limit));
        
        for (url, dest_path) in downloads {
            let client = self.client.clone();
            let tx = progress_tx.clone();
            let permit = semaphore.clone().acquire_owned().await?;
            
            tasks.spawn(async move {
                let result = {
                    let downloader = Downloader {
                        client,
                        concurrent_limit: 1, // Not used for single download
                    };
                    downloader.download_file(&url, &dest_path, Some(tx)).await
                };
                
                drop(permit); // Release semaphore
                result
            });
        }
        
        // Wait for all downloads to complete
        let mut errors = Vec::new();
        
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(_)) => {
                    // Download succeeded
                }
                Ok(Err(e)) => {
                    tracing::error!("Download failed: {}", e);
                    errors.push(e);
                }
                Err(e) => {
                    tracing::error!("Task panicked: {}", e);
                    errors.push(anyhow::anyhow!("Task panicked: {}", e));
                }
            }
        }
        
        if !errors.is_empty() {
            anyhow::bail!("{} download(s) failed", errors.len());
        }
        
        Ok(())
    }
}

/// Helper function to format bytes
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Helper function to format speed
pub fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
    
    #[test]
    fn test_progress_percent() {
        let progress = DownloadProgress {
            file_name: "test.txt".to_string(),
            downloaded: 50,
            total: Some(100),
            speed: 0,
            completed: false,
            error: None,
        };
        assert_eq!(progress.percent(), 50);
        
        let progress_unknown = DownloadProgress {
            file_name: "test.txt".to_string(),
            downloaded: 50,
            total: None,
            speed: 0,
            completed: false,
            error: None,
        };
        assert_eq!(progress_unknown.percent(), 0);
    }
}
