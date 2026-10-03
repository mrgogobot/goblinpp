> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 输入、参数、数组与切片

`input("prompt")` 读取一行 UTF-8 文本并返回文本。`argc` 是程序参数数量的只读值，包括 `argv(0)`；`argv(i)` 返回从零开始编号的文本参数。对于 Goblin++ 运行，`argv(0)` 是 `.gbl` 路径。将程序参数放在 `--` 之后：

```console
goblin++ examples/greeting.gbl
goblin++ examples/program_args.gbl -- Ada
goblin++ run examples/program_args.gbl -- Ada
```

`input` 和 `argv` 返回文本，而不是数字。使用 `parse_number(text)` 得到经过检查的有限十进制数；如果要求整数语法和精确范围，则使用 `parse_integer(text)`；参阅 [STRINGS.md](STRINGS.md)。每个提示及响应上限为 65,536 字节；每次运行最多允许 1,024 个提示和 256 个参数（包括 `argv(0)`）。`check` 从不读取标准输入；如果它执行到 `input`，会报告需要输入。VS Code 的 Run 命令显示输入框。Run with Arguments 要求提供 JSON 数组，例如 `["--name", "Ada"]`。

**隐私：** 提示文本、响应和参数会未经脱敏地保存在运行目录中的 `interaction.json` 内，并通过哈希与回执关联。不要通过 `input` 或命令行参数输入密码、令牌或其他秘密。`verify` 检查已保存的证据；`diff` 报告交互证据是否匹配。这提供的是可审计性，而不是保密性。

数组是一维且同类型的：元素必须全部是文本、全部是布尔值，或全部是具有相同量纲的物理量。空数组 `[]` 在首次 `append` 时确定类型。不支持嵌套数组。最多允许 100,000 个元素。

```goblin
values = [1 kg, 2 kg, 3 kg, 4 kg]
print(values[0])       # 1 kg
part = values[1:3]     # [2 kg, 3 kg]; stop is exclusive
part[0] = 20 kg        # values[1] stays 2 kg
extended = append(values, 5 kg)  # returns a new array
count = len(values)    # 4
for value in values {
    print(value)
}
seal values
```

索引从零开始，必须是精确的、非负的无量纲整数。切片允许省略边界（`values[:]`、`values[:2]`、`values[2:]`），`start:stop` 是左闭右开的区间。越界和反向边界会明确失败。赋值、切片、`append` 及直接 `for` 捕获的可迭代对象都使用**独立副本**：没有 Go 风格的共享底层数组。这一选择防止派生的科学数据子集悄悄改变其来源。`seal` 将数组快照为带类型、已计算哈希的产物。测试确认解释执行和编译执行产生相同数组结果。
