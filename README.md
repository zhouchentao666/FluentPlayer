<p align="center">
  <img src="public/icon.png" width="120" alt="Mio Music Logo" />
</p>

<h1 align="center">Mio Music</h1>

<p align="center">基于 Tauri 2 + Vue 3 的跨平台桌面音乐播放器</p>

<p align="center">
  <a href="README_EN.md">English</a>
</p>

---

## 简介

Mio Music 是一款跨平台桌面音乐播放器，基于 [CeruMusic](https://github.com/timeshiftsauce/CeruMusic) 重构，采用 **Tauri 2 (Rust)** 作为后端、**Vue 3 + TypeScript** 作为前端，通过插件架构访问多种音乐源。

> 本项目不直接托管或传输任何音乐数据，所有内容均通过插件从第三方音乐源获取。用户通过插件获取的所有数据，其合法性由插件提供者及用户自行负责。

<p align="center">
  <img src="public/home.png" width="100%" alt="Mio Music 三端演示" />
</p>

## 功能特性

### 音乐播放

- Rust 原生音频引擎（Rodio + cpal），支持 AAC、MP3、FLAC、WAV、OGG 等格式
- 10 段参数均衡器，内置 8 种预设（流行、摇滚、爵士、古典等）
- 低音增强、环绕声模拟（小/中/大空间）、立体声平衡控制
- 无缝播放（Gapless）与可配置交叉淡入淡出（Crossfade，默认 3000ms）
- A/B 双音频输出设备对比测试
- FFT 频谱实时可视化
- 系统媒体键集成

### 歌词系统

- 多格式解析：LRC、YRC（网易云逐字）、QRC（QQ 音乐）、TTML（Apple Music 风格）、LRC-A2
- 歌词翻译合并显示
- Apple Music 风格动态歌词（@applemusic-like-lyrics）
- 独立桌面歌词窗口（透明悬浮）
- 歌词字体自定义

### 音乐源

- Subsonic 服务器支持（自托管音乐）
- 插件系统：支持 `澜音插件` 和 `洛雪插件` 两种类型，可动态安装、配置、测试

### 界面与交互

- 专辑封面主色动态主题（自动提取主色调，适配明暗模式，生成 50+ CSS 变量）
- PixiJS 着色器背景渲染与音频响应式频谱
- Three.js 粒子动画欢迎页
- 虚拟滚动长列表（@tanstack/vue-virtual）
- 自定义标题栏与窗口控件
- 右键上下文菜单
- AI 助手对话界面
- 全局快捷键支持

### 数据管理

- 本地音乐库扫描与音频标签编辑（lofty）
- 下载管理器：暂停/恢复/取消/重试/批量操作，实时进度追踪
- S3 兼容云存储备份与恢复（AES-GCM 加密 + 密码保护）
- DLNA 投屏：设备发现、播放/暂停/音量/进度控制
- SQLite 本地数据库

### 其他

- 国际化：中文 / English（vue-i18n，11 个翻译命名空间）
- Logto OAuth/OIDC 用户认证
- 自动更新（GitHub Releases + Ed25519 签名验证）
- 路由预加载与组件懒加载
- 专辑封面 CORS 代理（Rust 后端 imgproxy）

## 技术栈

| 层级      | 技术                        |
| --------- | --------------------------- |
| 前端框架  | Vue 3 + TypeScript          |
| 构建工具  | Vite 6                      |
| 状态管理  | Pinia 3（持久化）           |
| UI 组件库 | TDesign Vue Next / Naive UI |
| 图形渲染  | Three.js / PixiJS           |
| 歌词组件  | @applemusic-like-lyrics     |
| 后端框架  | Tauri 2 (Rust)              |
| 音频引擎  | Rodio + cpal + Symphonia    |
| 音频处理  | rustfft (FFT) + biquad (EQ) |
| 数据库    | SQLite (rusqlite)           |
| 样式      | SCSS                        |

## 开发

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Tauri 2 环境依赖](https://v2.tauri.app/start/prerequisites/)

### 安装与运行

```bash
# 安装依赖
npm install

# 启动开发模式（Vite + Tauri）
npm run tauri dev

# 构建发布
npm run tauri build
```

### 其他命令

```bash
npm run dev        # 仅启动 Vite 开发服务器（端口 1420）
npm run build      # 类型检查 + Vite 构建
npm run preview    # 预览生产构建
```

### 推荐 IDE

[VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 项目结构

```
├── src/                          # Vue 前端
│   ├── main.ts                   # 应用入口（Pinia、i18n、Router、Logto 初始化）
│   ├── router/                   # 路由配置（hash 模式，含预加载）
│   ├── store/                    # Pinia 状态管理（16 个 store）
│   ├── views/                    # 页面组件（欢迎页、首页、音乐、设置、下载等）
│   ├── components/               # 通用组件（40+，含播放器、设置面板、AI 等）
│   ├── composables/              # Vue 组合式函数（动态主题、背景渲染等）
│   ├── locales/                  # i18n 翻译文件（zh-CN / en-US）
│   ├── bridge/                   # Electron → Tauri IPC 适配层
│   ├── services/                 # 服务层（musicSdk 等）
│   ├── utils/                    # 工具函数（下载、质量、代理、插件运行器等）
│   ├── types/                    # TypeScript 类型定义
│   └── assets/                   # 静态资源（CSS、字体、图标）
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── player/               # 音频引擎（播放、音效、频谱、媒体控制、缓存）
│   │   ├── music_sdk/            # 音乐源实现（kw/kg/wy/tx/mg/bd/xm + Subsonic）
│   │   ├── plugin/               # 插件管理器与运行引擎
│   │   ├── download/             # 下载管理（暂停/恢复）
│   │   ├── local_music/          # 本地音乐扫描与封面缓存
│   │   ├── audio_device/         # macOS CoreAudio 设备管理
│   │   ├── audio_capture/        # 系统音频捕获
│   │   ├── commands/             # Tauri 命令处理（配置、快捷键、S3 等）
│   │   └── db/                   # SQLite 数据库（音乐库、播放列表）
│   └── Cargo.toml
├── public/                       # 公共静态资源
├── scripts/                      # 构建脚本
└── package.json
```

## 致谢

- [CeruMusic](https://github.com/timeshiftsauce/CeruMusic) — 原始项目，感谢原作者 **时迁酱** 的开源贡献
- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [TDesign](https://tdesign.tencent.com/) — UI 组件库
- [Apple Music-like Lyrics](https://github.com/Steve-xmh/amll) — 歌词组件
- [LINUX DO](https://linux.do) — 社区支持
- [AutoEq](https://github.com/jaakkopasanen/AutoEq) — 自动均衡器预设数据

## 支持现项目作者

Mio Music 的持续维护、跨平台适配和问题修复由现项目作者投入。如果这个项目对你有帮助，欢迎通过下面的赞赏码支持后续开发。

<p align="center">
  <img src="src/assets/images/current-author-sponsor-qr.jpg" width="240" alt="现项目作者赞赏码" />
</p>

> 所有赞赏均为自愿支持，将直接用于现项目作者的后续开发与维护。

## 声明

本项目仅供学习交流使用，不直接获取、存储、传输任何音乐数据或版权内容，仅提供插件运行框架。禁止用于任何商业运营或侵犯第三方权益的场景。

## 许可证

本项目遵循原项目 CeruMusic 的开源协议。
