// Test rodio API
use rodio::{OutputStream, Sink};

fn main() {
    // Test OutputStream
    let result = OutputStream::try_default();
    println!("OutputStream::try_default() result type: {:?}", std::any::type_name_of_val(&result));
    
    if let Ok((stream, handle)) = result {
        println!("Stream type: {}", std::any::type_name_of_val(&stream));
        println!("Handle type: {}", std::any::type_name_of_val(&handle));
        
        // Test Sink
        let sink = Sink::connect_new(&handle);
        println!("Sink type: {}", std::any::type_name_of_val(&sink));
    }
}
