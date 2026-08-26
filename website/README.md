# Mio Music Website

这是 Mio Music 的独立静态官网，不接入现有 Tauri/Vue 桌面应用入口，也不改变桌面端架构。页面面向产品介绍与软件下载，按 Rust 原生音频引擎、均衡器、Apple Music 风格歌词、插件音源、专辑封面主题与下载/备份/DLNA 能力编写，会根据访问者系统自动推荐 macOS、Windows 或 Linux 版本。

## 本地预览

```bash
cd website
python3 -m http.server 4173
```

打开 `http://localhost:4173` 即可预览。

## 部署

官网为纯静态文件，可托管到任意静态站点服务（GitHub Pages、Cloudflare Pages、Netlify、Nginx 等）。把 `website/` 目录整体作为站点根发布即可，所有资源引用均为相对路径。

页面结构保留在 `index.html`，站点样式、翻译数据、交互与下载逻辑分别位于
`assets/site.css`、`assets/site-data.js`、`assets/site-ui.js`、
`assets/site-downloads.js` 与 `assets/site-init.js`。

发布地址沿用页面中的 canonical URL：

```text
https://mio888888.github.io/Mio-Music/
```

> 本仓库未附带自动发布 workflow。如需发布到 GitHub Pages，请自行在仓库
> `Settings -> Pages -> Build and deployment -> Source` 选择 `GitHub Actions`
> 并添加一个把 `website/` 作为静态站点上传的 workflow。`Deploy from a
> branch` 仅支持分支根目录或 `/docs`，不能直接发布 `website/` 目录。

## 下载链接

页面会自动识别访问者系统，并优先从 GitHub Releases 读取最新版本：

```text
https://github.com/Mio888888/Mio-Music/releases/latest
```

读取顺序：

1. GitHub API（`api.github.com/repos/Mio888888/Mio-Music/releases/latest`，返回 CORS 头）
2. 仓库默认分支的 `latest.json` 镜像（jsDelivr / raw.githubusercontent，若已提交）

读取成功后，页面会同步更新当前版本号、主下载按钮和 macOS / Windows / Linux
下载卡片。读取失败时会回退到 GitHub Releases 最新发布页，并保留页面内的保底版本
（`v0.2.9`）显示。

## 截图替换

截图轮播的图片位于 `assets/screenshots/`，当前用 `home.png`（三端拼接预览图）
占位。要换成真实功能截图，把新图片放入该目录，并更新 `index.html` 中对应
`<img src="./assets/screenshots/...">` 的路径即可。轮播共 6 个图位：播放主界面、
Apple Music 风格动态歌词、10 段均衡器与音效、频谱可视化、下载管理、AI 助手。

页面中的产品版本、功能范围和合规说明应随 `README.md`、`README_EN.md` 一起维护，
避免官网与实际桌面端能力脱节。
