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
const MODEL_URLS: &[&str] = &[
    "https://downloads.sourceforge.net/project/libpinyin/models/model20.text.tar.gz",
    "https://download.fcitx-im.org/data/model20.text.tar.gz",
];

fn main() {
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

fn emit_links(install: &Path) {
    println!("cargo:rustc-link-search=native={}", install.join("lib").display());
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
    // 运行时 libpinyin 数据/配置目录
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
            download(&tar, &format!("https://github.com/etorth/libpinyin/archive/{}.tar.gz", FORK_REF));
        }
        verify_sha512(&tar, FORK_SHA512);
        extract(&tar, root);
        let extracted = root.join(format!("libpinyin-{}", FORK_REF));
        if !extracted.exists() {
            panic!("fork 解压目录不存在: {}", extracted.display());
        }
        fs::rename(&extracted, &src).unwrap_or_else(|e| panic!("rename {} -> src: {}", extracted.display(), e));
    }
    // 2) model20 数据（可用 LIBPINYIN_MODEL_TARBALL 指定本地 tarball，跳过下载）。
    //    无论是否复用缓存包，都做 SHA512 校验，防止上次残留损坏包在解压/构建时出错。
    let model_tar = root.join("model20.text.tar.gz");
    if !model_tar.exists() {
        if let Ok(local) = env::var("LIBPINYIN_MODEL_TARBALL") {
            if Path::new(&local).exists() {
                fs::copy(&local, &model_tar).unwrap_or_else(|e| panic!("copy model tarball: {}", e));
            } else {
                download_with_fallback(&model_tar, MODEL_URLS);
            }
        } else {
            download_with_fallback(&model_tar, MODEL_URLS);
        }
    }
    verify_sha512(&model_tar, MODEL_SHA512);
    extract_into(&model_tar, &src.join("data"));
    // 3) autoreconf + configure + make + install
    run(Command::new("autoreconf").args(["-f", "-i"]).current_dir(&src), "autoreconf");
    let configure = src.join("configure");
    let db_cpp = db_include_flag();
    let db_ld = db_libdir().map(|d| format!("-L{}", d.display())).unwrap_or_default();
    let mut configure_env: Vec<(String, String)> = Vec::new();
    let pkgcfg = pkg_config_path();
    if !pkgcfg.is_empty() {
        configure_env.push(("PKG_CONFIG_PATH".into(), pkgcfg));
    }
    configure_env.push(("CPPFLAGS".into(), format!("{} {}", db_cpp, env_opt("CPPFLAGS"))));
    configure_env.push(("LDFLAGS".into(), format!("{} {}", db_ld, env_opt("LDFLAGS"))));
    let mut cfg = Command::new(&configure)
        .arg(format!("--prefix={}", install.display()))
        .args(["--with-dbm=BerkeleyDB", "--disable-libzhuyin", "--disable-dependency-tracking"])
        .current_dir(&src)
        .envs(configure_env)
        .spawn()
        .unwrap_or_else(|e| panic!("spawn configure: {}", e));
    let status = cfg.wait().unwrap_or_else(|e| panic!("wait configure: {}", e));
    if !status.success() {
        panic!("configure 失败（status {:?}）。依赖：glib-2.0、berkeley-db、autoconf/automake/libtool 需已安装。", status.code());
    }
    run(Command::new("make").args(["-j", "4"]).current_dir(&src), "make");
    run(Command::new("make").args(["install"]).current_dir(&src), "make install");
    if !install.join("lib/libpinyin.a").exists() {
        panic!("libpinyin 构建后缺少 lib/libpinyin.a: {}", install.display());
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

fn download_with_fallback(target: &Path, urls: &[&str]) {
    for u in urls {
        if target.exists() {
            return;
        }
        eprintln!("[libpinyin] 尝试下载模型数据: {}", u);
        let status = Command::new("curl")
            .args(["-sL", "--retry", "3", "--max-time", "600", "-o"])
            .arg(target)
            .arg(u)
            .status()
            .unwrap_or_else(|e| panic!("spawn curl: {}", e));
        if status.success() && target.exists() {
            return;
        }
    }
    panic!("模型数据 model20.text.tar.gz 下载失败（尝试了 {:?}）", urls);
}

fn verify_sha512(path: &Path, want: &str) {
    // macOS: shasum -a 512；Linux: sha512sum。两命令都试，兼容 CI(ubuntu) 与本地(mac)。
    let tries: &[(&str, &[&str])] = &[
        ("shasum", &["-a", "512"]),
        ("sha512sum", &[]),
    ];
    let mut got = String::new();
    for (cmd, args) in tries {
        let out = Command::new(cmd).args(*args).arg(path).output();
        if let Ok(o) = out {
            if o.status.success() {
                got = String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_lowercase();
                if !got.is_empty() {
                    break;
                }
            }
        }
    }
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
        panic!("pkg-config --libs {} 失败：需先安装 {}（如 brew install glib）", pkg, pkg);
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
        if pb.join("libdb.dylib").exists() || pb.join("libdb.so").exists() || pb.join("libdb.a").exists() {
            return Some(pb);
        }
    }
    None
}
