

use crate::ecs::systems::{System};
use ggez::GameResult;

pub struct GameEventSystem;

impl System for GameEventSystem {
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
       
        Ok(())
    }
}