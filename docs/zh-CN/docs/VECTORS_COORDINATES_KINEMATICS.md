> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 向量、坐标与运动学

Goblin++ 0.1.0-alpha.15 为普通欧氏向量计算和入门运动学提供了一组精简、具有量纲检查的基础功能。解释器和原生编译器实现相同的函数。

## 距离换算

换算函数名同时说明来源和目标：

```goblin
earth_sun = au2m(1)
same_distance = m2au(earth_sun)
nearest_parsec = pc2m(1)
one_light_year = ly2m(1)
```

`au2m`、`pc2m` 和 `ly2m` 要求无量纲数值，返回以 SI 米表示的长度。`m2au`、`m2pc` 和 `m2ly` 要求长度，返回无量纲数值。Goblin++ 使用 IAU 对天文单位的精确定义：149,597,870,700 米；使用由光速和儒略年确定的精确光年；并采用每秒差距 `648000 / pi` 个天文单位。

这些显式换算避免在任意单位表达式中把 `AU`、`pc` 或 `ly` 当作未明示的别名。

## 向量

向量是同质数组：所有分量的类型和量纲都必须兼容。

```goblin
size = magnitude([3 m, 4 m])
projection = dot([1 m, 2 m, 3 m], [4 kg, 5 kg, 6 kg])
normal = cross([1 m, 0 m, 0 m], [0 kg, 1 kg, 0 kg])
```

- `magnitude(vector)` 接受任意非空数值向量，其分量必须具有相同量纲。
- `dot(left, right)` 接受长度相等的非空向量。
- `cross(left, right)` 接受两个三分量向量。

结果量纲是推导得到的，不是猜测的。例如，长度向量与质量向量的点积具有 `kg*m` 量纲。

## 坐标约定

后缀说明角度约定：`d` 表示度，`r` 表示弧度。

```goblin
xy = polar2cartesiand(2 m, 90)
xyz = spherical2cartesiand(2 m, 90, 0)

r = cartesian_radius(xyz)
azimuth = cartesian_azimuthd(xyz)
inclination = cartesian_inclinationd(xyz)
```

Goblin++ 采用以下约定：

- 笛卡尔坐标轴为 `[x, y]` 或 `[x, y, z]`。
- 极坐标输入为 `(radius, azimuth)`，方位角从正 x 轴朝正 y 轴测量。
- 球坐标输入为 `(radius, inclination, azimuth)`。
- 倾角从正 z 轴向下测量：0 度为 +z，90 度位于 x-y 平面。
- 方位角在 x-y 平面内从正 x 轴朝正 y 轴测量。

`polar2cartesiand`/`polar2cartesianr` 返回 `[x, y]`。`spherical2cartesiand`/`spherical2cartesianr` 返回 `[x, y, z]`。逆变换辅助函数每次返回一个同质值，因为 Goblin++ 不会把长度和角度放入同一个混合量纲数组。`x = y = 0` 时的方位角、原点处的倾角、负半径以及超出范围的球坐标倾角都会被明确拒绝。

## 直线与旋转运动

```goblin
speed = velocity(10 m, 2 s)
vector_speed = velocity([10 m, 4 m, 0 m], 2 s)

ordinary_sum = velocity_add_galilean(speed, velocity(6 m, 2 s))
relativistic_sum = velocity_add_relativistic_collinear(c / 2, c / 2)

omega = angular_velocityd(360, 2 s)
tangent_speed = tangential_velocity(2 m, omega)
inward_acceleration = centripetal_acceleration(2 m, omega)
```

`velocity` 计算位移除以严格为正的经过时间。`velocity_add_galilean` 接受两个标量速度或两个等长速度向量。`velocity_add_relativistic_collinear` 实现一维狭义相对论公式 `(u + v) / (1 + uv/c^2)`；它的名称刻意不假装支持任意三维参考系变换。

`angular_velocityd` 将每单位经过时间的度数换算为弧度每秒；`angular_velocityr` 接受弧度。在当前的 SI 量纲模型中，弧度无量纲，所以角速度显示为 `s^-1`。用 `tangential_velocity(radius, omega)` 计算 `r * omega`，用 `centripetal_acceleration(radius, omega)` 计算 `r * omega^2`。

角动量使用符合右手定则的叉积：

```goblin
L1 = angular_momentum(position, momentum)
L2 = angular_momentum_from_velocity(position, mass, velocity_vector)
```

两个函数都要求三分量向量，返回 `r cross p`，量纲为 `kg*m^2/s`。

## 范围与安全边界

现有 Goblin++ 量纲引擎会检查传入这些函数的每个物理量。本模块不会重复实现或绕过它。

这些辅助函数建模的是欧氏笛卡尔几何、初等运动学、刚体圆周运动以及一维狭义相对论速度叠加。它们**不**实现参考系、张量、轨道传播、广义相对论、弯曲时空或不确定度传播。这些都需要独立明确的模型，以及经过独立验证的验收数据；一个含糊的 `curved_motion()` 函数，不过是穿戴得体的小哥布林罢了。
