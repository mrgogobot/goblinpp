> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 文本与 `g_strings`

文本本来就是 Goblin++ 的值类型。此阶段新增普通操作，而不是引入独立的 `g_strings` 语法或类型：

```goblin
first = "Ada"
last = "Lovelace"
name = first + " " + last
count = len(name)
print("{name} has {count} characters")

raw = " 2.5 "
number = parse_number(raw)
count = parse_integer("42")
answer = number * 2
print("answer = {answer}")
```

`+` 拼接两个文本值；它仍可将两个物理量相加。混合文本和物理量是错误：请显式使用 `to_text(value)`。从 alpha.21 起，默认有限数值文本能够往返还原已存储的 f64，包括有符号零。无单位文本可用 `parse_number` 读回；带单位文本包含 SI 单位，不被该函数接受。参阅 [数值文本](NUMERIC_TEXT.md)。`to_text` 也可渲染布尔值或文本，但拒绝数组；对于文本数组，请使用 `str_join`。`len(text)` 计数的是 Unicode 标量值，而不是 UTF-8 字节或用户感知的字素簇。因此 `len("π")` 为 1，而 `len("é")`（字母加组合附加符号）为 2。`len(array)` 仍计数元素。文本操作区分大小写，不对 Unicode 进行规范化，也不改变现有源码哈希规则。

从 alpha.20 起，源码字面值中的占位符立即捕获值，且只捕获一次。已存储／用户／数据文本在输出时绝不会再次展开。要获得字面文本 `{name}`，请使用 `"{{name}}"`。参阅 [受保护的值与迁移](PROTECTED_VALUES.md)。

`g_strings` 内置函数包括：

| 调用 | 结果 |
|---|---|
| `str_trim(text)` | 移除首尾 Unicode 空白后的副本 |
| `str_contains(text, needle)` | 布尔子串检查 |
| `str_replace(text, old, new)` | 替换所有不重叠匹配后的副本；`old` 不能为空 |
| `str_split(text, separator)` | 文本数组；保留空字段；分隔符不能为空 |
| `str_join(separator, text_array)` | 从一维文本数组拼接得到的文本 |
| `to_text(value)` | 可读的文本渲染 |
| `parse_number(text)` | 无量纲有限浮点物理量 |
| `parse_integer(text)` | 无量纲、可精确表示的整数物理量 |

`parse_number` 去除首尾空白，接受 ASCII 十进制语法，可带符号、小数点和指数，例如 `-3`、`.25` 或 `+1.2e3`。它拒绝单位、内嵌空白、`NaN`、无穷大、溢出及非零值下溢为零。超出 ±(2^53−1) 的纯整数写法会被拒绝，而不是悄悄舍入。十进制／科学记数法数值仍受普通 `f64` 精度限制。目前**没有独立的精确整数类型**；不要将 `parse_number` 用于大型星表标识符。无效转换产生已保存的运行失败，绝不会猜测一个值。科学单位必须显式写在 Goblin++ 表达式中，不能通过文本转换偷偷带入。

`parse_integer` 只接受可选的 `+` 或 `-`，后接 ASCII 十进制数字。它拒绝小数点、指数、单位、分隔符和超出 ±(2^53−1) 的值。结果仍存为无量纲 `f64` 物理量；此函数提供受检语法和范围，而不是任意精度整数类型。

新的文本生成操作每个结果最多为 1,048,576 个 UTF-8 字节；分割／拼接最多支持 100,000 个部分。解释执行和原生编译执行使用相同的核心文本实现，并由一致性及回执验证测试覆盖。源码字面值、输入、参数和输出证据保留现有的限制及隐私规则。`input()` 和 `argv()` 仍返回文本；请显式选择 `parse_number(...)` 或更严格的 `parse_integer(...)`。
