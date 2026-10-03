> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 表格、可复用函数与原生科学 I/O

Alpha.19 增加了严格的 CSV/TSV 输入、本地函数库，以及对现有 FITS 读取器和文件／绘图写入器的编译支持。这些是小型、显式的工具：并不意味着会猜测单位、自动决定缺失值处理策略、提供包管理器、施加巡天筛选条件，或增加新的 FITS 格式。

## 第一个实验

从项目目录运行所提供的示例：

```console
goblin++ examples/csv_modules.gbl
goblin++ examples/csv_modules.gbl --compile
goblin++ verify RUN_DIR
```

将 `RUN_DIR` 替换为每次运行打印的目录。两种模式都会报告三个观测值、`11 m` 的平均长度，以及 `3` 的 TSV 均值。文件位于该次运行的 `outputs` 目录下。每次运行都保存表格字节、库字节、日志、封存值及其哈希。

输入 `measurements.csv` 有意保持简单：

```csv
sample,length_m
A,10
B,11
C,12
```

程序显式读取其数值列：

```goblin
import "lib/measurements.gbl"
numbers = csv_numbers("measurements.csv", "length_m")
average = average_length(numbers)
print("average length = {average}")
seal average
```

库附加米这个单位，因为实验定义了该单位。`length_m` 这样的表头只是描述性文本，不是推断量纲的指令：

```goblin
g_func average_length(numbers) {
    lengths = []
    for number in numbers {
        lengths = append(lengths, number * 1 m)
    }
    return mean(lengths)
}
```

## CSV 与 TSV API

| CSV 调用 | 结果 |
|---|---|
| `csv_rows(file)` | 数据行数，不包括表头 |
| `csv_columns(file)` | 表头／列的数量 |
| `csv_headers(file)` | 精确保留且区分大小写的表头字符串数组 |
| `csv_column(file, "name")` | 字符串数组，包括空单元格 |
| `csv_numbers(file, "name")` | 经过检查的无量纲数值数组 |

TSV 使用完全相同的 API，前缀为 `tsv_`。对制表符分隔文件使用它；系统绝不会猜测分隔符。路径相对于**入口程序所在目录**解析，包括导入函数发起的调用。

输入必须是 UTF-8（接受开头的 UTF-8 BOM），记录使用 LF 或 CRLF，且必须有非空表头，各名称唯一且非空。带引号的字段支持双写引号，以及嵌入的分隔符／换行。行宽不一致、引号格式错误和未知列会失败。只有表头的表格有效，并返回空数组；`mean` 和 `sum` 仍会拒绝空观测集合。

文本列保留空单元格。数值列拒绝空单元格、格式错误的数字、NaN 和无穷大：**不会悄悄丢弃任何行**。它们使用与 `parse_number` 相同的受检转换。当数据有缺失值时，请在分析前自行提供明确的清理策略。限制为每个输入 16 MiB、100,000 个数据行、1,024 列及总计 1,000,000 个单元格。这是面向日常实验表格的输入功能，而不是巨型星表加载器。

首次加载会为字节建立快照；一次运行中的后续调用使用该快照。回执记录输入哈希、大小、分隔格式及请求的操作／列。独立验证读取已保存的证据，而不是可能已改变的实时输入。符号链接输入会被拒绝。

## 本地模块

`import "lib/measurements.gbl"` 是顶层的静态声明。库只能包含 `g_func` 定义和进一步的导入。顶层执行、变量、指令、封存和内联 Rust 都会被拒绝。函数保留现有的局部值复制作用域和 16 层调用深度上限。所有函数共享一个命名空间；重复名称会失败，而不是遮蔽另一个库。

导入相对于导入文件解析，必须位于入口目录之下，且必须使用 `.gbl` 文件。绝对路径、`..`、`.` 路径分量、反斜杠、符号链接路径及循环依赖会失败。重复导入会去重。限制为 32 个不同模块，每个模块 1 MiB。此阶段没有下载包、动态导入、带命名空间的导入或模块全局变量。

`check`、解释执行、编译执行和 `compile` 解析同一依赖图。规范化程序包含导入的函数定义；原始库字节单独保存在 `RUN_DIR/modules` 下。验证从这些快照重建依赖图，不需要原始库文件。

除了根源码和注册表之外，`freeze` 还固定每个导入路径及原始哈希。冻结后即使只修改库中的注释，也会以 `MODULE_CHANGED_AFTER_FREEZE` 拒绝运行。`GO_PARANOID` 也会在运行后检查库。修订复制入口源码，**不会复制库**：若要有意修订库，请保留一份版本化的库副本，并修改新入口文件的导入。不要编辑多个冻结程序共用的库。

## 编译执行中的 FITS 与文件

所有当前已实现的 FITS 调用、`write_text`、`write_csv`、`write_tsv`、`write_json`，以及 FITS 直方图／散点图现在均可编译。现有的科学计算语义、路径限制、量纲、空值处理和绘图采样保持不变。这不会增加 FITS 写入、压缩、WCS 变换、隐式 `TUNIT` 转换、批量导出或 JPEG。

```console
goblin++ examples/output_demo.gbl --compile
goblin++ examples/fits_selection.gbl --compile
```

原生程序包含直接生成的 Rust 操作，并调用共享 Rust 数据／输出辅助函数。它们不启动 Goblin++、Python 或解释器进程，也不在运行时解析 `.gbl` 文件。共享辅助函数源码和锁定依赖文件会保存在已计算哈希的编译器支持文件清单中。

编译使用数据的程序需要 Rust/Cargo 和已缓存的锁定 crate。生成的支持库构建使用 `--locked --offline`：绝不会悄悄获取依赖。在源码检出目录中，`cargo build --locked` 可先填充缓存（此命令可能需要网络访问）。不含数据／输出调用的程序仍采用更简单的 `rustc` 编译路径。

对于经审计的编译执行，启动器先执行参考求值，然后原生可执行文件独立运行。输入哈希／访问、输出描述信息、封存值和输出字节必须一致，才能获得 PASS。这需要两次求值；它是一致性关卡，不是性能宣称。`verify` 也会独立检查原生输出字节和清单。不受限制的内联 Rust 仍是单独授权的信任边界，不是沙箱。

## 独立可执行文件的边界

```console
goblin++ compile examples/csv_modules.gbl -o measurements
```

运行可执行文件时，数据文件必须**相对于其工作目录**可用，而不是相对于旧源码位置。它不再需要 `.gbl` 库或已安装的 Goblin++ 引擎。数据／输出构建创建新的 `goblin-native-outputs-PID-TIMESTAMP` 目录，在 stderr 打印其位置，并写入 `native-data.json` 和一个 `native-outputs` 子目录。不会覆盖现有输出文件。

独立可执行文件**不提供**启动器的完整保管链回执、冻结强制执行或 `GO_PARANOID` 运行后验证。需要这些保证时，请使用 `goblin++ file.gbl --compile`。独立清单是有用的元数据，不是经过身份认证的保管链账本。

所有保存的证据都是明文。哈希检测相对于已记录校验和的变化；它不会加密数据，也不会认证作者身份。
