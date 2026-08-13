# Obsidian ↔ Handy 标识物同步（轻量）

本期不做双向文件监听守护进程，采用**显式导入/导出**（可配合 vault 同步工具）。

## Handy → Obsidian

1. 设置 → 高级 → 标识物词库  
2. **导出 Markdown（复制）** 或 **写入 asr_markers.md**  
   - 复制：粘贴到 vault 中任意笔记  
   - 写入：`~/Library/Application Support/com.pais.handy/asr_markers.md`（可在 Obsidian 中用「打开文件夹」或软链进 vault）
3. 在 Obsidian 中维护表格行（误识 / 正确 / 类别）

## Obsidian → Handy

1. 复制笔记中的表格（含 `| ASR误识 | 正确形式 | 类别 |` 头）  
2. 粘贴到 Handy 标识物导入框 → **导入 Markdown 表**  
3. 待确认条目在 UI 中 **验证** 后才参与规则替换

## 离线 JSON

```bash
bun 代码研究/scripts/md_markers_to_json.mjs vault/标识物.md -o asr_markers.json
# 再把 JSON 放到 Handy app data 目录（高级用户）
```

## 不自动做

- 监听 vault 路径热更新  
- 冲突合并 UI  

需要自动化时优先用系统级 folder sync / Hazel / 快捷指令，而不是应用内 daemon。
