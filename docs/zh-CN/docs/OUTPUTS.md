> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 经审计的文件与图表

Goblin++ 0.1.0-alpha.19 可在解释执行及经审计的编译执行中生成文本、Markdown、CSV、TSV、JSON、SVG 和 PNG 产物。生成的文件绝不会出现在源码程序旁边，而是仅在唯一运行目录内以排他创建方式生成：

```text
RUN_DIR/outputs/filename
```

运行回执记录每个输出的相对路径、媒体类型、字节数、生成者、元数据和 SHA-256。`goblin++ verify RUN_DIR` 独立检查字节和大小。生成的文件名不能是绝对路径、包含目录，或通过 `..` 遍历路径；声明同一文件名两次会被拒绝，而不是视为覆盖。

生成输出的调用不要求 `GO_PARANOID` 或 `seal`。在日常程序和严格审计模式程序中，它们都采用相同的路径限制、仅创建新文件行为、回执哈希和独立验证。`GO_PARANOID` 增加运行后源码观察和回执自验证关卡；`seal` 则单独将具名值快照为带类型的产物。

## 文本

`write_text` 接受文件名及其后的一个或多个值。每个值成为一行。文本值使用与 `print` 相同的 `{variable}` 插值。

```goblin
samples = 12
accepted = 9
fraction = accepted / samples

write_text("summary.txt", "Acceptance report", "samples = {samples}", "accepted = {accepted}", "fraction = {fraction:.3f}")
write_text("notes.md", "# Acceptance report", "The accepted fraction is {fraction:.3f}.")

```

支持的扩展名为 `.txt` 和 `.md`。

## CSV 与 TSV

`write_csv(filename, columns, cells...)` 将列数之后的所有单元格按行分组。第一行为表头。单元格数量必须能被声明的列数整除。应直接传入值；不需要先将它们转为字符串。

```goblin
write_csv("summary.csv", 2, "metric", "value", "samples", samples, "accepted", accepted, "fraction", fraction)
write_tsv("summary.tsv", 2, "metric", "value", "samples", samples, "fraction", fraction)
```

含逗号、引号或换行的 CSV 单元格会用引号括起，内嵌引号会双写。TSV 文本单元格中的控制分隔符会替换为空格。这个初始输出阶段旨在支持结果表和适量的派生数据，而不是星表批量导出。

## JSON

`write_json(filename, key, value, ...)` 创建一个 JSON 对象。键必须是唯一字符串。

```goblin
write_json("summary.json", "samples", samples, "accepted", accepted, "fraction", fraction)
```

无量纲物理量变为 JSON 数字。有量纲物理量保留其 SI 值、量纲向量和 SI 单位名称，而不是悄悄丢失单位信息。

## FITS 直方图

```goblin
plot_fits_histogram("redshift.png", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
plot_fits_histogram("redshift.svg", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
```

参数依次为：

1. 输出文件名（`.png` 或 `.svg`）；
2. FITS 文件名；
3. 从零开始编号的二进制表 HDU；
4. 标量数值列；
5. 分箱数，范围为 2 至 500；
6. 最大采样行数，范围为 1 至 200,000；
7. 图标题。

## FITS 散点图

```goblin
plot_fits_scatter("z_error.png", file, 1, "Z", "Z_ERR", 50000, "Redshift and reported error")
```

参数依次为输出文件名、FITS 文件名、HDU、x 列、y 列、最大采样行数和标题。两列都必须是固定宽度的标量实数数值列。

## 采样是明确的

绘图在整个表格中以确定性的等间隔行索引选取数据，行数不超过请求值。跳过空值和 NaN。回执记录：

- 采样方法；
- 总体行数；
- 请求点数；
- 检查的行数；
- 有效绘制点数；
- 采样最小值及最大值；
- 输入 FITS 的 SHA-256。

因此，相同源码、FITS 字节和参数会产生相同的图像字节和哈希。这些图是描述性的采样视图。它们不会被悄悄当作全总体统计呈现，也不会施加天体类别、质量、警告掩码或巡天选择过滤条件。

## 为什么使用 SVG 和 PNG，而不是 JPEG？

SVG 适合可缩放图表，而 PNG 是无损格式。JPEG 通过有损压缩改变像素，可能引入视觉伪影，因此 alpha.3 不将其提供为权威科学输出。未来的展示导出功能可能增加 JPEG，同时保留无损原件作为已封存的权威版本。

## 验证

运行之后：

```console
goblin++ verify RUN_DIR
```

成功的输出检查如下：

```text
GENERATED_ARTIFACT_PATH:summary.csv .... PASS
GENERATED_ARTIFACT:summary.csv ......... PASS
GENERATED_ARTIFACT_BYTE_COUNT:summary.csv PASS
```

编辑、替换、截断或删除输出都会导致验证失败。

## 当前限制

- 两个引擎共享 Rust 输出辅助函数。经审计的编译执行将原生字节与参考输出比较；独立二进制程序输出具有不同边界，见 [原生 I/O 指南](DATA_MODULES_NATIVE.md)。
- 在此初始阶段，每个输出上限为 64 MiB。
- 输出文件名是扁平的，文件位于 `RUN_DIR/outputs` 下。
- FITS 到表格的批量导出仍等待明确的行选择和过滤语义。
- 尚未实现 FITS 写入；可信的写入器必须显式保留模式、头、单位和来源。
