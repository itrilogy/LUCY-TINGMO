# 品牌展示资产（docs/assets）

供 **GitHub README**、说明书页眉与软著图样引用。与运行时打包资源保持同源，修改图标时请同步：

| 本目录 | 工程运行时 / 源稿 |
| --- | --- |
| `tingmo-app-icon.svg` | `src/assets/tingmo-app-icon.svg` · `src-tauri/icons/*` |
| `luxi-lab.svg` | `src/assets/luxi-lab.svg`（关于页） |
| `luxi-lab-mark.svg` / `luxi-lab-lockup.svg` | `src/assets/` 同名文件 |

## 文件清单

| 文件 | 用途 |
| --- | --- |
| `tingmo-app-icon-256.png` | README 主展示（推荐） |
| `tingmo-app-icon-512.png` / `-1024.png` | 高清预览 / 登记图样 |
| `luxi-lab-logo.png` | 实验室 Logo 展示（480²） |
| `luxi-lab-lockup.png` | 横版字锁 |
| `luxi-lab-mark-200.png` | 几何备用标 |
| `*.svg` | 矢量源 |

## 再生示例

```bash
rsvg-convert -w 256 -h 256 src/assets/tingmo-app-icon.svg -o docs/assets/tingmo-app-icon-256.png
rsvg-convert -w 480 -h 480 src/assets/luxi-lab.svg -o docs/assets/luxi-lab-logo.png
```

色板：鹿溪绿 `#0D5E42` · 源启白 `#F5F7FA` · 进化蓝 `#00D2FF`。
