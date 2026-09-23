"""把播种时创建的压测角色重新挂到**合法账号名**下（一次性修复脚本）。

背景：首次播种用了 `ops_load<N>`（含下划线）——服务端 `validate_username` 只允许字母数字，
那些账号能写进库却登不进来（客户端表现为登录超时）。后来用合法前缀 `opsload<N>` 重新播种，
但角色名 `OpsLoad<N>` 已存在于是被跳过，角色仍挂在旧账号上 → 新账号没有角色，进不了图。
本脚本按角色名后缀把 account_username 改回合法账号名。

用法：python fix_seed_account_links.py <db> [--char-prefix OpsLoad] [--account-prefix opsload]
"""
import argparse
import json
import sqlite3


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("db")
    ap.add_argument("--char-prefix", default="OpsLoad")
    ap.add_argument("--account-prefix", default="opsload")
    a = ap.parse_args()
    con = sqlite3.connect(a.db)
    cur = con.cursor()
    fixed = 0
    names = [r[0] for r in cur.execute(
        "select name from characters where name like ?", (a.char_prefix + "%",))]
    for name in names:
        suffix = name[len(a.char_prefix):]
        if suffix.isdigit():
            cur.execute("update characters set account_username=? where name=?",
                        (f"{a.account_prefix}{suffix}", name))
            fixed += 1
    con.commit()
    sample = list(cur.execute(
        "select name, account_username, map_index, x, y from characters where account_username like ? limit 3",
        (a.account_prefix + "%",)))
    con.close()
    print(json.dumps({"ok": True, "repointed": fixed, "sample": sample}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
