// Test map libraries loading
// Run with: cargo run --example test_map_libraries

use std::sync::{Arc, Mutex};

// Simplified MLibrary stub for testing
struct MLibrary {
    count: usize,
}

impl MLibrary {
    fn open(path: &str) -> Result<Self, std::io::Error> {
        // Check if file exists
        let lib_path = format!("{}.lib", path);
        if std::path::Path::new(&lib_path).exists() {
            // Fake count for testing
            Ok(Self { count: 100 })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Library not found: {}", lib_path)
            ))
        }
    }
    
    fn count(&self) -> usize {
        self.count
    }
}

fn main() {
    println!("=== Testing Map Libraries Loading ===\n");
    
    let mut tiles_libraries: Vec<Arc<Mutex<MLibrary>>> = Vec::new();
    let mut loaded_count = 0;
    
    println!("📚 WemadeMir2 Libraries:");
    
    // WemadeMir2: MapLibs[0] = Tiles
    if let Ok(lib) = MLibrary::open("Data/Map/WemadeMir2/Tiles") {
        println!("  ✅ [0] Tiles.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    } else {
        println!("  ❌ [0] Tiles.lib not found");
    }
    
    // WemadeMir2: MapLibs[1] = SmTiles
    if let Ok(lib) = MLibrary::open("Data/Map/WemadeMir2/SmTiles") {
        println!("  ✅ [1] SmTiles.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    } else {
        println!("  ❌ [1] SmTiles.lib not found");
    }
    
    // WemadeMir2: MapLibs[2] = Objects
    if let Ok(lib) = MLibrary::open("Data/Map/WemadeMir2/Objects") {
        println!("  ✅ [2] Objects.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    } else {
        println!("  ❌ [2] Objects.lib not found");
    }
    
    // WemadeMir2: MapLibs[3-29] = Objects2-27
    for i in 2..=27 {
        let path = format!("Data/Map/WemadeMir2/Objects{}", i);
        if let Ok(lib) = MLibrary::open(&path) {
            println!("  ✅ [{}] Objects{}.lib ({} images)", i + 1, i, lib.count());
            tiles_libraries.push(Arc::new(Mutex::new(lib)));
            loaded_count += 1;
        }
    }
    
    // WemadeMir2: MapLibs[90] = Objects_32bit
    if let Ok(lib) = MLibrary::open("Data/Map/WemadeMir2/Objects_32bit") {
        println!("  ✅ [90] Objects_32bit.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    }
    
    println!("\n📚 ShandaMir2 Libraries:");
    
    // ShandaMir2: MapLibs[100] = Tiles
    if let Ok(lib) = MLibrary::open("Data/Map/ShandaMir2/Tiles") {
        println!("  ✅ [100] Tiles.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    }
    
    // ShandaMir2: MapLibs[101-109] = Tiles2-9
    for i in 1..=9 {
        let path = format!("Data/Map/ShandaMir2/Tiles{}", i + 1);
        if let Ok(lib) = MLibrary::open(&path) {
            println!("  ✅ [{}] Tiles{}.lib ({} images)", 100 + i, i + 1, lib.count());
            tiles_libraries.push(Arc::new(Mutex::new(lib)));
            loaded_count += 1;
        }
    }
    
    // ShandaMir2: MapLibs[110] = SmTiles
    if let Ok(lib) = MLibrary::open("Data/Map/ShandaMir2/SmTiles") {
        println!("  ✅ [110] SmTiles.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    }
    
    // ShandaMir2: MapLibs[111-119] = SmTiles2-9
    for i in 1..=9 {
        let path = format!("Data/Map/ShandaMir2/SmTiles{}", i + 1);
        if let Ok(lib) = MLibrary::open(&path) {
            println!("  ✅ [{}] SmTiles{}.lib ({} images)", 110 + i, i + 1, lib.count());
            tiles_libraries.push(Arc::new(Mutex::new(lib)));
            loaded_count += 1;
        }
    }
    
    // ShandaMir2: MapLibs[120] = Objects
    if let Ok(lib) = MLibrary::open("Data/Map/ShandaMir2/Objects") {
        println!("  ✅ [120] Objects.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    }
    
    // ShandaMir2: MapLibs[121-150] = Objects2-30
    for i in 1..=30 {
        let path = format!("Data/Map/ShandaMir2/Objects{}", i + 1);
        if let Ok(lib) = MLibrary::open(&path) {
            println!("  ✅ [{}] Objects{}.lib ({} images)", 120 + i, i + 1, lib.count());
            tiles_libraries.push(Arc::new(Mutex::new(lib)));
            loaded_count += 1;
        }
    }
    
    // ShandaMir2: MapLibs[190] = AniTiles1
    if let Ok(lib) = MLibrary::open("Data/Map/ShandaMir2/AniTiles1") {
        println!("  ✅ [190] AniTiles1.lib ({} images)", lib.count());
        tiles_libraries.push(Arc::new(Mutex::new(lib)));
        loaded_count += 1;
    }
    
    println!("\n=== Summary ===");
    println!("📚 Total libraries loaded: {}", loaded_count);
    println!("📦 Vec length: {}", tiles_libraries.len());
    println!("\n✅ Map libraries loading test complete!");
    
    // Test index mapping
    println!("\n=== Testing Index Mapping ===");
    test_index_mapping(tiles_libraries.len());
}

fn test_index_mapping(vec_len: usize) {
    let test_indices = vec![
        0, 1, 2, 5, 15, 29, 90,
        100, 105, 109, 110, 115, 119,
        120, 130, 150, 190
    ];
    
    for file_index in test_indices {
        if let Some(vec_index) = map_file_index_to_lib_index(file_index) {
            let status = if vec_index < vec_len { "✅" } else { "❌ OUT OF BOUNDS" };
            println!("  MapLibs[{:3}] → Vec[{:2}] {}", file_index, vec_index, status);
        } else {
            println!("  MapLibs[{:3}] → ❌ UNMAPPED", file_index);
        }
    }
}

fn map_file_index_to_lib_index(file_index: i32) -> Option<usize> {
    let vec_index = match file_index {
        // WemadeMir2 libraries
        0 => 0,       // Tiles
        1 => 1,       // SmTiles
        2..=29 => file_index as usize,  // Objects, Objects2-27
        90 => 29,     // Objects_32bit (special index)
        
        // ShandaMir2 Tiles libraries
        100..=109 => 30 + (file_index - 100) as usize,
        
        // ShandaMir2 SmTiles libraries
        110..=119 => 40 + (file_index - 110) as usize,
        
        // ShandaMir2 Objects libraries
        120..=150 => 50 + (file_index - 120) as usize,
        
        // ShandaMir2 animated tiles
        190 => 81,
        
        _ => return None,
    };
    
    Some(vec_index)
}
