> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ 中的电气工程

Alpha.17 增加了严格的 SI 电学物理量和 18 个函数，涵盖标量直流计算、理想无源元件以及显式的基尔霍夫平衡检查。所有函数都可在解释器和原生编译器中运行。其定义遵循 [BIPM《国际单位制手册》第九版，版本 3.02](https://www.bipm.org/documents/20126/41483022/SI-Brochure-9.pdf)。

## 第一个计算

将以下内容保存为 `load.gbl`：

```goblin
GO_PARANOID
voltage = 12 V
resistance = 4.7 kohm
current = ee_current(voltage, resistance)
power = ee_power(voltage, current)
milliamps = ee_in_unit(current, "mA")
milliwatts = ee_in_unit(power, "mW")
print("current = {milliamps:.3f} mA")
print("power = {milliwatts:.3f} mW")
seal current
seal power
```

使用 `goblin++ load.gbl` 运行。预期计算输出为：

```text
current = 2.553 mA
power = 30.638 mW
```

`goblin++ load.gbl --compile` 运行等效的编译版本，并将其封存值与解释器结果进行核对。`goblin++ compile load.gbl -o load` 创建独立可执行文件。`GO_PARANOID` 和 `seal` 仍保持原有的可选含义；证据存储与信任边界参见 [SECURITY.md](SECURITY.md)。

## 单位与换算

每个物理量存储一个 SI 数值和六个指数，顺序为 `[mass, length, time, temperature, amount, electric current]`（质量、长度、时间、温度、物质的量、电流）。

| 物理量 | 可识别的单位拼写 |
| --- | --- |
| 电流 | `A`, `mA`, `uA`, `µA` |
| 电荷 | `C` |
| 电压 | `V`, `mV`, `kV` |
| 电阻 | `ohm`, `Ω`, `kohm`, `kΩ`, `Mohm`, `MΩ` |
| 电容 | `F`, `mF`, `uF`, `µF`, `nF`, `pF` |
| 电感 | `H`, `mH`, `uH`, `µH` |
| 功率 | `W`, `mW`, `kW` |
| 电导 | `S`, `mS`, `uS`, `µS` |
| 频率 | `Hz`, `kHz`, `MHz` |

拼写区分大小写。ASCII `u` 形式和 `ohm` 形式是便于输入的别名；例如，`100 uF` 与 `100 µF` 具有相同的 SI 数值和量纲。带引号的化学式字符串中的 `C`、`F` 和 `H` 仍表示元素符号；单位后缀则出现在数值表达式中。

单位后缀在输入时执行换算。因此 `1 kohm + 500 ohm` 得到 `1500 ohm`。普通算术也会检查量纲：`(12 V) / (4.7 kohm)` 得到电流，而将伏特与安培相加会失败。

`ee_in_unit(quantity, "unit")` 返回以兼容电学单位表示的**无量纲报告数值**。在方程和 `seal` 语句中，应保留原始物理量：

```goblin
r = 4700 ohm
r_kohm = ee_in_unit(r, "kohm")
print("R = {r_kohm:.1f} kohm")
seal r
```

它会拒绝未知拼写、不兼容的量纲以及非电学单位。由于弧度无量纲，SI 量纲不能区分周期频率和角频率：`Hz` 与 `1/s` 共用同一指数向量。因此通用的时间倒数输出仍采用 `1/s`；请显式选择报告标签，并明确进行所需的 `2*pi` 换算。

## 函数

| 函数 | 计算与要求 |
| --- | --- |
| `ee_voltage(I, R)` | `V = I R`；电流与非负电阻 |
| `ee_current(V, R)` | `I = V / R`；电压与严格为正的电阻 |
| `ee_resistance(V, I)` | `R = V / I`；电流非零，结果必须非负 |
| `ee_power(V, I)` | `P = V I`；按被动符号约定给出的带符号功率 |
| `ee_power_i2r(I, R)` | `P = I² R`；电阻非负 |
| `ee_power_v2r(V, R)` | `P = V² / R`；电阻严格为正 |
| `ee_energy(P, t)` | 恒定带符号功率乘以非负时长；结果为焦耳 |
| `ee_charge(I, t)` | 恒定带符号电流乘以非负时长；结果为库仑 |
| `ee_series_resistance(values)` | 求非空非负电阻数组的和 |
| `ee_parallel_resistance(values)` | 按倒数之和计算等效电阻；数组非空且各电阻严格为正 |
| `ee_rc_time_constant(R, C)` | `tau = R C`；电阻、电容非负；结果为秒 |
| `ee_capacitor_energy(C, V)` | `E = C V² / 2`；电容非负；结果为焦耳 |
| `ee_inductor_energy(L, I)` | `E = L I² / 2`；电感非负；结果为焦耳 |
| `ee_kcl_residual(currents)` | 求非空电流数组的带符号和 |
| `ee_kvl_residual(voltages)` | 求非空电压数组的带符号和 |
| `ee_kcl_balanced(currents, tolerance)` | 布尔值：残差绝对值 ≤ 非负电流容差 |
| `ee_kvl_balanced(voltages, tolerance)` | 布尔值：残差绝对值 ≤ 非负电压容差 |
| `ee_in_unit(value, "unit")` | 以兼容电学单位表示的无量纲报告数值 |

`ee_power` 假设正电流流入正电压端。正功率表示吸收，负功率表示输出。端口方向由程序员选择。电阻辅助函数描述理想无源电阻；负电阻和除以零都会明确失败。并联辅助函数要求电阻严格为正；对于已知的理想短路，应显式表示，而不是向该函数传入零电阻支路。

## 基尔霍夫定律

对于一个节点，请选择一致的符号约定，例如流入为正、流出为负。对于一个闭合回路，请选择遍历方向，并用正负号表示电压升高和降低：

```goblin
currents = [3 mA, -2.001 mA, -1 mA]
residual = ee_kcl_residual(currents)
balanced = ee_kcl_balanced(currents, 0.01 mA)
print("node residual = {residual}; balanced = {balanced}")

voltages = [12 V, -7 V, -5 V]
loop_ok = ee_kvl_balanced(voltages, 0.001 V)
print("loop balanced = {loop_ok}")
```

第一个残差约为 `-0.001 mA`，在给定的 `0.01 mA` 容差内，其平衡检查为真。容差是调用者提供的决策阈值，不是测量不确定度。这些辅助函数只对用户给出的数值求和；它们不会发现电路连接关系，也不会求解联立电路。

## 范围与证据

这一阶段覆盖标量直流值，以及理想储能和时间常数关系。交流相量、复阻抗、电路拓扑、半导体模型、元件容差、不确定度传播和时域电路仿真仍属于后续工作。电学量纲能发现单位错误，但不能验证所假定的模型或电路拓扑。

数值仍采用有限 `f64` 的行为。溢出会被拒绝；舍入和下溢遵循浮点算术。并联电阻使用缩放，避免很小的正输入造成不必要的倒数溢出。

电学注册表标识及 SHA-256 会出现在运行证据中。新的冻结回执绑定六轴常数、单位定义、化学注册表和电学辅助函数注册表。历史回执保持原始哈希：alpha.16 之前的常数使用五轴，alpha.16 的五轴科学注册表仍可被识别。这并不意味着旧冻结已覆盖新的电学词汇。读取旧的五轴原生结果清单时，将电流指数设为零。

完整示例可尝试 [`examples/electrical.gbl`](../examples/electrical.gbl)。两个编辑器插件均提供 `ee_` 补全、单位和帮助。
