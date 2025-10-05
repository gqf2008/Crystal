// Example: Using browser helper utilities
// Run with: cargo run --example browser_helper_example

use mir2_client::utils::{open_default_browser, open_chrome_browser};

fn main() -> anyhow::Result<()> {
    println!("=== Browser Helper Example ===\n");
    
    // Example URLs
    let urls = [
        ("Official Website", "https://www.rust-lang.org"),
        ("Documentation", "https://doc.rust-lang.org"),
        ("GitHub", "https://github.com"),
    ];
    
    println!("Available browsers:");
    println!("1. Default Browser");
    println!("2. Google Chrome");
    println!("3. Exit");
    println!("\nNote: This is a demo. Uncomment the code below to actually open browsers.\n");
    
    // Demonstrate usage (commented out to avoid opening browsers during demo)
    
    // Open in default browser
    println!("To open in default browser:");
    println!("  open_default_browser(\"https://www.rust-lang.org\")?;");
    // Uncomment to actually open:
    // open_default_browser(urls[0].1)?;
    
    println!("\nTo open in Chrome:");
    println!("  open_chrome_browser(\"https://www.rust-lang.org\")?;");
    // Uncomment to actually open:
    // open_chrome_browser(urls[0].1)?;
    
    println!("\n✅ Example completed!");
    println!("💡 Tip: Remove #[ignore] from tests to run browser tests manually");
    
    Ok(())
}
