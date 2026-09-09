<div align="center">
  <img src="docs/assets/tingmo-app-icon-256.png" width="64" height="64" alt="听默 · Tingmo 产品标" />
  &nbsp;&nbsp;
  <img src="docs/assets/luxi-lab-logo.png" width="64" height="64" alt="鹿溪联合创新实验室 LUXI LAB" />
</div>

<h1 align="center">听默 · Tingmo</h1>

<p align="center">
  <strong>谛听万籁，默化成文</strong><br/>
  <em>Listening in silence, transcribing without friction.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Matrix-知行%C2%B7三动-0D5E42" alt="matrix" />
  <img src="https://img.shields.io/badge/Product-听默%20Tingmo-0D5E42" alt="product" />
  <img src="https://img.shields.io/badge/Lab-鹿溪联合创新实验室-047538" alt="lab" />
  <img src="https://img.shields.io/badge/Based%20on-Handy-6366f1" alt="upstream" />
  <img src="https://img.shields.io/badge/Stack-Tauri%202%20%7C%20React%20%7C%20Rust-blue" alt="stack" />
  <img src="https://img.shields.io/badge/Version-v0.9.4-f1c40f" alt="version" />
</p>

<p align="center">
  <b>鹿溪联合创新实验室</b>（LUXI Joint Innovation Lab）出品<br/>
  仓库：<a href="https://github.com/itrilogy/LUCY-TINGMO">itrilogy/LUCY-TINGMO</a>
</p>

---

一款**本地模型驱动**的桌面语音成文应用。

| 显示名 | 包 / 二进制 | 版本 | 定位 |
| :---: | :---: | :---: | :---: |
| **听默 · Tingmo** | `Tingmo.app` · `tingmo` | **0.9.4** | 实验室实验分支 |

---

## 🎨 品牌标识

| 标识 | 预览 | 说明 | 源文件 |
| :---: | :---: | --- | --- |
| **产品方标** | <img src="docs/assets/tingmo-app-icon-256.png" width="32" alt="听默" /> | 鹿溪绿圆角底板 + 声纹弧 / 溪流意象 / 源启点；用于 Dock、安装包、侧栏 | `docs/assets/tingmo-app-icon.svg` · `src/assets/tingmo-app-icon.svg` |
| **实验室主标** | <img src="docs/assets/luxi-lab-logo.png" width="32" alt="鹿溪实验室" /> | 鹿溪写实鹿标 + 实验室视觉；用于关于页、文档页眉 | `docs/assets/luxi-lab.svg` · `src/assets/luxi-lab.svg` |
| 实验室字锁（横版） | <img src="docs/assets/luxi-lab-lockup.png" height="24" alt="鹿溪字锁" /> | 标 +「鹿溪联合创新实验室」 | `docs/assets/luxi-lab-lockup.svg` |

**色板（LUXI CI）**

| Token | 色值 | 用途 |
| --- | --- | --- |
| 鹿溪绿 | `#0D5E42` | 主色 / 图标底板 / 强调 |
| 源启白 | `#F5F7FA` | 浅色背景 / 浅色文案底 |
| 进化蓝 | `#00D2FF` | 高光 / 波形点缀 |
| 标题金 | `#F1C40F` | 落点 / 显著信号 |

更多资产与再生步骤：[`代码研究/13-品牌LOGO资产与图标操作说明.md`](代码研究/13-品牌LOGO资产与图标操作说明.md) · [`docs/assets/`](docs/assets/)

---

## 软件定位

由 **鹿溪联合创新实验室** 在开源 [Handy](https://github.com/cjpais/Handy)（MIT）基础上二次开发与品牌化的 **本地语音成文（ASR）桌面软件**。

| 项 | 内容 |
| --- | --- |
| 软件全称（建议软著登记用） | **听默本地语音成文软件** |
| 软件简称 | **听默**（Tingmo） |
| 版本号 | **V0.9.4** |
| 技术形态 | 跨平台桌面应用（Tauri 2 + React/TypeScript + Rust） |
| 运行方式 | 本机离线推理为主；可选外部 LLM 后处理 |
| 数据目录 | 仍为 `com.pais.handy`（有意保留，兼容既有模型与历史） |
| 分发 | 开源仓库公开；非 Handy 官方发行 |

> **声明：** 本软件 **不是** Handy 官方产品，不代表上游项目立场。Handy 名称与品牌资产归其权利人所有；本软件使用「听默 / 鹿溪」自有品牌标识。

---

## 它做什么

1. **快捷键**开始/结束录音（支持按住说话）
2. **本地 ASR** 转写（Whisper 系、Parakeet、SenseVoice、Qwen3-ASR 等，视已装模型）
3. **Overlay** 展示结果；可复制；可按策略粘贴到前台应用
4. **历史**折叠查看、重命名、单条重新转写与后处理

全程默认本地处理：原始音频不上传云端（若启用云端 LLM 后处理除外）。

### 相对上游的 lab 差异（摘要）

| 维度 | 现状 |
| --- | --- |
| 品牌 / UI | 听默 + LUXI 配色；About 鹿溪标识；主窗约 880×640 |
| 更新检查 | 默认关闭；设置页/托盘更新入口置灰 |
| 粘贴 | 前台粘贴竞态修复；可配置粘贴策略 |
| Overlay | 关闭 / 药丸 / 直播等形态；展开尺寸统一；分段换行 |
| 长音频 | 自动分片；短窗引擎按能力切段 |
| 历史 | 标题、折叠、重命名、重试转写 |
| 文档 | 工程研究笔记 + **软著配套说明书**（见下） |

---

## 说明书（版权 / 软著配套）

面向软件著作权登记与交付审阅的正式文档：

| 文档 | 用途 |
| --- | --- |
| [**软件设计说明书**](docs/软件设计说明书.md) | 架构、模块、数据流、接口与安全设计 |
| [**软件功能说明书**](docs/软件功能说明书.md) | 功能清单、模块说明、业务规则 |
| [**软件使用说明书**](docs/软件使用说明书.md) | 安装、权限、操作流程、设置与排障 |
| [**版权与知识产权声明**](docs/版权与知识产权声明.md) | 权利归属、开源依赖、品牌与使用边界 |
| [文档索引](docs/README.md) | `docs/` 目录总览 |

工程调研与台账（非软著正文，供研发）：[`代码研究/`](代码研究/)

---

## 🔧 本地开发

**环境：** [Rust](https://rustup.rs/)（stable）· [Bun](https://bun.sh/)

```bash
bun install
bun run tauri dev
# macOS cmake 策略报错时：
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

bun run tauri build   # 产出 听默.app / dmg 等
```

首次开发需 VAD 模型：

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx \
  https://blob.handy.computer/silero_vad_v4.onnx
```

平台依赖见 [BUILD.md](BUILD.md)。约定见 [AGENTS.md](AGENTS.md)。

```bash
bun run lint
bun run format
```

**调试：** macOS `Cmd+Shift+D` · Windows/Linux `Ctrl+Shift+D`

### CLI（`tingmo`）

```bash
tingmo --toggle-transcription
tingmo --toggle-post-process
tingmo --cancel
tingmo --start-hidden
tingmo --no-tray
tingmo --debug
```

macOS：

```bash
/Applications/听默.app/Contents/MacOS/tingmo --toggle-transcription
```

---

## 🏗 架构（简）

| 层 | 技术 |
| --- | --- |
| 壳 / 系统集成 | Tauri 2 · 托盘 · 全局快捷键 · 剪贴板 / 粘贴 |
| 前端 | React · TypeScript · Tailwind · i18n · Zustand |
| 后端 | Rust Managers（Audio / Model / Transcription / History） |
| 音频 | cpal · Silero VAD · 重采样 |
| 推理 | `transcribe-cpp` · `transcribe-rs` 等本地引擎 |

**流程：** 快捷键 → 录音 → VAD → 分片/转写 → 标点/后处理（可选）→ Overlay / 剪贴板 / 历史。

---

## 数据路径

标识 **`com.pais.handy`**（与上游布局兼容）：

| 平台 | 应用数据 |
| --- | --- |
| macOS | `~/Library/Application Support/com.pais.handy/` |
| Windows | `%APPDATA%\com.pais.handy\` |
| Linux | `~/.config/com.pais.handy/` |

含 `settings_store.json`、`history.db`、`models/`、`recordings/`。  
部分模型可能位于 `~/.cache/huggingface/`。

---

## 许可证与致谢

- **本软件代码授权：** MIT — 见 [LICENSE](LICENSE)（以仓库文件为准）
- **上游：** [Handy](https://github.com/cjpais/Handy) · [handy.computer](https://handy.computer)
- **品牌：** 听默 / 鹿溪标识归 **鹿溪联合创新实验室** 使用与管理；Handy 品牌不得冒用
- **技术致谢：** Whisper · ggml / transcribe 生态 · Silero VAD · Tauri 等

```
基于开源 Handy 的鹿溪联合创新实验室实验构建
林深见鹿，源启清溪 — 鹿溪联合创新实验室
```

---

<div align="center">
  <img src="docs/assets/luxi-lab-logo.png" width="48" height="48" alt="LUXI LAB" />
  <p><strong>听默 · Tingmo</strong> · 谛听万籁，默化成文</p>
  <p>© 鹿溪联合创新实验室 · LUXI Joint Innovation Lab</p>
  <p><em>林深见鹿，源启清溪 · Deep Insights, Evolutionary Origin.</em></p>
</div>
