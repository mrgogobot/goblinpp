> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Alpha.21 无损数值文本（本地、尚未发布）

- Rust：在 macOS arm64／Rust 1.92.0 上最终完整运行 `cargo test --locked --offline --all-targets`，170 项通过、0 项失败，无测试被过滤。格式、将警告视为错误的 Clippy 和空白检查通过。
- VS Code 0.1.16：16 项通过、0 项失败、无跳过；包含实际引擎 check／run／compile／verify 验收门槛及往返还原补全帮助。
- JetBrains 0.3.6：13 项通过、0 项失败；安装包针对缓存的 IntelliJ 2024.3.7／JDK 21 离线构建。不宣称完成新的实际 CLion／PyCharm 测试或多版本兼容验证。
- 五个新增 Rust 验收门槛检查 100,000 个确定性有限 f64 位模式，包含带符号零、次正规数、极值和记法边界；两个引擎的 32 值 print／template／to_text／parse 往返还原、CSV／TSV／文本字节、JSON／封存和独立验证；显式精度不变、带单位 SI 输出、数值拒绝、alpha.20 冻结修订约束及保留的协议违规证据。
- 未修改且经 SHA-256 验证的 alpha.20 本地二进制复现了数值文本截断和失败的往返比较。其真实保留运行在 alpha.21 下独立验证通过，没有改写旧证据。
- 已有能量规范化哈希、量纲、旧回执／注册表兼容性、科学函数和严格审计保管链验收门槛通过。

这些是本地结果，不是外部 CI、所有 f64 值的穷尽枚举、任意精度或跨平台确定性数学。没有新增依赖。GBL-004 已实现；GBL-005 起仍在队列中。参见[数值契约与迁移指南](NUMERIC_TEXT.md)。`tools/verify_local.py <package-receipt>` 还检查产物哈希、ZIP／清单完整性和隔离安装，在两种模式下运行五个示例，并独立验证。创建不可变归档后，另行报告包验证输出。

## 上一阶段：Alpha.20 受保护值

- Rust：在 macOS arm64／Rust 1.92.0 上使用 `cargo test --locked --offline --all-targets`，165 项通过、0 项失败。格式、将警告视为错误的 Clippy 和空白检查通过。没有新增依赖。
- VS Code 0.1.15：16 项通过、0 项失败、无跳过。包含实际引擎 check／run／compile／verify 契约及更新的常量／append／提示帮助。
- JetBrains 0.3.5：13 项通过、0 项失败；安装包使用缓存的 IntelliJ 2024.3.7／JDK 21 离线构建。这不是新的实际 CLion／PyCharm 冒烟测试或多版本兼容性验证。
- 十一个新增 Rust 验收门槛覆盖所有注册常量别名、嵌套绑定验证、丢弃仅返回值调用、独立 append 副本、保留的拒绝和预览诊断；跨循环／数组／返回值／连接／文本／CSV／TSV／JSON 输出的即时模板；字面／嵌套 JSON 花括号、未知字段、格式／大小限制、常量证据／封存、input／argv／导入花括号、独立原生执行、run／compile 中旧冻结修订约束、历史回执完整性、政策 diff 及标记篡改。
- 一个真实 alpha.19 错误复现运行在 alpha.20 下仍独立验证通过。合成历史冻结／回执测试另外覆盖迁移和政策边界，不改写真实用户证据。

这些是本地结果，不是外部 CI、干净机器依赖安装、经认证的作者身份，也不是完整 WB-1／WB-2 工作流的科学验证。GBL-004 起仍在队列中。参见[迁移](PROTECTED_VALUES.md)和待办清单。打包后，使用 `tools/verify_local.py <package-receipt>` 检查归档清单和隔离安装。

## 上一阶段：Alpha.19 表格、模块与编译 I/O

- Rust 测试（`cargo test --locked --offline --all-targets`）：macOS arm64／Rust 1.92.0 上 154 项通过、0 项失败。
- Rust 格式、将警告视为错误的 Clippy 和空白检查：通过。
- VS Code 0.1.14：16 项通过、0 项失败；激活、230 个唯一词汇项、导入／表格高亮和补全检查；实际 CLI check／run／verify 验收门槛除此前功能外，还覆盖 CSV 加导入函数及编译 FITS。
- JetBrains 0.3.4：12 项通过、0 项失败；缓存的 IntelliJ 2024.3.7／JDK 21 插件构建通过。本阶段不宣称新的实际 CLion／PyCharm 冒烟测试或多版本验证器检查。
- 九个新增 Rust 集成验收门槛覆盖 CSV／TSV 与传递／去重库及原生 FITS／输出／绘图的组合；带引号的 Unicode／BOM／CRLF／多行数据；显式数值拒绝与保留失败；表格限制、符号链接、无效库执行、路径穿越／循环依赖／重名、冻结库记法拒绝、原生插值名称安全、仅模块语义 diff、严格审计执行后模块变更，以及无需现存库的独立验证。篡改表格／模块字节、原生输出字节、支持源代码或原生清单会导致验证失败。
- 原生可执行文件在 PATH 无 Goblin++ 引擎时可运行。解释与原生输出／封存描述符及载荷字节一致。仍覆盖已有能量规范化哈希和历史回执／冻结检查。
- 原先全面拒绝编译 FITS 的测试现在证明：惰性分支和未调用函数不读取缺失数据。选中／加权 FITS 摘要在编译模式下独立验证通过。

这些是本地测试结果，不是外部 CI、干净机器依赖缓存测试、经认证的作者身份或科学正确性主张。Cargo 数据构建锁定且离线；独立程序保管链边界记录于 [I/O 指南](DATA_MODULES_NATIVE.md)。

## 上一阶段：Alpha.18 统计功能第一步

- Rust 测试（`cargo test --locked --all-targets`）：145 项通过、0 项失败。
- Rust 格式、将警告视为错误的 Clippy 和空白检查：通过。
- VS Code 0.1.13：16 项通过、0 项失败，包含实际 alpha.18 解释和编译统计运行及独立回执验证。
- JetBrains 0.3.3：11 项通过、0 项失败；使用缓存的 IntelliJ Platform 2024.3.7 和 JDK 21 成功构建插件。此步骤不重复 alpha.17 较广泛的多 IDE 兼容性验证，也不宣称实际 IDE 冒烟测试。
- `sum` 和 `mean` 覆盖已知值、兼容单位归一化、包含电流的 SI 量纲、单值／全零／负值观测、抵消补偿、大／微小均值、无效类型／参数数量／空／混合数组、溢出拒绝、两种证据模式、不修改输入、函数调用及严格冻结拒绝。失败回执独立验证通过。
- 归约算法由解释程序和生成的原生程序逐字共享；测试还直接拒绝原始非有限值。

这些结果描述本地 alpha.18 第一步源码，不是已发布发行版、外部 CI 运行，也不宣称其余统计功能已提供。

## 上一阶段：Alpha.17 验收结果

当前 alpha.17 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked --all-targets`：137 项通过、0 项失败。电学验收门槛覆盖全部 18 个函数、34 种单位拼写、预期值和量纲、Unicode 输入、无源元件定义域、微小并联电阻、保留名称、标量／数组原生证据，以及两个引擎中可独立验证的失败；
- `cargo clippy --locked --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check`：通过；
- 发布构建报告 `goblin++ 0.1.0-alpha.17`；
- VS Code 扩展 0.1.12：针对实际 alpha.17 二进制，16 项通过、0 项失败，包含编译化学／电学和 Unicode 单位输入；
- JetBrains 插件 0.3.2：11 项通过、0 项失败；Plugin Verifier 报告兼容 IntelliJ IC-243.28141.18、PyCharm PY-253.33514.19 和 CLion CL-262.10968.117；
- 完整 `examples/electrical.gbl` 在解释和编译模式均通过，回执独立验证且封存产物匹配；
- 真实归档 alpha.15 和 alpha.16 二进制分别在两种模式生成冻结能量运行；alpha.17 独立验证所有四个运行，并识别每项历史冻结为 `FROZEN_VERIFIED`；
- 此验收门槛的电学注册表 SHA-256 为 `982afcad3968913c96fcf82e2123d654b64580aa11738f7e114ea161f7513da1`。

这些是 alpha.17 工作树的本地源码检查结果，不是新公开发行版或干净机器复现。IDE 验证仅覆盖具名构建。引擎测试及预构建包针对 macOS arm64；其他引擎平台未在此验收门槛验证。

## 上一阶段：Alpha.16 验收结果

当前 alpha.16 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked --all-targets`：125 项通过、0 项失败。化学验收门槛覆盖值和量纲、不支持的语法、两个引擎的全部 43 个注册元素、可验证运行证据、冻结证据，以及历史 alpha.16 之前的冻结兼容性；
- `cargo clippy --locked --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check`：通过；
- 发布构建报告 `goblin++ 0.1.0-alpha.16`；
- VS Code 扩展 0.1.11：针对实际 alpha.16 二进制，16 项通过、0 项失败，包含编译化学和 Unicode 单位输入；
- JetBrains 插件 0.3.1：10 项通过、0 项失败；Plugin Verifier 报告兼容 IntelliJ IC-243.28141.18、PyCharm PY-253.33514.19 和 CLion CL-262.10968.117；
- `examples/chemistry.gbl` 在解释和原生编译模式均通过，注册表为 `IUPAC-2021-ABRIDGED-COMMON-v1`；两条路径也被独立验证的集成测试覆盖；
- 此验收门槛的化学注册表 SHA-256 为 `c26c5bf4c5df38fa25ffad332a7608727167650bb946f88f372e646c33059f9e`。

这些是 alpha.16 工作树的本地源码检查结果，不是新公开发行版或干净检出复现。平台验证器结果覆盖具名 IDE 构建；Rust 引擎测试在 macOS arm64 上运行。

## 上一阶段：Alpha.15 验收结果

当前 alpha.15 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked --all-targets`：118 项通过、0 项失败；五个新增科学集成测试覆盖参考转换、向量代数、坐标约定、线性和旋转运动、显式失败路径、保留名称、原生一致性及可独立验证运行；
- `cargo clippy --locked --all-targets -- -D warnings` 和 `cargo fmt --all -- --check`：通过；
- 发布构建报告 `goblin++ 0.1.0-alpha.15`；
- VS Code 扩展 0.1.10 针对当前二进制测试：16 项通过、0 项失败，包含真实 CLI check／run／verify 验收门槛；
- `examples/vector_motion.gbl` 在解释和原生编译模式均通过，两个保留运行都独立验证通过；
- alpha.13 无后缀三角函数调用保留弧度结果，并对每个使用的旧函数名发出一条保留的 `G302` 警告；较早的规范化哈希及 Python 0.0.7 兼容验收门槛保持不变。

这些是 alpha.15 工作树的本地源码检查结果，不是新公开发行版、干净检出复现或跨平台结果。公开 alpha.13 发行版及 Zenodo 记录保持不变。

## 上一阶段：Alpha.14 验收结果

Alpha.14 在 alpha.15 工作开始前通过 113 项 Rust 测试和 16 项 VS Code 测试。其验收门槛覆盖显式度／弧度正向及反向三角函数、`atan2`、角度转换、定义域和量纲拒绝、保留名称、旧名称警告、解释器／原生一致性及可独立验证运行。

## 上一阶段：Alpha.13 验收结果

当前 alpha.13 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked --all-targets`：112 项通过、0 项失败；五个新增 alpha.13 集成测试覆盖科学数学值和量纲、解释器／原生一致性、可验证成功和失败运行、定义域及参数数量拒绝，以及保留内置名称；
- `cargo clippy --locked --all-targets -- -D warnings` 和 `cargo fmt --all -- --check`：通过；
- 测试验收使用的调试构建报告 `goblin++ 0.1.0-alpha.13`；
- VS Code 扩展 0.1.8 针对当前二进制测试：16 项通过、0 项失败，包含真实 CLI check／run／verify 验收门槛；
- `examples/scientific_math.gbl` 由 `tests/math_builtins.rs` 中相同的解释器／编译器和回执验证契约覆盖；
- 较早的规范化哈希和保留的 Python 0.0.7 兼容语料：保持不变并通过。

这些是 alpha.13 工作树的本地源码检查结果，不是新公开发行版、干净检出复现或跨平台结果。公开 alpha.12 归档保持不变。

## 上一阶段：Alpha.12 验收结果

Alpha.12 在公开发布前通过 107 项 Rust 测试和 16 项 VS Code 测试。其控制流验收门槛覆盖短路布尔逻辑、最近循环的 `break`／`continue`、独立直接数组遍历、带检查的余数和整数解析、嵌套、类型／范围失败，以及解析时循环控制拒绝。

## 上一阶段：Alpha.11 验收结果

当前 alpha.11 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked`：101 项通过、0 项失败；三个新增集成测试覆盖缩放后的 FITS 选择、缺失单元格、可选正权重、跨越 8 MiB 分块边界的扫描、可验证负向路径，以及 FITS 调用的原生编译拒绝；
- `cargo clippy --locked --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check`：通过；
- `cargo build --locked --release`：通过；二进制报告 `goblin++ 0.1.0-alpha.11`；
- VS Code 扩展 0.1.6 针对该二进制：16 项通过、0 项失败，包含真实 `fits_select_stats` check／run／verify；
- 隔离的 `examples/fits_selection.gbl` 检查和严格审计运行：通过；选中 2 行、使用 2 行、权重和 10、加权平均 Z 1.6125；独立 `verify`：通过。

随附的三行 FITS 样例测试功能，不是 DESI 科学主张。真实星表筛选、权重和结果仍需独立科学验证。这是本地检查，不是公开发行版、干净检出复现或跨平台结果。

## 上一阶段：Alpha.10 验收结果

当前 alpha.10 本地源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked`：98 项通过、0 项失败，包含六个新增 `g_strings` 测试和一个明确的 Python 参考差异测试；
- `cargo clippy --locked --all-targets -- -D warnings`、格式和 `git diff --check`：通过；
- `cargo build --locked --release`：通过；二进制报告 `goblin++ 0.1.0-alpha.10`；
- VS Code 0.1.5 针对构建的 alpha.10 CLI 测试：16 项通过、0 项失败，包含编译文本操作冒烟测试；
- `examples/strings.gbl` 只读检查：`CHECK_STATUS=PASS`；
- 解释／原生文本操作、所提供输入的带检查转换、Unicode 标量长度、源代码冻结、封存值和独立运行验证：通过；
- 无效数值文本、存在精度风险的普通整数、空分隔符、混合类型和文本大小／片段上限：显式失败已测试。

文本 `+` 是有意的 alpha.10 扩展：Python 0.0.7 以 `G000` 拒绝它。保留参考语料仍通过，但不宣称此案例的精确语义一致。这些是本地检查，不是公开发行版或跨平台复现。

## 上一阶段：Alpha.9 验收结果

Alpha.9 源码检查验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked`：91 项通过、0 项失败，包含七个新增 `g_func` 测试；
- `cargo clippy --locked --all-targets -- -D warnings`：通过；
- `cargo fmt --all` 和 `git diff --check`：通过；
- VS Code 0.1.4 针对构建的 alpha.9 CLI 测试：16 项通过、0 项失败；
- 解释及编译函数调用、嵌套调用、提前返回、前向声明、局部数组副本、受限解释输出、冻结源代码、封存结果和独立运行验证：通过；
- 缺少 `return`、参数数量错误、递归限制、未知符号、无效声明及函数内编译 FITS 调用拒绝：显式失败已测试。

这些是本地源码检查结果，不是公开发行版、干净检出复现或跨平台结果。当移植验收门槛仍未完成时，Python 0.0.7 仍为规范性参考。

## 上一阶段：Alpha.8 验收结果

Alpha.8 验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --offline --locked`：84 项通过、0 项失败（80 个交互阶段和回归测试加四个数组测试）；
- `cargo clippy --offline --locked --all-targets -- -D warnings`：通过；
- `cargo fmt --all -- --check`：通过；
- VS Code 0.1.3 使用 alpha.8 CLI 的 `node --test`：16 项通过、0 项失败；
- 使用 `cargo build --release --offline --locked` 构建发布二进制并安装到隔离前缀：通过；
- 数组示例以解释和编译模式运行：均为 `PASS`，均可独立验证，`diff` = `IDENTICAL_RESULT`；
- 独立切片和赋值副本、`append`、`len`、索引循环、带类型数组封存、无效边界／类型、规范化哈希和冻结修改拒绝：通过；
- 交互文本输入、`--` 后从零开始的参数、保留交互证据和篡改检测：通过；
- 隔离校验和保管链账本审计：通过。不宣称作者身份认证。

当 `PORTING_MATRIX.md` 中待处理迁移验收门槛仍未完成时，原始 Python 0.0.7 参考仍具有规范性。macOS arm64 二进制不是跨平台构建。

## 上一阶段：Alpha.6 验收结果

当前 alpha.6 验收门槛（macOS arm64，Rust 1.92.0）：

- `cargo test --locked --offline`：72 项通过、0 项失败，包含七个新增分支测试；
- `cargo clippy --all-targets --locked --offline -- -D warnings`：通过；
- `cargo fmt -- --check`：通过；
- `if`／`else if`／`else` 和 `switch`／`case`／`default` 解释器／编译器一致性、封存值和独立运行验证：通过；
- 惰性分支求值、首次匹配／无贯穿、default 路径和嵌套循环分支：通过；
- 非布尔条件、单位／类型不匹配、格式错误分支语法、保留的执行机制失败、冻结分支修改拒绝，以及不可达分支内编译数据调用拒绝：通过；
- VS Code 0.1.1 编辑器／CLI 集成：15 项通过、0 项失败。

## 继承的 alpha.5 验收门槛

验证环境：macOS arm64，Rust 1.92.0。

发布验收门槛：

- `cargo test --locked --offline`：65 项通过、0 项失败；
- 测试分项：20 个库测试、17 个验收测试、5 个 CLI 测试、8 个控制流测试、8 个严格审计政策测试及 7 个参考兼容测试；
- 不含 `GO_PARANOID` 或 `seal` 的日常程序：通过，生成文本文件且运行可验证；
- 不含 `GO_PARANOID` 的 TXT／CSV／TSV／JSON／SVG／PNG 输出：通过，保留原有路径、重名、大小和哈希防护；
- 严格审计源代码执行后证据和独立重算哈希：通过；检测到篡改；
- 改变、缺失或无法保存的源代码结束证据：拒绝，并保留可验证失败；
- 损坏的严格审计运行证据：自检失败 `G405`，不登记账本；
- 运行回执 v1 验证兼容性及格式错误源代码的失败验证：通过；
- Python 0.0.7 规范性兼容：所有保留的语法、规范化、语义和错误代码案例通过；四个此前被拒绝的比较是有意的 alpha.4 新增项；
- 三个额外精确 Unicode 规范化哈希与 Python 0.0.7 匹配；
- `cargo clippy --all-targets --locked -- -D warnings`：通过；
- `cargo fmt -- --check`：通过；
- 解释／编译 `for` 和 `while` 输出、封存值和独立运行验证：通过；
- 有符号及零长度范围、嵌套循环、布尔条件、量纲检查和语法拒绝：通过；
- 无限 `while` 达到共享的一百万次迭代上限，并创建可验证的执行机制失败运行：通过；
- 使用解释循环进行真实 FITS 像素累积，并保留输入证据：通过；
- 执行前拒绝冻结循环体修改：通过；
- 循环内的编译 seal 在封存时对值做快照：通过；
- 发布构建及随附二进制安装器冒烟测试：通过；
- 发布版 `examples/loops.gbl` 解释和编译运行：通过；均已验证，虽执行引擎不同，diff 为 `IDENTICAL_RESULT`；
- 已安装可执行文件身份：Mach-O 64-bit arm64，调用名为 `goblin++`；
- 从独立生成器复现确定性 FITS 样例：逐字节通过；
- 使用已安装发布二进制运行输出教程：通过；
- TXT、CSV、TSV、JSON、SVG 和 PNG 类型识别：通过；
- PNG 解码器及视觉检查验收门槛：通过；
- 生成产物路径、SHA-256 和字节数验证：通过；
- 独立运行间确定性的生成输出哈希：通过；
- 改变生成输出：由 `verify` 检测；
- 绝对、嵌套和向父目录穿越的输出名：拒绝；
- 重复输出名：拒绝且不覆盖；
- 无 `GO_PARANOID` 的输出调用：alpha.5 接受且可独立验证；
- 执行空白或仅注释源代码：保留为执行机制失败，而不是 `PASS`；
- 带单位 JSON 值：保留 SI 值、量纲向量和单位；
- 编译输出／FITS 调用：显式拒绝，不进行不完整编译；
- 所有 alpha.2 FITS、证据、冻结、修订、账本、解释器、编译器和内联 Rust 测试仍通过。

真实文件绘图验收门槛：

- 输入：`specObj-dr16.fits`，6,727,078,080 字节，只读打开；
- 源 SHA-256 保持 `968ccae3f6ca7a166fcbf3d95e4a6d4b3be59d945e9328132b509f69f5687b0d`；
- 二进制表：5,789,200 行、133 个可读列；
- 5,000 行确定性 `Z` 直方图样本成功解码；
- 5,000 行确定性成对 `Z`／`Z_ERR` 散点样本成功解码；
- `goblin++ check` 报告两个生成输出预览，`CHECK_STATUS=PASS`；
- 权威性仍为 `PREVIEW_ONLY_NOT_EVIDENCE`；此验收门槛没有创建运行、绘图文件或星表副本；
- 未对未筛选样本声称任何科学解释。

已有 FITS 证据仍保留：

- 测试样例 SHA-256：`6724062c9bf311427ffa525ca547cb57bdbf461c36bc7321d4061d35295c3d71`；
- FITS 证据复用在 Unix 上使用硬链接身份，并拒绝损坏的存储；
- 大文件发现和完整哈希与独立操作系统校验和一致；
- 输入符号链接、随机组、格式错误结构和不支持的值类型仍被拒绝。

上述 alpha.3 真实文件 FITS／绘图验收门槛属于继承证据；未为 alpha.5 重新运行 6.7 GB 预览。此证据支持 alpha 政策实现，不是生产安全声明。它不免除 `PORTING_MATRIX.md` 中任何待处理项目。
