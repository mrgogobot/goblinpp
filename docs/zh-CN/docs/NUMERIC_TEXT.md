> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 无损的默认数值文本（alpha.21）

默认数值文本采用能够往返还原**已存储的有限 f64** 的最短十进制表示，包括 `-0`。这防止输出悄悄丢失精度。它不会使浮点算术变得精确，也不保证跨平台超越函数结果完全相同。

```goblin
value = 0.12345678901234567
saved = to_text(value)
restored = parse_number(saved)
same = value == restored
print("{saved}; {same}")       # 0.12345678901234566; true
print("{value:.3f}")          # 0.123, explicitly rounded
```

上述源码十进制数会舍入到最接近的可表示二进制浮点值。输出恢复的是该已存储值，而不是源码写法中的每一位数字。超出 f64 精确整数范围的星表 ID 仍需要未来的精确整数功能；这次更改不会让它们变得安全。

## 适用范围

- `print(value)`、默认 `{value}` 占位符、数组显示和 `to_text`。
- `write_csv` / `write_tsv` 中的数值单元格及 `write_text` 的数值参数。
- 解释器和生成的原生代码共同使用同一个格式化器。

非零数的绝对值小于 `1e-4` 或至少为 `1e15` 时使用科学记数法；其他值采用定点记法。指数带显式符号。科学记数法使 `parse_number` 能够读取大浮点数，同时不会放宽它对可能已舍入的纯整数输入的拒绝规则。

`-0` 保持为 `-0`；极小数／次正规数不会被替换为零。`parse_number(to_text(value))` 逐位往返还原有限的**无单位**值。带单位的值包含其 SI 单位标签：`to_text(2 kg)` 产生 `2 kg`，而 `parse_number` 有意拒绝它。复合单位输入仍在待办列表中。

显式 `.Nf`、`.Ne` 和 printf 精度仍是展示选择，可能丢失精度。图表标签／坐标保留现有展示精度；不要将图像／SVG 几何数据用作数值数据导出。

JSON 导出和封存物理量已经使用往返还原序列化，原生封存清单也已经存储精确位模式。这些机制得到保留和测试，而不是被替换。非有限结果、溢出、数值解析中非零数下溢为零，以及不安全的纯整数文本仍会被拒绝。

## 冻结项目与历史证据

部分十进制输出及其 SHA-256 哈希因此发生变化。Alpha.21 在新运行和冻结回执中已纳入哈希的 `language_semantics` 字段记录 `goblin.eager-text-and-roundtrip-numbers.v2`。

历史运行和旧冻结的完整性仍可验证。在 alpha.21 下运行或编译 alpha.20（或更早版本）的冻结会以 `LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE` 被拒绝；通过经审计的运行器启动时，会保留协议违规运行。不要删除或重写旧冻结。请显式采用该变更：

```console
goblin++ revise experiment.gbl experiment_R1.gbl --reason "adopt lossless numeric text"
goblin++ freeze experiment_R1.gbl
goblin++ experiment_R1.gbl
```

冻结修订后的源码之前，请先审查它。跨策略比较报告 `LANGUAGE_SEMANTICS_CHANGE`，即使特定程序的输出没有改变也如此。若需完全相同的旧输出，请保留旧引擎和已归档证据。

可尝试 `examples/numeric_text.gbl`。此阶段不增加依赖、新数学函数、更高资源上限或跨平台确定性数学。
