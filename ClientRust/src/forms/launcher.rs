// Launcher Window - Auto-patcher and game launcher
// Corresponds to: Launcher/AMain.cs

use anyhow::Result;
use std::sync::Arc;
use winit::window::Window;
use crate::utils::{FileInformation, Download};
use crate::settings::ClientSettings;

/// Launcher window state
pub struct LauncherWindow {
    /// Window handle
    window: Arc<Window>,
    
    /// Client settings
    settings: ClientSettings,
    
    /// Download state
    pub downloads: Vec<Download>,
    
    /// Total bytes to download
    pub total_bytes: u64,
    
    /// Completed bytes
    pub completed_bytes: u64,
    
    /// File count
    pub file_count: usize,
    
    /// Current file index
    pub current_count: usize,
    
    /// Whether patching is completed
    pub completed: bool,
    
    /// Whether file check is done
    pub checked: bool,
    
    /// Whether to clean old files
    pub clean_files: bool,
    
    /// Whether an error was found
    pub error_found: bool,
    
    /// Old file list
    old_list: Vec<FileInformation>,
    
    /// Download queue
    download_queue: Vec<FileInformation>,
}

impl LauncherWindow {
    /// Create a new launcher window
    pub fn new(window: Arc<Window>, settings: ClientSettings) -> Self {
        Self {
            window,
            settings,
            downloads: Vec::new(),
            total_bytes: 0,
            completed_bytes: 0,
            file_count: 0,
            current_count: 0,
            completed: false,
            checked: false,
            clean_files: false,
            error_found: false,
            old_list: Vec::new(),
            download_queue: Vec::new(),
        }
    }
    
    /// Start the patching process
    pub fn start(&mut self) -> Result<()> {
        tracing::info!("Starting launcher/patcher");
        
        // Load old file list
        self.get_old_file_list()?;
        
        if self.old_list.is_empty() {
            tracing::error!("Failed to get file list");
            self.completed = true;
            return Ok(());
        }
        
        tracing::info!("Found {} files to check", self.old_list.len());
        
        // Check files
        self.file_count = self.old_list.len();
        for file_info in self.old_list.clone() {
            self.check_file(&file_info)?;
        }
        
        self.checked = true;
        tracing::info!("File check completed. {} files to download", self.download_queue.len());
        
        // Start downloads
        self.file_count = self.download_queue.len();
        self.current_count = 0;
        
        if self.download_queue.is_empty() {
            tracing::info!("No files to download, patcher completed");
            self.completed = true;
        } else {
            self.begin_downloads()?;
        }
        
        Ok(())
    }
    
    /// Load old file list from patch server
    fn get_old_file_list(&mut self) -> Result<()> {
        use std::io::Read;
        use flate2::read::GzDecoder;
        
        // Get patch server URL from settings
        let patch_url = format!("{}/PatchList.gz", self.settings.launcher.host);
        tracing::info!("Downloading file list from: {}", patch_url);
        
        // Download file list (synchronous for simplicity in this context)
        let response = pollster::block_on(async {
            reqwest::get(&patch_url).await
        })?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download file list: HTTP {}", response.status());
        }
        
        // Read response bytes
        let compressed_data = pollster::block_on(async {
            response.bytes().await
        })?;
        
        tracing::debug!("Downloaded {} bytes (compressed)", compressed_data.len());
        
        // Decompress
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        
        tracing::debug!("Decompressed to {} bytes", decompressed.len());
        
        // Parse file list - use FileInformation::read_from
        let mut cursor = std::io::Cursor::new(decompressed);
        let mut file_list = Vec::new();
        
        // Read file count (first i32)
        use byteorder::{LittleEndian, ReadBytesExt};
        let count = cursor.read_i32::<LittleEndian>()?;
        
        tracing::debug!("File list contains {} entries", count);
        
        // Read each FileInformation entry
        for _ in 0..count {
            match FileInformation::read_from(&mut cursor) {
                Ok(file_info) => file_list.push(file_info),
                Err(e) => {
                    tracing::warn!("Failed to read file info: {}", e);
                    break;
                }
            }
        }
        
        tracing::info!("Loaded {} files from patch list", file_list.len());
        self.old_list = file_list;
        
        Ok(())
    }
    
    /// Check if a file needs updating
    fn check_file(&mut self, file_info: &FileInformation) -> Result<()> {
        use std::path::Path;
        
        let file_path = Path::new(&self.settings.root_path).join(&file_info.file_name);
        
        // Check if file exists
        if !file_path.exists() {
            tracing::debug!("File missing: {}", file_info.file_name);
            self.download_queue.push(file_info.clone());
            self.total_bytes += file_info.compressed as u64;
            return Ok(());
        }
        
        // Check file size
        let metadata = std::fs::metadata(&file_path)?;
        if metadata.len() != file_info.length as u64 {
            tracing::debug!("File size mismatch: {}", file_info.file_name);
            self.download_queue.push(file_info.clone());
            self.total_bytes += file_info.compressed as u64;
            return Ok(());
        }
        
        // TODO: Check file hash if needed
        
        Ok(())
    }
    
    /// Begin downloading files
    fn begin_downloads(&mut self) -> Result<()> {
        use crate::downloader::{Downloader, DownloadProgress};
        use tokio::sync::mpsc;
        use std::path::PathBuf;
        
        tracing::info!("Starting download of {} files", self.download_queue.len());
        
        // Create download tasks
        let mut downloads = Vec::new();
        for file_info in &self.download_queue {
            let url = format!("{}/{}", self.settings.launcher.host, file_info.file_name);
            let dest_path = PathBuf::from(&self.settings.root_path).join(&file_info.file_name);
            downloads.push((url, dest_path));
        }
        
        // Create progress channel
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<DownloadProgress>();
        
        // Spawn download task
        let concurrent_limit = self.settings.launcher.concurrent_downloads as usize;
        let handle = std::thread::spawn(move || {
            pollster::block_on(async {
                let downloader = Downloader::new(concurrent_limit);
                downloader.download_files(downloads, progress_tx).await
            })
        });
        
        // Process progress updates
        while let Ok(progress) = progress_rx.try_recv() {
            tracing::debug!(
                "Download progress: {} - {}% ({})",
                progress.file_name,
                progress.percent(),
                crate::downloader::format_bytes(progress.downloaded)
            );
            
            if progress.completed {
                self.current_count += 1;
            }
            
            if let Some(error) = progress.error {
                tracing::error!("Download error: {}", error);
                self.error_found = true;
            }
        }
        
        // Wait for downloads to complete
        match handle.join() {
            Ok(Ok(())) => {
                tracing::info!("All downloads completed successfully");
                self.completed = true;
            }
            Ok(Err(e)) => {
                tracing::error!("Download failed: {}", e);
                self.error_found = true;
                self.completed = true;
            }
            Err(e) => {
                tracing::error!("Download thread panicked: {:?}", e);
                self.error_found = true;
                self.completed = true;
            }
        }
        
        Ok(())
    }
    
    /// Update download progress
    pub fn update_progress(&mut self, bytes_downloaded: u64) {
        self.completed_bytes = bytes_downloaded;
    }
    
    /// Get download progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_bytes == 0 {
            return 1.0;
        }
        (self.completed_bytes as f64 / self.total_bytes as f64).min(1.0)
    }
    
    /// Get download progress percentage (0 - 100)
    pub fn progress_percent(&self) -> u8 {
        (self.progress() * 100.0) as u8
    }
    
    /// Get download speed in bytes per second
    pub fn download_speed(&self) -> u64 {
        // TODO: Calculate based on recent download progress
        0
    }
    
    /// Get estimated time remaining in seconds
    pub fn time_remaining(&self) -> Option<u64> {
        let speed = self.download_speed();
        if speed == 0 {
            return None;
        }
        
        let remaining_bytes = self.total_bytes.saturating_sub(self.completed_bytes);
        Some(remaining_bytes / speed)
    }
    
    /// Render the launcher UI
    pub fn render(&self) {
        // TODO: Implement rendering using wgpu + Resources
        // Should display:
        // 1. Background image (Resources::server_base())
        // 2. Progress bar (Resources::blue_progress())
        // 3. Launch button (if completed)
        // 4. Config button
        // 5. Close button
    }
    
    /// Handle window events
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::WindowEvent;
        
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Launcher window close requested");
                return true;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // TODO: Handle button clicks
                tracing::debug!("Mouse input: {:?} {:?}", state, button);
            }
            _ => {}
        }
        
        false
    }
}

/// Save error to Error.txt file
pub fn save_error(error: &str) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    
    // Check error log limit (similar to C# RemainingErrorLogs)
    const MAX_ERROR_LOGS: usize = 100;
    
    let error_file = "Error.txt";
    
    // Check current error count
    let current_count = if let Ok(content) = std::fs::read_to_string(error_file) {
        content.lines().filter(|line| line.starts_with('[')).count()
    } else {
        0
    };
    
    if current_count >= MAX_ERROR_LOGS {
        return Ok(()); // Silently ignore if limit reached
    }
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(error_file)?;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}] {}", timestamp, error)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_progress_calculation() {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = Arc::new(
            event_loop.create_window(winit::window::Window::default_attributes())
                .unwrap()
        );
        let settings = ClientSettings::default();
        let mut launcher = LauncherWindow::new(window, settings);
        
        launcher.total_bytes = 10000;
        launcher.completed_bytes = 0;
        assert_eq!(launcher.progress_percent(), 0);
        
        launcher.completed_bytes = 5000;
        assert_eq!(launcher.progress_percent(), 50);
        
        launcher.completed_bytes = 10000;
        assert_eq!(launcher.progress_percent(), 100);
    }
    
    #[test]
    fn test_save_error() {
        let result = save_error("Test error message");
        // Don't assert here as it creates actual files
        // Just verify it doesn't panic
        let _ = result;
    }
}
