// ============================================================================
// 内置拼音 IME：构建时自动获取并编译 libpinyin（与 mir2x 的 vcpkg port 一致）
// ============================================================================
// 流程（与 mir2x ports/libpinyin/portfile.cmake 对齐）：
//   1) 拉取 etorth/libpinyin fork 源码（固定 REF + SHA512 校验）
//   2) 下载 model20.text.tar.gz 模型数据（SHA512 校验），解压进源码 data/
//   3) autoreconf -f -i && ./configure --with-dbm=BerkeleyDB --disable-libzhuyin \
//      --disable-dependency-tracking && make && make install -> OUT_DIR/libpinyin/install
// 产物：lib/libpinyin.a、include/libpinyin-2.11.91/pinyin.h、lib/libpinyin/data/*.bin
//
// 逃生口：若环境变量 LIBPINYIN_DIR 指向已安装根（含 lib/libpinyin.a），则跳过自动构建直接复用。
// 链接依赖：glib-2.0（pkg-config）、berkeley-db、libc++/libstdc++。
// ----------------------------------------------------------------------------

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORK_REF: &str = "f21ef9a12a14eef626a89a38bb06e8ed115c38ca";
const FORK_SHA512: &str = "d3e293d5a4a7bcf6dfc2f96724e2c16ecb2046d38de4d8c7fc349bd6eab3b4472f01cba317c50d75ef6afa773c9e007a273fd065a7418e0819a420616ef12858";
const MODEL_SHA512: &str = "ed4d0607ad35e0e7ea424670539ddcd81a2b03c1da914b9c00cb748cf065f29471502d40b9a189852001da1fb9178c3bcc4675d7efebea5d081d78bfeee9b5d6";
// 模型数据的候选下载点。
//
// 每个候选都按自己的 SHA512 **单独**校验后才被接受（见 download_verified）：2026-10-01 的
// 事故就是 SourceForge 对下载请求返回 200 + 空文件/错误页，而旧代码只看 curl 退出码与文件
// 是否存在就认定成功并 return —— 于是永远不会去试下一个候选，最后在统一校验处 panic，
// 兜底形同虚设。
// 另外 fcitx 那条（download.fcitx-im.org/data/model20.text.tar.gz）实测已 404，删掉。
const MODEL_URLS: &[&str] = &[
    "https://downloads.sourceforge.net/project/libpinyin/models/model20.text.tar.gz",
    "https://master.dl.sourceforge.net/project/libpinyin/models/model20.text.tar.gz",
    "https://netcologne.dl.sourceforge.net/project/libpinyin/models/model20.text.tar.gz",
    "https://cfhcable.dl.sourceforge.net/project/libpinyin/models/model20.text.tar.gz",
];
/// 兜底容器：Slackware 源码镜像提供的是 model20.text.tar.lz（同一个 model20 数据，只是
/// lzip 压缩）。已用逐文件 SHA-256 比对验证：解出的 18 个文件与 model20.text.tar.gz 完全一致。
/// 2026-10-01 SourceForge 全线不可用时，它是唯一活着的源。
const MODEL_LZ_URLS: &[&str] = &[
    "https://mirrors.slackware.com/slackware/slackware64-current/source/x/libpinyin/model20.text.tar.lz",
];
const MODEL_LZ_SHA512: &str = "4c3d568601500eedf3eae3e4462c5150eba1548d5e15f038056c7c63116042d88034276631650cc66f14b69f31bb0c1d0c3de08a3f15db463f19e82b694e375b";

fn main() {
    emit_build_stamp();
    // 逃生口：已有 libpinyin 安装根
    if let Ok(dir) = env::var("LIBPINYIN_DIR") {
        let p = PathBuf::from(&dir);
        if p.join("lib/libpinyin.a").exists() {
            emit_links(&p);
            emit_dirs(&p);
            return;
        }
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let root = out.join("libpinyin");
    let install = root.join("install");
    let marker = install.join(".built");

    if !marker.exists() {
        fs::create_dir_all(&root).unwrap_or_else(|e| panic!("create {}: {}", root.display(), e));
        build_libpinyin(&root, &install);
        fs::write(&marker, "ok").unwrap_or_else(|e| panic!("write marker: {}", e));
    }

    emit_links(&install);
    emit_dirs(&install);
}

/// 构建戳：把「这份二进制到底出自哪个提交」固化进 exe，供
/// ① 启动日志（`main.rs` 打印）、② `build_stamp` control RPC、③ 夹具断言共用。
///
/// 为什么需要它（实测代价）：owner 多次拿**旧构建**的截图/体验当缺陷报（写邮件窗「错位」、
/// 底部对话框滚动/对齐、地图灯光、魔法特效），每次都要先花一轮去证明"代码早改过了"。
/// repo 里也已有 `LESSON_运行目标分支e2e前需重建二进制避免陈旧target误报`——
/// 判据必须能回答"被测 exe 是不是当前提交构建的"，否则夹具与 owner 都可能对着旧二进制下结论。
///
/// 必须放在 `main()` **最前面**：下面 `LIBPINYIN_DIR` 逃生口会提前 `return`。
fn emit_build_stamp() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // 仓库根 = crate 目录的上一级（本仓布局 Client-Bevy/ 在根下）
    let repo = manifest
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest);
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    };
    // `%H` 全量 + `%h` 短哈希都留：短哈希够读，全量够与 master 精确比对
    let full = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let short = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = match git(&["status", "--porcelain"]) {
        Some(_) => "1",
        None => "0",
    };
    // **故意不嵌"构建时刻"**：那会让每次 build.rs 重跑都改 env ⇒ 指纹变化 ⇒ 整 crate 重编。
    // "这份 exe 多新"由 `build_stamp` RPC 运行时读 exe 的 mtime/size 给出，编译期只固化
    // 「出自哪个提交、工作区是否 dirty」这两个真正稳定的量。
    println!("cargo:rustc-env=CRYSTAL_BUILD_COMMIT={full}");
    println!("cargo:rustc-env=CRYSTAL_BUILD_COMMIT_SHORT={short}");
    println!("cargo:rustc-env=CRYSTAL_BUILD_DIRTY={dirty}");
}

fn emit_links(install: &Path) {
    println!(
        "cargo:rustc-link-search=native={}",
        install.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=pinyin");
    // glib-2.0（pkg-config）
    for (k, v) in pkg_config_libs("glib-2.0") {
        println!("{}={}", k, v);
    }
    // berkeley-db
    let db_libdir = db_libdir();
    if let Some(d) = &db_libdir {
        println!("cargo:rustc-link-search=native={}", d.display());
    }
    println!("cargo:rustc-link-lib=dylib=db");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=dylib=c++");
    #[cfg(not(target_os = "macos"))]
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

fn emit_dirs(install: &Path) {
    // 运行时 libpinyin 数据/配置目录。
    // 注意：env! 固化的是构建机绝对路径，仅作开发回退；运行时解析在
    // src/ui/pinyin_ime.rs（exe 相对 libpinyin/{data,conf} 优先，此处路径垫底）。
    println!("cargo:rustc-env=LIBPINYIN_DIR={}", install.display());
    println!(
        "cargo:rustc-env=LIBPINYIN_DATA_DIR={}",
        install.join("lib/libpinyin/data").display()
    );
    println!(
        "cargo:rustc-env=LIBPINYIN_CONF_DIR={}",
        install.join("lib/libpinyin/conf").display()
    );
}

fn build_libpinyin(root: &Path, install: &Path) {
    let src = root.join("src");
    // 1) fork 源码（可用 LIBPINYIN_FORK_TARBALL 指定本地 tarball，跳过下载）
    if !src.join("configure.ac").exists() {
        let tar = root.join("fork.tar.gz");
        if let Ok(local) = env::var("LIBPINYIN_FORK_TARBALL") {
            if Path::new(&local).exists() {
                fs::copy(&local, &tar).unwrap_or_else(|e| panic!("copy fork tarball: {}", e));
            }
        } else {
            download(
                &tar,
                &format!(
                    "https://github.com/etorth/libpinyin/archive/{}.tar.gz",
                    FORK_REF
                ),
            );
        }
        verify_sha512(&tar, FORK_SHA512);
        extract(&tar, root);
        let extracted = root.join(format!("libpinyin-{}", FORK_REF));
        if !extracted.exists() {
            panic!("fork 解压目录不存在: {}", extracted.display());
        }
        fs::rename(&extracted, &src)
            .unwrap_or_else(|e| panic!("rename {} -> src: {}", extracted.display(), e));
    }
    // 2) model20 数据（可用 LIBPINYIN_MODEL_TARBALL 指定本地 tarball，跳过下载）。
    //    无论是否复用缓存包，都做 SHA512 校验，防止上次残留损坏包在解压/构建时出错。
    let model_tar = root.join("model20.text.tar.gz");
    // 复用的缓存包也要先验签：残留的坏包（今天的事故形态）会让构建一路走到解压/编译才炸，
    // 报错点离原因很远。
    if model_tar.exists() && !sha512_matches(&model_tar, MODEL_SHA512) {
        eprintln!("[libpinyin] 缓存包校验失败，删除后重新获取: {}", model_tar.display());
        let _ = fs::remove_file(&model_tar);
    }
    if !model_tar.exists() {
        if let Ok(local) = env::var("LIBPINYIN_MODEL_TARBALL") {
            if Path::new(&local).exists() {
                fs::copy(&local, &model_tar)
                    .unwrap_or_else(|e| panic!("copy model tarball: {}", e));
            } else {
                download_with_fallback(&model_tar, &model_urls());
            }
        } else {
            download_with_fallback(&model_tar, &model_urls());
        }
    }
    // 3) 兜底容器：.tar.lz（需要解压器：lzip / 7z / python3 依次尝试；都没有就跳过候选，
    //    不让缺工具把构建搞红）。解压成功即直接解进 data/，不再走 .tar.gz 的校验路径。
    let mut model_ready = model_tar.exists();
    // 记住 .tar.lz 这一路最后一次失败的真实原因：否则兜底也失败时，panic 尾行只说「取不到」，
    // CI 日志里容易被读成网络问题（实际可能是「没有解压器」）。
    let mut lz_err: Option<String> = None;
    if !model_ready {
        for u in MODEL_LZ_URLS {
            let lz = root.join("model20.text.tar.lz");
            if !download_verified(&lz, u, MODEL_LZ_SHA512) {
                continue;
            }
            match extract_lz_into(&lz, &src.join("data")) {
                Ok(()) => {
                    model_ready = true;
                    break;
                }
                Err(e) => {
                    eprintln!("[libpinyin]   .tar.lz 解压/解包失败（{}），跳过该候选", e);
                    lz_err = Some(e);
                }
            }
        }
    }
    if !model_ready {
        panic!(
            "模型数据取不到：.tar.gz 候选 {} 个与 .tar.lz 兜底 {} 个都失败（.tar.lz 最后一次失败原因：{}）—— 可用 LIBPINYIN_MODEL_URLS 指定镜像，或用 LIBPINYIN_MODEL_TARBALL 指定本地包",
            model_urls().len(),
            MODEL_LZ_URLS.len(),
            lz_err.as_deref().unwrap_or("未尝试（下载/校验阶段就没过）")
        );
    }
    if model_tar.exists() {
        verify_sha512(&model_tar, MODEL_SHA512);
        extract_into(&model_tar, &src.join("data"));
    }
    // 3) autoreconf + configure + make + install
    // Windows(MSYS2) 下 autoreconf/configure 是 shell 脚本，native 进程无法直接 spawn，
    // 必须经 bash -c 执行；Unix 用 sh -c（行为等价）。
    run_script("autoreconf -f -i", &src, &[], "autoreconf");
    let configure = src.join("configure");
    let db_cpp = db_include_flag();
    let db_ld = db_libdir()
        .map(|d| format!("-L{}", d.display()))
        .unwrap_or_default();
    let mut configure_env: Vec<(String, String)> = Vec::new();
    let pkgcfg = pkg_config_path();
    if !pkgcfg.is_empty() {
        configure_env.push(("PKG_CONFIG_PATH".into(), pkgcfg));
    }
    configure_env.push((
        "CPPFLAGS".into(),
        format!("{} {}", db_cpp, env_opt("CPPFLAGS")),
    ));
    configure_env.push((
        "LDFLAGS".into(),
        format!("{} {}", db_ld, env_opt("LDFLAGS")),
    ));
    // 路径转正斜杠：Windows 下传给 msys bash 避免反斜杠被吞。
    let configure_cmd = format!(
        "'{}' --prefix='{}' --with-dbm=BerkeleyDB --disable-libzhuyin --disable-dependency-tracking",
        configure.display().to_string().replace('\\', "/"),
        install.display().to_string().replace('\\', "/"),
    );
    run_script(
        &configure_cmd,
        &src,
        &configure_env,
        "configure（依赖：glib-2.0、berkeley-db、autoconf/automake/libtool 需已安装）",
    );
    run(
        Command::new("make").args(["-j", "4"]).current_dir(&src),
        "make",
    );
    run(
        Command::new("make").args(["install"]).current_dir(&src),
        "make install",
    );
    if !install.join("lib/libpinyin.a").exists() {
        panic!(
            "libpinyin 构建后缺少 lib/libpinyin.a: {}",
            install.display()
        );
    }
}

/// 执行 autotools 脚本类命令（autoreconf/configure）。
/// Windows(MSYS2) 下它们是 shell 脚本，native 进程直接 spawn 会“program not found”，
/// 须经 bash -c；Unix 用 sh -c（行为等价）。
fn run_script(cmd: &str, cwd: &Path, envs: &[(String, String)], what: &str) {
    // Windows 下裸 `bash` 会命中 WSL 的 System32/bash.exe（报 “WSL has no installed
    // distributions”），须用 MSYS2 bash 的绝对路径（CI 经 LIBPINYIN_SHELL 注入）。
    let shell = if cfg!(windows) {
        env::var("LIBPINYIN_SHELL").unwrap_or_else(|_| "bash".to_string())
    } else {
        "sh".to_string()
    };
    let mut command = Command::new(shell);
    command.args(["-c", cmd]).current_dir(cwd);
    for (k, v) in envs {
        command.env(k, v);
    }
    let status = command
        .status()
        .unwrap_or_else(|e| panic!("spawn {}: {}", what, e));
    if !status.success() {
        panic!("{} 失败", what);
    }
}

fn run(cmd: &mut Command, what: &str) {
    let status = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", what, e))
        .wait()
        .unwrap_or_else(|e| panic!("wait {}: {}", what, e));
    if !status.success() {
        panic!("{} 失败", what);
    }
}

fn env_opt(k: &str) -> String {
    env::var(k).unwrap_or_default()
}

fn download(target: &Path, url: &str) {
    eprintln!("[libpinyin] 下载 {} -> {}", url, target.display());
    let status = Command::new("curl")
        .args(["-sL", "--retry", "3", "--max-time", "600", "-o"])
        .arg(target)
        .arg(url)
        .status()
        .unwrap_or_else(|e| panic!("spawn curl: {}", e));
    if !status.success() || !target.exists() {
        panic!("下载失败: {}", url);
    }
}

/// 候选下载点：LIBPINYIN_MODEL_URLS（逗号分隔）可整体替换内置列表 —— 给运维/CI 一个
/// 不改编代码就能换镜像的逃生口，也是本地端到端测试注入假镜像的入口。
fn model_urls() -> Vec<String> {
    match env::var("LIBPINYIN_MODEL_URLS") {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => MODEL_URLS.iter().map(|s| s.to_string()).collect(),
    }
}

/// 下载**一个**候选并就地校验：不通过就删掉自己的产物并返回 false（调用方接着试下一个）。
/// 三条失败线：curl 非零/未落地、内容过小（200 + 空文件/错误页，今天的事故形态）、SHA512 不匹配。
fn download_verified(target: &Path, url: &str, want: &str) -> bool {
    // 上一次候选的残渣绝不能被当成这一次的结果（旧代码正是栽在 target.exists() 上）。
    let _ = fs::remove_file(target);
    eprintln!("[libpinyin] 尝试下载模型数据: {}", url);
    let status = Command::new("curl")
        .args([
            "-sSL",
            "--retry", "5",
            "--retry-delay", "5",
            "--retry-all-errors",
            "--connect-timeout", "20",
            "--max-time", "900",
            "-o",
        ])
        .arg(target)
        .arg(url)
        .status()
        .unwrap_or_else(|e| panic!("spawn curl: {}", e));
    if !status.success() || !target.exists() {
        eprintln!("[libpinyin]   下载失败（curl 非零或未落地），换下一个候选");
        let _ = fs::remove_file(target);
        return false;
    }
    let size = fs::metadata(target).map(|m| m.len()).unwrap_or(0);
    if size < 1024 {
        eprintln!("[libpinyin]   响应只有 {} 字节，判为坏响应（空文件/错误页），换下一个候选", size);
        let _ = fs::remove_file(target);
        return false;
    }
    if !sha512_matches(target, want) {
        eprintln!("[libpinyin]   SHA512 校验未通过，换下一个候选（坏文件已删除）");
        let _ = fs::remove_file(target);
        return false;
    }
    eprintln!("[libpinyin]   SHA512 校验通过: {}", target.display());
    true
}

/// 依次试候选，命中即 true；全部失败返回 false（由调用方决定是否还有别的容器/是否 panic）。
fn download_with_fallback(target: &Path, urls: &[String]) -> bool {
    for u in urls {
        if download_verified(target, u, MODEL_SHA512) {
            return true;
        }
    }
    false
}

fn sha512_of(path: &Path) -> String {
    // macOS: shasum -a 512；Linux: sha512sum。两命令都试，兼容 CI(ubuntu) 与本地(mac)。
    let tries: &[(&str, &[&str])] = &[("shasum", &["-a", "512"]), ("sha512sum", &[])];
    let mut got = String::new();
    for (cmd, args) in tries {
        let out = Command::new(cmd).args(*args).arg(path).output();
        if let Ok(o) = out {
            if o.status.success() {
                // Windows(msys) 的 shasum/sha512sum 输出首 token 可能带 `\` 前缀，
                // 只保留开头非 hex 字符剥离后的哈希（如 `\d3e293...` -> `d3e293...`）。
                let token = String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_lowercase();
                got = token
                    .trim_start_matches(|c: char| !c.is_ascii_hexdigit())
                    .to_string();
                if !got.is_empty() {
                    break;
                }
            }
        }
    }
    got
}

/// 不匹配就 panic —— 用于源码包与最终成品（这两处失败就该停下）。
fn verify_sha512(path: &Path, want: &str) {
    let got = sha512_of(path);
    if got != want {
        panic!(
            "SHA512 校验失败: 得到 {}，期望 {}（文件: {}）",
            got,
            want,
            path.display()
        );
    }
    eprintln!("[libpinyin] SHA512 校验通过: {}", path.display());
}

/// 非 panic 的判定：候选下载与缓存复用都靠它决定「接受还是换下一个」。
fn sha512_matches(path: &Path, want: &str) -> bool {
    sha512_of(path) == want
}

fn extract(tar: &Path, into: &Path) {
    let status = Command::new("tar")
        .arg("xzf")
        .arg(tar)
        .arg("-C")
        .arg(into)
        .status()
        .unwrap_or_else(|e| panic!("spawn tar: {}", e));
    if !status.success() {
        panic!("解压失败: {}", tar.display());
    }
}

/// 把 .tar.lz 解成 tar 再解进 `into`：解压器按 lzip -> python3 -> python 依次尝试
/// （都不在就返回 Err，调用方跳过该候选 —— 缺工具不该让构建红）。
/// 注：7-Zip 不支持 lzip 容器（实测 `7z x -so` 退出码 2），所以链条里没有它。
/// python 兜底用内嵌解码器：多成员 lzip = 每个成员 [6 字节头][raw LZMA1][20 字节 trailer]。
fn extract_lz_into(lz: &Path, into: &Path) -> Result<(), String> {
    // 产物名写死，避免 with_extension("tar") 把 model20.text.tar.lz 变成 model20.text.tar.tar
    let tar = lz.with_file_name("model20.plain.tar");
    let lz_s = lz.to_string_lossy().to_string();
    let tar_s = tar.to_string_lossy().to_string();
    let py = "import lzma,sys\nb=open(sys.argv[1],'rb').read()\npos=0\nout=open(sys.argv[2],'wb')\nwhile pos<len(b):\n    d=b[pos+5]\n    ds=1<<(d&0x1f)\n    ds-=(ds//16)*((d>>5)&7)\n    dec=lzma.LZMADecompressor(format=lzma.FORMAT_RAW,filters=[{'id':lzma.FILTER_LZMA1,'dict_size':ds,'lc':3,'lp':0,'pb':2}])\n    out.write(dec.decompress(b[pos+6:]))\n    pos+=6+(len(b)-pos-6-len(dec.unused_data))+20\nout.close()\n";
    let mut ok = false;
    for (cmd, args) in [("lzip", vec!["-dc".to_string(), lz_s.clone()])] {
        if let Ok(out) = fs::File::create(&tar) {
            if let Ok(status) = Command::new(cmd)
                .args(&args)
                .stdout(out)
                .stderr(std::process::Stdio::null())
                .status()
            {
                if status.success() && fs::metadata(&tar).map(|m| m.len() > 1024).unwrap_or(false) {
                    ok = true;
                    break;
                }
            }
        }
        let _ = fs::remove_file(&tar);
    }
    // python 兜底：Ubuntu/macOS 上叫 python3，Windows 上通常只有 python —— 两个名字都要试。
    for pycmd in ["python3", "python"] {
        if ok {
            break;
        }
        let status = Command::new(pycmd)
            .arg("-c")
            .arg(py)
            .arg(&lz_s)
            .arg(&tar_s)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        ok = matches!(status, Ok(s) if s.success())
            && fs::metadata(&tar).map(|m| m.len() > 1024).unwrap_or(false);
        if !ok {
            let _ = fs::remove_file(&tar);
        }
    }
    if !ok {
        return Err("没有可用的 lzip 解压器（lzip / 7z / python3 / python 均不可用或都失败）".into());
    }
    fs::create_dir_all(into).map_err(|e| format!("create {}: {}", into.display(), e))?;
    let status = Command::new("tar")
        .arg("xf")
        .arg(&tar)
        .arg("-C")
        .arg(into)
        .status()
        .map_err(|e| format!("spawn tar: {}", e))?;
    if !status.success() {
        return Err(format!("tar xf 失败: {}", tar.display()));
    }
    Ok(())
}

fn extract_into(tar: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap_or_else(|e| panic!("create {}: {}", into.display(), e));
    let status = Command::new("tar")
        .arg("xzf")
        .arg(tar)
        .arg("-C")
        .arg(into)
        .status()
        .unwrap_or_else(|e| panic!("spawn tar: {}", e));
    if !status.success() {
        panic!("解压模型数据失败: {}", tar.display());
    }
}

/// 解析 pkg-config --libs 输出为 cargo 链接指令（-L -> rustc-link-search=native，-l -> rustc-link-lib）
fn pkg_config_libs(pkg: &str) -> Vec<(String, String)> {
    let out = Command::new("pkg-config")
        .args(["--libs"])
        .arg(pkg)
        .output()
        .unwrap_or_else(|e| panic!("spawn pkg-config: {}", e));
    if !out.status.success() {
        panic!(
            "pkg-config --libs {} 失败：需先安装 {}（如 brew install glib）",
            pkg, pkg
        );
    }
    let mut v = Vec::new();
    for tok in String::from_utf8_lossy(&out.stdout).split_whitespace() {
        if let Some(lib) = tok.strip_prefix("-L") {
            v.push(("cargo:rustc-link-search=native".into(), lib.into()));
        } else if let Some(lib) = tok.strip_prefix("-l") {
            v.push(("cargo:rustc-link-lib".into(), lib.into()));
        }
    }
    v
}

fn pkg_config_path() -> String {
    env::var("PKG_CONFIG_PATH").unwrap_or_default()
}

fn db_include_flag() -> String {
    if let Ok(d) = env::var("BERKELEY_DB_INCLUDE") {
        return format!("-I{}", d);
    }
    for p in [
        "/opt/homebrew/opt/berkeley-db/include",
        "/usr/local/opt/berkeley-db/include",
        "/usr/include",
        "/usr/local/include",
    ] {
        if Path::new(p).join("db.h").exists() {
            return format!("-I{}", p);
        }
    }
    String::new()
}

fn db_libdir() -> Option<PathBuf> {
    if let Ok(d) = env::var("BERKELEY_DB_LIBDIR") {
        return Some(PathBuf::from(d));
    }
    for p in [
        "/opt/homebrew/opt/berkeley-db/lib",
        "/usr/local/opt/berkeley-db/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
    ] {
        let pb = PathBuf::from(p);
        if pb.join("libdb.dylib").exists()
            || pb.join("libdb.so").exists()
            || pb.join("libdb.a").exists()
        {
            return Some(pb);
        }
    }
    None
}
