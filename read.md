# InkReader 开发计划

InkReader 是一个面向本地漫画仓库的桌面阅读软件。第一阶段目标不是做大而全的平台，而是先做一个轻量、稳定、可长期扩展的本地漫画仓库阅读器。

## 1. 项目定位

### 1.1 核心目标

InkReader 的第一目标是：

- 管理本地漫画仓库
- 支持扫描仓库内的多部漫画
- 支持读取每部漫画的元数据、封面、章节和图片页
- 记录每部漫画、每个章节的阅读进度
- 提供流畅、安静、专注的阅读体验
- 后续扩展支持 `cbz`、`zip`、`cbr`、`rar` 等漫画压缩包格式

### 1.2 目标目录结构

第一版需要优先支持仓库结构，而不是只支持单本漫画文件夹。

目标结构如下：

```text
漫画仓库/
  漫画 A/
    元数据.json
    cover.jpg
    第1话/
      0001.jpg
      0002.jpg
      0003.jpg
    第2话/
      0001.jpg
      0002.jpg
  漫画 B/
    元数据.json
    cover.jpg
    第1话/
      0001.jpg
      0002.jpg
```

规则：

- 用户添加的是“漫画仓库”目录
- 仓库下的一级子目录视为候选漫画目录
- 每个漫画目录可以包含 `元数据.json`
- 每个漫画目录可以包含 `cover.jpg`
- 每个漫画目录下的章节目录包含图片页
- 章节目录名优先与 `元数据.json` 中的 `chapterInfos[].chapterTitle` 对应
- 图片页按自然顺序排序

### 1.3 元数据格式

示例仓库中的 `元数据.json` 包含这些核心字段：

```json
{
  "id": 1440181,
  "name": "作品名称",
  "addtime": "1779157971",
  "description": "作品描述",
  "author": ["作者"],
  "tags": ["标签"],
  "chapterInfos": [
    {
      "chapterId": 1440181,
      "chapterTitle": "第1话",
      "order": 1
    }
  ]
}
```

第一版应读取并保存：

- `id`
- `name`
- `addtime`
- `description`
- `author`
- `tags`
- `chapterInfos`

不需要第一版展示全部字段，但数据库应保留必要信息，避免后续重扫时丢失元数据。

### 1.4 非目标

第一版不做以下内容：

- 在线漫画源
- 账号系统
- 云同步
- 插件系统
- 自动更新
- 多语言
- 复杂主题市场
- OCR、翻译、AI 图像处理

这些功能会显著提高复杂度，不适合作为初始版本的一部分。

## 2. 技术选型

### 2.1 总体技术栈

- 桌面框架：Tauri 2
- 前端框架：Vue 3
- 前端语言：TypeScript
- 前端构建：Vite
- 前端包管理：pnpm
- 后端语言：Rust
- 后端包管理：Cargo
- 数据库：SQLite
- 数据库访问：Rust 侧封装
- 样式方案：先使用普通 CSS 或轻量 CSS 变量体系，后续按需要引入 Tailwind CSS

### 2.2 选择 Tauri 的原因

Tauri 更适合作为 InkReader 的最终形态：

- 安装包体积小
- 运行时更轻
- 内存占用更低
- Rust 适合处理本地文件、数据库、缓存、解压和仓库扫描任务
- 前端仍然可以使用 Vue 构建现代 UI

### 2.3 选择 Vue 3 的原因

Vue 3 对该项目比较合适：

- 页面结构清晰
- 组件组织直观
- 状态管理简单
- 适合快速迭代 UI
- AI 生成和维护 Vue 组件相对稳定

### 2.4 选择 pnpm 的原因

前端使用 pnpm：

- 安装速度快
- 依赖锁定稳定
- 磁盘占用低
- 对 Vite/Vue/Tauri 生态支持成熟

Rust 部分使用 Cargo，这是 Tauri 后端的标准工具链。

## 3. 产品形态

### 3.1 应用主页面

第一版应用包含四个主要页面：

- 仓库页
- 书架页
- 阅读器页
- 设置页

仓库页负责添加和管理漫画仓库。书架页负责展示仓库扫描出的漫画。

### 3.2 仓库页

仓库页用于管理本地漫画仓库。

第一版功能：

- 添加漫画仓库
- 显示仓库路径
- 显示仓库内漫画数量
- 显示上次扫描时间
- 支持重新扫描仓库
- 支持移除仓库记录

注意：移除仓库只删除 InkReader 的数据库记录，不删除用户原始文件。

### 3.3 书架页

书架页用于展示已扫描出的漫画。

第一版功能：

- 显示漫画封面
- 显示漫画标题
- 显示作者
- 显示标签摘要
- 显示章节数量
- 显示阅读进度
- 支持继续阅读
- 支持进入漫画详情或直接进入阅读器
- 支持重新扫描所属漫画或所属仓库

后续功能：

- 搜索漫画
- 标签筛选
- 作者筛选
- 最近阅读
- 收藏
- 批量导入目录
- 拖拽导入仓库

### 3.4 阅读器页

阅读器页是核心体验。

第一版功能：

- 单页阅读
- 章节切换
- 上一页/下一页
- 上一章/下一章
- 键盘翻页
- 当前章节页码显示
- 当前章节/总章节显示
- 返回书架
- 自动保存阅读进度
- 基础图片适配

后续功能：

- 双页模式
- 长条滚动模式
- 适应宽度
- 适应高度
- 原始尺寸
- 全屏模式
- 右到左阅读方向
- 自动隐藏工具栏
- 快速跳页
- 页面缩放
- 背景色设置
- 页面间距设置

### 3.5 设置页

设置页用于调整阅读偏好。

第一版可以先只保留基础结构，不急着做大量设置项。

后续设置项：

- 默认阅读模式
- 默认适配方式
- 默认阅读方向
- 书架排序方式
- 是否记住窗口大小
- 是否启动时打开上次阅读
- 背景色
- 快捷键配置
- 缓存清理

## 4. 第一版范围

第一版要控制范围，只做最小可用版本。

### 4.1 必须完成

- 创建 Tauri + Vue + TypeScript 项目
- 配置 pnpm 开发命令
- 配置 Rust command
- 前端可以调用 Rust 后端
- 支持选择本地漫画仓库目录
- Rust 扫描仓库下的漫画目录
- Rust 读取 `元数据.json`
- Rust 识别 `cover.jpg`
- Rust 识别章节目录
- Rust 扫描章节目录内图片
- 图片按自然顺序排序
- 前端显示漫画书架
- 前端显示漫画章节
- 支持单页翻页阅读
- 支持键盘左右键翻页
- 支持保存阅读进度
- 支持关闭后重新打开仍保留仓库、书架和进度

### 4.2 暂不处理

- 压缩包格式
- 缩略图生成
- 复杂缓存系统
- 双页模式
- 长条模式
- 高级设置
- 自动更新
- 安装包签名

## 5. 开发阶段

## 阶段 0：环境准备

### 阶段 0 目标

确保本地环境可以创建、运行、构建 Tauri 项目。

### 需要安装

- Node.js LTS
- pnpm
- Rust stable，通过 Rustup 安装，必须包含 `rustc` 和 `cargo`
- Tauri 依赖
- WebView2 Runtime，Windows 通常已经自带

### 阶段 0 验收标准

- `cargo --version` 和 `rustc --version` 成功

## 阶段 1：项目骨架

### 阶段 1 目标

建立稳定的项目基础结构。

### 阶段 1 任务

- 初始化 Tauri 2 + Vue 3 + TypeScript 项目
- 整理前端目录结构
- 整理 Rust 后端目录结构
- 建立基础路由
- 建立基础布局
- 建立前端 API 调用封装
- 建立第一个 Rust command 用于连通性测试

### 前端页面

- `RepositoryPage.vue`
- `LibraryPage.vue`
- `ReaderPage.vue`
- `SettingsPage.vue`

### Rust 模块

- `commands`
- `db`
- `scanner`
- `cache`
- `metadata`

### 阶段 1 验收标准

- 应用能启动
- 四个页面可以切换
- 前端可以成功调用 Rust command
- 项目命令清晰可用

## 阶段 2：漫画仓库扫描

### 阶段 2 目标

支持用户选择一个本地漫画仓库，并扫描仓库中的多部漫画。

### 阶段 2 任务

- 使用 Tauri 文件选择能力打开目录选择框
- Rust 接收仓库根路径
- Rust 遍历仓库一级子目录
- 判断每个一级子目录是否为有效漫画目录
- 读取漫画目录中的 `元数据.json`
- 读取或记录漫画目录中的 `cover.jpg`
- 根据 `chapterInfos` 识别章节
- 兼容没有出现在 `chapterInfos` 中但实际存在的章节目录
- 扫描章节目录中的图片文件
- 过滤支持的图片格式
- 对章节和图片进行自然排序
- 返回仓库扫描结果给前端
- 前端展示扫描出的漫画列表

### 有效漫画目录规则

满足以下任意条件即可视为候选漫画目录：

- 存在 `元数据.json`
- 存在 `cover.jpg` 且存在至少一个章节目录
- 存在至少一个包含图片的子目录

优先级：

1. 如果存在 `元数据.json`，以元数据为主
2. 如果元数据缺失或损坏，用目录名作为标题
3. 如果 `cover.jpg` 存在，作为封面
4. 如果 `cover.jpg` 不存在，后续阶段可用第一张图片生成封面

### 支持格式

第一版支持：

- `jpg`
- `jpeg`
- `png`
- `webp`

后续再考虑：

- `gif`
- `avif`
- `bmp`

### 排序规则

需要支持自然排序，例如：

- `1.jpg`
- `2.jpg`
- `10.jpg`

应排序为：

- `1.jpg`
- `2.jpg`
- `10.jpg`

而不是：

- `1.jpg`
- `10.jpg`
- `2.jpg`

章节优先使用 `chapterInfos[].order` 排序。没有元数据排序信息的章节，再按目录名自然排序。

### 阶段 2 验收标准

- 可以选择本地漫画仓库
- 可以扫描出仓库中的漫画
- 可以读取漫画标题、作者、标签、章节信息
- 可以识别 `cover.jpg`
- 可以扫描章节图片
- 章节顺序符合元数据或自然排序
- 图片顺序符合正常阅读顺序
- 空仓库或无有效漫画目录有清晰错误提示

## 阶段 3：基础阅读器

### 阶段 3 目标

完成可阅读的单页章节阅读器。

### 阶段 3 任务

- 创建阅读器状态
- 支持从书架选择漫画进入阅读器
- 默认打开上次阅读章节和页面
- 没有进度时默认打开第一章第一页
- 显示当前章节的当前页图片
- 支持上一页
- 支持下一页
- 支持上一章
- 支持下一章
- 支持键盘左/右方向键翻页
- 支持空格下一页
- 显示当前页码
- 显示当前章节总页数
- 显示当前章节标题
- 支持返回书架
- 处理图片加载失败状态

### UI 要点

阅读器界面应保持克制：

- 图片是视觉中心
- 工具栏不遮挡图片主体
- 翻页反馈要直接
- 页码信息清楚但不抢眼
- 章节切换要容易触达
- 不做营销式页面
- 不添加无意义装饰

### 阶段 3 验收标准

- 可以从第一章第一页读到最后一章最后一页
- 可以从最后一页返回上一页
- 跨章节翻页正常
- 键盘翻页正常
- 图片加载失败时不会导致页面崩溃

## 阶段 4：SQLite 数据库

### 阶段 4 目标

保存仓库、漫画、章节和阅读进度。

### 阶段 4 任务

- 初始化 SQLite 数据库
- 建立数据库文件路径
- 建立数据库迁移机制
- 创建 `repositories` 表
- 创建 `books` 表
- 创建 `chapters` 表
- 创建 `settings` 表
- 封装基础数据库操作
- 保存仓库信息
- 保存漫画元数据
- 保存章节信息
- 保存阅读进度
- 启动时读取仓库和书架数据

### 阶段 4 验收标准

- 添加到应用的仓库关闭后仍存在
- 仓库扫描出的漫画关闭应用后仍存在
- 阅读到某一章节某一页后关闭应用，再打开能继续阅读
- 数据库初始化失败时有错误提示

## 阶段 5：仓库和书架功能

### 阶段 5 目标

让用户可以管理漫画仓库和仓库内漫画。

### 阶段 5 任务

- 添加漫画仓库
- 显示仓库列表
- 重新扫描仓库
- 移除仓库记录
- 显示漫画列表
- 显示标题
- 显示作者
- 显示标签摘要
- 显示章节数
- 显示总页数
- 显示阅读进度
- 点击漫画进入阅读器
- 继续上次阅读
- 重新扫描单本漫画

### 标题规则

标题优先级：

1. `元数据.json` 中的 `name`
2. 漫画目录名

### 封面规则

封面优先级：

1. 漫画目录下的 `cover.jpg`
2. 后续阶段生成的封面缓存
3. 空封面占位

### 阶段 5 验收标准

- 可以添加一个漫画仓库
- 书架可以显示仓库内多本漫画
- 每本漫画有独立阅读进度
- 每本漫画有独立章节列表
- 移除仓库只删除数据库记录，不删除用户原始文件

## 阶段 6：阅读体验增强

### 阶段 6 目标

把阅读器从能用提升到好用。

### 阶段 6 任务

- 图片预加载
- 限制预加载范围
- 当前页前后各预加载若干页
- 跨章节边界预加载
- 释放远离当前页的图片资源
- 支持适应宽度
- 支持适应高度
- 支持原始尺寸
- 支持全屏
- 支持页面跳转
- 支持章节跳转

### 内存策略

漫画图片可能很大，不能一次性加载整本。

建议策略：

- 当前页必须加载
- 前后 2 页预加载
- 即将跨章节时预加载下一章第一张图
- 离当前页较远的图片释放引用
- 封面和缩略图使用单独缓存

### 阶段 6 验收标准

- 几百页漫画可以正常阅读
- 连续翻页没有明显卡顿
- 内存不会随翻页无限增长
- 跨章节阅读体验自然

## 阶段 7：封面和缩略图缓存

### 阶段 7 目标

让书架加载更快、更稳定。

### 阶段 7 任务

- 优先使用漫画目录中的 `cover.jpg`
- 为封面生成应用内部缩略图
- 缓存封面缩略图
- 书架优先读取缓存封面
- 支持封面缓存失效后重新生成
- 对没有 `cover.jpg` 的漫画，用第一章第一张图生成封面

### 缓存原则

- 不修改用户原始漫画文件
- 缓存写入应用数据目录
- 缓存可以安全删除
- 缓存缺失时可重新生成

### 阶段 7 验收标准

- 书架显示封面
- 重启后封面加载速度稳定
- 删除缓存后应用可以重新生成封面

## 阶段 8：阅读模式扩展

### 阶段 8 目标

支持更完整的漫画阅读习惯。

### 阶段 8 任务

- 单页模式
- 双页模式
- 长条滚动模式
- 从左到右阅读
- 从右到左阅读
- 自动隐藏工具栏
- 背景色设置
- 页面间距设置

### 双页模式注意点

需要处理：

- 封面是否单独显示
- 奇偶页配对
- 从右到左时的页面顺序
- 宽图是否单独显示
- 跨章节时是否继续双页配对

### 阶段 8 验收标准

- 日漫双页阅读方向正确
- 长条模式滚动流畅
- 切换阅读模式不会丢失进度

## 阶段 9：压缩包支持

### 阶段 9 目标

支持常见漫画压缩包格式。

### 优先级

先支持：

- `zip`
- `cbz`

后支持：

- `rar`
- `cbr`

可选支持：

- `7z`

### zip/cbz 任务

- 识别 zip/cbz 文件
- 支持仓库目录下的压缩包作为漫画条目
- 读取压缩包内图片列表
- 读取压缩包内可能存在的元数据和封面
- 自然排序
- 按需读取当前页图片
- 避免一次性解压整个压缩包
- 必要时建立临时缓存

### rar/cbr 任务

- 调研 Rust 生态中的 rar 支持
- 处理平台兼容问题
- 决定是否引入外部解压程序或库
- 处理加密压缩包错误

### 阶段 9 验收标准

- 可以将 cbz/zip 作为仓库内漫画识别
- 可以直接阅读 cbz/zip
- 大型压缩包不会一次性占用大量内存

## 阶段 10：打包和发布

### 阶段 10 目标

生成可安装的桌面应用。

### 阶段 10 任务

- 配置 Tauri build
- 配置应用图标
- 配置应用名称
- 配置 Windows 安装包
- 测试安装和卸载
- 测试应用数据目录
- 测试数据库路径

### 暂不处理

- 代码签名
- 自动更新
- 多渠道发布

这些可以在功能稳定后再做。

### 阶段 10 验收标准

- 可以构建 Windows 安装包
- 安装后可以正常运行
- 卸载不会删除用户漫画原始文件

## 6. 推荐目录结构

```text
InkReader/
  package.json
  pnpm-lock.yaml
  index.html
  vite.config.ts
  tsconfig.json
  src/
    main.ts
    App.vue
    router/
      index.ts
    api/
      tauri.ts
      repositories.ts
      library.ts
      reader.ts
      settings.ts
    pages/
      RepositoryPage.vue
      LibraryPage.vue
      ReaderPage.vue
      SettingsPage.vue
    components/
      repositories/
        RepositoryList.vue
        RepositoryCard.vue
      library/
        BookGrid.vue
        BookCard.vue
        EmptyLibrary.vue
        ChapterList.vue
      reader/
        ReaderViewport.vue
        ReaderToolbar.vue
        PageIndicator.vue
        ChapterSelector.vue
      common/
        AppShell.vue
        IconButton.vue
        Modal.vue
    stores/
      repositories.ts
      library.ts
      reader.ts
      settings.ts
    styles/
      base.css
      theme.css
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/
      main.rs
      lib.rs
      commands/
        mod.rs
        repositories.rs
        library.rs
        reader.rs
        settings.rs
      db/
        mod.rs
        migrations.rs
        repositories.rs
        books.rs
        chapters.rs
        settings.rs
      scanner/
        mod.rs
        repository.rs
        book.rs
        chapter.rs
        image.rs
        sort.rs
      metadata/
        mod.rs
        comic_metadata.rs
      cache/
        mod.rs
        covers.rs
      models/
        mod.rs
        repository.rs
        book.rs
        chapter.rs
        page.rs
        settings.rs
      errors.rs
```

## 7. 前后端职责划分

### 7.1 前端负责

- 页面展示
- 用户交互
- 阅读器布局
- 键盘快捷键
- 状态管理
- 调用 Tauri command
- 展示错误信息

### 7.2 Rust 后端负责

- 仓库扫描
- 漫画目录识别
- 元数据解析
- 章节识别
- 图片文件识别
- 自然排序
- 数据库读写
- 封面缓存
- 压缩包读取
- 应用数据目录管理
- 文件路径安全处理

### 7.3 边界原则

前端不要直接处理复杂本地文件逻辑。

Rust 后端返回结构化数据，前端只负责渲染和交互。

## 8. API 设计草案

### 8.1 仓库相关

```ts
type Repository = {
  id: string
  name: string
  path: string
  bookCount: number
  lastScannedAt?: string
  createdAt: string
  updatedAt: string
}
```

前端 API：

```ts
listRepositories(): Promise<Repository[]>
addRepository(path: string): Promise<RepositoryScanResult>
rescanRepository(id: string): Promise<RepositoryScanResult>
removeRepository(id: string): Promise<void>
```

### 8.2 书架相关

```ts
type Book = {
  id: string
  repositoryId: string
  sourceId?: string
  title: string
  path: string
  kind: 'folder' | 'zip' | 'rar'
  coverPath?: string
  description?: string
  authors: string[]
  tags: string[]
  chapterCount: number
  totalPages: number
  lastChapterId?: string
  lastPage: number
  createdAt: string
  updatedAt: string
}
```

前端 API：

```ts
listBooks(repositoryId?: string): Promise<Book[]>
getBook(id: string): Promise<Book>
rescanBook(id: string): Promise<Book>
removeBookRecord(id: string): Promise<void>
updateBookProgress(bookId: string, chapterId: string, page: number): Promise<void>
```

### 8.3 章节相关

```ts
type Chapter = {
  id: string
  bookId: string
  sourceChapterId?: string
  title: string
  path: string
  order: number
  pageCount: number
}
```

前端 API：

```ts
listBookChapters(bookId: string): Promise<Chapter[]>
```

### 8.4 阅读器相关

```ts
type Page = {
  index: number
  name: string
  uri: string
  width?: number
  height?: number
}
```

前端 API：

```ts
listChapterPages(chapterId: string): Promise<Page[]>
getPageUri(chapterId: string, page: number): Promise<string>
```

第一版可以直接返回图片文件路径或 Tauri 可访问 URI。后续压缩包支持时，再改成由后端提供按需读取的资源地址。

### 8.5 设置相关

```ts
type ReaderSettings = {
  mode: 'single' | 'double' | 'scroll'
  fit: 'width' | 'height' | 'original'
  direction: 'ltr' | 'rtl'
  background: string
}
```

前端 API：

```ts
getSettings(): Promise<ReaderSettings>
saveSettings(settings: ReaderSettings): Promise<void>
```

## 9. 数据库设计草案

### 9.1 repositories 表

```sql
CREATE TABLE repositories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  book_count INTEGER NOT NULL DEFAULT 0,
  last_scanned_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### 9.2 books 表

```sql
CREATE TABLE books (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  source_id TEXT,
  title TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  metadata_path TEXT,
  cover_path TEXT,
  description TEXT,
  authors_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  source_addtime TEXT,
  raw_metadata_json TEXT,
  chapter_count INTEGER NOT NULL DEFAULT 0,
  total_pages INTEGER NOT NULL DEFAULT 0,
  last_chapter_id TEXT,
  last_page INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);
```

### 9.3 chapters 表

```sql
CREATE TABLE chapters (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL,
  source_chapter_id TEXT,
  title TEXT NOT NULL,
  path TEXT NOT NULL,
  order_index INTEGER NOT NULL,
  page_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);
```

### 9.4 settings 表

```sql
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### 9.5 后续可能增加

```sql
CREATE TABLE tags (...);
CREATE TABLE book_tags (...);
CREATE TABLE authors (...);
CREATE TABLE book_authors (...);
CREATE TABLE reading_history (...);
```

第一版先把作者和标签以 JSON 字符串保存在 `books` 表，避免过早引入复杂关系表。等搜索、筛选、统计功能稳定后，再拆成正规关系表。

## 10. UI 设计原则

### 10.1 总体风格

InkReader 应该是工具型软件，不是营销网站。

界面方向：

- 安静
- 清晰
- 高对比但不刺眼
- 阅读区域优先
- 工具栏克制
- 信息密度适中

### 10.2 仓库页

仓库页应清楚展示用户已经添加了哪些本地仓库。

核心信息：

- 仓库名称
- 仓库路径
- 漫画数量
- 上次扫描时间
- 重新扫描操作

### 10.3 书架页

书架页应优先支持快速继续阅读。

核心信息：

- 封面
- 标题
- 作者
- 标签摘要
- 章节数量
- 阅读进度
- 最近更新时间

不需要在第一版加入复杂统计和装饰元素。

### 10.4 阅读器页

阅读器页应该最大化图片阅读空间。

原则：

- 图片居中
- 背景稳定
- 工具栏可隐藏
- 操作反馈直接
- 页码信息低干扰
- 章节信息清晰
- 不让按钮遮挡漫画内容

## 11. 错误处理

需要处理的错误场景：

- 用户取消选择仓库
- 仓库路径不存在
- 仓库中没有有效漫画目录
- 漫画目录缺少 `元数据.json`
- `元数据.json` 损坏或字段缺失
- 漫画目录缺少 `cover.jpg`
- 章节目录不存在
- 章节目录没有图片
- 图片读取失败
- 数据库初始化失败
- 数据库写入失败
- 原漫画路径被移动或删除
- 权限不足
- 压缩包损坏，后续阶段处理

错误提示应清楚说明问题，但不要把 Rust 错误栈直接展示给普通用户。

## 12. 测试策略

### 12.1 第一版测试重点

- 仓库扫描
- 漫画目录识别
- 元数据解析
- 章节识别
- 自然排序
- 图片格式过滤
- 阅读进度保存
- 仓库增删
- 书架刷新
- 页面翻页边界
- 跨章节翻页边界
- 数据库初始化

### 12.2 Rust 测试

优先给这些逻辑写单元测试：

- 自然排序
- 图片扩展名识别
- 元数据 JSON 解析
- 仓库扫描结果构建
- 章节排序
- 扫描结果过滤
- 数据库基础读写

### 12.3 前端测试

第一版不强制引入复杂前端测试。

可以优先手动验证：

- 页面切换
- 添加仓库
- 重新扫描仓库
- 书架显示
- 章节切换
- 键盘翻页
- 进度更新
- 空状态
- 错误状态

等功能稳定后，再考虑 Vitest。

## 13. 开发命令

建议统一命令：

```bash
pnpm install
pnpm dev
pnpm build
pnpm typecheck
pnpm lint
```

Rust 侧常用命令：

```bash
cargo check
cargo test
cargo build
```

Tauri 构建命令：

```bash
pnpm tauri dev
pnpm tauri build
```

也可以在 `package.json` 中封装为：

```json
{
  "scripts": {
    "dev": "tauri dev",
    "build": "tauri build",
    "typecheck": "vue-tsc --noEmit",
    "lint": "eslint .",
    "test": "vitest"
  }
}
```

## 14. 推荐开发顺序

建议严格按以下顺序推进：

1. 初始化 Tauri + Vue 项目
2. 建立页面和路由
3. 打通前端到 Rust command
4. 实现仓库目录选择
5. 实现仓库扫描
6. 实现元数据解析
7. 实现章节扫描和图片自然排序
8. 实现基础阅读器
9. 实现阅读进度保存
10. 实现 SQLite 数据库
11. 实现仓库页和书架页
12. 实现封面缓存
13. 实现图片预加载和内存控制
14. 实现双页和长条阅读模式
15. 实现 zip/cbz 支持
16. 实现 rar/cbr 支持
17. 实现打包发布

不要在前 10 步之前处理压缩包、主题、多语言、在线源等功能。

## 15. 第一版验收清单

第一版完成时，应满足：

- 应用可以正常启动
- 可以添加本地漫画仓库
- 可以扫描仓库内的漫画目录
- 可以读取 `元数据.json`
- 可以识别 `cover.jpg`
- 可以在书架看到漫画
- 书架能显示标题、作者、标签摘要和章节数
- 可以进入漫画阅读器
- 可以看到章节信息
- 图片顺序正确
- 章节顺序正确
- 可以上一页/下一页
- 可以跨章节翻页
- 可以用键盘翻页
- 可以显示当前章节页码和总页数
- 可以保存阅读进度
- 关闭应用后重新打开，仓库和书架仍存在
- 关闭应用后重新打开，可以继续上次阅读
- 原始漫画文件不会被修改
- 移除仓库不会删除用户文件

## 16. 风险点

### 16.1 Tauri 文件访问权限

Tauri 2 的权限配置需要认真处理。文件选择、文件读取、资源访问都可能涉及 capability 配置。

解决方式：

- 初期只通过文件选择器获得用户授权路径
- 后端集中处理文件访问
- 不在前端散落本地路径读取逻辑

### 16.2 图片路径暴露和加载

前端直接加载本地图片路径可能遇到权限和编码问题。

解决方式：

- 统一由 Rust 返回可加载的资源 URI
- 后续压缩包格式也走同一套页面读取接口

### 16.3 大图和内存

漫画图片可能很大，预加载过多会导致内存上涨。

解决方式：

- 第一版只加载当前页
- 第二阶段加入有限预加载
- 明确释放远离当前页的图片引用

### 16.4 元数据不稳定

不同来源的 `元数据.json` 字段可能缺失、类型不一致或编码异常。

解决方式：

- 元数据解析使用宽松结构
- 必填字段缺失时使用目录名兜底
- 原始 JSON 可保存在数据库，便于后续重新解析
- 单本漫画解析失败不应阻断整个仓库扫描

### 16.5 仓库规模

仓库可能包含大量漫画和图片，完整扫描可能耗时较长。

解决方式：

- 扫描过程后续支持进度反馈
- 第一版可以同步扫描，但代码结构要为异步和增量扫描留余地
- 重扫时尽量复用已有数据库记录

### 16.6 压缩包支持复杂度

zip/cbz 简单，rar/cbr 会复杂很多。

解决方式：

- 先支持 zip/cbz
- rar/cbr 后置
- 先调研库和平台兼容性，再决定实现方式

## 17. 里程碑

### Milestone 1：能启动

- Tauri + Vue 项目可运行
- 页面结构完成
- 前后端通信成功

### Milestone 2：能扫描仓库

- 选择仓库
- 扫描漫画目录
- 读取元数据
- 识别封面和章节

### Milestone 3：能阅读

- 书架展示漫画
- 选择漫画
- 选择章节
- 单页翻页
- 键盘翻页

### Milestone 4：能保存

- SQLite 接入
- 仓库保存
- 书架保存
- 阅读进度保存

### Milestone 5：像个阅读器

- 封面缓存
- 预加载
- 适应宽度/高度
- 全屏
- 更完整的阅读器 UI

### Milestone 6：支持压缩包

- zip/cbz 支持
- 后续 rar/cbr 支持

### Milestone 7：可发布

- Windows 安装包
- 应用图标
- 基础发布流程

## 18. 当前建议

当前最合适的第一步是初始化项目，并完成 Milestone 1。

第一轮实际开发只做：

- 项目初始化
- 页面框架
- 前后端 command 测试
- 基础样式

第二轮开始进入仓库扫描：

- 添加仓库目录选择
- 扫描仓库一级子目录
- 解析 `元数据.json`
- 识别 `cover.jpg`
- 识别章节目录和图片页

确认仓库扫描结果稳定后，再进入阅读器实现。
