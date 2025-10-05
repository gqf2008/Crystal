// particles/mod.rs
// 
// 粒子类型模块入口
// 
// 简化设计：直接使用 Particle 结构体，与 C# 原版一致
// C# 使用继承但所有粒子行为相同，只是初始化参数不同
// Rust 不需要子类，直接在 ParticleEngine 中设置参数

pub mod particle;

pub use particle::{Particle, BlendMode};
