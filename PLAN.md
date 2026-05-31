# InkReader

InkReader 是一个基于 Tauri 2 + Vue 3 + TypeScript + Rust + SQLite 的本地漫画仓库阅读器。

## 当前实现范围

已实现第一版主链路：

- 添加本地漫画仓库
- 扫描仓库一级漫画目录
- 读取 `元数据.json`
- 识别 `cover.jpg`
- 识别章节目录
- 扫描章节图片页
- 图片自然排序
- 仓库和书架数据写入 SQLite
- 书架展示漫画封面、标题、作者、标签、章节数、页数
- 单页阅读器
- 章节切换
- 左右键和空格翻页
- 跨章节翻页
- 阅读进度保存
- 移除仓库记录，不删除原始文件

## 预期仓库结构

```text
漫画仓库/
  漫画 A/
    元数据.json
    cover.jpg
    第1话/
      0001.jpg
      0002.jpg
```

## 环境要求

运行 Tauri 需要这些工具：

- Node.js LTS
- pnpm
- Rust stable，包括 `rustc` 和 `cargo`

Windows 上如果运行 `pnpm tauri:dev` 出现 `cargo metadata ... program not found`，说明系统找不到 Cargo。安装 Rustup 后重新打开 PowerShell：

```powershell
winget install Rustlang.Rustup
cargo --version
rustc --version
```

也可以单独运行环境检查：

```bash
pnpm check:env
```

## 开发命令

```bash
pnpm install
pnpm tauri:dev
```

前端检查：

```bash
pnpm typecheck
```

Rust 检查：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## 打包

安装版构建：

```bash
pnpm tauri:build:installer
```

便携版构建：

```bash
pnpm tauri:build:portable
```

便携版会先构建 release 可执行文件，然后生成类似下面的目录和 zip：

```text
src-tauri/target/release/portable/InkReader-portable-0.1.0-windows-x64/
  InkReader.exe
  data/
  README-portable.txt
```

Windows 便携版依赖系统已安装 WebView2 Runtime。

## 数据位置

InkReader 的 SQLite 数据库、收藏、阅读进度和设置会存到软件自己的目录，不再默认写入 Windows AppData：

```text
<InkReader.exe 所在目录>/data/inkreader.sqlite3
```

因此：

- 便携版数据在解压目录的 `data/` 下。
- 安装版数据在安装目录的 `data/` 下。
- 如果希望数据不在 C 盘，请把便携版解压到非 C 盘，或安装时选择非 C 盘且当前用户可写入的目录。
- 首次启动新版本时，如果旧的 AppData 数据库存在且新位置还没有数据库，应用会复制旧数据库到新的 `data/` 目录；旧 AppData 数据库会保留作为备份。
- 程序目录必须可写，否则应用无法创建或打开数据库。

## 备注

当前会优先支持本地文件夹仓库。压缩包、缩略图缓存和打包发布的更多细节属于后续阶段。
