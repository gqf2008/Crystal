"""给容量标定播种压测账号 + 角色（写进**部署副本**的 DB，不动开发库）。

为什么不能靠协议注册：服务端对 NewAccount / NewCharacter 有 IP 防刷
（>2 账号/小时、>4 角色/小时 → 封 IP 24h），单机注册不出几百个会话。
容量标定要的是"服务端在 N 条真实会话下的表现"，测试账号怎么来的不影响结论——
所以离线播种：账号直接用模板账号的 password_hash（密码不变），角色行复制模板角色再改
名字/账号/落点。

用法：python seed_load_accounts.py <db> <count> [--prefix ops_load] [--char-prefix OpsLoad]
      [--template-account test] [--template-char bevychar] [--map-index 1] [--x 296] [--y 221]
输出：JSON 摘要（插入了多少账号/角色、各自名字）
"""
import argparse
import json
import sqlite3
import sys


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("db")
    ap.add_argument("count", type=int)
    ap.add_argument("--prefix", default="ops_load")
    ap.add_argument("--char-prefix", default="OpsLoad")
    ap.add_argument("--template-account", default="test")
    ap.add_argument("--template-char", default="bevychar")
    ap.add_argument("--map-index", type=int, default=1)
    ap.add_argument("--x", type=int, default=296)
    ap.add_argument("--y", type=int, default=221)
    ap.add_argument("--password-hash-from", default="", help="默认用 --template-account 的 hash")
    # 账号名必须过 `ServerRust/src/util/validation.rs::validate_username`：
    # **只允许 ASCII 字母数字**（下划线会被拒，登录时只留一条 warn，客户端看到的是"没反应"）。
    # 实测踩过：用 `ops_load1` 播种 200 个账号，全部登录超时。故默认前缀不含下划线。
    ap.add_argument("--no-underscore-check", action="store_true")
    a = ap.parse_args()

    con = sqlite3.connect(a.db)
    con.row_factory = sqlite3.Row
    cur = con.cursor()

    hash_src = a.password_hash_from or a.template_account
    if not a.no_underscore_check and not a.prefix.isalnum():
        print(json.dumps({"error": f"账号前缀必须全是字母数字（服务端 validate_username 拒绝下划线等）：{a.prefix}"}, ensure_ascii=False))
        return 2
    row = cur.execute("select password_hash from accounts where username=?", (hash_src,)).fetchone()
    if not row:
        print(json.dumps({"error": f"模板账号不存在: {hash_src}"}, ensure_ascii=False))
        return 2
    pwd_hash = row["password_hash"]

    tpl = cur.execute("select * from characters where name=?", (a.template_char,)).fetchone()
    if not tpl:
        print(json.dumps({"error": f"模板角色不存在: {a.template_char}"}, ensure_ascii=False))
        return 2
    cols = tpl.keys()

    acc_cols = [r[1] for r in cur.execute("pragma table_info(accounts)")]
    made_accounts, made_chars, skipped = [], [], []
    for i in range(1, a.count + 1):
        acc = f"{a.prefix}{i}"
        char = f"{a.char_prefix}{i}"
        if cur.execute("select 1 from accounts where username=?", (acc,)).fetchone():
            skipped.append(acc)
        else:
            cur.execute("insert into accounts (username, password_hash) values (?,?)", (acc, pwd_hash))
            made_accounts.append(acc)
        if cur.execute("select 1 from characters where name=?", (char,)).fetchone():
            skipped.append(char)
            continue
        values = []
        for c in cols:
            v = tpl[c]
            if c == "name":
                v = char
            elif c == "account_username":
                v = acc
            elif c == "map_index":
                v = a.map_index
            elif c == "x":
                v = a.x
            elif c == "y":
                v = a.y
            elif c == "is_online":
                v = 0
            values.append(v)
        cur.execute(
            f"insert into characters ({','.join(cols)}) values ({','.join('?' * len(cols))})",
            values,
        )
        made_chars.append(char)
    con.commit()
    total_acc = cur.execute("select count(*) from accounts").fetchone()[0]
    total_chr = cur.execute("select count(*) from characters").fetchone()[0]
    con.close()
    print(json.dumps({
        "ok": True,
        "db": a.db,
        "accounts_added": len(made_accounts),
        "characters_added": len(made_chars),
        "skipped_existing": skipped[:10],
        "accounts_total": total_acc,
        "characters_total": total_chr,
        "password": "与模板账号相同（默认 test → 123456）",
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
