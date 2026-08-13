# 13 · 品牌 LOGO 资产与图标操作说明

> **文档性质**：听默 App / 任务栏 / 鹿溪实验室 LOGO 的归档位置、工程路径与再生步骤  
> **日期**：2026-08-08  
> **产品**：听默（Tingmo）  
> **Obsidian 归档**：`/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/`  
> **关联**：`12-听默品牌UI打包与开放问题归档.md`

---

## 1. 资产分两处保存

| 位置 | 作用 |
|------|------|
| **工程内** | 构建与运行实际引用的路径（改 UI / 重新打包用） |
| **Obsidian lab** | 设计归档、对外展示、备份（不参与编译） |

**原则**：工程以「会进包或会被代码 import」为准；Obsidian 存源稿与说明，避免 `src/assets` 堆草稿。

---

## 2. Obsidian 归档目录

```text
/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/
├── README.md
├── tingmo/          # 应用 / Dock 图标 SVG 与快照
├── luxi-lab/        # 鹿溪实验室 SVG
└── tray-png/        # 任务栏托盘 PNG（当前无 SVG）
```

打开：

```bash
open "/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO"
```

### 2.1 听默 App / Dock（`tingmo/`）

| 文件 | 说明 |
|------|------|
| **`tingmo-app-icon.svg`** | **主源稿**：圆角绿底 + 声纹 / 溪流 / 源启青点 |
| `tingmo-app-icon-1024.png` | 由 SVG 导出 1024，便于预览 |
| `icon-512-packaged.png` | 工程 `src-tauri/icons/icon.png` 快照 |
| `logo-packaged.png` | 工程 `src-tauri/icons/logo.png` 快照 |

### 2.2 鹿溪实验室（`luxi-lab/`）

| 文件 | 说明 |
|------|------|
| **`luxi-lab-original.svg`** | 关于页开发者区使用的原图（写实鹿标） |
| `luxi-lab-mark.svg` | 几何 Y+L 切分标（实验 / 备用） |
| `luxi-lab-lockup.svg` | 横版：标 +「鹿溪联合创新实验室」 |
| `luxi-lab-mark-512.png` | mark 预览 |

### 2.3 任务栏 / 托盘（`tray-png/`）

| 文件 | 状态 |
|------|------|
| `tray_idle.png` / `tray_idle_dark.png` | 空闲 |
| `tray_recording*.png` | 录音中 |
| `tray_transcribing*.png` | 转写中 |
| `tray_colored_*.png` | Linux 等彩色托盘 |

**说明**：当前工程任务栏为 **PNG**，没有独立 tray SVG；改托盘需改 PNG 或从 SVG 重新导出覆盖 `src-tauri/resources/`。

---

## 3. 工程内对应路径

| 用途 | 工程路径 | 是否进安装包 |
|------|----------|----------------|
| App 图标源 SVG | `src/assets/tingmo-app-icon.svg` | 否（仅源稿） |
| 打包图标集 | `src-tauri/icons/*`（icns/ico/png） | **是** |
| 托盘图标 | `src-tauri/resources/tray_*.png` 等 | **是** |
| 关于页鹿溪原图 | `src/assets/luxi-lab.svg` | **是**（被 import） |
| 鹿溪几何标 / 字锁 | `src/assets/luxi-lab-mark.svg`、`luxi-lab-lockup.svg` | 仅当代码 import 时 |

前端 Vite **只打包被 import 的** `src/assets` 文件；未引用的 SVG **不会**进 app 前端。

---

## 4. 色板

| 名称 | 色值 | 用途 |
|------|------|------|
| 鹿溪绿 | `#0D5E42` | 主色 / 底板 |
| 源启白 | `#F5F7FA` | 线条 / 浅底 |
| 进化蓝 | `#00D2FF` | 高光 / 源点 |

---

## 5. 操作流程

### 5.1 修改应用 / Dock 图标

1. 编辑源稿（二选一，保持同步）：
   - 工程：`src/assets/tingmo-app-icon.svg`
   - 归档：`Obsidian/.../tingmo/tingmo-app-icon.svg`
2. 导出 1024 PNG 并生成全套图标：

```bash
cd /Users/kwangwah/Project/Handy

# 需本机有 rsvg-convert（librsvg）与 bun
rsvg-convert -w 1024 -h 1024 src/assets/tingmo-app-icon.svg -o /tmp/tingmo-app-icon-1024.png
bunx --bun @tauri-apps/cli icon /tmp/tingmo-app-icon-1024.png -o src-tauri/icons
cp /tmp/tingmo-app-icon-1024.png src-tauri/icons/logo.png

# 同步归档（可选）
cp src/assets/tingmo-app-icon.svg \
  "/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/tingmo/"
cp /tmp/tingmo-app-icon-1024.png \
  "/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/tingmo/"
```

3. 重新打包安装后，Dock 才会更新：

```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build
```

4. 若 Dock 仍显示旧图：卸载旧 app、再装 dmg；或从程序坞移除图标后重新固定。

### 5.2 修改任务栏（托盘）图标

1. 准备对应尺寸的 PNG（通常模板 22–128px，透明底）。
2. 覆盖：

```text
src-tauri/resources/tray_idle.png
src-tauri/resources/tray_idle_dark.png
src-tauri/resources/tray_recording.png
src-tauri/resources/tray_recording_dark.png
src-tauri/resources/tray_transcribing.png
src-tauri/resources/tray_transcribing_dark.png
# Linux 彩色（可选）
src-tauri/resources/handy.png          # idle 彩色
src-tauri/resources/recording.png
src-tauri/resources/transcribing.png
```

3. 同步到 Obsidian：

```bash
cp src-tauri/resources/tray_*.png \
  "/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/tray-png/"
```

4. 重新 `tauri dev` 或 `tauri build` 验证。

### 5.3 修改关于页鹿溪 LOGO

- 文件：`src/assets/luxi-lab.svg`（组件 `LuxiLabLogo.tsx` 引用）
- 展示：圆角 `rounded-2xl` + 轻微描边（仅 UI，不改 SVG 本体）
- 改图后：

```bash
cp src/assets/luxi-lab.svg \
  "/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO/luxi-lab/luxi-lab-original.svg"
```

### 5.4 从 Obsidian 回灌到工程

```bash
ARCHIVE="/Users/kwangwah/Obsidian/departments/lab/听默-品牌资产-LOGO"
PROJ="/Users/kwangwah/Project/Handy"

cp "$ARCHIVE/tingmo/tingmo-app-icon.svg" "$PROJ/src/assets/"
cp "$ARCHIVE/luxi-lab/luxi-lab-original.svg" "$PROJ/src/assets/luxi-lab.svg"
cp "$ARCHIVE/luxi-lab/luxi-lab-mark.svg" "$PROJ/src/assets/"
cp "$ARCHIVE/luxi-lab/luxi-lab-lockup.svg" "$PROJ/src/assets/"
cp "$ARCHIVE/tray-png/tray_"*.png "$PROJ/src-tauri/resources/"
# 若用了 colored 命名，需手动对应 handy.png / recording.png / transcribing.png
```

---

## 6. 打包与资产检查清单

- [ ] 改 SVG 后已 `tauri icon` 更新 `src-tauri/icons`
- [ ] 托盘 PNG 已按亮/暗与状态覆盖全套
- [ ] 关于页 `luxi-lab.svg` 已确认
- [ ] 重要源稿已复制到 Obsidian `听默-品牌资产-LOGO`
- [ ] `tauri build` 安装后检查 Dock + 托盘 + 关于页

```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build
# 产物示例：
# src-tauri/target/release/bundle/macos/听默.app
# src-tauri/target/release/bundle/dmg/听默_0.9.4_aarch64.dmg
```

---

## 7. 常见问题

| 问题 | 说明 |
|------|------|
| 为什么任务栏没有 SVG？ | 托盘用模板 PNG；可从 SVG 导出后替换 resources |
| 未 import 的 asset 会进包吗？ | **不会**进前端 dist；`icons/` 与 `resources/` 会进包 |
| Dock 图标改了还是旧的？ | 需重打包安装；系统图标缓存有时需重启或重固定 |
| 开发态 Dock 不对？ | `tauri dev` 二进制名/图标常与正式 .app 不一致，以安装包为准 |

---

## 8. 修订记录

| 日期 | 摘要 |
|------|------|
| 2026-08-08 | 初版：Obsidian 归档路径 + 工程对照 + 再生/回灌/打包步骤 |
