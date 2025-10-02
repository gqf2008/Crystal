pub mod binary;
pub mod client_data;
pub mod client_packets;
pub mod enums;
pub mod item;
pub mod map;
pub mod packet;
pub mod packet_ids;
pub mod protocol_packets; // New modular packet structure from ClientRust
pub mod server_packets; // Old implementation (will be deprecated)
pub mod stats;
pub mod world_map;

#[allow(unused_imports)]
pub use crate::{
    binary::*, client_data::*, client_packets::*, enums::*, item::*, map::*, packet::*,
    packet_ids::*, protocol_packets::*, server_packets::*, stats::*, world_map::*,
};
