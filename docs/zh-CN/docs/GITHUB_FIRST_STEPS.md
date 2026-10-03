> 简体中文译本；对应 Goblin++ 0.1.0-alpha.21 文档快照。命令、标识符及代码示例保持原样。

# 在 GitHub 上管理 Goblin++：初次使用指南

仅有 `.git` 目录并不会上传项目。如果你已经将此仓库以私有形式发布，请跳过下面的首次上传步骤，使用[发布检查清单](../RELEASE_CHECKLIST.md)完成其余审查。可以这样理解这些术语：**仓库（repository）**是项目文件夹及其变更历史；**提交（commit）**是已保存的快照；**发布／推送（publish/push）**将这些快照复制到 GitHub；**发行版（release）**为经审查的版本加上标签，供他人下载。

## 最简单的途径：GitHub Desktop

1. 项目公开前，请审阅[发布检查清单](../RELEASE_CHECKLIST.md)，包括[软件许可证](../LICENSE)和[文档许可证](../LICENSE-DOCS.md)。确认你有权授权每个收录文件，并审查尚未完成的安全和科学验收门槛。
2. 安装 [GitHub Desktop](https://desktop.github.com/)，登录你要使用的 GitHub 账户。在浏览器登录 GitHub 不会自动登录 GitHub Desktop。
3. 在 Desktop 中选择 **File → Add Local Repository…**，选择此 `goblinpp` 文件夹。如果提示仓库不存在，请寻求帮助；不要再创建一个嵌套仓库。
4. 在 **Changes** 选项卡中查看待提交文件。不应出现构建产物、个人运行记录或大型研究数据。输入摘要，例如 `Prepare Goblin++ alpha source repository`，然后选择 **Commit to main**。此步骤仅发生在你的电脑上。
5. 准备上传时，选择 **Publish repository**。点击最终 Publish 按钮前，确认账户、仓库名和 **Keep this code private** 复选框。首次保持私有是合理做法；检查清单完成前不要公开。
6. 在 GitHub 打开 **Actions** 选项卡，检查 Rust 和编辑器测试结果。绿色结果表示软件检查通过，不代表科学有效性。发布发行版前先修复失败项。

官方指南：[在 GitHub Desktop 中添加本地仓库](https://docs.github.com/en/desktop/adding-and-cloning-repositories/adding-a-repository-from-your-local-computer-to-github-desktop)和[发布已有项目](https://docs.github.com/en/desktop/adding-and-cloning-repositories/adding-an-existing-project-to-github-using-github-desktop)。

## 后续：公开发布与 Zenodo

完成其余审查并通过检查后，再决定是否公开仓库。GitHub **发行版（release）**由版本标签创建；它不等于一次提交或上传。Zenodo 可以归档 GitHub 发行版并分配 DOI，但这需要单独连接和审查。在 Zenodo 真正分配 DOI 之前，不要猜测 DOI，也不要将其写入引用文件。
