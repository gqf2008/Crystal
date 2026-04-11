// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

// 简化网络模块
pub mod handlers;         // NetworkEvent 定义
pub mod builder;          // NetworkBuilder + NetContext
mod client;               // 内部实现：Network (Read + Write + 两线程)
mod mock;                 // 模拟网络实现（用于开发工具）

// 导出
pub use builder::{NetworkBuilder, NetContext};
pub use handlers::NetworkEvent;

use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::path::Path;
use mir2_shared::SelectInfo;

static GLOBAL_NET: Lazy<Mutex<Option<NetContext>>> = Lazy::new(|| Mutex::new(None));
static GLOBAL_CHARACTERS: Lazy<Mutex<Option<Vec<SelectInfo>>>> = Lazy::new(|| Mutex::new(None));

pub(crate) fn read_config_ini() -> Option<String> {
	// 优先读取当前工作目录（方便用户直接在运行目录放 config.ini）
	if let Ok(content) = std::fs::read_to_string("config.ini") {
		return Some(content);
	}

	// 回退到 crate 根目录（即 Client-Macroquad/），避免从仓库根目录启动时读不到配置。
	let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini");
	std::fs::read_to_string(manifest_path).ok()
}

#[derive(Debug, Clone)]
pub struct NetworkRuntimeConfig {
	pub server_addr: String,
	pub use_mock: bool,
	/// ClientVersion 的 MD5(16 bytes)，用于通过服务端版本校验。
	///
	/// 对应服务端 `Settings.VersionHashes`，如果服务端 `CheckVersion=true`，则必须匹配其中之一。
	pub client_version_hash: [u8; 16],
	/// 远程玩家走路插值时长（毫秒），用于消除“瞬移感”
	pub remote_interp_walk_ms: u32,
	/// 远程玩家跑路插值时长（毫秒），用于消除“瞬移感”
	pub remote_interp_run_ms: u32,
}

impl Default for NetworkRuntimeConfig {
	fn default() -> Self {
		Self {
			server_addr: "127.0.0.1:7000".to_string(),
			// 默认走 mock：保证离线可跑；需要真服时在 config.ini 设置 UseMock=false
			use_mock: true,
			// 默认 0：如果服务端开启 CheckVersion，这会被拒绝，需要在 config.ini 配置实际 hash
			client_version_hash: [0u8; 16],
			// 默认值匹配当前手感（walk≈0.16s, run≈0.11s）
			remote_interp_walk_ms: 160,
			remote_interp_run_ms: 110,
		}
	}
}

fn parse_hex_16_bytes(value: &str) -> Option<[u8; 16]> {
	let mut s = value.trim();
	if s.starts_with("0x") || s.starts_with("0X") {
		s = &s[2..];
	}

	// 允许中间有空格或分隔符
	let mut hex = String::with_capacity(s.len());
	for ch in s.chars() {
		if ch.is_ascii_hexdigit() {
			hex.push(ch);
		}
	}
	if hex.len() != 32 {
		return None;
	}

	let mut out = [0u8; 16];
	for (i, chunk) in out.iter_mut().enumerate() {
		let bytes = hex.as_bytes();
		let pair = [bytes[i * 2], bytes[i * 2 + 1]];
		let byte_str = std::str::from_utf8(&pair).ok()?;
		*chunk = u8::from_str_radix(byte_str, 16).ok()?;
	}
	Some(out)
}

/// 从 `config.ini` 读取网络运行时配置。
///
/// 支持：
/// - [Network] UseMock=true/false
/// - [Network] ServerAddr=IP:PORT
/// - 兼容键：ServerAddress=IP, ServerPort=7000
/// - [Network] ClientVersionHash=32位HEX (用于服务端 CheckVersion)
/// - [Network] RemoteInterpWalkMs=160
/// - [Network] RemoteInterpRunMs=110
pub fn load_network_runtime_config() -> NetworkRuntimeConfig {
	let mut cfg = NetworkRuntimeConfig::default();

	let Some(content) = read_config_ini() else {
		return cfg;
	};

	let mut section = String::new();
	let mut server_address: Option<String> = None;
	let mut server_port: Option<u16> = None;

	for raw in content.lines() {
		let line = raw.trim();
		if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
			continue;
		}

		if line.starts_with('[') && line.ends_with(']') {
			section = line[1..line.len() - 1].trim().to_string();
			continue;
		}

		let Some((k, v)) = line.split_once('=') else {
			continue;
		};
		let key = k.trim();
		let value = v.trim();

		let in_network = section.eq_ignore_ascii_case("Network") || section.is_empty();
		if !in_network {
			continue;
		}

		if key.eq_ignore_ascii_case("UseMock") {
			cfg.use_mock = match value.to_ascii_lowercase().as_str() {
				"1" | "true" | "yes" | "y" | "on" => true,
				"0" | "false" | "no" | "n" | "off" => false,
				_ => cfg.use_mock,
			};
			continue;
		}

		if key.eq_ignore_ascii_case("ServerAddr") {
			if !value.is_empty() {
				cfg.server_addr = value.to_string();
			}
			continue;
		}

		if key.eq_ignore_ascii_case("ServerAddress") {
			if !value.is_empty() {
				server_address = Some(value.to_string());
			}
			continue;
		}

		if key.eq_ignore_ascii_case("ServerPort") {
			if let Ok(p) = value.parse::<u16>() {
				server_port = Some(p);
			}
			continue;
		}

		if key.eq_ignore_ascii_case("RemoteInterpWalkMs") {
			if let Ok(ms) = value.parse::<u32>() {
				cfg.remote_interp_walk_ms = ms;
			}
			continue;
		}

		if key.eq_ignore_ascii_case("RemoteInterpRunMs") {
			if let Ok(ms) = value.parse::<u32>() {
				cfg.remote_interp_run_ms = ms;
			}
			continue;
		}

		if key.eq_ignore_ascii_case("ClientVersionHash") {
			if let Some(hash) = parse_hex_16_bytes(value) {
				cfg.client_version_hash = hash;
			} else {
				tracing::warn!(
					"Invalid ClientVersionHash '{}', expected 32 hex digits; using default.",
					value
				);
			}
			continue;
		}
	}

	if let (Some(addr), Some(port)) = (server_address, server_port) {
		cfg.server_addr = format!("{}:{}", addr, port);
	}

	cfg
}

/// 设置全局网络上下文（用于场景间移交连接）。
pub fn set_global_net(net: NetContext) {
	let mut guard = GLOBAL_NET.lock().expect("GLOBAL_NET poisoned");
	*guard = Some(net);
}

/// 取走全局网络上下文（用于进入 GameScene 时接管连接）。
pub fn take_global_net() -> Option<NetContext> {
	GLOBAL_NET.lock().expect("GLOBAL_NET poisoned").take()
}

/// 设置全局角色列表（用于 LoginScene -> SelectScene 的数据移交）。
pub fn set_global_characters(characters: Vec<SelectInfo>) {
	let mut guard = GLOBAL_CHARACTERS.lock().expect("GLOBAL_CHARACTERS poisoned");
	*guard = Some(characters);
}

/// 取走全局角色列表（用于创建 SelectScene）。
pub fn take_global_characters() -> Option<Vec<SelectInfo>> {
	GLOBAL_CHARACTERS.lock().expect("GLOBAL_CHARACTERS poisoned").take()
}
