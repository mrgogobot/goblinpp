# 欢迎使用 Goblin++ / Simplified Chinese reader guide

**简体中文文档快照：0.1.0-alpha.21 · 2026-10-03 · 原作者：Malin Hess**

欢迎中文科研工作者、学生和编程爱好者！本目录帮助你安装、编写程序、读取实验数据，并理解 Goblin++ 的量纲检查和可验证运行证据。你可以用中文提出文档问题，不必先把问题翻译成英文。

## 从这里开始

1. 阅读[中文项目概览](README.md)，再阅读[第一天入门教程 PDF](docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.pdf)。复制代码时使用[教程 Markdown 全文](docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.md)或仓库根目录的原始示例。
2. 安装语言和插件请使用 [alpha.21 发布页](https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.21)。本目录是文档快照，不是另一个安装包，也不会替换语言引擎或插件配置。
3. 从源码安装时，在**仓库根目录**依照[英文安装说明](../../README.md#build-and-install)操作，不要在本翻译目录运行安装器。使用本地二进制归档包时，在解压后的安装包目录操作。
4. 编辑器指南：[VS Code](vscode/README.md) / [JetBrains](jetbrains/README.md)。编辑器提示是辅助；实际语法和能力以正在使用的 CLI 为准。
5. 先运行仓库根目录 `examples/` 中的小例子，再使用自己的数据。此快照中的 `examples/` 也是原样副本，但不会随语言开发自动更新。

## 按任务查阅

- 数据与文件：[FITS](docs/FITS.md)、[CSV/TSV、本地模块及编译模式 I/O](docs/DATA_MODULES_NATIVE.md)、[输出与图表](docs/OUTPUTS.md)。
- 语言基础：[条件与循环](docs/CONTROL_FLOW.md)、[输入与数组](docs/INTERACTION_AND_ARRAYS.md)、[函数](docs/FUNCTIONS.md)、[字符串](docs/STRINGS.md)。
- 科学计算：[数学](docs/MATH.md)、[向量、坐标与运动](docs/VECTORS_COORDINATES_KINEMATICS.md)、[化学](docs/CHEMISTRY.md)、[电气工程](docs/ELECTRICAL_ENGINEERING.md)、[基础统计](docs/STATISTICS.md)。
- 研究证据：[安全边界](docs/SECURITY.md)、[受保护的值](docs/PROTECTED_VALUES.md)、[无损数字文本](docs/NUMERIC_TEXT.md)。
- 术语核对：[中英术语表](GLOSSARY_zh-CN.md)。

## 版本、语法与证据

PDF 是 **alpha.12 历史教程的完整翻译**，不是 alpha.21 最新规范。有些历史限制、安装包名称和“五个基本量纲”描述已经发生变化；请查看中文项目概览中的译者说明及当前专题参考。对应英文源快照是提交 `1a6144a32569b2abbd3236f2509da0cae538e393`，并不意味着此翻译会自动跟随以后版本更新。

代码和命令保持原样：`GO_PARANOID`、`seal`、函数名、变量名、单位、运算符及参数语法不会因为阅读中文文档而改变。代码应保留原有半角标点。不要凭 OCR 结果把 `>=` 改成 `==`，也不要把 `-- args` 合并为 `--args`。中文是文档语言，不是另一套 Goblin++ 语法。

`RUN_STATUS=PASS` 不证明模型或科学结论正确。`VERIFICATION_STATUS=PASS` 验证保存的证据，不会重新进行实验。`GO_PARANOID` 与 SHA-256 不提供加密；源码、交互输入、数据和输出可能明文保存，不要输入密码、令牌或未经授权的保密研究材料。

## 帮助改进中文文档

本维护版是 AI 辅助翻译，经过代码保留、结构、术语及 PDF 检查，但未经独立的中文母语科学专业人员审定。欢迎在 [GitHub Issues](https://github.com/mrgogobot/goblinpp/issues) 用中文反馈（提交需要 GitHub 账户）；也可以按[贡献指南](../../CONTRIBUTING.md)提出修订。

反馈请包含：文件路径和章节、原句、建议译文、英文依据，以及涉及代码时的版本与最小复现。截图适合作为补充，不能替代精确的代码文本。请勿公开密码、私人数据或未脱敏的运行证据。

[社区校对目录](../../community-review/README.md)存放尚未验证或接受的 AI 建议，与本维护版分开。它们不是官方勘误，也不应直接覆盖教程或示例。

## 核对与许可

[翻译清单](TRANSLATION_MANIFEST.json)保存源提交、源文与译文哈希及历史参考来源；[质量检查记录](QUALITY_CHECKS.json)说明已完成的检查范围。`rust-alpha19-editor.json` 仅保留历史引用，不能用来替换当前编辑器配置。

原始许可与第三方声明原样保留。原创文档文字与图解的许可范围见 [LICENSE-DOCS.md](LICENSE-DOCS.md)；代码示例采用 [MIT](LICENSE)。中文许可说明仅供理解，不替代原条款。品牌素材不因收入翻译目录而重新许可。

---

**English:** This is the maintained Simplified Chinese documentation snapshot for alpha.21, including the explicitly historical alpha.12 tutorial. Code and commands are unchanged. Community AI proposals are separate and unverified. Chinese feedback is welcome; please provide source evidence and keep programming changes separate from translation corrections.
