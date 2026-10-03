> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ for Visual Studio Code 0.1.16（本地，尚未发布）

> 译者版本说明：标题中的“本地，尚未发布”是源文档快照保留的标签，不表示 alpha.21 发布状态。本文末尾仍有较早阶段“不支持编译 FITS／输出”的表述；alpha.19 起已支持，当前规则请参阅引擎的 `docs/DATA_MODULES_NATIVE.md`。下文保留原文的历史描述。

为 Goblin++ Rust 引擎 **0.1.0-alpha.21** 提供编辑器支持（默认数值文本无损）。补全帮助说明可往返还原的数值文本和带符号零；只读常数、必须使用的 append 副本结果以及立即捕获模板值的规则仍然保留。本扩展保持现有的 `goblinpp-project.goblinpp` 标识，更新较早的扩展，而不是创建第二个语言模式。

补全和高亮包含 `import`、CSV/TSV 读取器以及已有的 FITS／输出调用；这些调用现在也可用于编译执行。严格的表格和本地库规则请参阅引擎的 `docs/DATA_MODULES_NATIVE.md`。

它提供 `.gbl` 识别、文件图标、语法高亮、补全、悬停帮助、代码片段、文档符号、可选的非语义 Icon View（图标视图），以及显式的运行、编译运行、检查、验证、冻结、修订、状态、修订谱系、账本审计和 doctor 命令。词汇覆盖循环、分支、`g_func`/`return`、`g_strings` 文本操作、数组、交互、布尔值、FITS 读取器、生成输出函数，以及可选的 `GO_PARANOID`/`seal` 指令。随包的 0.0.7 词汇表只为常数别名和单位而保留；Rust 增补是编辑器辅助，不是新的规范性语言说明。

## 安装

这个源码仓库包含扩展代码，而不是打包好的 VSIX。以下安装说明适用于另行构建并审核的 VSIX 发布包。

扩展软件和图标采用 [MIT 许可证](LICENSE)；其原创教程／文档正文和图示采用 [CC BY 4.0 许可证](LICENSE-DOCS.md)。代码示例仍采用 MIT 许可证。

在 VS Code 中打开 **Extensions → ⋯ → Install from VSIX…**（扩展 → 更多 → 从 VSIX 安装），选择 `goblinpp-vscode-0.1.16.vsix`。如出现提示，请重新加载 VS Code。Goblin++ Rust 引擎需另行安装；扩展不随包提供或安装引擎。

扩展依次按以下顺序寻找可执行文件：

1. 如果设置了 `goblinpp.executablePath`，首先使用它；
2. 受信任工作区内 macOS arm64 的 `dist/macos-arm64/goblin++`；
3. macOS/Linux 上的 `~/.local/bin/goblin++`；
4. `PATH` 中的 `goblin++`。

如果 VS Code 找不到可执行文件，请把 **Goblin++: Executable Path**（可执行文件路径）设为绝对路径。旧的 `goblinpp.pythonPath` 设置已不再使用。Rust 引擎目前附带 macOS arm64 二进制文件；其他平台需要从源码构建。

## 日常模式或严格审计模式

日常 `.gbl` 程序既不需要 `GO_PARANOID`，也不需要 `seal`：

```goblin
samples = 12
accepted = 9
fraction = accepted / samples
print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

生成的文件仍限制在 `RUN_DIR/outputs` 内，并计算哈希。可选的 `seal value` 将具名值保存为单独的带类型工件。`GO_PARANOID` 启用运行结束后的源文件证据检查，并在登记到账本之前设置回执验证关卡。它不是操作系统沙箱，也不是持续的源文件监视器。

`if` 和 `switch` 代码片段生成真实的 Goblin++ 代码块。条件必须是布尔值；switch 使用第一个完全匹配的 case，不会贯穿执行后续 case。完整示例参见引擎的 `examples/branching.gbl` 和 `docs/CONTROL_FLOW.md`。

`input()` 的运行提示显示在 VS Code 输入框中；回答以明文运行证据保存，因此不要输入秘密。**Run Current File With Arguments**（带参数运行当前文件）要求输入 JSON 参数数组。数组提供 `len`、`append` 和独立切片的补全与代码片段；参见引擎的 `docs/INTERACTION_AND_ARRAYS.md`。

文本补全覆盖 `parse_number`、`parse_integer`、`to_text`、`str_trim`、`str_contains`、`str_replace`、`str_split` 和 `str_join`。控制流帮助覆盖直接数组迭代、`break`/`continue`、`and`/`or`/`not` 和 `%`。普通文本 `+` 和 `len(text)` 在引擎中可用。准确的 Unicode、数值和大小限制语义参见 `docs/STRINGS.md`。

科学数学补全覆盖 `abs`、`sqrt`、`min`、`max`、舍入、指数／对数函数、明确区分度／弧度的三角函数、`atan2d`/`atan2r`、角度换算和 `hypot`。定义域和量纲规则由引擎强制执行，而不是编辑器；参见引擎的 `docs/MATH.md`。

科学补全还覆盖天文距离换算、同质向量的 `magnitude`/`dot`/`cross`、显式的极坐标和球坐标换算、速度、明确命名的伽利略速度叠加与共线相对论速度叠加，以及旋转运动辅助函数。坐标约定会显示在补全详情中；引擎始终是权威。参见引擎的 `docs/VECTORS_COORDINATES_KINEMATICS.md`。

化学补全覆盖范围明确的元素注册表、化学式摩尔质量、质量／物质的量换算、物质的量浓度、稀释、实验室单位以及常数 `R` 和 `m_u`。编辑器绝不会推断化学式含义；元素可用性、量纲、注册表证据和拒绝规则都由引擎负责。参见引擎的 `docs/CHEMISTRY.md`。

## 信任边界

扩展使用参数数组启动 `goblin++`，不使用 shell。在不受信任的工作区中，进程命令被禁用。运行和编译运行都会先保存当前文件。冻结和修订需要明确确认；修订还要求新路径和理由。保存时检查默认关闭，而且只是预览，绝非运行证据。有效性和保管状态由 Rust CLI 决定，而不是语法颜色、建议或本扩展。

编译运行尚不支持 FITS 或输出调用。内联 Rust 是任意原生代码，必须通过 CLI 按确切哈希授权；VS Code 的编译运行按钮不会绕过这一审批。

参见[编辑器指南](docs/EDITOR_GUIDE.md)、[编写教程](docs/AUTHORING_TUTORIAL.md)和[只读检查](docs/CHECK.md)。
