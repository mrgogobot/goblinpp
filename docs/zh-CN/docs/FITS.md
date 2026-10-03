> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# FITS 结构探查与导入

Goblin++ 将 FITS 结构与科学解释视为两个独立问题。在选择要分析的数值之前，先了解文件包含什么。

## 先检查

若需快速探查大型文件的结构：

```console
goblin++ fits-info specObj-dr16.fits --quick
```

此命令读取每个 HDU 的头，但有意不对完整文件计算哈希。其输出标记为 `QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE`。

若需将完整文件的 SHA-256 纳入检查报告：

```console
goblin++ fits-info specObj-dr16.fits
goblin++ fits-info specObj-dr16.fits --json
```

两种检查模式都不会创建运行回执。正常 `.gbl` 执行则会创建。

## 索引约定

- HDU 索引从零开始：主 HDU 为 `0`，第一个扩展为 `1`。
- 表格行和重复列元素的索引从零开始。
- 图像轴遵循 FITS 记法，从一开始：`NAXIS1` 是轴 `1`。

## 函数

| 函数 | 结果 |
|---|---|
| `fits_hdu_count(file)` | HDU 数量 |
| `fits_header(file, key)` | 主头中的值（兼容形式） |
| `fits_header(file, hdu, key)` | 显式指定 HDU 头中的值 |
| `fits_axis(file, axis)` | 主图像的轴长度（兼容形式） |
| `fits_axis(file, hdu, axis)` | 显式指定图像 HDU 的轴长度 |
| `fits_count(file[, hdu])` | 图像元素数量 |
| `fits_pixel(file[, hdu], index)` | 展平后的图像值 |
| `fits_mean(file[, hdu])` | 有限且非 `BLANK` 图像值的均值 |
| `fits_rows(file, hdu)` | 二进制表的行数 |
| `fits_columns(file, hdu)` | 二进制表的列数 |
| `fits_column(file, hdu, name, row)` | 标量数值、逻辑或文本单元格 |
| `fits_column(file, hdu, name, row, element)` | 重复数值／逻辑列中的元素 |
| `fits_column_valid_count(file, hdu, name)` | 非空数值元素数量 |
| `fits_column_mean(file, hdu, name)` | 非空数值元素的补偿求和均值 |
| `fits_column_min(file, hdu, name)` | 最小非空数值元素 |
| `fits_column_max(file, hdu, name)` | 最大非空数值元素 |
| `fits_select_stats(file, hdu, selection, lower, upper, value[, weight])` | 标量数值列上的选中行数、权重和与均值 |

直接读取空值单元格返回 `G601`；聚合统计跳过整数 `TNULL` 值和浮点 NaN。列匹配不区分 ASCII 大小写。重复数值列在直接读取单元格时要求第五个 `element` 参数，而统计包含所有元素。

## 筛选和加权表格摘要

`fits_select_stats` 以有大小限制的数据块扫描完整表格（每行／块最多 8 MiB），不采样，也不修改 FITS 文件。其选择列、值列及可选权重列都必须是受支持的*标量数值*列。下界包含在区间内，上界不包含：`lower <= selection < upper`。边界必须是有限的无量纲数，且满足 `lower < upper`。FITS `TSCAL`/`TZERO` 应用于物理值；整数 `TNULL` 和浮点 NaN 按 FITS 存储规则视为缺失值。缺失的选择值不能匹配。值或权重缺失的选中行计入选中数量，但不用于计算。每个选中行中的非缺失权重都必须严格为正；零或负权重会明确失败。没有可用行时会报错，而不会编造均值。

返回数组包含四个无量纲值：`[selected_rows, used_rows, weight_sum, mean]`。没有权重列时，每个可用行的权重为 1，`mean` 是普通的选中样本均值。有权重列时，`mean = sum(value * weight) / sum(weight)`；求和使用补偿累加，任何非有限乘积或和都会被拒绝。

```goblin
GO_PARANOID
stats = fits_select_stats("sample.fits", 1, "Z", 0, 3, "Z", "QUALITY")
selected = stats[0]
used = stats[1]
weighted_mean = stats[3]
print("selected = {selected}; used = {used}; weighted mean = {weighted_mean}")
seal stats
```

精确区间、列和权重绝不会从星表中推断：应根据科学问题选择它们。运行回执记录源码、输入哈希和结构化访问描述；`seal stats` 将结果保存为接受独立检查的产物。此初始操作不会施加巡天专用质量筛选、不确定度处理、随机星表，或 FITS `TUNIT` 转换。Alpha.19 在两个引擎中通过相同 Rust 辅助函数支持这些 FITS 调用。有关编译和证据边界，请参阅 `examples/fits_selection.gbl` 及 [原生 I/O 指南](DATA_MODULES_NATIVE.md)。

## 星表示例

```goblin
GO_PARANOID

rows = fits_rows("specObj-dr16.fits", 1)
first_class = fits_column("specObj-dr16.fits", 1, "CLASS", 0)
first_z = fits_column("specObj-dr16.fits", 1, "Z", 0)
valid_z = fits_column_valid_count("specObj-dr16.fits", 1, "Z")
mean_z = fits_column_mean("specObj-dr16.fits", 1, "Z")

print("rows = {rows}")
print("first class = {first_class}")
print("first redshift = {first_z}")
print("valid redshifts = {valid_z}")
print("unfiltered mean redshift = {mean_z}")

seal rows
seal valid_z
seal mean_z
```

上述均值有意标记为未筛选。Goblin++ 不会根据列名推断样本选择、质量筛选、天体类别、警告掩码或宇宙学含义。在结果可以解释之前，这些科学选择必须成为明确的语言功能或明确的源码。

## 证据与大型文件

首次对大型 FITS 文件进行证据级运行时，会读取文件以计算哈希，然后再次读取以建立经过验证、按校验和寻址的证据对象。请确保项目文件系统有足够的可用空间保存一份副本。后续运行在支持硬链接时，将该对象硬链接到各自运行目录，因此每次运行不会再占用另一份完整副本的空间。

`--quick` 适合探查结构，绝不适合提出证据性主张。

## 明确的限制

本版本不读取 ASCII 表格值、随机组、分块压缩图像、位数组、复数值或变长数组堆。它会在可能的情况下报告这些结构，并拒绝不受支持的访问。不解释 WCS、声明的 FITS 单位、不确定度模型、过滤条件，以及星表专用语义。
