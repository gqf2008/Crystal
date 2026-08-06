//! 工具模块

pub mod ime;
pub mod logging;

// .NET DateTime ticks → Unix milliseconds 转换
// .NET DateTime 以 100-nanosecond ticks 为单位，从 0001-01-01 开始计数
const DOTNET_TICKS_AT_UNIX_EPOCH: i64 = 621_355_968_000_000_000i64;
const TICKS_PER_MILLISECOND: i64 = 10_000i64;

/// 将 .NET DateTime ticks 转换为 Unix 毫秒时间戳
///
/// 输入值也可能是已经是 Unix 毫秒（服务器有时发送两种格式），
/// 通过阈值自动判断：大于 1e15 视为 .NET ticks，否则视为 Unix ms。
pub fn dotnet_ticks_to_unix_ms(ticks: i64) -> i64 {
    if ticks > 1_000_000_000_000_000i64 {
        (ticks - DOTNET_TICKS_AT_UNIX_EPOCH) / TICKS_PER_MILLISECOND
    } else {
        ticks
    }
}
