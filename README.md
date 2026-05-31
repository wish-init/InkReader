# InkReader

InkReader 是一款基于 Tauri 2、Vue 3、TypeScript、Rust 和 SQLite 构建的本地漫画仓库阅读器。它面向离线漫画收藏管理，支持文件夹漫画、压缩包漫画、书架管理、阅读进度、收藏夹和书签等常用阅读工作流。

## 功能特性

- 本地漫画仓库：添加本地目录，扫描目录下的漫画文件夹和压缩包。
- 多格式支持：支持文件夹漫画，以及 `zip`、`cbz`、`rar`、`cbr` 压缩包。
- 元数据识别：读取 `元数据.json`、`cover.jpg`、作者、标签、简介和章节信息。
- 书架管理：支持搜索、标签筛选、排序、标题覆盖和增量重新扫描。
- 收藏管理：支持默认收藏和自定义收藏夹。
- 阅读记录：自动保存最近阅读章节和页码，并提供历史记录页面。
- 阅读模式：支持单页、双页和长条滚动阅读。
- 阅读设置：支持阅读方向、图片适配、背景色、亮度、对比度、翻页动画和预加载缓存。
- 书签：可为指定页面添加书签，并在阅读器中快速跳转。
- 本地优先：数据保存在应用目录下的 SQLite 数据库中，移除仓库记录不会删除原始漫画文件。

## 截图

项目暂未内置截图。发布开源版本时建议补充以下图片：

- 书架页面
- 阅读器页面
- 漫画详情页面
- 设置页面

## 技术栈

- 桌面框架：Tauri 2
- 前端框架：Vue 3 + TypeScript + Vite
- UI 组件：Naive UI
- 后端：Rust
- 数据库：SQLite via `rusqlite`
- 压缩包读取：`zip`、`unrar-ng`

## 环境要求

- Node.js LTS
- pnpm 10+
- Rust stable，包括 `rustc` 和 `cargo`
- Windows WebView2 Runtime，通常 Windows 10/11 已自带

可以运行以下命令检查开发环境：

```bash
pnpm check:env
```

## 快速开始

```bash
pnpm install
pnpm tauri:dev
```

常用开发命令：

```bash
pnpm dev                 # 仅启动 Vite 前端开发服务器
pnpm tauri:dev           # 启动 Tauri 桌面开发模式
pnpm typecheck           # TypeScript 类型检查
pnpm build               # 前端构建和类型检查
```

Rust 检查和测试：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## 打包

生成 Windows NSIS 安装包：

```bash
pnpm tauri:build:installer
```

生成 Windows 便携版：

```bash
pnpm tauri:build:portable
```

便携版产物目录类似：

```text
src-tauri/target/release/portable/InkReader-portable-0.1.0-windows-x64/
  InkReader.exe
  data/
  README-portable.txt
```

## 漫画仓库结构

InkReader 将仓库目录下的每个一级文件夹或受支持压缩包识别为一本漫画。

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

单层图片目录也会被识别为一本漫画，并自动生成正文章节：

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

文件夹漫画的章节目录中也可以放置压缩包，InkReader 会将这些压缩包识别为章节：

```text
漫画仓库/
  漫画 G/
    cbz/
      cover.jpg
      第 1 话.cbz
      第 2 话.cbz
```

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

说明：

- `name` 会作为书架显示标题。
- `id` 会作为来源 ID，用于辅助识别重复漫画。
- `author` 和 `tags` 会展示在漫画详情和筛选中。
- `chapterInfos[].chapterTitle` 需要和章节目录名匹配，`order` 用于控制章节排序。
- 如果没有 `cover.jpg`，InkReader 会尝试使用第一张图片作为封面。

## 支持的图片和压缩包

支持作为阅读页面的图片格式：

- `jpg`
- `jpeg`
- `png`
- `webp`

支持的压缩包格式：

- `zip`
- `cbz`
- `rar`
- `cbr`

压缩包内可包含 `元数据.json` 和 `cover.jpg`。图片会按自然顺序排序。

## 数据存储

应用数据默认存放在程序目录下：

```text
<InkReader.exe 所在目录>/data/inkreader.sqlite3
```

注意事项：

- 便携版数据位于解压目录的 `data/` 下。
- 安装版数据位于安装目录的 `data/` 下。
- 程序目录需要可写，否则应用无法创建或打开数据库。
- 如果不希望数据存放在系统盘，可以将便携版解压到其他磁盘，或安装到其他可写目录。

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
src-tauri/src/db/            SQLite 初始化和数据访问
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

提交 PR 时请尽量说明：

- 修改目的和影响范围
- 已执行的检查或测试
- 涉及 UI 的变更截图

## 许可协议

本项目基于 MIT License 开源，详见 [LICENSE](LICENSE)。
