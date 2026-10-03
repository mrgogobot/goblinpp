> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# Goblin++ 只读检查

扩展针对已保存的源文件调用 `goblin++ check FILE --json`。这是运行机制预览，不是证据：

```text
AUTHORITY=PREVIEW_ONLY_NOT_EVIDENCE
EVIDENCE_CREATED=NO
CUSTODY_CHECKED=NO
```

对于已解析的程序，Rust CLI 返回 `goblin.check.v1`。其中报告 `status`、一个可为空的 `diagnostic`、非致命迁移 `warnings`、起始源文件和规范化哈希，以及 stdout、明确封存的值和生成输出的预览。扩展检查报告没有声称具有证据或保管权限。由于 v1 不提供精确源码范围，求值失败显示为文档级诊断。

词法和解析失败目前会在 CLI 输出 JSON 之前退出。扩展识别其 stderr 中的 `G001`/`G002` 消息，并显示文档级预览诊断。其他命令失败仍是命令失败，而不是源码诊断。扩展绝不会编造行号。

Check（检查）可能读取引用的 FITS 输入，或在内存中渲染输出预览。它不会写入运行目录、回执、输出文件或账本事件，也不会检查冻结源文件的授权。**Check on Save**（保存时检查）需要主动启用，较早检查产生的过期结果会被忽略。

在把结果视为已保存的证据之前，请先使用 **Run Current File**（运行当前文件），再使用 **Verify Run Directory**（验证运行目录）。通过检查并不意味着模型、数据选择、单位解释或结论在科学上可靠。
