

use crate::ecs::systems::{System};
use ggez::GameResult;

pub struct PlayerControlSystem;

impl System for PlayerControlSystem {
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
       
        Ok(())
    }
}