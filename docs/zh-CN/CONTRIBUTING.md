> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 为 Goblin++ 做出贡献

感谢你帮助科学计算变得更容易检查。本项目仍处于 alpha 阶段：测试通过是某一具体行为的证据，并不证明科学有效性，也不证明与早期 Python 解释器完全兼容。

提出变更前，请阅读[移植矩阵](docs/PORTING_MATRIX.md)和[安全边界](docs/SECURITY.md)。说明拟改变的行为，添加或更新正向与负向测试，并说明测试实际支持哪些主张。处理 FITS 或其他科学数据时，应明确列出数据选择、单位、空值处理和来源假设。

在仓库根目录执行本地检查：

```console
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cd vscode && npm test
```

不要提交真实研究数据、个人运行目录、访问令牌、凭据或私有路径。小型 `examples/sample.fits` 测试样例由 `tools/generate_sample_fits.rs` 生成，供测试使用。

在公布私密报告渠道之前，不要通过公开 issue 报告疑似安全漏洞。发布检查清单会跟踪这一尚未建立的渠道。软件贡献采用 [MIT](LICENSE)；原创文档正文和图示采用 [CC BY 4.0](LICENSE-DOCS.md)。仅贡献你有权按这些条款授权的材料，并明确标注任何第三方材料。
