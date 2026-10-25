# 下载与安装

## 获取安装包

所有版本都在 GitHub Releases 发布：<https://github.com/Dlany-Cohhh/ProfileX/releases>

| 文件 | 适用场景 |
| --- | --- |
| `ProfileX_1.0.0_x64-setup.exe` | NSIS 安装包，双击安装，推荐个人使用 |
| `ProfileX_1.0.0_x64_en-US.msi` | MSI 安装包，适合批量部署或需要静默安装的场景 |

系统要求：Windows 10 / 11（x64）。浏览器需为 Chrome 或其它 Chromium 内核浏览器。

## 安装

1. 下载 `ProfileX_1.0.0_x64-setup.exe`。
2. 双击运行，按向导完成安装（可选择安装目录）。
3. 首次启动若出现 Windows SmartScreen 提示「Windows 已保护你的电脑」，点 **更多信息 → 仍要运行**。这是因为安装包未购买代码签名证书，属于正常现象。
4. 安装完成后从开始菜单或桌面快捷方式启动 ProfileX。

## 卸载

- 设置 → 应用 → 已安装的应用 → ProfileX → 卸载；或在安装目录运行卸载程序。
- 卸载**不会**删除你的配置数据，如需彻底清理，手动删除：
  - `%APPDATA%\com.bujic.profilex\`（数据库与默认浏览器数据）
  - `文档\ProfileX\Backups\`（默认备份目录）

## 数据存放位置

| 内容 | 默认路径 | 是否可改 |
| --- | --- | --- |
| 配置数据库 | `%APPDATA%\com.bujic.profilex\profiles.db` | 否 |
| 浏览器数据目录 | `%APPDATA%\com.bujic.profilex\profiles\` | 是（设置 → 配置数据目录） |
| 备份目录 | `文档\ProfileX\Backups\` | 是（设置 → 备份目录） |

修改数据目录或备份目录时，应用会把已有文件迁移到新位置，迁移期间请保持所有配置已关闭。

## 便携使用

ProfileX 没有便携版。若想在多台机器间同步配置，建议使用「备份 / 导入备份」功能：备份包内含配置元信息，导入后名称、图标、标签、端口与启动参数都会一并还原。
