# InkReader

InkReader 是一款面向本地离线漫画收藏的桌面阅读器。项目基于 Tauri 2、Vue 3、TypeScript、Rust 和 SQLite 构建，重点解决本地漫画仓库扫描、书架管理、压缩包阅读、阅读进度和收藏整理等日常需求。

## 下载

Windows 用户可以从 GitHub Releases 下载已构建版本：

[InkReader v1.0 Release](https://github.com/wish-init/InkReader/releases/tag/v1.0)

## 主要功能

- 本地仓库管理：添加本地目录，扫描目录下的漫画文件夹和压缩包。
- 多格式阅读：支持文件夹漫画，以及 `zip`、`cbz`、`rar`、`cbr` 压缩包漫画。
- 元数据读取：支持读取 `元数据.json`、`cover.jpg`、作者、标签、简介和章节信息。
- 书架筛选与排序：支持搜索、仓库筛选、标签多选、阅读状态、收藏状态、标题/页数/创建时间/最近阅读排序。
- 书架视图设置：支持网格、紧凑和列表布局，以及封面尺寸、作者、标签显示配置。
- 标题与元数据编辑：支持自定义漫画标题、恢复扫描标题，并可编辑标题、简介、作者和标签。
- 收藏管理：支持默认收藏和自定义收藏夹，支持批量加入、移动和移出收藏夹。
- 阅读历史与状态：自动保存最近阅读章节和页码，支持标记已读或未读。
- 阅读器模式：支持单页、双页、长条滚动，支持从左到右或从右到左阅读。
- 阅读器设置：支持图片适配、背景色、亮度、对比度、翻页动画、预加载缓存和空格滚动速度。
- 书签：支持为指定页添加书签，并在阅读器中快速跳转。
- 本地优先：数据保存在应用程序旁边的 SQLite 数据库中，移除仓库记录不会删除原始漫画文件。

## 技术栈

- 桌面框架：Tauri 2
- 前端：Vue 3 + TypeScript + Vite
- UI 组件：Naive UI
- 后端：Rust
- 数据库：SQLite via `rusqlite`
- 压缩包读取：`zip`、`unrar-ng`
- 图片处理：`image`

## 环境要求

- Node.js LTS
- pnpm 10+
- Rust stable，包括 `rustc` 和 `cargo`
- Windows WebView2 Runtime，通常 Windows 10/11 已自带

检查开发环境：

```bash
pnpm check:env
```

## 快速开始

```bash
pnpm install
pnpm tauri:dev
```

常用命令：

```bash
pnpm dev                    # 仅启动 Vite 前端开发服务器
pnpm tauri:dev              # 启动 Tauri 桌面开发模式
pnpm typecheck              # TypeScript 类型检查
pnpm build                  # 前端类型检查和构建
pnpm tauri:build:installer  # 构建 Windows NSIS 安装包
pnpm tauri:build:portable   # 构建 Windows 便携版
```

Rust 检查和测试：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## 打包输出

构建安装包：

```bash
pnpm tauri:build:installer
```

构建便携版：

```bash
pnpm tauri:build:portable
```

便携版产物位于：

```text
src-tauri/target/release/portable/InkReader-portable-<version>-windows-x64/
  InkReader.exe
  data/
  README-portable.txt
```

同目录还会生成对应的 `.zip` 压缩包。

## 漫画仓库结构

InkReader 会把仓库目录下的每个一级文件夹或受支持压缩包识别为一本漫画。

文件夹漫画示例：

```text
漫画仓库/
  漫画 A/
    元数据.json
    cover.jpg
    第 1 话/
      0001.jpg
      0002.jpg
    第 2 话/
      0001.jpg
      0002.jpg
```

单层图片目录也会被识别为一本漫画，并自动生成“正文”章节：

```text
漫画仓库/
  漫画 B/
    cover.jpg
    0001.jpg
    0002.jpg
```

压缩包漫画示例：

```text
漫画仓库/
  漫画 C.cbz
  漫画 D.zip
  漫画 E.cbr
  漫画 F.rar
```

文件夹漫画的章节目录中也可以放置压缩包，InkReader 会把这些压缩包识别为章节：

```text
漫画仓库/
  漫画 G/
    cbz/
      cover.jpg
      第 1 话.cbz
      第 2 话.cbz
```

扫描规则：

- 仓库只扫描一级文件夹和一级压缩包作为漫画入口。
- 图片按自然顺序排序，例如 `1.jpg`、`2.jpg`、`10.jpg`。
- 文件夹漫画优先使用根目录 `cover.jpg` 作为封面；没有封面时使用第一张正文图片。
- 压缩包漫画优先使用包内 `cover.jpg` 作为封面；没有封面时使用第一张图片。
- 重新扫描会基于文件签名跳过未变化的漫画，并尽量保留阅读进度、收藏和书签。

## 元数据格式

`元数据.json` 使用 camelCase 字段。示例：

```json
{
  "id": "comic-001",
  "name": "漫画标题",
  "addtime": "2026-01-01T00:00:00Z",
  "description": "漫画简介",
  "author": ["作者 A", "作者 B"],
  "tags": ["动作", "冒险"],
  "chapterInfos": [
    {
      "chapterId": "chapter-001",
      "chapterTitle": "第 1 话",
      "order": 1
    }
  ]
}
```

字段说明：

- `name` 用作书架显示标题。
- `id` 用作来源 ID，可辅助识别重复漫画。
- `description`、`author`、`tags` 会显示在详情页，并用于标签筛选。
- `chapterInfos[].chapterTitle` 需要和章节目录名匹配。
- `chapterInfos[].order` 用于控制章节排序。

## 支持格式

可作为阅读页面的图片格式：

- `jpg`
- `jpeg`
- `png`
- `webp`

可作为漫画或章节的压缩包格式：

- `zip`
- `cbz`
- `rar`
- `cbr`

## 数据存储

应用数据保存在可执行文件所在目录的 `data/` 下：

```text
<InkReader.exe 所在目录>/data/inkreader.sqlite3
```

缩略图缓存也保存在同一数据目录下。便携版的数据位于解压目录的 `data/` 中，因此可以通过把便携版解压到非系统盘来避免数据写入 C 盘。

注意事项：

- 程序目录必须可写，否则应用无法创建或打开数据库。
- 从旧版本升级时，如果新位置没有数据库，应用会尝试从系统应用数据目录迁移旧数据库。
- 删除仓库记录只会删除 InkReader 的索引数据，不会删除原始漫画文件。

## 首次加载说明

首次添加或打开包含大量漫画的仓库时，InkReader 需要扫描本地文件、读取元数据并生成书架缩略图缓存，因此耗时可能较长。缩略图和数据库缓存生成后，后续打开和重新扫描会明显更快。

## 项目结构

```text
src/                         Vue 前端源码
src/api/                     Tauri command 调用封装和共享类型
src/components/              通用组件和业务组件
src/pages/                   页面级组件
src/router/                  前端路由
src/styles/                  全局样式
src-tauri/                   Tauri/Rust 工程
src-tauri/src/commands/      Tauri commands
src-tauri/src/db/            SQLite 初始化、迁移和数据访问
src-tauri/src/metadata/      漫画元数据解析
src-tauri/src/models/        后端数据模型
src-tauri/src/scanner/       本地仓库、图片和压缩包扫描
scripts/                     环境检查和打包脚本
```

## 贡献

欢迎通过 Issue 和 Pull Request 参与改进。提交代码前建议至少运行：

```bash
pnpm typecheck
cargo test --manifest-path src-tauri/Cargo.toml
```

提交 PR 时请说明修改目的、影响范围，以及已经执行的检查或测试。涉及界面变化时建议附带截图。

## 许可证

本项目基于 MIT License 开源，详见 [LICENSE](LICENSE)。
