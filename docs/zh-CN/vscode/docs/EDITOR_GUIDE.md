> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ VS Code 编辑器指南

本指南面向 Rust 引擎 0.1.0-alpha.17 和扩展 0.1.12。

> 译者版本说明：这份指南保留了 alpha.17 的历史边界。alpha.19 起，FITS 和输出调用已支持编译执行；alpha.21 的当前规则参见引擎的 `docs/DATA_MODULES_NATIVE.md`。下文“不支持编译 FITS／输出”的陈述仅描述原指南的历史阶段。

## 1. 打开受信任的文件夹

在 VS Code 中打开 `.gbl` 项目文件夹；只有在信任文件和可执行文件后，才信任该文件夹。扩展不会在不受信任的工作区中启动 Goblin++ 进程。`.gbl` 文件会自动获得 Goblin++ 高亮和 Goblin 图标。

## 2. 选择可执行文件

如果 Goblin++ 命令已能在终端中运行，扩展可能自动找到它。在 macOS arm64 上，先检查受信任工作区中的 `dist/macos-arm64/goblin++`，再检查 `~/.local/bin/goblin++`，最后检查 `PATH`。如果自动查找失败，或你希望使用特定版本，请将 `goblinpp.executablePath` 设为绝对路径。扩展不使用 Python 或 `.venv`。

## 3. 编辑

补全和颜色覆盖赋值、基于 range 或直接数组的 `for`、`while`、`break`/`continue`、`and`/`or`/`not`、`if`/`else if`/`else`、`switch`/`case`/`default`、`g_func`/`return`、`g_strings` 调用、科学数学、向量／坐标／运动学辅助函数、布尔值、比较、数组、FITS 函数、文本／表格／绘图输出，以及可选的 `GO_PARANOID` 和 `seal`。成对括号自动闭合。代码片段包括 `everyday`、`paranoid`、`for`、`foreach`、`while`、`if`、`switch`、`g_func`、`sqrt`、`hypot`、`au2m`、`vector`、`spherical`、`rotation`、`parse_number`、`parse_integer`、`str_split`、`array`、`append`、`input`、`argv`、`fitsmean`、`fitsselect` 和 `writecsv`。

建议是辅助，不是科学公式正确性的证明。FITS 元数据、列名和单位仍需要人为解释。Rust 解析器和求值器才是权威。

使用 **Goblin++: Insert Scientific Symbol**（插入科学符号）输入受支持的别名和记法。`hbar`/`ħ` 是已注册常数的拼写；`ω` 只是合法标识符字符，除非你为它赋值。**Goblin++: Toggle Icon View**（切换图标视图）显示视觉装饰，不改变源文件字节。**Set Icon for Identifier**（为标识符设置图标）创建显式的*仅视觉*映射。语言中不存在 `[iconmode]` 语法。Icon View 跳过字符串、注释和内联 Rust 代码块。

## 4. 检查、运行和验证

Command Palette（命令面板）提供：

- **Goblin++: Check Current File** — 只读预览，不是证据；
- **Goblin++: Run Current File** — 解释器运行，生成回执；
- **Goblin++: Run Current File With Arguments** — 输入文本参数的 JSON 数组，在 `--` 之后传递；
- **Goblin++: Compile and Run Current File** — 在受支持的情况下执行原生编译程序；
- **Goblin++: Verify Run Directory** — 对保存的证据重新计算哈希；
- **Goblin++: Show Current File Status** 和 **Show Current File Lineage** — 显示当前文件状态及修订谱系；
- **Goblin++: Audit Workspace Ledger** 和 **Check Workspace** — 审计工作区账本及检查工作区；
- **Goblin++: Freeze Current File** 和 **Create Child Revision** — 冻结当前文件及创建子修订。

编辑器在 **Goblin++** 输出通道显示命令输出。Run（运行）会先保存文件。只读的 Status（状态）和 Lineage（修订谱系）会要求先保存已修改的缓冲区，而不是检查磁盘上的过期文本。保存时检查默认禁用，直到你明确启用 **Goblin++: Toggle Check on Save**（切换保存时检查）。

当运行调用 `input()` 时，扩展显示输入框并转发回答。回答和参数以明文交互证据保存；绝不要用它输入秘密。`check` 不会请求输入。数组切片和 `append` 返回独立副本，不是 Go 风格的共享视图。

状态栏区分命令通过、预览失败、运行机制失败、协议违规和运行验证失败。它只是摘要；证据是 CLI 回执和输出。

编译 FITS 和输出调用目前会被拒绝。内联 Rust 代码块需要在 CLI 中按 SHA-256 明确授权，并以用户的操作系统权限运行。编译运行按钮不会自动授权代码块。

## 5. 仅在审查后冻结

冻结锁定精确的源文件字节。扩展会请求模态确认。后续编辑——即使只改变记法——也必须通过带非空理由的显式子修订完成。CLI 独立执行该规则。不要删除回执或账本文件来清除拒绝；请使用 Status、Lineage、Audit Ledger 和 Verify 了解原因。

## 故障排查

如果 Goblin++ 无法启动，请将 `goblinpp.executablePath` 设为 Rust 二进制文件的绝对路径，并在终端运行 `goblin++ --version`。如果没有高亮，请确认文件以 `.gbl` 结尾，或在 VS Code 的语言选择器中选择 **Goblin++**。如果预览通过而运行拒绝，这是因为预览有意不检查冻结或保管状态；请检查完整运行输出。
