> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 公开发布检查清单

这是公开 alpha.13 发布之后各次变更可重复使用的检查清单。一次提交不等于新发行版；每个新版本仍需单独审查、标签、发布资产和保存记录。本清单将决定明确列出，而不是悄悄替作者决定。当前工作中的 Rust 引擎为 `0.1.0-alpha.18`（尚未发布的统计功能第一步）；VS Code 扩展源代码版本为 `0.1.13`，JetBrains 插件源代码版本为 `0.3.3`。

## 将 GitHub 仓库公开之前

- [x] 将源代码和确定性测试样例与构建产物、运行记录、生成输出及用户研究数据分离。
- [x] 添加注明 Malin Hess 的引用文件。在提供 ORCID 或铸造 DOI 前，特意不填写这些字段。
- [x] 添加只进行测试和构建检查、不进行发布的 GitHub Actions 工作流。
- [x] 为软件添加 MIT，为原创文档正文和图示添加 CC BY 4.0；明确范围，并使 Rust／VS Code 包元数据与之匹配。
- [x] 确认 Malin Hess 为原创项目材料的具名权利人。由 GPT 生成的项目标志依照 [BRANDING.md](BRANDING.md) 作为独立品牌材料处理；依赖 crate 保留各自许可证。
- [x] 审查受跟踪文件清单和本地 Git 历史中的明显秘密、私有数据、第三方源码／资产及再分发问题。在此快照中未发现个人运行数据或明显秘密；若发布前加入新材料，须重复审查。
- [ ] 决定仓库可见性，并确认 GitHub 账户／仓库名。首次私有上传可以撤回；公开可见是另一项决定。
- [ ] 建立私密安全报告渠道，并说明受支持的版本。
- [ ] 确认贡献政策及任何必要的 AI 辅助披露。
- [x] 显著展示隐私警告：`input()` 回答、程序参数、源代码、导入数据证据和运行日志可能以明文保留。`GO_PARANOID` 和 SHA-256 哈希不会对它们加密。
- [x] 将日常文本操作和提示输入保留在语言支持验收门槛中：`input("Please enter your name:")`、文本连接、`len(text)`、`str_trim`／`str_contains`／`str_replace`／`str_split`／`str_join`、`to_text`，以及带检查的 `parse_number` 和 `parse_integer`。解释器／编译器和保留交互的测试位于 `tests/interaction.rs` 与 `tests/g_strings.rs`；这些操作已经实现，并非留待未来发布。输入仍是明文证据，不是密码提示。

## 宣称 alpha 版本可供研究者使用之前

- [ ] 在 GitHub 运行工作流，包括 macOS 和 Linux；修复任何失败。
- [ ] 在干净检出中重新检查 `cargo test --locked`、Rust 格式／lint 检查以及扩展测试。
- [x] 完成 macOS arm64 [依赖许可证审查](docs/DEPENDENCY_LICENSE_AUDIT.md)，并在 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) 和 `third-party/` 中收录精确锁定的 crate 声明及 Rust 标准库声明。依赖、工具链、功能、目标或打包方式变更时，重新生成此目标专用声明包。扩展没有 npm 依赖，其 VSIX 包含两个项目许可证文件。
- [ ] 在干净机器上复现源代码归档并验证其 SHA-256。
- [ ] 重新构建 VS Code 扩展包，检查 MIT 和 CC BY 声明；编辑本源码仓库不会改变此前分发的 VSIX 文件。
- [ ] 审阅 `docs/PORTING_MATRIX.md`，在发布说明中列出尚不支持的语言、FITS、保管链和编译器功能。当其规范性语料及模式迁移验收门槛仍未通过时，不得宣称与 Python 0.0.7 一致。
- [ ] 审阅 `docs/SECURITY.md`；`GO_PARANOID` 是证据政策，不是沙箱；校验和账本并不认证作者身份。
- [ ] 根据独立已知值验证有代表性的科学结果，并发布可以再分发的测试数据／假设。
- [ ] 在 Git 中标记精确版本；仅上传经审查的二进制／资产并记录其哈希。不要悄悄替换已发布的标签或资产。

## 有价值的新增项，但不自动阻挡公开 alpha

- [ ] 按 [ENCRYPTION_PROPOSAL.md](docs/ENCRYPTION_PROPOSAL.md) 设计并测试**可选文件和目录加密**。实现前，不对运行目录或导出文件做出机密性声明。只有首次公开发布承诺加密存储或传输时，这才成为发布阻挡项。
- [ ] 继续完成 [PORTING_MATRIX.md](docs/PORTING_MATRIX.md) 中尚待完成的 Rust 移植验收门槛：规范性语料一致性、模式迁移、经认证的账本作者身份、中断账本头恢复、旧记录接纳，以及更广泛的 FITS 语义。这些项目阻挡“稳定替代 Python 参考实现”的主张，不阻挡坦诚说明局限的公开 alpha。

## GitHub 发布之后的 Zenodo

- [ ] 将选定 GitHub 仓库连接到 Zenodo，并启用归档。
- [ ] 从经审查的标签创建 GitHub 发行版，确认 Zenodo 收录预期归档。分享 DOI 前检查元数据和作者信息。
- [ ] 仅在验证后才将铸造的 DOI 加入 `CITATION.cff`，并说明其对应的发行版。
