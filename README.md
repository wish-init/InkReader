# InkReader

InkReader 是一款面向本地离线漫画收藏的桌面阅读器。它使用 Tauri 2、Vue 3、TypeScript、Rust 和 SQLite 构建，重点解决本地漫画仓库扫描、书架管理、压缩包阅读、阅读进度、收藏整理、元数据维护和数据备份等日常需求。

项目当前以 Windows 桌面使用为主要目标。应用只索引和读取用户选择的漫画文件，不会修改、移动或删除原始漫画文件。

## 主要功能

### 本地仓库

- 添加本地漫画仓库目录。
- 扫描仓库下的一层漫画文件夹和一层压缩包。
- 支持手动重扫和启动后自动扫描。
- 支持增量扫描，未变化的漫画会跳过深度扫描。
- 扫描过程中显示进度、扫描阶段和持久化阶段。
- 保存扫描历史、跳过条目、失败条目和重复漫画诊断。
- 移除仓库记录时只删除 InkReader 索引，不删除原始漫画文件。

### 书架与检索

- 后端分页加载书架，适合较大本地库。
- 支持搜索标题、按仓库筛选、按作者筛选、按标签筛选、排除标签。
- 支持阅读状态筛选：全部、未读、阅读中、已读。
- 支持收藏状态筛选：全部、已收藏、未收藏。
- 支持元数据缺失筛选：缺少简介、作者、标签、封面、发布时间。
- 支持按标题、页数、创建时间、最近阅读、发布时间排序。
- 支持网格、紧凑、列表三种书架布局。
- 支持封面尺寸、作者显示、标签显示、标题行数和标题字号设置。

### 收藏与聚合

- 支持默认收藏和自定义收藏夹。
- 支持批量加入、移动、移出收藏夹。
- 支持收藏夹名称、简介和封面维护。
- 支持作者聚合页和标签聚合页，便于从人物或标签维度浏览漫画。

### 漫画详情与元数据

- 显示标题、简介、作者、标签、章节数、页数、阅读进度等信息。
- 支持自定义漫画标题，并可恢复扫描标题。
- 支持编辑标题、简介、作者和标签。
- 支持元数据健康检查，集中查看缺失元数据、缺失封面、无图片条目和重复漫画。

### 阅读器

- 支持单页、双页、长条滚动阅读模式。
- 支持从左到右和从右到左阅读方向。
- 支持宽度适配、高度适配和原始尺寸。
- 支持章节切换、页码跳转、上一页/下一页、上一章/下一章。
- 自动保存阅读进度和最近阅读记录。
- 支持标记已读、标记未读。
- 支持书签创建、删除、检查和抽屉式跳转。
- 支持亮度、对比度、背景色、翻页动画、空格滚动比例、长按空格滚动速度。
- 支持自动滚动速度、启动延迟和手动滚动停止行为设置。
- 支持有限图片预加载缓存，降低长章节阅读时的内存风险。

### 压缩包阅读

- 支持将 `zip`、`cbz`、`rar`、`cbr` 作为漫画条目扫描。
- 支持文件夹漫画中把压缩包作为章节。
- 支持按需读取压缩包内页面，不需要一次性解压整本漫画。
- 提供 `archive://` 自定义协议读取压缩包内图片。
- 后端对压缩包页面字节做小型 LRU 缓存，改善连续阅读体验。

### 缓存、备份与设置

- 自动生成和复用书架缩略图缓存。
- 支持查看缩略图缓存统计。
- 支持清理托管缩略图缓存。
- 支持重建缺失缩略图。
- 支持创建 SQLite 数据库备份。
- 支持从备份恢复数据库，恢复前会生成回滚文件。
- 支持阅读器设置、书架视图设置和单本阅读器设置。
- 支持设置导出、设置导入和恢复默认设置。
- 支持操作日志，记录备份、恢复、缓存维护等高影响操作。

## 下载

如果只想使用已构建版本，可以从 GitHub Releases 下载 Windows 安装包或便携包：

[InkReader Releases](https://github.com/wish-init/InkReader/releases)

便携包可以直接运行 `InkReader.exe`。便携包的数据默认保存在可执行文件旁边的 `data/` 目录中，适合放在非系统盘或随项目目录迁移。

## 技术栈

| 层级 | 技术 |
|---|---|
| 桌面框架 | Tauri 2 |
| 前端 | Vue 3、TypeScript、Vite |
| UI 组件 | Naive UI |
| 路由 | Vue Router hash history |
| 后端 | Rust |
| 数据库 | SQLite via `rusqlite` |
| 压缩包 | `zip`、`unrar-ng` |
| 图片处理 | `image` |
| 排序 | 自然排序，适配 `1.jpg`、`2.jpg`、`10.jpg` |

## 环境要求

- Node.js LTS。
- pnpm 10+，本项目锁定 `pnpm@10.11.0`。
- Rust stable，包含 `rustc` 和 `cargo`。
- Windows WebView2 Runtime。Windows 10/11 通常已自带。
- Windows 构建 Tauri 时需要可用的 MSVC/Rust Windows 工具链。

检查本地开发环境：

```bash
pnpm check:env
```

## 快速开始

安装依赖：

```bash
pnpm install
```

启动桌面开发模式：

```bash
pnpm tauri:dev
```

也可以使用项目默认启动命令：

```bash
pnpm start
```

只启动 Vite 前端开发服务器：

```bash
pnpm dev
```

前端开发服务器地址由 Tauri 配置为：

```text
http://127.0.0.1:1420
```

## 常用命令

```bash
pnpm check:env              # 检查 Node.js、pnpm、rustc、cargo
pnpm dev                    # 仅启动 Vite 前端开发服务器
pnpm start                  # 检查环境并启动 Tauri 开发模式
pnpm tauri:dev              # 检查环境并启动 Tauri 开发模式
pnpm typecheck              # TypeScript 类型检查
pnpm build                  # 前端类型检查和 Vite 构建
pnpm tauri:build            # 构建 Windows NSIS 安装包
pnpm tauri:build:installer  # 构建 Windows NSIS 安装包
pnpm tauri:build:portable   # 构建 Windows 便携包和 zip
```

Rust 常用检查：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## 打包输出

构建安装包：

```bash
pnpm tauri:build:installer
```

构建便携包：

```bash
pnpm tauri:build:portable
```

便携包输出位置：

```text
src-tauri/target/release/portable/InkReader-portable-<version>-windows-x64/
  InkReader.exe
  data/
  README-portable.txt
```

同目录还会生成对应的 `.zip` 文件。

## 支持的漫画结构

InkReader 会把仓库目录下的一层文件夹或一层受支持压缩包识别为漫画入口。

### 多章节文件夹漫画

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

### 单层图片目录

单层图片目录会被识别为一本漫画，并自动生成名为 `正文` 的章节：

```text
漫画仓库/
  漫画 B/
    cover.jpg
    0001.jpg
    0002.jpg
    0003.jpg
```

### 压缩包漫画

```text
漫画仓库/
  漫画 C.cbz
  漫画 D.zip
  漫画 E.cbr
  漫画 F.rar
```

### 压缩包章节

文件夹漫画中也可以把压缩包作为章节：

```text
漫画仓库/
  漫画 G/
    cover.jpg
    cbz/
      第 1 话.cbz
      第 2 话.cbz
```

## 扫描规则

- 仓库只扫描一层文件夹和一层压缩包作为漫画入口。
- 支持的图片页格式为 `jpg`、`jpeg`、`png`、`webp`。
- 支持的压缩包格式为 `zip`、`cbz`、`rar`、`cbr`。
- 图片和章节使用自然排序。
- 文件夹漫画优先使用根目录 `cover.jpg` 作为封面。
- 没有 `cover.jpg` 时，使用第一张正文图片作为封面候选。
- 压缩包漫画优先使用包内 `cover.jpg`，否则使用包内第一张图片。
- 漫画目录存在 `元数据.json` 时，优先使用其中的标题、作者、标签、简介和章节顺序。
- 单本漫画解析失败不会修改原始文件，错误会记录到扫描诊断中。
- 重扫会基于路径、修改时间、大小等签名跳过未变化条目，并尽量保留阅读进度、收藏、书签和自定义标题。

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

- `id`：来源 ID，可用于辅助识别来源。
- `name`：书架显示标题。
- `addtime`：来源发布时间或添加时间，会映射为发布时间。
- `description`：漫画简介。
- `author`：作者列表。
- `tags`：标签列表。
- `chapterInfos[].chapterTitle`：章节标题，通常与章节目录名匹配。
- `chapterInfos[].order`：章节排序值。

## 数据存储

InkReader 的运行数据保存在可执行文件所在目录旁边的 `data/` 中：

```text
<InkReader.exe 所在目录>/data/inkreader.sqlite3
<InkReader.exe 所在目录>/data/thumbs/
```

开发模式下，可执行文件通常位于：

```text
src-tauri/target/debug/
```

因此开发数据通常位于：

```text
src-tauri/target/debug/data/inkreader.sqlite3
```

注意事项：

- 应用目录需要可写，否则数据库和缩略图缓存无法创建。
- 从旧版本升级时，如果新位置没有数据库，应用会尝试从系统应用数据目录迁移旧数据库。
- 删除仓库记录只删除 InkReader 索引，不删除漫画源文件。
- 缩略图缓存只保存应用生成的缓存文件，可以通过应用内缓存维护功能重建。

## 项目结构

```text
src/                              Vue 前端源码
src/api/                          Tauri command 调用封装和共享类型
src/components/common/            应用外壳和通用组件
src/components/library/           书架列表、卡片和视图设置组件
src/components/reader/            阅读器工具栏、视口、章节和书签抽屉
src/composables/library/          书架列表、选择、上下文菜单、缩略图水合逻辑
src/composables/reader/           阅读器导航、键盘、进度、预加载、缩放、自动滚动逻辑
src/pages/                        页面级组件
src/router/                       前端路由
src/styles/                       全局样式和主题变量
src/utils/                        纯工具函数和默认设置

src-tauri/                        Tauri/Rust 工程
src-tauri/src/lib.rs              Tauri builder、AppState、archive 协议和 command 注册
src-tauri/src/commands/           Tauri command 入口
src-tauri/src/db/                 SQLite 初始化、迁移、查询、备份、缓存维护
src-tauri/src/metadata/           元数据 JSON 解析
src-tauri/src/models/             Rust 与前端共享的序列化模型
src-tauri/src/scanner/            仓库、文件夹、压缩包、图片、自然排序扫描逻辑
src-tauri/src/thumbnail.rs        缩略图生成

scripts/                          环境检查和便携包脚本
docs/performance/                 大库和长章节性能基线
```

## 前端页面

当前路由：

| 路径 | 页面 |
|---|---|
| `/library` | 书架 |
| `/books/:bookId` | 漫画详情 |
| `/reader/:bookId` | 阅读器 |
| `/favorites` | 收藏 |
| `/authors` | 作者聚合 |
| `/tags` | 标签聚合 |
| `/history` | 阅读记录 |
| `/health` | 元数据健康 |
| `/repositories` | 仓库管理 |
| `/settings` | 设置 |

## 后端职责

Rust 后端负责：

- 本地路径校验。
- 仓库扫描和增量扫描。
- 文件夹漫画和压缩包漫画识别。
- `元数据.json` 解析。
- 图片页过滤、自然排序和 URI 生成。
- SQLite 迁移、读写、事务和索引维护。
- 阅读进度、历史、书签、收藏和设置持久化。
- 缩略图生成、清理和重建。
- 数据库备份与恢复。
- archive 协议读取压缩包内图片。

前端负责：

- 页面展示和用户交互。
- 调用 `src/api/` 中的 Tauri command 封装。
- 管理页面状态、筛选条件、阅读器设置和本地 UI 偏好。
- 展示错误、扫描诊断、缓存维护结果和备份恢复结果。

## 性能基线

项目包含大库和长章节性能基线文档：

[docs/performance/large-library-baseline.md](docs/performance/large-library-baseline.md)

该文档覆盖：

- 1000 本和 5000 本本地库测试方法。
- 500 页长章节测试方法。
- 30 分钟长条滚动稳定性检查。
- 缩略图水合检查。
- SQLite 索引盘点和查询计划检查流程。

涉及扫描、书架分页、缩略图、阅读器预加载、SQLite 索引的改动，建议按该文档记录前后数据。

## 开发约定

- 前端 Tauri 类型集中在 `src/api/tauri.ts`。
- 具体 API 模块通过共享 `call<T>()` 封装调用 Tauri command。
- Rust 暴露给前端的模型使用 `#[serde(rename_all = "camelCase")]`。
- 后端列表 API 负责筛选、排序、分页和总数，前端不应自行修正总数。
- 修改字段名、command 名、设置 key、路由或枚举选项前，先用 `rg` 搜索镜像位置。
- 数据库多表写入应通过事务保持一致性。
- 任何仓库、缓存、备份、恢复功能都必须避免影响用户原始漫画文件。

## 推荐检查

文档或前端类型改动：

```bash
pnpm typecheck
```

前端构建检查：

```bash
pnpm build
```

Rust 改动：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

完整桌面构建：

```bash
pnpm tauri:build:portable
```

## 常见问题

### 启动失败或白屏

先确认环境：

```bash
pnpm check:env
```

然后重新启动：

```bash
pnpm tauri:dev
```

### 无法创建数据库

确认 `InkReader.exe` 所在目录可写。便携包建议解压到普通用户目录或非系统盘，不要放在需要管理员权限写入的位置。

### 仓库扫描不到漫画

检查仓库结构是否满足以下条件之一：

- 仓库一层子目录中包含图片页或章节目录。
- 仓库一层存在 `zip`、`cbz`、`rar`、`cbr` 文件。
- 图片扩展名是 `jpg`、`jpeg`、`png`、`webp`。

### 压缩包无法读取

确认压缩包未损坏、未加密，并且扩展名是 `zip`、`cbz`、`rar` 或 `cbr`。如果单个压缩包失败，扫描诊断中会保留失败路径和原因。

### 缩略图缺失

可以在设置或缓存维护入口中重建缺失缩略图。缩略图缓存缺失不会影响漫画源文件。

### 想迁移数据

优先使用应用内数据库备份功能创建备份文件，再在目标环境中恢复。便携版也可以整体迁移包含 `InkReader.exe` 和 `data/` 的目录。

## 贡献

提交改动前建议至少运行与改动范围匹配的检查。涉及界面变化时建议附截图或说明主要交互路径。涉及扫描、阅读器、缓存或数据库性能时，建议补充可复现数据集和性能基线记录。

## 许可证

本项目基于 MIT License 开源，详见 [LICENSE](LICENSE)。
