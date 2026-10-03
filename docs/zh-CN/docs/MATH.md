> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ 0.1.0-alpha.14 中的科学数学

科学数学内置函数同时适用于 Rust 解释器和原生编译器。它们的结果仍是普通的 Goblin++ 物理量，可以打印、放入数组、由 `g_func` 返回或封存。

## 平方根、绝对值、极值与距离

```goblin
root = sqrt(81)
length = sqrt((3 m)^2)
magnitude = abs(-4 kg)
smallest = min(3 m, 1 m, 2 m)
largest = max(3 m, 1 m, 2 m)
diagonal = hypot(3 m, 4 m)
```

- `abs(value)` 保持输入量纲。
- `sqrt(value)` 要求数值非负，且每个基本量纲的指数均为偶数。结果将这些指数减半。不会暗中引入分数量纲。
- `min(first, second, ...)` 和 `max(first, second, ...)` 要求至少两个参数，且量纲完全相同。
- `hypot(x, y)` 要求量纲一致，并返回相同量纲。

## 舍入

`floor`、`ceil` 和 `round` 接受一个无量纲值。`round` 遵循 Rust `f64` 的行为：正好位于中点的情况向远离零的方向舍入。Goblin++ 不会暗中按 SI 单位对物理量舍入，因为用户想要采用的舍入单位可能有歧义。

## 指数与对数函数

`exp`、`ln` 和 `log10` 要求输入无量纲。`ln` 和 `log10` 要求输入大于零。溢出、无穷大和 NaN 都会被拒绝。

```goblin
growth = exp(1)
natural = ln(growth)
decades = log10(1000)
```

## 三角函数

推荐函数名明确包含角度约定：

- `sind`、`cosd` 和 `tand` 将无量纲输入解释为度；
- `sinr`、`cosr` 和 `tanr` 将无量纲输入解释为弧度；
- `asind`、`acosd` 和 `atand` 返回以度计的无量纲角度数值；
- `asinr`、`acosr` 和 `atanr` 返回以弧度计的无量纲角度数值；
- `atan2d(y, x)` 和 `atan2r(y, x)` 接受量纲一致的输入，拒绝未定义的数值对 `(0, 0)`，并分别返回度或弧度；
- `deg2rad` 和 `rad2deg` 对无量纲角度数值进行显式换算。

反正弦与反余弦将输入限制在闭区间 `[-1, 1]`。度和弧度目前尚不是不同的物理量类型：不要写 `30 deg` 或 `0.5 rad`。函数名后缀规定并强制执行角度约定。

```goblin
opposite = sinr(pi / 2)
half = sind(30)
angle_r = atan2r(1 m, 1 m)
angle_d = atan2d(1 m, 1 m)
converted = deg2rad(180)
```

为便于迁移，旧名称 `sin`、`cos`、`tan`、`asin`、`acos`、`atan` 和 `atan2` 保留 alpha.13 的弧度行为。每个被使用的旧名称在每次运行中发出一条会被保存的 `G302` 警告，引导作者使用明确的 `r` 或 `d` 形式。这些别名已弃用，只能在明确声明的破坏兼容性的语言版本中删除；它们绝不会悄然改为度。

## 明确拒绝的情况

以下示例会失败，而不是凭空造出结果：

```goblin
bad_root = sqrt(-1)        # real-number domain error
bad_unit = sqrt(3 m)       # fractional dimension would be required
bad_log = ln(0)            # logarithm domain error
bad_trig = sind(1 m)       # trigonometry requires a dimensionless angle
bad_pair = hypot(1 m, 1 s) # dimensions do not match
```

Goblin++ 当前实现的是实数 `f64` 数学。复数、分数量纲物理量、不确定度传播以及一等角度单位是独立的后续设计问题，不能从这些内置函数推断其已受支持。
