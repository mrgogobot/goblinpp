> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 依赖许可证审查：macOS arm64 Alpha.12

快照：Goblin++ 0.1.0-alpha.17 的 `Cargo.lock`。Alpha.17 没有新增依赖；锁定的第三方依赖树仍是 2026-09-28 审查的那一份。这是 macOS arm64 alpha 二进制的证据记录，不是法律意见，也不是对未来目标平台的主张。

## 源代码检出使用了什么

五个直接运行时 crate 是 `chrono`、`clap`、`serde`、`serde_json` 和 `sha2`。直接的仅测试 crate 是 `tempfile`。它们在已安装包清单中均声明 `MIT OR Apache-2.0`。VS Code 扩展未声明 npm 包依赖。

构建 macOS arm64 二进制所用的锁定常规依赖树包含 40 个外部包。该数量包含构建时过程宏依赖，并有意采用保守方式：保留构建时包的声明，比悄悄遗漏可能适用的声明更稳妥。

此常规依赖树中所有包的已安装清单均提供 MIT 授权途径。值得注意的表达式包括：

| 包 | 声明的许可证表达式 | 审查说明 |
| --- | --- | --- |
| `unicode-ident` 1.0.24 | `(MIT OR Apache-2.0) AND Unicode-3.0` | 除所选 MIT／Apache 授权途径外，还须遵守 Unicode-3.0 条款。其随附的 `LICENSE-UNICODE` 要求在适用时保留版权及许可声明。 |
| `memchr` 2.8.3 | `Unlicense OR MIT` | MIT 是可选途径。 |
| `generic-array` 0.14.7, `strsim` 0.11.1, `zmij` 1.0.23 | `MIT` | 再分发这些组件时适用 MIT 声明。 |

此常规依赖树中的其他包声明 `MIT OR Apache-2.0` 或 `Apache-2.0 OR MIT`。此清单不覆盖 crate 自身的许可证文件、文件级声明或 Rust 标准库的权利。

## 已保存的发布证据

仓库现在保存：

1. [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)，列出精确锁定的 macOS arm64 常规依赖树中全部 40 个包；
2. 在这些精确版本的本地缓存 crate 根目录中发现的上游 license、copying、notice、copyright 和 Unlicense 文件，位于 `third-party/licenses/crates/`；
3. 当前 Rust 工具链标识、Rust 库版权文档及该工具链 Rust 文档随附的所有许可证文本，位于 `third-party/licenses/rust-standard-library/`；以及
4. VS Code 扩展 0.1.7 内的两个项目许可证文件。VSIX 未声明 npm 依赖；其打包源码已与经审查的扩展源码比对，唯一差异是 VSCE 按预期改写 README 链接。

`tools/collect_third_party_notices.py` 离线执行收集，并拒绝不完整或存在歧义的本地注册表匹配。生成的声明包必须随 macOS arm64 二进制归档一同提供。

这完成了已识别的 Alpha.12 macOS arm64 声明收集验收门槛。它不预先批准 Linux、Windows、嵌入式、不同功能配置或后续发行版。当 `Cargo.lock`、Rust 工具链、目标、功能或打包流程改变时，须重新执行清点。

从已准备好的本地 Cargo 缓存复现依赖树：

```console
cargo tree --locked --offline --target aarch64-apple-darwin -e normal
cargo tree --locked --offline --target x86_64-unknown-linux-gnu -e normal
```

项目源代码仓库未内置第三方 crate 源码。项目的 MIT 许可证不取代依赖自身的许可证。
