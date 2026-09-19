//! 邮箱死锁加固·源码扫描门禁（上线审查 #23：gate 有界邮箱 1024 + world 内联 ask
//! 构成循环等待）。禁止在 `src/actors/**` 出现对 gate 的阻塞式下行：
//! `tell(SendToClient { .. }).await`——world actor 一旦内联阻塞等 gate 邮箱空位，
//! 而 gate 处理器又内联 ask world，双方互等即死锁（单 actor 邮箱 FIFO）。
//! 下行一律 `try_send`（失败必须 warn! 记录上下文，邮箱满丢包由 gate 会话通道
//! 积满踢线路径兜底）。
//!
//! 扫描模式（第四轮加固版）：先剔除注释与字符串/字符字面量（`'a` 生命周期
//! 不再被误当字符字面量吞掉后续代码），再按 `;` 切语句并把每个语句的所有
//! 空白压缩移除（换行/空格/`tell (` 等写法差异一律归一）。判定规则：
//!   1. 主规则——压缩后的语句同时含 `SendToClient` 与 `.await` 且不含
//!      `try_send` 即违规（覆盖全路径写法 `tell(crate::gate::actor::SendToClient..)`）。
//!   2. 暂存追踪——`let f = <含 SendToClient 的构造/tell>` 语句记录绑定名 f，
//!      后续语句出现 `f.await` 或 `tell(f)`（且无 try_send）同样违规
//!      （覆盖「拆语句 let f=tell(..); f.await;」与「构造消息变量
//!      let m=SendToClient{..}; tell(m).await;」两类绕过）。
//!
//! 第五轮加固：
//!   3. let 暂存追踪扩大到语句任意位置（if/match 块内
//!      `.. { let f = tell(SendToClient{..}); f.await; }` 不再漏判），并支持
//!      解构宽匹配 `let (a, f) = (.., tell(SendToClient{..}))`——模式内全部
//!      标识符登记，宁可过登不可漏登；
//!   4. 剥离器支持原始字符串 `r".." / r#".."# / r##".."##`（含 br 前缀）与字节
//!      字符串 `b".."`——此前 raw string 内容里未转义的引号会腐蚀字符串边界，
//!      把后续真实代码吞进「字符串」而漏判；
//!   5. try_send 豁免改词边界判定——`report_try_send_failure` 之类标识符子串
//!      不再误抑制；`Box::pin(f).await` 消费形态纳入暂存追踪（`(f).await`）。
//!
//! 已知盲区（声明在案，当前代码库无此写法，新增写法评审时人工拦截）：
//! 先声明后赋值 `let f; f = tell(..); f.await`、join!/select! 宏内等待、
//! 跨函数传递 future 后再 await。
//!
//! 刻意保留的白名单例外：`src/actors/world/session.rs` 的 `world.logout_success`
//! 任务内 S.LogOutSuccess → LogOutCleanup 两处刻意 `.await`（FIFO 保序关键路径：
//! 客户端必须先收到 LogOutSuccess 再由 gate 清理会话，同一任务顺序 tell 由 gate
//! 邮箱 FIFO 保序；该任务本身是 spawn 出的 fire-and-forget，world actor 并不内联
//! 阻塞，不构成循环等待）。其中 SendToClient 那一处命中主规则，由豁免表
//! `world/session.rs = 1` 兜底；LogOutCleanup 不是 SendToClient，不命中规则。
//!
//! 红绿说明：本测试在残留未清时天然红（列出全部命中点），清理后转绿；
//! 其他 agent 名下文件尚未迁移的残留以「存量豁免表」按文件计数豁免（只减不增），
//! 豁免表外文件出现任何命中即红。注意：第四轮加固后扫描更宽（修掉了生命周期
//! 吞代码、放开了 tell( 邻接限制），个别文件的存量真实残留可能高于旧豁免计数，
//! 超出的文件由对应 owner 迁移清零后自然转绿——豁免计数只减不增，不得上调。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 存量豁免表：文件（相对 src/actors/，正斜杠）→ 允许的最大残留数。
/// 均为其他 agent 名下文件的未迁移残留；后续清理时应把对应条目计数下调直至删除。
/// 豁免只减不增：任何文件的新增命中都会打破计数断言而变红。
/// player.rs 第四轮已迁移清零（32 处全部 try_send 化），条目移除（缺省即 0）。
/// 第五轮：account/combat/guild/hero/market/social 六文件由对应 lane 迁移清零，
/// 条目移除（account/combat 本就无条目，缺省即 0；guild/hero/market/social 删除）。
fn grandfathered_residuals() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("world/effects.rs", 1),
        ("world/elements.rs", 3),
        ("world/item.rs", 32),
        ("world/mail.rs", 2),
        ("world/npc.rs", 37),
        ("world/npc_script.rs", 4),
        ("world/quest.rs", 2),
        // world.logout_success 任务内 S.LogOutSuccess 的刻意 .await（见文件头注释）
        ("world/session.rs", 1),
        ("world/tick.rs", 19),
    ])
}

/// UTF-8 首字节 → 序列长度（用于多字节字符字面量如 '中' 的判定）
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >= 0xF0 {
        4
    } else if b >= 0xE0 {
        3
    } else if b >= 0xC0 {
        2
    } else {
        1
    }
}

/// 剔除 Rust 源码中的注释与字符串/字符字面量（保留代码结构与分号），
/// 避免扫描误伤注释里的模式说明或字符串内容。
/// 字符字面量与生命周期区分：`'x'` / `'\n'` / `'中'` 是字符字面量（剔除），
/// `'a` / `'static` / `'_` 后跟非 `'` 的是生命周期（保留引号，不得吞掉后续代码）。
/// 原始字符串 `r".." / r#".."# / r##".."##`（含 `br` 前缀）按 # 个数配对闭合
/// 整体剔除——其内容可含未转义引号与反斜杠，按普通字符串处理会腐蚀边界。
fn strip_comments_and_strings(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        // 原始字符串 r".." / r#".."#（含 br".." 形态）：先试探 r 后 0+ 个 # 再
        // 跟引号；不满足则是普通标识符（含 r#ident 原始标识符），按代码放行
        if b == b'r' || (b == b'b' && next == Some(b'r')) {
            let mut j = i + if b == b'b' { 2 } else { 1 };
            let mut hashes = 0usize;
            while bytes.get(j) == Some(&b'#') {
                hashes += 1;
                j += 1;
            }
            if bytes.get(j) == Some(&b'"') {
                j += 1;
                while j < bytes.len() {
                    if bytes[j] == b'"' && (0..hashes).all(|k| bytes.get(j + 1 + k) == Some(&b'#'))
                    {
                        j += 1 + hashes;
                        break;
                    }
                    if bytes[j] == b'\n' {
                        out.push('\n');
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        // 字节字符串 b".."：转义规则与普通字符串一致，同法剔除
        if b == b'b' && next == Some(b'"') {
            i += 2;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }
        match (b, next) {
            // 行注释
            (b'/', Some(b'/')) => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // 块注释（支持嵌套）
            (b'/', Some(b'*')) => {
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        if bytes[i] == b'\n' {
                            out.push('\n');
                        }
                        i += 1;
                    }
                }
            }
            // 字符串字面量
            (b'"', _) => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        if bytes[i] == b'\n' {
                            out.push('\n');
                        }
                        i += 1;
                    }
                }
            }
            // 字符字面量 / 生命周期：先判定形态，字符字面量才剔除
            (b'\'', _) => {
                let b1 = bytes.get(i + 1).copied();
                let is_char = match b1 {
                    // 转义字符字面量：'\n'、'\''、'\u{..}'
                    Some(b'\\') => true,
                    // 多字节 UTF-8 字符字面量：'中'
                    Some(c) if c >= 0x80 => {
                        let len = utf8_len(c);
                        bytes.get(i + 1 + len) == Some(&b'\'')
                    }
                    // 单字节：'x' 形态（次次字节是闭合引号）才是字符字面量，
                    // 否则是生命周期（'a、'static、'_）
                    Some(_) => bytes.get(i + 2) == Some(&b'\''),
                    None => false,
                };
                if is_char {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == b'\\' {
                            i += 2;
                        } else if bytes[i] == b'\'' {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                } else {
                    out.push('\'');
                    i += 1;
                }
            }
            _ => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

/// 词边界包含判定：word 两侧不得紧邻标识符字符
/// （防 `report_try_send_failure` 之类标识符子串误抑制 try_send 豁免）。
fn contains_word(haystack: &str, word: &str) -> bool {
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let hb = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(word) {
        let abs = start + rel;
        let before_ok = abs == 0 || !is_ident(hb[abs - 1]);
        let after = abs + word.len();
        let after_ok = after >= hb.len() || !is_ident(hb[after]);
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// 从 from 起找 let 绑定的等号（跳过 `==` 双等与 `=>` 匹配臂；从左往右
/// 第一个独立 `=` 即绑定等号——比较运算不可能出现在模式与绑定等号之间）。
fn has_binding_equals(b: &[u8], from: usize) -> bool {
    let mut k = from;
    while k < b.len() {
        if b[k] == b'=' {
            let prev_ok = k == 0 || b[k - 1] != b'=';
            let next_ok = k + 1 >= b.len() || !matches!(b[k + 1], b'=' | b'>');
            if prev_ok && next_ok {
                return true;
            }
        }
        k += 1;
    }
    false
}

/// 从原始（未压缩空白）语句中提取全部 let 绑定名。必须看原始语句：
/// 压缩后 `let f` 粘成 `letf`，关键字与绑定名之间没有词边界可判。
///   - 语句任意位置的 let（if/match 块内 `.. { let f = tell(..)` 形态）；
///   - 解构宽匹配：`let (a, f) = (.., tell(SendToClient{..}))` 等 `(`/`[`/`{`
///     开头的模式，登记模式内全部标识符（过滤 mut/ref，宁可过登不可漏登）。
fn extract_let_bindings(stmt: &str) -> Vec<String> {
    let b = stmt.as_bytes();
    let is_ident_start = |x: u8| x.is_ascii_alphabetic() || x == b'_';
    let is_ident = |x: u8| x.is_ascii_alphanumeric() || x == b'_';
    let skip_ws = |mut k: usize| {
        while k < b.len() && b[k].is_ascii_whitespace() {
            k += 1;
        }
        k
    };
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i + 3 <= b.len() {
        let is_let = &b[i..i + 3] == b"let"
            && (i == 0 || !is_ident(b[i - 1]))
            && (i + 3 >= b.len() || !is_ident(b[i + 3]));
        if !is_let {
            i += 1;
            continue;
        }
        let mut j = skip_ws(i + 3);
        // 可选 mut
        if b[j..].starts_with(b"mut") && (j + 3 >= b.len() || !is_ident(b[j + 3])) {
            j = skip_ws(j + 3);
        }
        if j < b.len() && is_ident_start(b[j]) {
            // 普通绑定：let f = ..（类型标注 let f: T = .. 由 has_binding_equals 归一）
            let start = j;
            while j < b.len() && is_ident(b[j]) {
                j += 1;
            }
            if has_binding_equals(b, skip_ws(j)) {
                out.push(stmt[start..j].to_string());
            }
            i = j;
        } else if j < b.len() && matches!(b[j], b'(' | b'[' | b'{') {
            // 解构模式：扫到配平点，收集模式内全部标识符，确认有绑定等号才登记
            let mark = out.len();
            let mut depth = 0i32;
            let mut tok: Option<usize> = None;
            let mut k = j;
            while k < b.len() && (k == j || depth > 0) {
                let x = b[k];
                match x {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
                if is_ident(x) {
                    if tok.is_none() {
                        tok = Some(k);
                    }
                } else if let Some(s) = tok.take() {
                    let t = &stmt[s..k];
                    if t != "mut" && t != "ref" {
                        out.push(t.to_string());
                    }
                }
                k += 1;
            }
            if !has_binding_equals(b, skip_ws(k)) {
                out.truncate(mark);
            }
            i = k.max(j + 1);
        } else {
            i = j.max(i + 3);
        }
    }
    out
}

/// 统计一个源文件中的阻塞下行残留数。
/// 语句切分后先把所有空白压缩移除（归一化），再判定：
///   1. 主规则：语句含 `SendToClient` 且含 `.await` 且不含词边界 `try_send`；
///   2. 暂存追踪：语句任意位置的 `let f = <含 SendToClient>`（含 if/match 块内、
///      解构宽匹配）记录绑定名，后续 `f.await` / `tell(f)` / `Box::pin(f).await`
///      （无词边界 try_send）同样违规。
fn count_blocking_send_to_client(src: &str) -> usize {
    let stripped = strip_comments_and_strings(src);
    let mut pending: Vec<String> = Vec::new();
    let mut hits = 0usize;
    for stmt in stripped.split(';') {
        // 空白归一化：移除全部空白字符，换行/空格写法差异一律消除
        let c: String = stmt.split_whitespace().collect();
        if c.is_empty() {
            continue;
        }
        let has_stc = c.contains("SendToClient");
        let has_await = c.contains(".await");
        let has_try = contains_word(&c, "try_send");
        if has_stc && has_await && !has_try {
            hits += 1;
            continue;
        }
        if has_stc && !has_await {
            // let 绑定暂存：语句任意位置的 let（含块内）+ 解构宽匹配
            // （必须看原始语句 stmt——压缩后 let 与绑定名粘连，无词边界可判）
            for ident in extract_let_bindings(stmt) {
                pending.push(ident);
            }
            continue;
        }
        if has_await && !has_try && !pending.is_empty() {
            for ident in &pending {
                // 消费形态：f.await / tell(f).await / Box::pin(f).await
                if c.contains(&format!("{ident}.await"))
                    || c.contains(&format!("tell({ident})"))
                    || c.contains(&format!("({ident}).await"))
                {
                    hits += 1;
                    break;
                }
            }
        }
    }
    hits
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {:?}: {}", dir, e)) {
        let path = entry.expect("read dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_blocking_tell_send_to_client_await_in_actors() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let actors_dir = manifest.join("src").join("actors");
    let mut files = Vec::new();
    collect_rs_files(&actors_dir, &mut files);
    files.sort();
    assert!(!files.is_empty(), "src/actors 下未扫到任何 .rs 文件");

    let allowed = grandfathered_residuals();
    let mut violations = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(&actors_dir)
            .expect("strip actors prefix")
            .to_string_lossy()
            .replace('\\', "/");
        let src = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {:?}: {}", path, e));
        let count = count_blocking_send_to_client(&src);
        let limit = allowed.get(rel.as_str()).copied().unwrap_or(0);
        if count > limit {
            violations.push(format!(
                "{}: 阻塞残留 {} 处（豁免上限 {}）",
                rel, count, limit
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "发现 tell(SendToClient ..).await 阻塞下行残留（gate 有界邮箱环形等待死锁隐患）:\n  {}",
        violations.join("\n  ")
    );
}

/// 扫描器自检：喂入故意违规/合规样本，验证判定逻辑本身可靠
/// （「没见过失败的守卫是装饰」——扫描器必须先证明自己能红）。
#[test]
fn scanner_detects_blocking_pattern_and_ignores_try_send_and_comments() {
    let blocking = r#"
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: packet,
            })
            .await;
    "#;
    assert_eq!(count_blocking_send_to_client(blocking), 1);

    // 单行形态
    let inline = r#"let _ = self.gate_ref.tell(SendToClient { session_id, data }).await;"#;
    assert_eq!(count_blocking_send_to_client(inline), 1);

    // 全路径形态：tell(crate::gate::actor::SendToClient { .. }).await
    let full_path = r#"
        let _ = self
            .gate_ref
            .tell(crate::gate::actor::SendToClient {
                session_id,
                data: packet,
            })
            .await;
    "#;
    assert_eq!(count_blocking_send_to_client(full_path), 1);

    // 绕过变体①：拆语句暂存 future——let f = tell(..); f.await;
    let split_stmt = r#"
        let f = self.gate_ref.tell(SendToClient { session_id, data });
        f.await;
    "#;
    assert_eq!(count_blocking_send_to_client(split_stmt), 1);

    // 绕过变体②：先构造消息变量——let m = SendToClient { .. }; tell(m).await;
    let msg_var = r#"
        let m = SendToClient { session_id, data };
        self.gate_ref.tell(m).await;
    "#;
    assert_eq!(count_blocking_send_to_client(msg_var), 1);

    // 绕过变体③：换行/多余空白/tell ( 空格写法
    let spaced = "let _ = self.gate_ref\n    .tell ( SendToClient {\n        session_id, data\n    } )\n    . await ;";
    assert_eq!(count_blocking_send_to_client(spaced), 1);

    // 绕过变体④：泛型生命周期 'a 不得被当作字符字面量吞掉后续代码
    let lifetime = r#"
        fn dup<'a>(x: &'a str) -> &'a str { x }
        let _ = self.gate_ref.tell(SendToClient { session_id, data }).await;
    "#;
    assert_eq!(count_blocking_send_to_client(lifetime), 1);

    // 多字节字符字面量（'中'）不得被误判为生命周期而吞掉闭合引号后的代码
    let mb_char = r#"
        let c = '中';
        let _ = self.gate_ref.tell(SendToClient { session_id, data }).await;
    "#;
    assert_eq!(count_blocking_send_to_client(mb_char), 1);

    // 非阻塞 try_send：合规
    let non_blocking = r#"
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id,
                data: packet,
            })
            .try_send();
    "#;
    assert_eq!(count_blocking_send_to_client(non_blocking), 0);

    // 暂存 future 但走 try_send：合规
    let split_try = r#"
        let f = self.gate_ref.tell(SendToClient { session_id, data });
        f.try_send();
    "#;
    assert_eq!(count_blocking_send_to_client(split_try), 0);

    // 构造消息变量但走 try_send：合规
    let msg_var_try = r#"
        let m = SendToClient { session_id, data };
        self.gate_ref.tell(m).try_send();
    "#;
    assert_eq!(count_blocking_send_to_client(msg_var_try), 0);

    // 注释/字符串中出现的模式不得误伤
    let commented = r#"
        // 旧写法：gate_ref.tell(SendToClient { session_id, data }).await;
        /* tell(SendToClient { x }).await; */
        let s = "tell(SendToClient { .. }).await";
        let _ = gate_ref.tell(SendToClient { session_id, data }).try_send();
    "#;
    assert_eq!(count_blocking_send_to_client(commented), 0);

    // ===== 第五轮加固变体：每个红样本都必须真红 =====

    // 变体⑤：if/match 块内 let 绑定——暂存追踪不得只看语句开头
    // （旧版 strip_prefix("let") 漏判）
    let block_let = r#"
        async fn f(&self) {
            if cond {
                let fut = self.gate_ref.tell(SendToClient { session_id, data });
                fut.await;
            }
        }
    "#;
    assert_eq!(count_blocking_send_to_client(block_let), 1);

    // 变体⑥：原始字符串 r#".."# 内容里的未转义引号不得腐蚀剥离器吞掉后续代码
    // （旧版把 `" quote` 之后的真实违规整段当字符串吞掉 → 漏判）
    let raw_str = r##"
        let s = r#"a lone " quote"#;
        let _ = self.gate_ref.tell(SendToClient { session_id, data }).await;
    "##;
    assert_eq!(count_blocking_send_to_client(raw_str), 1);

    // 变体⑦：Box::pin(f).await 消费形态
    let boxed = r#"
        let f = self.gate_ref.tell(SendToClient { session_id, data });
        Box::pin(f).await;
    "#;
    assert_eq!(count_blocking_send_to_client(boxed), 1);

    // 变体⑧：标识符含 try_send 子串不得误抑制（词边界判定）
    let try_substring = r#"
        if !report_try_send_failure {
            let _ = self.gate_ref.tell(SendToClient { session_id, data }).await;
        }
    "#;
    assert_eq!(count_blocking_send_to_client(try_substring), 1);

    // 变体⑨：解构绑定 let (a, f) = (.., tell(SendToClient{..}))——宽匹配登记 f
    let destructure = r#"
        let (a, f) = (helper(), self.gate_ref.tell(SendToClient { session_id, data }));
        f.await;
    "#;
    assert_eq!(count_blocking_send_to_client(destructure), 1);

    // 合规对照①：真 try_send 在词边界判定下仍豁免（不得误伤）
    let real_try = r#"
        if !report_try_send_failure {
            let _ = self.gate_ref.tell(SendToClient { session_id, data }).try_send();
        }
    "#;
    assert_eq!(count_blocking_send_to_client(real_try), 0);

    // 合规对照②：原始字符串里的违规样本不得误伤（剥离后不计）
    let raw_str_inert = r##"
        let s = r#"tell(SendToClient { x }).await"#;
        let _ = gate_ref.tell(SendToClient { session_id, data }).try_send();
    "##;
    assert_eq!(count_blocking_send_to_client(raw_str_inert), 0);

    // 合规对照③：br##".."## 多层 # 原始字符串 + 块内 try_send
    let raw_str_hashes = r###"
        let s = br##"a " b "# c"##;
        async fn f(&self) {
            if cond {
                let _ = self.gate_ref.tell(SendToClient { session_id, data }).try_send();
            }
        }
    "###;
    assert_eq!(count_blocking_send_to_client(raw_str_hashes), 0);
}
