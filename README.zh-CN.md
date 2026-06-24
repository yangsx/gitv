# gitv

[**English version**](README.md)

一款现代化的跨平台 Git 仓库可视化工具。GPU 加速的提交图、流式数据加载和持久化缓存——基于 Rust + Tauri 构建。

可以把它看作是 gitk 的替代品：支持 wgpu 渲染、存储节点以图节点形式展示、亚 100 毫秒搜索，以及缓存仓库的即时重开。

## 特性

- **GPU 加速的提交图** — wgpu 渲染器（Vulkan/Metal/DX12），支持 Canvas 2D 回退；虚拟化视口，10 万+ 提交下保持 60 FPS
- **配色模式** — 按分支或按作者着色；色盲安全调色板（绿色盲、红色盲、蓝色盲、高对比度）
- **连线样式** — 实线/虚线/点线作为非颜色分支标识
- **存储浏览** — 存储节点作为独立图节点显示，带有分支边线，而非 gitk 的双节点双差异显示
- **差异查看器** — 统一视图或并排视图；普通/词差异/仅统计模式；空白符修饰符（忽略空白变更、忽略所有空白、忽略空行）；滚动同步文件列表
- **双提交比较** — Ctrl+点击或右键点击选择任意两个提交，显示合并差异
- **补丁文本搜索** — 使用正则表达式搜索提交差异，匹配行在差异查看器中高亮显示
- **提交搜索** — RoaringBitmap 倒排索引，10 万+ 提交仓库上实现亚 100ms 的提交消息和作者搜索
- **文件树浏览器** — 浏览任意提交下的仓库内容；查看文件内容、追溯和文件历史（支持 `--follow` 重命名追踪）
- **引用日志** — 浏览 reflog 条目，导航到任意提交（包括 rebase 或 reset 后的悬挂提交）
- **命令面板** — Ctrl+P 模糊搜索所有操作、最近仓库和导航
- **持久化缓存** — 磁盘缓存（postcard 序列化）实现已访问仓库的即时重开；引用变更时增量更新
- **键盘导航** — 完整的键盘控制：方向键、J/K、Page Up/Down、Home/End、作者跳转、分支切换
- **多实例** — 每次启动打开独立窗口；无标签页复杂度，无共享状态
- **首选项** — 持久化 JSON 配置于 `$XDG_CONFIG_HOME/gitv/preferences.json`，带防抖自动保存；主题（深色/浅色/自动）、字体大小、图/差异默认设置、渲染器选择
- **国际化** — 英文、简体中文、德文（社区贡献；在 `locales/` 中添加 JSON 文件即可增加语言）
- **命令行** — `gitv /仓库路径`、`gitv /仓库1 /仓库2`
- **调试叠加层** — F12 切换实时 FPS、内存、图统计信息、IPC 时序和加载阶段时序

## 前提条件

- **Rust** — 最新稳定版（edition 2024）
- **Node.js** — 20+ 和 npm
- **Linux** — GTK 3+ 开发库（`libgtk-3-dev`、`libwebkit2gtk-4.1-dev`、`libxdo-dev` 等——参见 [Tauri 文档](https://v2.tauri.app/start/prerequisites/)）
- **GPU** — Vulkan（Linux/Windows）、Metal（macOS）或 DirectX 12（Windows）。若 wgpu 无法初始化，自动回退到 Canvas 2D

## 快速开始

```bash
# 克隆
git clone https://github.com/yangsx/gitv && cd gitv

# 安装前端依赖
cd frontend && npm install && cd ..

# 开发模式运行
cargo tauri dev

# 生产构建
cargo tauri build
```

打包后的安装包位于 `target/release/bundle/` 目录下。

## 命令行用法

```bash
# 打开仓库
gitv /path/to/repo

# 打开多个仓库（各在独立窗口中）
gitv /repo1 /repo2

# 设置日志级别（debug、trace）
gitv /path/to/repo --log-level=debug
```

## 键盘快捷键

| 快捷键 | 操作 |
|--------|------|
| **文件** | |
| `Ctrl+O` | 打开仓库 |
| `Ctrl+Shift+O` | 在新窗口中打开仓库 |
| `Ctrl+W` | 关闭仓库（返回欢迎页） |
| `Ctrl+Q` | 退出应用 |
| `Ctrl+R` | 刷新 |
| **导航** | |
| `↓` / `J` | 下一个提交 |
| `↑` / `K` | 上一个提交 |
| `PageDown` | 下一页 |
| `PageUp` | 上一页 |
| `Home` | 第一个提交 |
| `End` | 最后一个提交 |
| `Alt+N` | 同一作者的下一个提交 |
| `Alt+P` | 同一作者的上一个提交 |
| **图** | |
| `Ctrl+Shift+M` | 切换隐藏合并提交 |
| `Ctrl+Shift+A` | 切换按作者着色 |
| `Ctrl+Shift+G` | 切换图方向 |
| **视图** | |
| `Ctrl+M` | 切换全屏 |
| `Ctrl+,` | 首选项 |
| `F12` / `Ctrl+Shift+D` | 调试叠加层 |
| **分支** | |
| `Alt+B` | 下一个分支（聚焦） |
| `Alt+Shift+B` | 上一个分支（聚焦） |
| **帮助** | |
| `Ctrl+P` | 命令面板 |
| `F1` / `Ctrl+/` | 键盘快捷键帮助 |
| `Escape` | 清除选择 / 关闭弹窗 / 退出全屏 |
| `Ctrl+Click` / `Cmd+Click` | 选择第二个提交进行比较 |

## 为什么选择 gitv 而非 gitk？

| 特性 | gitv | gitk |
|------|------|------|
| 渲染 | GPU 加速（wgpu + Canvas 2D 回退） | Tk 画布（CPU） |
| 存储显示 | 单图节点带分支边 + 合并差异 | 双节点双差异显示 |
| 搜索 | RoaringBitmap 索引（10 万提交亚 100ms） | 线性扫描 |
| 差异模式 | 普通、词差异、仅统计；空白符修饰符 | 仅普通 |
| 打开方式 | 独立启动器 + 仓库选择器 | 需在仓库目录内运行 |
| 多仓库 | 多实例，各在独立窗口中 | 单窗口标签页 |
| 缓存 | 持久化磁盘缓存，即时重开 | 无 |
| 配色模式 | 按分支、按作者、色盲安全调色板 | 仅按分支 |
| 引用日志 | 独立侧边栏面板，条目导航 | 不直接支持 |
| 首选项持久化 | 自动保存 JSON 配置 | 仅会话内有效 |

## 架构

```
gitv/
├── src-tauri/                  # Rust 后端（Tauri 2.0）
│   ├── src/
│   │   ├── commands/           # IPC 命令
│   │   └── lib.rs              # 应用设置、状态、命令注册
│   └── Cargo.toml
├── crates/
│   ├── gitv-git-core/          # 纯 Rust Git 逻辑（无 Tauri 依赖）— 99 个测试
│   │   └── src/
│   │       ├── repository.rs   # 基于 gix 的仓库抽象
│   │       ├── graph/          # 布局计算器、存储插入
│   │       ├── search/         # RoaringBitmap 搜索引擎
│   │       ├── stream/         # 流式提交迭代器
│   │       ├── cache/          # 持久化磁盘缓存
│   │       └── models.rs       # 核心类型（Oid、CommitInfo、Diff 等）
│   └── gitv-wgpu-renderer/     # 离屏 wgpu 渲染器（WGSL 着色器）
│       └── src/
│           ├── lib.rs          # WgpuRenderer（初始化、渲染、回读）
│           └── vertex.rs       # 顶点类型（NodeInstance、EdgeVertex）
├── frontend/                   # Svelte 5 + TypeScript
│   ├── src/
│   │   ├── routes/             # +layout.svelte、+page.svelte
│   │   └── lib/
│   │       ├── components/     # 20+ 个组件（扁平结构，无深层嵌套）
│   │       ├── stores/         # 8 个状态存储（仓库、首选项、布局等）
│   │       ├── graph/          # graph-math.ts、edge-interaction.ts
│   │       ├── locales/        # en.json、zh-CN.json、de.json
│   │       └── utils/          # a11y.ts、markdown.ts、format-date.ts
│   └── package.json
├── design.md                   # 架构文档（3196 行）
├── requirements.md             # 70 项需求
└── AGENTS.md                   # AI 代理约定与工具链
```

### 关键设计决策

- **解耦的 Git 核心**：`gitv-git-core` 是独立的 crate，提供基于 trait 的模拟接口——可独立测试，无 Tauri 依赖
- **Oid**：20 字节二进制 newtype（`[u8; 20]`），而非字符串——节省 3 倍内存，哈希更快
- **二进制 IPC**：postcard 序列化传输提交批次（体积小 3-5 倍，速度快 5-10 倍于 JSON）
- **虚拟滚动**：仅渲染可见提交；图画布和提交列表共享同步滚动容器
- **双渲染器**：wgpu GPU 管线 + Canvas 2D 回退，用户可选

## 故障排除

| 问题 | 可能原因 | 解决方法 |
|------|----------|----------|
| 窗口显示但图为空白 | wgpu 初始化失败，回退到 Canvas 2D | 检查终端中的 GPU 错误；尝试 `--log-level=debug` |
| 大型仓库首次打开缓慢 | 无缓存——需要完整遍历 | 正常现象；再次打开将 < 200ms |
| 有效路径提示"不是 Git 仓库" | 位于仓库子目录？ | gitv 会自动发现根目录——请尝试仓库根路径 |
| GPU 加速不可用 | Linux/WSL 缺少 Vulkan/DX12 驱动 | 安装 GPU 驱动，或在首选项中使用 `--renderer=canvas2d` |
| 缓存过期或错误 | 远程推送了新提交 | 点击刷新按钮（Ctrl+R） |
| 键盘快捷键无效 | 焦点在输入框中 | Ctrl 快捷键仍可用；纯字母快捷键（J、K）需要焦点不在输入框内 |

## 项目文档

- [`design.md`](design.md) — 完整架构文档：组件层次结构、数据模型、CLI、键盘快捷键、ADR、测试策略、可访问性
- [`requirements.md`](requirements.md) — 70 项需求及验收标准
- [`AGENTS.md`](AGENTS.md) — AI 代理约定、代码质量门禁、工具链

## 开发

### 一次性设置

```bash
# Rust 工具链（如尚未安装）
rustup default stable

# 前端依赖
cd frontend && npm install && cd ..

# Linux：系统依赖（请根据你的发行版参考 Tauri 前提条件）
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev \
  libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev
```

### 开发工作流

以热重载开发模式运行应用：

```bash
cargo tauri dev
```

这会启动前端 Vite 开发服务器（文件变更时 HMR）并在首次启动时编译 Rust 后端。Tauri 窗口会自动打开。

**Tauri DevTools**：右键点击窗口 → 检查元素，打开 WebView 开发者工具（控制台、网络、元素）。

### 项目布局

参见上方[架构](#架构)部分。关键目录：

| 目录 | 用途 |
|------|------|
| `src-tauri/` | Rust 后端（Tauri 命令） |
| `crates/gitv-git-core/` | 纯 Rust Git 逻辑（无 Tauri 依赖） |
| `crates/gitv-wgpu-renderer/` | 离屏 wgpu GPU 渲染器 |
| `frontend/src/lib/components/` | Svelte 5 组件（约 20 个，扁平结构） |
| `frontend/src/lib/stores/` | Svelte 状态存储（8 个文件） |
| `frontend/src/lib/locales/` | 国际化 JSON 文件 |
| `.github/workflows/` | CI 工作流 |

### 测试

```bash
# 运行所有 Rust 测试（推荐使用 nextest）
cargo nextest run --workspace

# 运行特定 crate
cargo nextest run -p gitv-git-core

# 运行特定测试
cargo nextest run -p gitv-git-core --test graph_tests

# 使用 cargo test（备选）
cargo test --workspace
```

`gitv-git-core` 中有 99 个测试——无 Tauri 依赖，完全可模拟。

### 基准测试

```bash
# 运行所有 criterion 基准测试
cargo bench --manifest-path crates/gitv-git-core/Cargo.toml

# 运行特定基准测试
cargo bench --bench graph_bench
```

CI 会将 PR 结果与 main 分支基线进行比较，超过 10% 回退则判定失败。

### 性能目标

CI 通过 `scripts/check_bench_targets.py` 检查基准测试结果是否满足绝对预算（本地运行：`python3 scripts/check_bench_targets.py target/criterion`）。Intel Core i7-1165G7 上的当前测量值：

| 状态 | 基准测试 | 测量值 | 预算 | 对比预算 |
|------|----------|--------|------|----------|
| ✅ PASS | 搜索（文本，10 万提交）< 100 ms | 5.796 ms | 100.000 ms | 低于预算 94.2% |
| ✅ PASS | 搜索（正则，10 万提交）< 100 ms | 13.825 ms | 100.000 ms | 低于预算 86.2% |
| ✅ PASS | 搜索（作者，10 万提交）< 100 ms | 933.606 µs | 100.000 ms | 低于预算 99.1% |
| ✅ PASS | 搜索索引构建（10 万提交）< 5 s | 147.937 ms | 5.000 s | 低于预算 97.0% |
| ✅ PASS | 图布局线性（1 万提交）< 2 s | 19.146 ms | 2.000 s | 低于预算 99.0% |
| ✅ PASS | 图布局分支（1 万提交）< 2 s | 19.949 ms | 2.000 s | 低于预算 99.0% |

### 覆盖率

```bash
cargo tarpaulin --manifest-path crates/gitv-git-core/Cargo.toml --out Html
# 目标：>= 80% 行覆盖率
```

### 调试

```bash
# 启用详细日志
gitv /path/to/repo --log-level=trace
```

- **F12** 切换调试叠加层（FPS、内存、图统计信息、IPC 时序）
- 日志写入应用数据目录中的滚动文件
- 恐慌时自动捕获回溯——崩溃日志保留以供诊断

### 添加语言（国际化）

1. 创建 `frontend/src/lib/locales/xx.json`（小写文件名，如 `ja.json`）
2. 根据 `en.json` 翻译字符串
3. 语言文件会被 `import.meta.glob` 自动发现——无需注册

现有语言：英语（`en.json`）、简体中文（`zh-CN.json`）、德语（`de.json`）。

### CI 清单

推送前，请在本地运行所有质量门禁：

```bash
# Rust
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace
cargo doc --workspace --no-deps

# 前端
cd frontend && npm run lint && npm run check && npm run build
```

同样的检查会在 CI 中运行（`.github/workflows/ci.yml`）。额外的工作流还会运行覆盖率检查、安全审计（每周）和 push/PR 时的基准测试。

### 贡献变更

1. 从 `main` 创建功能分支
2. 进行变更，确保所有 CI 门禁在本地通过
3. 为新功能编写或更新测试
4. 发起拉取请求
5. AI 生成的提交需要包含 `Co-Authored-By` 尾注

## 许可证

MIT
