// Example: How to use GameClient with the protocol.rs system
// 示例：如何使用 GameClient 和 protocol.rs 系统
//
// This example demonstrates the complete flow from raw packet bytes
// to game state updates and UI events.

use crate::network::{GameClient, GameEvent};

/// Example 1: Basic packet handling
/// 示例 1：基础数据包处理
#[allow(unused_variables, unused_mut)]
pub async fn example_basic() {
    let mut client = GameClient::new();
    
    // Simulate receiving a packet from the network
    // In real code, this would come from a TCP socket
    let packet_data: Vec<u8> = vec![/* raw packet bytes */];
    
    // Note: dispatch_packet signature is actually:
    // pub fn dispatch_packet(data: &[u8], handler: &mut dyn PacketHandler, debug: bool)
    // For this example, we'd use:
    // match dispatch_packet(&packet_data, &mut client, false) {
    //     Ok(()) => println!("✅ Packet processed successfully"),
    //     Err(e) => eprintln!("❌ Packet error: {}", e),
    // }
    
    println!("✅ Client created and ready to process packets");
    
    // Check game stats
    let stats = client.get_stats();
    println!("Packets received: {}", stats.packets_received);
    println!("Objects in world: {}", stats.objects_count);
    println!("Chat messages: {}", stats.chat_messages_count);
}

/// Example 2: Using event channel for UI updates
/// 示例 2：使用事件通道进行 UI 更新
pub async fn example_with_ui_events() {
    let mut client = GameClient::new();
    
    // Create event channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    client.set_event_channel(tx);
    
    // Spawn task to handle UI events
    let ui_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                GameEvent::Connected => {
                    println!("🎮 Connected to server!");
                }
                
                GameEvent::PlayerSpawned { player } => {
                    println!("👤 Welcome, {}!", player.name);
                    println!("   Level: {}", player.level);
                    println!("   Health: {}/{}", player.health, player.max_health);
                    println!("   Gold: {}", player.gold);
                }
                
                GameEvent::ChatReceived { message } => {
                    println!("[{:?}] {}", message.chat_type, message.text);
                }
                
                GameEvent::PlayerMoved { location } => {
                    println!("📍 Moved to ({}, {})", location.x, location.y);
                }
                
                GameEvent::ObjectSpawned { object } => {
                    match object {
                        crate::network::game_client::GameObject::Player { name, .. } => {
                            println!("👤 Player {} appeared", name);
                        }
                        crate::network::game_client::GameObject::Monster { name, .. } => {
                            println!("👹 Monster {} spawned", name);
                        }
                        crate::network::game_client::GameObject::Npc { name, .. } => {
                            println!("🏪 NPC {} appeared", name);
                        }
                        _ => {}
                    }
                }
                
                GameEvent::ObjectRemoved { object_id } => {
                    println!("🗑️  Object {} removed", object_id);
                }
                
                GameEvent::GroupInviteReceived { inviter } => {
                    println!("💌 {} invited you to join their group", inviter);
                    // In real code, show UI dialog for accept/decline
                }
                
                GameEvent::Disconnected { reason } => {
                    println!("❌ Disconnected: {}", reason);
                    break;
                }
                
                _ => {}
            }
        }
    });
    
    // Process packets (in real code, this would be in a network loop)
    // ... packet processing ...
    
    // Wait for UI task to finish
    let _ = ui_task.await;
}

/// Example 3: Thread-safe shared client (for async environments)
/// 示例 3：线程安全的共享客户端（用于异步环境）
#[allow(unused_variables, unused_mut)]
pub async fn example_shared_client() {
    use crate::network::new_shared_client;
    
    let client = new_shared_client();
    
    // Clone for different async tasks
    let client_clone1 = client.clone();
    let client_clone2 = client.clone();
    
    // Task 1: Network packet receiver
    let network_task = tokio::spawn(async move {
        loop {
            // Receive packet from network
            let packet_data: Vec<u8> = vec![/* ... */];
            
            // Lock and process packet
            let mut client = client_clone1.write().await;
            // dispatch_packet(&packet_data, &mut *client, false)?;
            
            // tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            break; // In real code, this would continue
        }
    });
    
    // Task 2: Game logic update loop
    let game_task = tokio::spawn(async move {
        loop {
            // Read game state
            let client = client_clone2.read().await;
            
            if let Some(player) = &client.player {
                // Update game logic based on player state
                println!("Player health: {}/{}", player.health, player.max_health);
            }
            
            // tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            break; // In real code, this would continue
        }
    });
    
    let _ = tokio::join!(network_task, game_task);
}

/// Example 4: Inspecting game state
/// 示例 4：检查游戏状态
#[allow(dead_code)]
pub fn example_inspect_state(client: &GameClient) {
    // Player info
    if let Some(player) = &client.player {
        println!("=== Player Info ===");
        println!("Name: {}", player.name);
        println!("Level: {}", player.level);
        println!("Location: ({}, {})", player.location.x, player.location.y);
        println!("Health: {}/{}", player.health, player.max_health);
        println!("Mana: {}/{}", player.mana, player.max_mana);
        println!("Experience: {}/{}", player.experience, player.max_experience);
        println!("Gold: {}", player.gold);
        println!("Credit: {}", player.credit);
    }
    
    // Map info
    if let Some(map) = &client.map_info {
        println!("\n=== Map Info ===");
        println!("Title: {}", map.title);
        println!("File: {}", map.file_name);
        println!("Index: {}", map.map_index);
    }
    
    // Group info
    if !client.group.members.is_empty() {
        println!("\n=== Group ({} members) ===", client.group.members.len());
        for member in &client.group.members {
            println!("  - {} (Level {})", member.name, member.level);
        }
    }
    
    // Objects in world
    println!("\n=== World Objects ({}) ===", client.objects.len());
    for (id, obj) in client.objects.iter().take(5) {
        match obj {
            crate::network::game_client::GameObject::Player { name, .. } => {
                println!("  [{}] Player: {}", id, name);
            }
            crate::network::game_client::GameObject::Monster { name, .. } => {
                println!("  [{}] Monster: {}", id, name);
            }
            crate::network::game_client::GameObject::Npc { name, .. } => {
                println!("  [{}] NPC: {}", id, name);
            }
            _ => {}
        }
    }
    
    // Recent chat
    println!("\n=== Recent Chat ({}) ===", client.chat_messages.len());
    for msg in client.chat_messages.iter().rev().take(5) {
        println!("  [{:?}] {}", msg.chat_type, msg.text);
    }
    
    // Statistics
    let stats = client.get_stats();
    println!("\n=== Statistics ===");
    println!("Total packets received: {}", stats.packets_received);
    println!("Objects in world: {}", stats.objects_count);
    println!("Chat messages: {}", stats.chat_messages_count);
}

/// Example 5: Complete game loop skeleton
/// 示例 5：完整的游戏循环骨架
#[allow(unused_variables)]
pub async fn example_game_loop() {
    use tokio::time::{Duration, interval};
    use super::new_shared_client;
    
    let client = new_shared_client();
    
    // Set up event channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    {
        let mut client = client.write().await;
        client.set_event_channel(tx);
    }
    
    // UI event handler
    let ui_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            // Handle UI updates
            match event {
                GameEvent::ChatReceived { message } => {
                    println!("{}", message.text);
                }
                _ => {}
            }
        }
    });
    
    // Network packet receiver (simulated)
    let client_network = client.clone();
    let network_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(10));
        
        loop {
            interval.tick().await;
            
            // In real code: read from TCP socket
            // let packet_data = socket.read().await?;
            
            // Process packet
            // let mut client = client_network.write().await;
            // dispatch_packet(&packet_data, &mut *client)?;
            
            break; // Demo only
        }
    });
    
    // Game logic update loop
    let client_game = client.clone();
    let game_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(16)); // ~60 FPS
        
        loop {
            interval.tick().await;
            
            // Read game state
            let client = client_game.read().await;
            
            // Update game logic, animations, etc.
            // This is where you'd update:
            // - Animation frames
            // - Movement interpolation
            // - Spell effects
            // - Damage numbers
            // - etc.
            
            if let Some(_player) = &client.player {
                // Game logic here
            }
            
            break; // Demo only
        }
    });
    
    // Wait for all tasks
    let _ = tokio::join!(ui_task, network_task, game_task);
}

// =============================================================================
// The beauty of this architecture:
//
// 1. ✅ Zero boilerplate - Just implement the methods you need
// 2. ✅ Type-safe - Compiler checks all packet handling
// 3. ✅ Separation of concerns - Network / State / UI are decoupled
// 4. ✅ Testable - Easy to mock packets for testing
// 5. ✅ Extensible - Add new packets without breaking existing code
// 6. ✅ Performance - Zero-copy packet deserialization
// 7. ✅ Async-ready - Works with tokio and other async runtimes
//
// From 100% protocol coverage to a working game in clean, idiomatic Rust! 🚀
// =============================================================================
