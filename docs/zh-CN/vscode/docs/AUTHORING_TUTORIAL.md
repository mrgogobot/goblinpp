> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 编写你的第一个 Goblin++ 程序

这份简短教程使用 Rust 引擎 0.1.0-alpha.17。在受信任的项目文件夹中创建一个新的 `.gbl` 文件。VS Code 扩展帮助你输入代码；代码的含义由 Goblin++ 决定。

> 译者版本说明：本教程描述 alpha.17 的历史边界。alpha.19 起，FITS 和生成输出调用已支持编译执行；alpha.21 的准确规则请参阅引擎的 `docs/DATA_MODULES_NATIVE.md`。末尾保留原文历史边界说明，避免将旧教程误当作当前限制。

## 1. 从日常计算开始

编写 `acceptance_ratio.gbl`：

```goblin
samples = 12
accepted = 9
fraction = accepted / samples

print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

选择 **Goblin++: Run Current File**（运行当前文件）。你应看到 `accepted fraction = 0.750` 和 `RUN_STATUS=PASS`。Goblin++ 将 `answer.txt` 放在报告的 `RUN_DIR/outputs` 下，而不是源文件旁边。不要求 `GO_PARANOID` 或 `seal`。运行仍具有回执和哈希。

这回答了什么问题？“12 个样本中有多大比例被接受？”结果是无量纲的。一次运行在技术上有效，仍可能回答了错误的科学问题，因此在开展正式分析前，请写明问题和假设。

## 2. 使用循环

```goblin
total = 0
for n in range(1, 6) {
    total = total + n
}
print("total = {total}")
```

`range(1, 6)` 遍历 1 到 5：不包含终止值。当条件为布尔值时，也可使用 `while condition { ... }`。解释器与编译器共用一百万次循环体迭代的限制；它能发现失控循环，但不是安全沙箱。

你也可以为可重用的计算命名。在编辑器中输入 `g_func`，获得起始代码片段：

```goblin
g_func fraction_of(accepted, samples) {
    return accepted / samples
}

fraction = fraction_of(9, 12)
print("fraction = {fraction:.3f}")
```

参数是局部副本。函数在实际执行的路径上必须显式 `return`；如果需要单独的带类型工件，请在调用者中放置 `seal fraction`。限制和编译模式边界参见引擎的 `docs/FUNCTIONS.md`。

来自 `input()` 或 `argv()` 的文本不会自动变为数值。有限无单位十进制值使用 `parse_number(raw)`；经过检查的十进制整数语法使用 `parse_integer(raw)`；无效文本会产生可审计的失败。文本 `+`、`len(text)` 和 `str_...` 内置函数覆盖日常字符串操作。在科学计算中使用文本换算之前，请参阅引擎的 `docs/STRINGS.md`。

## 3. 作出决策

```goblin
fraction = 9 / 12
if fraction >= 0.75 {
    verdict = "meets threshold"
} else {
    verdict = "below threshold"
}
switch verdict {
    case "meets threshold" { code = 1 }
    default { code = 0 }
}
print("verdict = {verdict}; code = {code}")
```

`if` 要求布尔条件。`switch` 执行第一个完全匹配的 case，不会贯穿执行后续 case。对于近似的浮点决策，应在 `if` 中使用容差比较，而不是 `switch` 的相等判断。阈值是科学选择：请记录其适用理由。

## 4. 读取科学文件

将随包的 `sample.fits` 放在 `.gbl` 文件旁边。其第二个 HDU 是一个小型二进制表：

```goblin
file = "sample.fits"
hdus = fits_hdu_count(file)
rows = fits_rows(file, 1)
mean_z = fits_column_mean(file, 1, "Z")

print("HDUs = {hdus}")
print("rows = {rows}")
print("mean Z = {mean_z}")
write_json("summary.json", "rows", rows, "mean_z", mean_z)
```

HDU 索引从零开始。Goblin++ 在运行中为输入计算哈希；文件只读。`Z` 是列名，不保证具有宇宙学解释。对于不熟悉的文件，请先用 `goblin++ fits-info FILE --quick` 检查其 HDU 和列；快速检查明确**不计算哈希**，不属于运行证据。

如需针对全表、边界明确且可选加权的汇总，请参阅 `examples/fits_selection.gbl` 和引擎的 `docs/FITS.md`。随包的三行测试数据用于教授语法，不是 DESI 科学结果。编辑器的 `fitsselect` 代码片段只是起点：你必须为自己的星表选择有依据的列、区间和权重。

## 5. 决定保存什么

只有希望把 `mean_z` 保存为独立的带类型科学工件时，才添加 `seal mean_z`。打印和写文件并不需要它。如果希望增加运行结束后的源文件观测和自验证关卡，请在顶部添加 `GO_PARANOID`。任何一种选择都不能让科学假设变成真，也不能让不受信任的原生代码安全执行。

对运行报告的 `RUN_DIR` 使用 **Goblin++: Verify Run Directory**（验证运行目录）。只有审查过程序和假设之后，才冻结精确的源文件字节。之后若要修改冻结源文件，应以明确理由创建子修订。

## 当前边界

FITS 和生成输出调用可用于解释器，但尚不可用于编译执行（包括在 `g_func` 中）。内联 Rust 要求单独审核的确切哈希，并拥有你的账户全部操作系统权限。一维数组和 `g_func` 函数可用；嵌套集合仍是后续工作。任何编辑器颜色或补全都不能取代检查数据和检验论断。数据优先；哥布林靠后。
