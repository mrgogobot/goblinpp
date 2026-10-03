> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 受保护的值与即时文本捕获（alpha.20）

本正确性更新在两个执行引擎中处理了 WB-1 待办事项的前三项。它不宣称其他科学功能新增项已经完成。

## 常数名称是只读的

`h` 始终表示普朗克常数；`c` 始终表示光速。所有已注册别名均受保护。`h = 2` 会在对右侧求值之前失败，明确指出 **h 是已注册常数**。请选择 `height` 或 `concentration` 等描述性变量名。保护范围包括嵌套赋值、函数名、参数和循环变量。`print(h)`、`print("{h}")` 和 `seal h` 使用相同的注册表值，并在运行证据中记录其使用。

## 使用仅返回值的内置函数结果

数组和切片仍采用独立副本语义：

```goblin
samples = []
samples = append(samples, 1)
print(samples)
```

单独的 `append(samples, 1)` 会被拒绝，而不是悄悄忽略。其他已注册、仅返回值的内置函数（如 `sqrt(9)` 或 `str_trim(" text ")`）的直接语句调用同样会被拒绝。请赋值、返回，或将其结果传给使用它的操作。`print`、`printf`、写入器、绘图、`input` 和用户自定义函数仍可作为语句，因为它们可能具有预期的副作用。这不是针对任意表达式或用户函数的通用未使用值检查器。执行验证会在产生副作用之前检查嵌套语句体。

## 源码模板立即捕获，且只捕获一次

```goblin
x = 1
saved = "{x}"
x = 2
print(saved)                    # 1
write_text("saved.txt", saved) # 1, not 2
```

该规则适用于对*任意源码字符串表达式*求值时，而不只是赋值：数组元素、函数实参和返回字符串都遵循相同规则。在循环中，新构造的模板捕获该次迭代的值。函数模板读取局部参数／变量。`{argc}` 和常数别名（包括 `π` 和 `ħ`）也可使用。数值格式仍为 `.Nf` 和 `.Ne`；量纲会保留。

未知的 `{missing}` 会立即失败。格式错误的标识符占位符及未闭合的占位符会失败，不会成为延迟求值模板。使用 `"{{x}}"` 得到字面文本 `{x}`；以这种方式配对的双写左右花括号会转义占位符。非占位符的 JSON／代码花括号保持字面形式，包括嵌套 JSON 的右花括号。单独的左花括号或右花括号视为文本。文本展开上限为 1,048,576 个 UTF-8 字节。

`print`、提示或文件导出绝不会再次扫描已保存文本。由 `input`、`argv`、CSV/TSV/FITS 读取器或字符串操作返回的文本始终是普通文本，即使包含 `{x}` 也一样。插入的占位符文本不会递归展开。如果你希望匹配字面花括号，请转义源码字面搜索模式，例如 `str_replace(text, "{{x}}", "replacement")`。

## 现有冻结项目

虽然源码语法／规范化哈希没有改变，执行规则已发生变化。对于 alpha.20，新的运行和冻结回执会在 `language_semantics` 中对策略 `goblin.eager-text-and-protected-values.v1` 计算哈希。Alpha.21 用 `goblin.eager-text-and-roundtrip-numbers.v2` 取代该策略，以记录无损的默认数值格式；参阅 [数值文本](NUMERIC_TEXT.md)。Alpha.19 及更早版本的运行证据仍可独立验证；验证检查的是历史完整性，而不是现在执行是否会产生相同结果。

不含此策略（或采用其他策略）的现有冻结会以 `LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE` 被拒绝。其完整性仍可验证，并可作为显式修订的父版本。不要删除或重写该冻结：

```sh
goblin++ revise experiment.gbl experiment_R1.gbl --reason "adopt alpha.20 immediate text capture"
# Review/edit the child; rename variables that collide with constants.
goblin++ freeze experiment_R1.gbl
goblin++ experiment_R1.gbl
```

如果需要旧执行语义，请使用旧引擎。跨策略时期的比较被分类为 `LANGUAGE_SEMANTICS_CHANGE`，而不是相同结果或仅记法变化的结果。这里的功能均不认证作者身份，也不提供沙箱；保管链及输入隐私限制保持不变。
