use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// 派生宏：将System标记为纯逻辑系统
///
/// **编译期验证**: 要求类型已实现 `LogicSystem` trait，否则编译失败。
///
/// # 用法
/// ```rust
/// #[derive(LogicSystem)]
/// struct MovementSystem;
///
/// impl LogicSystem for MovementSystem {
///     fn priority(&self) -> u32 { priority::MOVEMENT }
///
///     fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
///         // 逻辑代码
///         Ok(())
///     }
/// }
/// ```
///
/// # 编译期检查
/// 此宏会生成一个 trait bound 约束，确保类型实现了 `LogicSystem` trait。
/// 如果忘记实现，会在编译时报错而不是在运行时静默失败。
#[proc_macro_derive(LogicSystem)]
pub fn derive_logic_system(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        // 编译期验证：确保 #name 已实现 LogicSystem trait
        // 如果未实现，此行会触发编译错误 "the trait bound `#name: LogicSystem` is not satisfied"
        const _: () = {
            fn assert_impl<T: crate::ecs::systems::LogicSystem>() {}
            fn check() {
                assert_impl::<#name>();
            }
        };

        impl crate::ecs::systems::IntoSystemKind for #name {
            fn into_kind(self: Box<Self>) -> crate::ecs::systems::SystemKind {
                crate::ecs::systems::SystemKind::Update(self)
            }
        }
    };

    TokenStream::from(expanded)
}


/// 派生宏：将系统标记为混合系统（同时需要更新和渲染）
///
/// **编译期验证**: 要求类型已实现 `RenderSystem` trait，否则编译失败。
///
/// # 用法
/// ```rust
/// #[derive(RenderSystem)]
/// struct ParticleSystem;
///
/// impl RenderSystem for ParticleSystem {
///     fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
///         // 更新粒子状态
///         Ok(())
///     }
///
///     fn draw(&mut self, world: &hecs::World) -> GameResult {
///         // 绘制粒子
///         Ok(())
///     }
/// }
/// ```
///
/// # 编译期检查
/// 此宏会生成一个 trait bound 约束，确保类型实现了 `RenderSystem` trait。
/// 如果忘记实现，会在编译时报错而不是在运行时静默失败。
#[proc_macro_derive(RenderSystem)]
pub fn derive_render_system(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        // 编译期验证：确保 #name 已实现 RenderSystem trait
        // 如果未实现，此行会触发编译错误 "the trait bound `#name: RenderSystem` is not satisfied"
        const _: () = {
            fn assert_impl<T: crate::ecs::systems::RenderSystem>() {}
            fn check() {
                assert_impl::<#name>();
            }
        };

        impl crate::ecs::systems::IntoSystemKind for #name {
            fn into_kind(self: Box<Self>) -> crate::ecs::systems::SystemKind {
                crate::ecs::systems::SystemKind::Render(self)
            }
        }
    };

    TokenStream::from(expanded)
}
