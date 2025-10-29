use crate::ecs::systems::{System};
use ggez::GameResult;

pub struct NetworkRecvSystem;

impl System for NetworkRecvSystem {
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
       
        Ok(())
    }
}