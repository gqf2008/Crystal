---
name: 任务 / 批次 Issue
about: 单一任务或同类批次任务（含 checklist），避免碎片化
title: "[Crystal-<worktree>] <类型>: <简述>"
labels: ["enhancement"]
---

## 现状
（为什么需要做：代码现状 / 与 C# 的差异 / 引用的问题编号）

## 目标
（做成什么样；对齐哪个 C# 文件/机制，给出路径与关键数值）

## 批次清单（同类任务合并，逐项勾选）
- [ ] 子项 1（含 C# 出处）
- [ ] 子项 2
- [ ] ...

## 验收
- [ ] `cargo check` 通过
- [ ] `cargo test` 通过（ServerRust / Client-Bevy 对应侧）
- [ ] 行为与 C# 对齐（或重构类：e2e/快照 diff 为空）

## 关联
- 关联 PR/Issue：#
