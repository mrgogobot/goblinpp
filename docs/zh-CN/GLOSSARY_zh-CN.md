# Goblin++ 中英术语对照

这份表解释术语，不改变语言关键字、函数名或机器状态。

| English / identifier | 简体中文 | 说明 |
| --- | --- | --- |
| evidence-first | 以证据为先 | 执行与可检查的来源记录共同设计；不是“结果一定正确”。 |
| quantity | 物理量 / 量值 | 有限 SI 数值与量纲；无量纲数字也是 quantity。 |
| dimension | 量纲 | 如质量、长度、时间；不是“单位名称”或数组维度。 |
| unit | 单位 | 如 kg、m、s；决定数值缩放和量纲。 |
| dimensionless | 无量纲 | 没有物理量纲；不表示未知单位。 |
| radians / degrees | 弧度 / 度 | `r` / `d` 后缀明确输入或输出约定；“度”是角度的度数约定，不泛指所有角度。 |
| seal | 封存 | 具名值的带类型快照；不加密。关键字仍为 `seal`。 |
| receipt | 回执 | 绑定运行状态、输入、日志、产物及哈希的结构化记录。 |
| freeze | 冻结 | 固定精确源码字节和相关规范化/注册表状态。 |
| revision | 修订 | 带必需理由、连接冻结父版本的显式子版本。 |
| lineage | 修订谱系 | 版本之间明确的父子来源关系。 |
| custody ledger | 保管链账本 | 项目事件的追加式 SHA-256 哈希链；不等同于作者认证。 |
| canonical program | 规范化程序 | 用来区分语义与源码记法的解析结构。 |
| notation-only change | 仅记法变化 | 字节改变，规范化含义未改变；冻结后仍会拒绝执行。 |
| hash / digest | 哈希 / 摘要 | 本文常指 SHA-256；不是秘密，也不是签名。 |
| checksum-only | 仅校验和 | `CHECKSUM_ONLY`；验证完整性，不认证作者身份。 |
| artifact | 产物 | 运行保存的文件或带类型科学值。 |
| postflight | 运行后检查 | 执行结束时的检查；不是持续监控。 |
| interpreter | 解释器 | Rust 求值器执行 Goblin++ 程序结构。 |
| native compilation | 原生编译 | 生成 Rust 并构建原生可执行程序。 |
| independent copy | 独立副本 | 数组/切片修改不共享底层存储。 |
| half-open interval | 左闭右开区间 | 包含起点，不包含终点。 |
| short-circuit | 短路求值 | 可确定结果时，不继续求值后续布尔操作数。 |
| round-trip formatting | 往返还原格式 | 文本重新解析后还原同一个存储浮点值。 |
| signed zero | 带符号的零 | `0` 与 `-0` 的浮点位模式不同。 |
| deterministic sampling | 确定性采样 | 同一输入和参数采用固定采样规则；不是随机代表性保证。 |
| HDU | 头数据单元 | FITS 的 Header/Data Unit；Goblin++ HDU 索引从零起。 |
| null / NaN | 空值 / 非数（NaN） | 缺失值与特殊浮点状态，不能随意当作零。 |
| compensated summation | 补偿求和 | 减少求和舍入误差的算法；不消除所有误差。 |
| molar mass | 摩尔质量 | 每单位物质的量的质量。 |
| amount concentration | 物质的量浓度 | 物质的量除以体积。 |
| KCL / KVL | 基尔霍夫电流定律 / 电压定律 | 检查带符号残差，不能替代实际电路模型。 |
| angular momentum | 角动量 | 与角速度不同，必须遵守函数的向量和量纲约定。 |
| `GO_PARANOID` | 严格审计模式指令 | 可选的更严格证据策略；不是保密或沙箱承诺。 |
| `PROTOCOL_VIOLATION` | 协议违规 | 策略被违反，运行保存为可验证的拒绝证据。 |
| `MACHINERY_FAIL` | 执行机制失败 | 程序/数值/数据等明确失败状态；不能当作科学成功。 |
