// Test rodio API
use rodio::{OutputStream, Sink};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test OutputStream API
    let (_stream, handle) = OutputStream::try_default()?;
    
    // Test Sink API
    let sink = Sink::connect_new(&handle);
    
    println!("Rodio API test successful!");
    println!("Handle type: {}", std::any::type_name_of_val(&handle));
    
    Ok(())
}
