# Community translation review / 社区翻译校对

> **UNVERIFIED AI REVIEW - NOT THE OFFICIAL TUTORIAL OR ACCEPTED ERRATA.**
>
> **未经验证的 AI 校对材料：不是官方教程，也不是已接受的勘误。请先阅读维护版中文文档。**

For learning Goblin++, use the [maintained Chinese reader guide](../docs/zh-CN/START_HERE.md), [maintained Chinese tutorial Markdown](../docs/zh-CN/docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.md), and [English tutorial PDF](../docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.pdf). The tutorial is historical alpha.12 material; the companion references cover the alpha.21 snapshot.

学习 Goblin++ 请使用[维护版中文入口](../docs/zh-CN/START_HERE.md)、[维护版中文教程 Markdown](../docs/zh-CN/docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.md)及[英文原教程](../docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.pdf)。教程明确保留 alpha.12 历史版本；配套参考文档对应 alpha.21。

## What is here / 本目录内容

- [zh_CN.md](zh_CN.md): a user-provided DeepSeek proposed tutorial rewrite, not an approved replacement.
- [ERRATA_zh-CN.md](ERRATA_zh-CN.md): DeepSeek's claimed findings, not verified or accepted project errata.
- [PROVENANCE.json](PROVENANCE.json): origin, status and SHA-256 hashes of the two supplied files.

这两份用户提供的 DeepSeek 文件按原始字节保留，便于核对。它们内部的“修订版”“全部问题”“阻塞发布”等表述属于校对文件自己的主张，不代表维护者确认。文件未被修改，也没有被应用到语言引擎、维护版文档或示例。

## Important cautions / 重要提醒

A comparison on 2026-10-03 found several claimed corruptions absent from the distributed PDF and Markdown. For example, the maintained files correctly contain `energy = mass * c^2`, `if value % 2 == 0`, `GO_PARANOID`, `write_tsv` and `$HOME`. Apparent OCR/extraction errors must be checked against actual source text and rendered pages before being treated as document defects; the extraction cause is not confirmed.

2026-10-03 的核对发现，若干声称的损坏并不存在于发布的 PDF 和 Markdown 中。上述代码符号及名称在维护版中正确。疑似 OCR 或提取错误应对照真实源码和 PDF 页面核实，不能直接当作文档错误；具体提取原因尚未确认。

The proposed rewrite also changes programming content. These changes have **not** been accepted:

| Maintained source / 维护版 | Proposed rewrite / 建议版 | Concern / 问题 |
| --- | --- | --- |
| `fraction >= 0.75` | `fraction == 0.75` | Changes a threshold into exact equality; e.g. 0.9 no longer passes that branch. / 将阈值改为精确相等，改变程序含义。 |
| `Z_ERR` | `Z_ERROR` | Requests a different FITS column; valid names depend on the actual file. / 请求了不同的 FITS 列，不能凭猜测替换。 |
| `goblin++ run file.gbl -- args` | `goblin++ run file.gbl --args` | Removes the CLI argument separator and changes command syntax. / 删除 CLI 参数分隔符，改变命令语法。 |

The rewrite changes 12 of the 34 tutorial code blocks, including formatting changes, altered example strings and an omitted output line. That count alone does not make every change wrong, but it means this is not a byte-preserving translation. The maintained tutorial's 34 code blocks match the English originals.

建议版改动了 34 个教程代码块中的 12 个，包括排版、示例字符串及删除一行输出。这个数字不代表每项改动都错误，但说明它不是保持原始代码不变的翻译。维护版的 34 个教程代码块与英文原文一致。

## How to help / 如何参与

Chinese-speaking scientists and programmers are welcome to review terminology and readability. Please report the file path, section, original text, proposed wording and English-source evidence. For a claimed code defect, also provide the Goblin++ version, exact input and reproducible result. Keep meaning-preserving translation edits separate from proposed language/code changes.

欢迎中文科研工作者和程序员改善术语及可读性。请提供文件路径、章节、原句、建议译文及英文依据。涉及代码错误时，还应提供版本、精确输入和可复现结果。翻译修订与语言或代码变更应分别审查。

Use the [contribution guide](../CONTRIBUTING.md) or [GitHub Issues](https://github.com/mrgogobot/goblinpp/issues). Do not upload secrets, confidential research data or unredacted private run evidence.

See the project's [documentation license scope](../LICENSE-DOCS.md). These review copies are identified as user-provided AI material; their inclusion is not an endorsement of their technical claims or a replacement for original notices.
