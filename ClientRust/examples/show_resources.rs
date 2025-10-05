// Example: Display information about embedded resources
// Run with: cargo run --example show_resources

use mir2_client::resources::Images;
use image::GenericImageView;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Mir2 Client Embedded Resources ===\n");
    
    // Progress bars
    println!("📊 Progress Bars:");
    print_image_info("  Blue Progress", Images::load_blue_progress()?);
    print_image_info("  Green Progress", Images::load_green_progress()?);
    
    // Buttons
    println!("\n🔘 Launch Buttons:");
    print_image_info("  Base State", Images::load_launch_base()?);
    print_image_info("  Hover State", Images::load_launch_hover()?);
    print_image_info("  Pressed State", Images::load_launch_pressed()?);
    
    println!("\n⚙️  Config Buttons:");
    print_image_info("  Base State", Images::load_config_base()?);
    print_image_info("  Hover State", Images::load_config_hover()?);
    print_image_info("  Pressed State", Images::load_config_pressed()?);
    
    println!("\n❌ Close Buttons:");
    print_image_info("  Base State", Images::load_cross_base()?);
    print_image_info("  Hover State", Images::load_cross_hover()?);
    print_image_info("  Pressed State", Images::load_cross_pressed()?);
    
    println!("\n☑️  Checkboxes:");
    print_image_info("  Base State", Images::load_checkf_base2()?);
    print_image_info("  Hover State", Images::load_checkf_hover()?);
    print_image_info("  Pressed State", Images::load_checkf_pressed()?);
    
    println!("\n🖼️  Background:");
    print_image_info("  Server Selection", Images::load_server_base()?);
    
    // Calculate total size
    println!("\n📦 Total Embedded Resources:");
    let total_bytes = Images::blue_progress().len() +
                     Images::green_progress().len() +
                     Images::launch_base().len() +
                     Images::launch_hover().len() +
                     Images::launch_pressed().len() +
                     Images::config_base().len() +
                     Images::config_hover().len() +
                     Images::config_pressed().len() +
                     Images::cross_base().len() +
                     Images::cross_hover().len() +
                     Images::cross_pressed().len() +
                     Images::checkf_base2().len() +
                     Images::checkf_hover().len() +
                     Images::checkf_pressed().len() +
                     Images::server_base().len() +
                     Images::config_base1().len() +
                     Images::config_check_off1().len() +
                     Images::config_check_on().len() +
                     Images::config_radio_on().len() +
                     Images::new_progress_end_blue().len() +
                     Images::new_progress_end_green().len() +
                     Images::pfffft().len() +
                     Images::radio_unactive().len() +
                     Images::textboxes().len();
    
    println!("  Total: {} KB ({} bytes)", total_bytes / 1024, total_bytes);
    println!("  Resources: 25 PNG images");
    
    Ok(())
}

fn print_image_info(name: &str, img: image::DynamicImage) {
    let (width, height) = img.dimensions();
    println!("{}: {}x{} pixels", name, width, height);
}
