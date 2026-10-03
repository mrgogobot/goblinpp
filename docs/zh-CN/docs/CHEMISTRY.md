> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 化学基础

Goblin++ 0.1.0-alpha.16 增加了一个刻意保持精简的化学功能层，用于常见的实验室计算。它同时适用于解释器和原生编译器。引擎始终是权威；编辑器建议只是编写辅助。

## 注册表与证据

元素表的标识为 `IUPAC-2021-ABRIDGED-COMMON-v1`。它明确列出 43 种常见元素，采用 [IUPAC 元素周期表](https://iupac.org/what-we-do/periodic-table-of-elements/) 中的简化标准原子量。注册表的标识、内容及 SHA-256 都纳入科学注册表和冻结证据。因此，元素表发生变化后，不能悄无声息地被当作同一个已冻结的科学计算环境。

`chem_registry_version()` 返回确切的注册表标识。`chem_atomic_number("C")` 和 `chem_atomic_weight("O")` 要求使用准确且区分大小写的元素符号。原子量是无量纲的相对值，不是同位素质量，也不表示对样品同位素组成作出了判断。

摩尔气体常数 `R` 由精确的 SI 定义常数推导而来。本版本的原子质量常数 `m_u` 和道尔顿单位 `Da` 使用 CODATA-2022 数值；其测量值属性保留在注册表证据中。参见 [CODATA 推荐值](https://codata.org/initiatives/data-science-and-stewardship/fundamental-physical-constants/)。

## 化学式与计算辅助函数

```goblin
GO_PARANOID

water_molar_mass = chem_molar_mass("H2O")
water_amount = chem_moles(36.03 g, water_molar_mass)
recovered_mass = chem_mass(water_amount, water_molar_mass)

stock = chem_concentration(0.1 mol, 100 mL)
diluted = chem_dilution(stock, 10 mL, 100 mL)

print("H2O molar mass = {water_molar_mass}")
print("water amount = {water_amount}")
print("diluted concentration = {diluted}")

seal water_molar_mass
seal diluted
```

`chem_molar_mass` 支持准确的元素符号、正整数计数和圆括号分组，例如 `H2O`、`C6H12O6` 和 `Ca(OH)2`。化学式限制为 1,024 字节，分组嵌套最多八层，每个计数不超过 1,000,000。

进行量纲检查的算术辅助函数包括：

- `chem_moles(mass, molar_mass)`；
- `chem_mass(amount, molar_mass)`；
- `chem_concentration(amount, volume)`；
- `chem_dilution(concentration, initial_volume, final_volume)`，实现 `C2 = C1 V1 / V2`，并拒绝终体积小于初始体积的情况。

这一阶段新增的实用单位有 `L`、`mL`、`uL`、`µL`、`nm`、`pm`、`angstrom`、`Å`、`Pa`、`kPa`、`bar`、`atm` 和 `Da`。

## 有意设定的边界

这一阶段**不会**推断电荷、同位素记法、方括号分组、水合物点号、反应、化学计量配平、pH、活度、平衡、动力学、热力学状态、不确定度或生物序列的含义。不支持的化学式会失败，而不是被猜测解释。当同位素组成重要时，应使用样品特定的原子质量。

设计原则很简单：提供实用的实验室算术，让假设清晰可见，不在亲切的函数名背后藏一个自作聪明的“化学神谕”。
