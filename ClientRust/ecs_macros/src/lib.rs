use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// 派生宏：将System标记为纯逻辑系统
/// 
/// # 用法
/// ```rust
/// #[derive(LogicSystem)]
/// struct MovementSystem;
/// 
/// impl System for MovementSystem {
///     fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
///         // 逻辑代码
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_derive(LogicSystem)]
pub fn derive_logic_system(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let expanded = quote! {
        impl IntoSystemKind for #name {
            fn into_kind(self: Box<Self>) -> SystemKind {
                SystemKind::Update(self)
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// 派生宏：将DrawSystem标记为纯渲染系统
/// 
/// # 用法
/// ```rust
/// #[derive(RenderSystem)]
/// struct MapRenderSystem;
/// 
/// impl DrawSystem for MapRenderSystem {
///     fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
///         // 渲染代码
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_derive(RenderSystem)]
pub fn derive_render_system(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let expanded = quote! {
        impl IntoSystemKind for #name {
            fn into_kind(self: Box<Self>) -> SystemKind {
                SystemKind::Draw(self)
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// 派生宏：将系统标记为混合系统（同时需要更新和渲染）
/// 
/// **编译期强制要求实现 HybridSystem trait**
/// 
/// # 用法
/// ```rust
/// #[derive(HybridSystem)]
/// struct ParticleSystem;
/// 
/// impl HybridSystem for ParticleSystem {
///     // ✅ 必须实现update() - 否则编译失败
///     fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
///         // 更新粒子状态
///         Ok(())
///     }
///     
///     // ✅ 必须实现draw() - 否则编译失败
///     fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
///         // 绘制粒子
///         Ok(())
///     }
/// }
/// ```
/// 
/// # 编译期检查
/// 此宏要求：
/// 1. 必须实现 `HybridSystem` trait（包含 update 和 draw 两个方法）
/// 2. 两个方法都没有默认实现，必须显式编写
/// 
/// # 选择正确的宏
/// - 只需要update → 使用 `#[derive(LogicSystem)]` + 实现 `System`
/// - 只需要draw → 使用 `#[derive(RenderSystem)]` + 实现 `DrawSystem`
/// - 需要update和draw → 使用 `#[derive(HybridSystem)]` + 实现 `HybridSystem`
#[proc_macro_derive(HybridSystem)]
pub fn derive_hybrid_system(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let expanded = quote! {
        impl IntoSystemKind for #name {
            fn into_kind(self: Box<Self>) -> SystemKind {
                SystemKind::Hybrid(self)
            }
        }
    };
    
    TokenStream::from(expanded)
}
