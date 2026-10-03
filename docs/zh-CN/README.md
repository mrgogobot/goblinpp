# Goblin++ Rust 引擎 0.1.0-alpha.21（本地版，原文标注为尚未发布）

> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。
>
> 译者说明：本译本保留提交快照的历史措辞。alpha.21 已于 2026-10-02 发布；原文的“尚未发布”并非当前发布状态。部分段落仍描述早期版本的限制；编译模式的 FITS/文件输出支持请以 [DATA_MODULES_NATIVE.md](docs/DATA_MODULES_NATIVE.md) 为准。

Alpha.21 使默认数值文本能无损还原已存储的有限浮点值，包括带符号的零。显式格式（例如 `.3f`）仍会对显示标签进行舍入。升级已有冻结项目之前，请阅读[数值文本与迁移](docs/NUMERIC_TEXT.md)。历史回执仍可验证。Alpha.20 的常数保护、必须使用返回值的副本式 append，以及字符串即时捕获行为均予保留。

<img src="assets/goblinpp-logo.png" alt="Goblin++ 小地精吉祥物；为好奇的头脑而设计的实用语言；基于 Rust；创意在这里编译" width="300">

Goblin++ 是一种以证据为先的科学编程语言。本版本开启从 Python 0.0.7 参考实现向原生 Rust 引擎的可审计迁移。

> **隐私警告：**运行证据可能以明文保存源代码、`input()` 提示和回答、命令行参数、导入的库和 FITS/CSV/TSV 数据、生成文件，以及 stdout/stderr。未经审查证据的存储位置及访问权限，不要使用密码、令牌或保密研究数据。`GO_PARANOID` 和 SHA-256 保护的是完整性，不是机密性。参见[安全与信任边界](docs/SECURITY.md)。

## 下载与文档

- [Goblin++ Alpha.12 第一天入门教程](docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.pdf)
- [安装 VS Code 扩展](vscode/README.md)
- [构建或安装 JetBrains IDE 插件](jetbrains/README.md)
- [电气工程示例与函数指南](docs/ELECTRICAL_ENGINEERING.md)
- [基础统计：求和与均值](docs/STATISTICS.md)
- [CSV/TSV、本地模块及编译模式科学 I/O](docs/DATA_MODULES_NATIVE.md)
- [无损数值文本与显式显示精度](docs/NUMERIC_TEXT.md)

普通命令解释执行已保存的 `.gbl` 文件：

```console
goblin++ experiment.gbl
```

原生编译需明确指定：

```console
goblin++ experiment.gbl --compile
goblin++ compile experiment.gbl -o experiment
```

第一种形式会执行原生产物，并在运行目录中保留生成的 Rust、编译后二进制文件、stdout、stderr、封存的科学产物及哈希。第二种形式只生成独立二进制文件，不执行它。

Alpha.19 支持 CSV/TSV 输入、本地 `import` 函数库，以及原生 FITS/文件/绘图调用。使用数据的编译需要 Cargo 和已缓存、受锁文件约束的 crate；构建在离线模式进行。独立输出不包含启动器的保管链、冻结和运行后检查保证。选择模式之前，请阅读链接中的指南。

## 仓库状态

本源代码仓库包含 Rust 引擎、测试和示例、[`vscode/`](vscode/) 中的 VS Code 扩展，以及 [`jetbrains/`](jetbrains/) 中的 JetBrains 插件源码。它**不包含**预编译二进制文件、个人运行目录，或超出小型确定性 FITS 测试数据的科学输入。首个公开 alpha 已在 GitHub 和 Zenodo 保存；开发在此继续。后续发布门槛见 [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)。

## 构建与安装

需要 Rust 1.92 或更新版本。

```console
cargo test --locked
./install.sh --prefix "$HOME/.local"
goblin++ --version
```

本源码检出使用已提交的锁文件在本机构建命令。只有在单独发布归档中存在捆绑的 macOS arm64 二进制文件时，安装器才会使用它；在源码仓库中会从源码构建。可传入 `--build-from-source` 明确选择源码构建。

Cargo 的内部二进制目标名为 `goblinpp`，因为 Rust crate 标识符不能包含 `+`。安装器提供预期命令名 `goblin++`。

## 日常程序与严格审计程序

日常程序无需 `GO_PARANOID`，也无需 `seal`：

```goblin
samples = 12
accepted = 9
fraction = accepted / samples
print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

生成文件仍被限制在唯一的 `RUN_DIR/outputs` 目录内，并通过 SHA-256 记录和验证。完成的运行会保留源代码、日志和回执；验证无误的运行登记到校验和账本。严格审计自检失败会阻止账本登记。`seal` 是可选声明，用于将一个具名科学值快照为单独的带类型产物；`print` 或文件输出不需要它。

`GO_PARANOID` 是主动启用的更严格证据策略。它是源代码语法，也是规范化程序的一部分：

```goblin
GO_PARANOID

mass = 1 kg
energy = mass * c^2
print("Energy = {energy}")
seal energy
```

除了基本证据，严格审计运行会在执行后再次观察并保存源文件字节。结束时哈希不同会拒绝 `PASS`，并记录可验证的协议违规；源文件缺失也会拒绝 `PASS`。登记账本前，会独立检查回执。这是结束时的一次观察，不是持续监控：在检查前被恢复的编辑无法借此发现。冻结字节约束、输出范围限制、数据哈希以及任意内联 Rust 的精确哈希授权，对两种模式均适用。`GO_PARANOID` 不是操作系统沙箱；参见 [SECURITY.md](docs/SECURITY.md)。

## 电气工程

Alpha.17 将电流加入为第六个 SI 基本量纲，引入安培、伏特、欧姆、法拉、亨利等电气单位，以及 18 个 `ee_` 辅助函数：

```goblin
current = ee_current(12 V, 4.7 kohm)
milliamps = ee_in_unit(current, "mA")
print("current = {milliamps:.3f} mA")
tau = ee_rc_time_constant(10 kohm, 100 uF)
node_ok = ee_kcl_balanced([3 mA, -2 mA, -1 mA], 0.01 mA)
```

欧姆定律、带符号的功率和能量、理想无源元件、电阻网络及显式 KCL/KVL 残差均可在两种执行引擎中运行。函数指南说明符号约定、量纲要求、转换输出和模型范围。参见 [ELECTRICAL_ENGINEERING.md](docs/ELECTRICAL_ENGINEERING.md) 和 `examples/electrical.gbl`。

## 日常控制流

`for`、`while`、`break`、`continue`、`if`/`else if`/`else` 以及 `switch`/`case`/`default` 都是解释模式和编译模式中的 Goblin++ 语法，无需 Python 或内联 Rust。布尔表达式使用短路的 `and`、`or` 和 `not`：

```goblin
GO_PARANOID
total = 0
for i in range(1, 6) {
    if i % 2 == 0 {
        continue
    }
    total = total + i
}
for value in [2, 4, 6] {
    if value > 4 { break }
    total = total + value
}
remaining = 3
while remaining > 0 {
    remaining = remaining - 1
}
print("total = {total}")
seal total

if total > 10 and not total == 99 {
    verdict = "high"
} else {
    verdict = "low"
}
switch verdict {
    case "high" { code = 1 }
    default { code = 0 }
}
print("verdict = {verdict}; code = {code}")
```

`range` 的终点不包含在范围内，接受一个、两个或三个无量纲整数参数。`for value in array` 遍历独立数组值。`%` 是经过检查的整数余数，符号跟随被除数。比较与布尔运算不推断数字的真假。`switch` 使用精确、考虑量纲的相等判断，采用第一个匹配项，且不贯穿到后续分支。嵌套循环共享每次运行一百万次迭代的限制；超限会生成保存下来的可验证失败。参见 [CONTROL_FLOW.md](docs/CONTROL_FLOW.md) 和 `examples/everyday_alpha12.gbl`。

## 科学数学

核心数学函数在解释器与原生编译器中的行为一致：

```goblin
root = sqrt(81)
distance = hypot(3 m, 4 m)
smallest = min(3 kg, 1 kg, 2 kg)
signal = sinr(pi / 2)
half = sind(30)
decades = log10(1000)
```

`abs`、`min`、`max` 和 `hypot` 保持兼容量纲。`sqrt` 只接受单位指数全部为偶数的非负值，因此 `sqrt((3 m)^2)` 为 `3 m`，而 `sqrt(3 m)` 被拒绝。`floor`、`ceil`、`round`、`exp`、`ln` 和 `log10` 要求无量纲输入。三角函数名说明角度约定：`sind`/`cosd`/`tand` 使用度，`sinr`/`cosr`/`tanr` 使用弧度，反函数和 `atan2` 的同类后缀说明返回值单位。`deg2rad` 与 `rad2deg` 进行显式转换。旧的无后缀名称暂时保持弧度行为，但会发出被保存的 `G302` 迁移警告。定义域无效和非有限结果会成为明确、可保存的失败。参见 [MATH.md](docs/MATH.md) 和 `examples/scientific_math.gbl`。

## 向量、坐标与运动

科学辅助函数保持量纲，并在函数名中体现约定：

```goblin
distance = au2m(1)
direction = spherical2cartesiand(2 m, 90, 45)
speed = velocity([10 m, 4 m, 0 m], 2 s)
normal = cross([1 m, 0 m, 0 m], [0 m, 1 m, 0 m])
omega = angular_velocityd(360, 2 s)
```

距离转换支持天文单位、秒差距和光年的双向转换。`magnitude`、`dot`、`cross` 操作同质数组。极坐标和球坐标转换使用显式度／弧度后缀；球坐标倾角从 +z 轴量起，方位角从 x-y 平面中的 +x 轴量起。速度相加从不隐式选择模型：请使用 `velocity_add_galilean` 或一维的 `velocity_add_relativistic_collinear`。旋转辅助函数涵盖角速度、切向速度、向心加速度和角动量。这些是欧氏空间/初等力学辅助函数，并非弯曲时空或广义相对论。参见 [VECTORS_COORDINATES_KINEMATICS.md](docs/VECTORS_COORDINATES_KINEMATICS.md) 和 `examples/vector_motion.gbl`。

## 化学基础

常见实验室计算使用小型、版本化的化学注册表，而非不断增加新的关键字：

```goblin
water_molar_mass = chem_molar_mass("H2O")
water_amount = chem_moles(36.03 g, water_molar_mass)
stock = chem_concentration(0.1 mol, 100 mL)
diluted = chem_dilution(stock, 10 mL, 100 mL)
print("water amount = {water_amount}")
```

两种执行引擎均支持精确元素符号查找、有界化学式摩尔质量计算、质量/物质的量转换、物质的量浓度及显式稀释。实验室单位包括升和微升、纳米、埃、常见压力单位和道尔顿。运行和冻结证据保留元素注册表的标识及哈希。本基础层不推断同位素、电荷、水合物、反应、pH、平衡、动力学或生物序列语义。参见 [CHEMISTRY.md](docs/CHEMISTRY.md) 和 `examples/chemistry.gbl`。

## G funk：用户定义函数

用 `g_func` 声明函数，再按名称调用。`return` 提供返回值：

```goblin
g_func energy(mass) {
    return mass * c^2
}

result = energy(1 kg)
print("Energy = {result}")
seal result
```

函数可在解释器和原生编译器中运行，也可在调用位置之后声明。参数和局部赋值保留在函数内部；数组参数为独立副本。每条实际到达的路径都必须返回值。递归最多允许 16 个活动调用。`GO_PARANOID`、`seal` 和已授权内联 Rust 应保留在顶层；在调用者中封存返回结果。解释模式函数可以读取 FITS 和写入运行范围内的输出，但原文此处仍说原生编译拒绝这些内置函数。参见 [FUNCTIONS.md](docs/FUNCTIONS.md) 和 `examples/functions.gbl`。

> 译者说明：上一句的编译限制是旧版本描述；alpha.19 之后的支持范围请参见 [DATA_MODULES_NATIVE.md](docs/DATA_MODULES_NATIVE.md)。

## 交互与数组

`input("prompt")` 读取文本；`argc` 和 `argv(i)` 提供 `--` 之后的命令行参数。交互证据会被保存，因此绝不要输入秘密信息。数组支持 `[]`、索引、索引赋值、左闭右开切片、`len`、返回副本的 `append`，以及直接的 `for value in array` 迭代。切片和迭代值是**独立副本**，不是 Go 风格的共享视图。参见 [INTERACTION_AND_ARRAYS.md](docs/INTERACTION_AND_ARRAYS.md)、`examples/greeting.gbl`、`examples/program_args.gbl` 和 `examples/arrays.gbl`。

```goblin
greeting = input("Please enter your name:")
print("Hello, {greeting}!")
```

## 文本操作：无需新模式的 g_strings

文本值支持用 `+` 拼接，以及用 `len(text)` 计算 Unicode 标量数量。`parse_number(text)` 显式转换有限、无单位的十进制输入；`parse_integer(text)` 只接受 `f64` 精确范围内的有符号或无符号十进制整数。`to_text(value)` 将值呈现为标签。`str_trim`、`str_contains`、`str_replace`、`str_split` 和 `str_join` 在两种执行模式中支持日常文本处理：

```goblin
name = str_trim("  Ada  ")
message = "Hello, " + name
answer = parse_number("2.5") * 2
count = parse_integer("42")
print("{message}; answer = {answer}")
```

这不是新的 `g_strings` 类型，而是现有文本值上的一组小型显式操作。两个解析器都不接受单位，也不接受超出 ±(2^53−1) 的整数。完整约定见 [STRINGS.md](docs/STRINGS.md) 和 `examples/strings.gbl`。

## 原生科学数据导入

编写程序前，先检查不熟悉的 FITS 文件：

```console
goblin++ fits-info catalogue.fits --quick
goblin++ fits-info catalogue.fits --json
```

`--quick` 仅读取结构头信息，并明确报告没有计算校验和。不带该参数时，`fits-info` 以流式方式对整个文件计算 SHA-256。两者均不生成运行证据。

原生导入器无需 Python 或 CFITSIO 即可处理主图像、图像扩展和定宽二进制表。HDU 与行索引从零开始；图像轴索引保留 FITS 从一开始的约定：

```goblin
hdus = fits_hdu_count("catalogue.fits")
name = fits_header("catalogue.fits", 1, "EXTNAME")
rows = fits_rows("catalogue.fits", 1)
columns = fits_columns("catalogue.fits", 1)
first_class = fits_column("catalogue.fits", 1, "CLASS", 0)
mean_z = fits_column_mean("catalogue.fits", 1, "Z")
minimum_z = fits_column_min("catalogue.fits", 1, "Z")
maximum_z = fits_column_max("catalogue.fits", 1, "Z")
valid_z = fits_column_valid_count("catalogue.fits", 1, "Z")
stats = fits_select_stats("catalogue.fits", 1, "Z", 0.4, 0.6, "Z", "WEIGHT")
```

现有主图像调用仍有效，也都接受显式 HDU：

```goblin
width = fits_axis("observation.fits", 0, 1)
pixels = fits_count("observation.fits", 0)
first = fits_pixel("observation.fits", 0, 0)
mean = fits_mean("observation.fits", 0)
```

图像值支持 `BITPIX` 8、16、32、64、-32、-64，并处理 `BSCALE`、`BZERO` 和整数 `BLANK`。二进制表访问支持 `L`、`B`、`I`、`J`、`K`、`A`、`E`、`D` 定宽列，包括重复元素、`TSCAL`、`TZERO` 和整数 `TNULL`。数字统计跳过空整数和浮点 NaN，并使用补偿求和。

`fits_select_stats` 提供有界的全表汇总，具有明确的左闭右开数字筛选及可选正权重。四元素结果依次报告选中行数、可用行数、权重和及加权均值；它不会替你选择巡天筛选条件或权重。用于科学工作前，请阅读 [FITS.md](docs/FITS.md) 和 `examples/fits_selection.gbl`。

输入以只读方式打开，处理时使用有界内存。运行对完整输入流式计算 SHA-256，并在 `.goblin/imports/SHA256.fits` 中保存经验证的第二份副本；文件系统允许时，运行目录硬链接到这个按校验和寻址的对象，避免每次运行都复制一份完整数据。捆绑的 `examples/sample.fits` 是确定性的，可用 `tools/generate_sample_fits.rs` 重现。

ASCII 表可被发现但不能读取。随机组、压缩图像、位列、复数列、变长数组、WCS 解释和自动单位转换仍是明确限制。Alpha.19 在编译程序中支持已实现的 FITS 调用；参见[原生 I/O](docs/DATA_MODULES_NATIVE.md)。

完整函数与证据约定见 [FITS.md](docs/FITS.md)。

## 可审计文件与绘图

生成文件属于运行产物，不是不受跟踪的副作用。它们写入 `RUN_DIR/outputs` 下，记录媒体类型、大小、生成者、元数据和 SHA-256，并由 `verify` 独立检查。

```goblin
write_text("summary.txt", "rows = {rows}", "mean Z = {mean_z}")
write_csv("summary.csv", 2, "metric", "value", "rows", rows, "mean_z", mean_z)
write_tsv("summary.tsv", 2, "metric", "value", "rows", rows)
write_json("summary.json", "rows", rows, "mean_z", mean_z)

plot_fits_histogram("redshift.png", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
plot_fits_scatter("z_error.svg", file, 1, "Z", "Z_ERR", 50000, "Redshift and reported error")
```

FITS 绘图采用确定性的均匀行采样，在回执中保留采样方法和数量。支持 SVG 和 PNG。由于 JPEG 有损，权威输出路径有意不使用它。文件名不能越出运行目录，重复声明被拒绝，本阶段每份输出限制为 64 MiB。

输出调用不论是否启用 `GO_PARANOID` 都可运行；两种模式均施加同样的范围限制、重名拒绝、大小上限与验证。

参见 [OUTPUTS.md](docs/OUTPUTS.md)、`examples/everyday.gbl` 和可执行教程 `examples/output_demo.gbl`。

## 内联 Rust

内联 Rust 是任意原生代码，因此使用明确边界和精确字节授权：

```goblin
GO_PARANOID
x = 1

RUST_INLINE_BEGIN
println!("reviewed native operation");
RUST_INLINE_END

seal x
```

先在不执行的情况下获取代码块哈希：

```console
goblin++ check program.gbl
```

审查代码块后，授权这一精确摘要：

```console
goblin++ program.gbl --compile \
  --allow-inline-rust FULL_64_CHARACTER_SHA256
```

修改一个字节就会改变摘要，策略随即恢复拒绝。内联块按源码顺序执行，可读取生成的 `goblin_env`；alpha 约定不允许它们修改已封存的 Goblin++ 变量。解释器模式始终拒绝内联 Rust。

## 保管链工作流程

```console
goblin++ freeze experiment.gbl
goblin++ experiment.gbl
goblin++ verify experiment-runs/RUN_DIRECTORY

goblin++ revise experiment.gbl experiment_R1.gbl \
  --reason "document the scientific reason"
```

冻结源码的修改会在求值前失败。仅改变记法仍是协议违规，因为冻结承诺的是精确字节；但未变的规范化含义会单独报告。拒绝会保存为可验证运行。

实用检查命令包括 `check`、`fits-info`、`status`、`lineage`、`audit-ledger`、`doctor`、`diff` 和 `capabilities`。

## 发布状态

这是 alpha 基础版本，不代表迁移已完成。用于科学生产环境前，请阅读 [PORTING_MATRIX.md](docs/PORTING_MATRIX.md) 和 [SECURITY.md](docs/SECURITY.md)。

## 许可

Goblin++ 软件、示例、测试数据与扩展资源采用 [MIT](LICENSE) 许可。原创教程/文档的文字与图解采用 [CC BY 4.0](LICENSE-DOCS.md) 许可。文档中的代码示例仍采用 MIT 许可。文档许可文件给出精确范围；第三方依赖保留各自许可。受锁文件约束的 macOS arm64 二进制依赖清单、上游许可文件及 Rust 标准库声明保留在 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。[项目标志](assets/goblinpp-logo.png) 是独立品牌素材，说明见 [BRANDING.md](BRANDING.md)，不受上述两项项目许可覆盖。
