> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ 0.1.0-alpha.14 中的控制流

> **译者说明：** 本文保留 alpha.14 时期关于原生编译拒绝 FITS／输出调用的历史限制。alpha.19 及以后版本已增加原生支持，详见 [DATA_MODULES_NATIVE.md](DATA_MODULES_NATIVE.md)；实际能力请结合当前 CLI 检查。

Goblin++ 在解释执行和编译执行模式中支持区间及直接数组 `for`、`while`、`break`、`continue`、`if`/`else if`/`else`、`switch`/`case`/`default`，以及短路布尔逻辑。这里的示例是可执行的 `.gbl`，不是 Python，也不是内联 Rust。

```goblin
GO_PARANOID

sum = 0
for i in range(1, 6) {
    sum = sum + i
}
print("sum = {sum}")
seal sum
```

花括号界定语句体；每条语句仍以换行结束。允许嵌套循环。循环变量在每次迭代开始时赋值；只要循环执行过，循环结束后它仍可见。零次迭代的循环不会创建该变量。在循环体内修改循环变量不会改变下一个 `range` 值。

`range(stop)` 从零开始。`range(start, stop)` 使用步长一。`range(start, stop, step)` 允许非零正或负步长。终止值不包含在范围内。参数在循环开始时仅求值一次，必须是无量纲、可精确表示的整数，范围为 ±(2^53−1)。零步长、带单位的边界或小数边界会明确失败。

可以直接遍历数组。可迭代对象仅求值一次，成为一个独立值，因此循环期间对原数组的编辑不会改变所访问的元素：

```goblin
sum = 0
for value in [1, 2, 3, 4, 5] {
    if value % 2 == 0 { continue }
    if value > 3 { break }
    sum = sum + value
}
```

`continue` 开始最近一层循环的下一次迭代；`break` 退出最近一层循环。两者在循环外都会被拒绝。`%` 要求操作数为可精确表示的无量纲整数，拒绝右侧为零，并沿用被除数的符号（`-7 % 3` 为 `-1`）。

```goblin
remaining = 3
while remaining > 0 {
    print("remaining = {remaining}")
    remaining = remaining - 1
}
finished = remaining == 0
seal finished
```

比较运算 `==`、`!=`、`<`、`<=`、`>` 和 `>=` 产生 `true` 或 `false`。数值比较要求量纲一致。文本与布尔值支持相等及不相等比较，不支持排序比较。`while` 条件必须是布尔值；系统有意不根据数值推断真假。

`and`、`or` 和 `not` 同样要求布尔操作数。`and` 和 `or` 采用短路求值：仅在需要时求值右侧。优先级由低到高依次为 `or`、`and`、`not`、比较、算术。在复杂的科学条件中，使用括号仍是最清晰的选择。

```goblin
score = 9
if score < 5 {
    label = "low"
} else if score >= 9 {
    label = "high"
} else {
    label = "middle"
}

switch label {
    case "low" { code = 1 }
    case "high" { code = 2 }
    default { code = 3 }
}
print("label = {label}; code = {code}")
```

`if` 以及每个实际到达的 `else if` 都要求布尔条件；仅执行所选语句体。`switch` 对选择表达式仅求值一次，然后按源码顺序对各 case 标签求值，直至出现第一个精确的 `==` 匹配。不存在贯穿执行。`default` 可省略，但必须唯一且位于最后；至少需要一个 `case`。case 标签可以是表达式。用于匹配的物理量必须具有相同量纲；无法比较的值类别或量纲会明确失败，而不是悄悄跳过某个 case。浮点相等是精确比较，因此对近似科学结果应在 `if` 中使用容差比较。分支中的赋值与循环赋值一样，在分支之后仍可见；未执行的分支不会创建变量。

所有循环体执行次数（包括嵌套循环）共享每次运行 1,000,000 次的上限。下一次迭代将以 `G203 LOOP LIMIT EXCEEDED` 被拒绝；失败运行仍可审计。该上限是防止意外无限循环的防护措施，不是完整的资源沙箱。单次迭代中的昂贵工作，以及循环之外已获授权的内联 Rust，仍需要使用者判断。

`GO_PARANOID` 和内联 Rust 块仍是顶层结构。源码冻结对精确源码字节和规范化 AST 计算哈希。编译执行中会检查封存值（包括布尔值）的解释器／原生结果一致性。现有的、引入分支之前的源码哈希保持不变。解释执行的分支可调用 FITS 和输出函数；原生编译仍会拒绝程序中任何位置的此类调用，包括不可达分支。

当前边界：直接 `for` 遍历内存中的一维数组，而不是 FITS 行或嵌套集合。用户自定义 `g_func` 函数见 [FUNCTIONS.md](FUNCTIONS.md)。大型科学星表分析应继续使用内置的流式 FITS 聚合；单有循环并不能形成有依据的巡天选择校正。
