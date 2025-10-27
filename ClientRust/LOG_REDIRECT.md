# Rust 日志重定向到文件

使用 `env_logger` 或修改代码将日志输出到文件。

## 方法1: 使用 PowerShell 重定向

```powershell
cd ClientRust
cargo run --package mir2_client --bin mir2x 2>&1 | Tee-Object -FilePath game.log
```

## 方法2: 在代码中配置 env_logger

修改 `src/bin/mir2x/main.rs`:

```rust
use std::fs::File;
use env_logger::Builder;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 日志输出到文件
    let log_file = File::create("game.log")?;
    Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    
    // ... 其余代码
}
```

## 方法3: 使用 >> 重定向(PowerShell)

```powershell
cargo run --package mir2_client --bin mir2x > game.log 2>&1
```

## 方法4: 实时监控 + 保存

```powershell
cargo run --package mir2_client --bin mir2x 2>&1 | Tee-Object -FilePath "game_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
```

## 推荐: 使用 PowerShell Tee-Object

可以同时看到输出和保存到文件:
```powershell
$timestamp = Get-Date -Format 'yyyyMMdd_HHmmss'
cargo run --package mir2_client --bin mir2x 2>&1 | Tee-Object -FilePath "logs/game_$timestamp.log"
```
