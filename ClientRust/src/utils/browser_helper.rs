// Browser Helper - Open URLs in web browsers
// Corresponds to: Client/Utils/BrowserHelper.cs

use anyhow::Result;

/// Open a URL in the system's default browser
/// 
/// This function attempts to open the URL using the system's default handler.
/// On Windows, it uses the shell's "open" command.
/// On macOS, it uses "open".
/// On Linux, it tries "xdg-open".
pub fn open_default_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Windows: use cmd /c start
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS: use open command
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux: try xdg-open
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }
    
    tracing::info!("Opened URL in browser: {}", url);
    Ok(())
}

/// Open a URL in Google Chrome specifically
/// Falls back to default browser if Chrome is not available
pub fn open_chrome_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Try to launch Chrome on Windows
        let chrome_paths = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];
        
        for path in &chrome_paths {
            if std::path::Path::new(path).exists() {
                match std::process::Command::new(path)
                    .arg(url)
                    .spawn()
                {
                    Ok(_) => {
                        tracing::info!("Opened URL in Chrome: {}", url);
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to open Chrome at {}: {}", path, e);
                    }
                }
            }
        }
        
        // Fallback to default browser
        tracing::warn!("Chrome not found, using default browser");
        return open_default_browser(url);
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows, try to find chrome in PATH
        match std::process::Command::new("google-chrome")
            .arg(url)
            .spawn()
            .or_else(|_| std::process::Command::new("chrome").arg(url).spawn())
        {
            Ok(_) => {
                tracing::info!("Opened URL in Chrome: {}", url);
                Ok(())
            }
            Err(_) => {
                tracing::warn!("Chrome not found, using default browser");
                open_default_browser(url)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore] // Ignore by default as it opens a browser
    fn test_open_default_browser() {
        let result = open_default_browser("https://www.rust-lang.org");
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore] // Ignore by default as it opens a browser
    fn test_open_chrome_browser() {
        let result = open_chrome_browser("https://www.rust-lang.org");
        assert!(result.is_ok());
    }
}
