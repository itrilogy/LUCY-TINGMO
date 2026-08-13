# 回归报告：鑫苑路9号 全量转写（Phase 0 自动分片）

| 项 | 值 |
|----|-----|
| 日期 | 2026-07-29 |
| 二进制 | `src-tauri/target/debug/handy`（含 Phase 0） |
| 命令 | `-f xinyuan-road-9-16k.wav --model SenseVoiceSmall-Q8_0 --debug --json` |
| 音频 | 4638.12 s（≈ **77.3 min**），16 kHz mono |
| 模型 | `handy-computer/SenseVoiceSmall-gguf/SenseVoiceSmall-Q8_0.gguf` |
| 后端 | Metal **MTL0**（Apple M2） |
| exit | **0** |

## 关键结果

| 指标 | 修复前（同类长音频，安装版 history） | **本次 Phase 0** |
|------|--------------------------------------|------------------|
| 策略 | 整段 `run(1808s+)` | **Auto-split → 160 段 × ≤30s** |
| `beyond the ~30 s window` | 有 WARN | **0 次** |
| 输出字数 | ~500 字级乱码（30min 样例） | **18 941 字** |
| 耗时 | 30min 样例曾 ~192s 且质量崩 | **87.5 s** 处理 77.3 min 音频（**53×** 实时） |
| 可读性 | 叠字/崩溃 | **主题连贯**（卷烟加工标准、质量安全体系、培训收尾「谢谢大家」） |

## 日志原文（要点）

```text
Auto-splitting 4638.12s of audio into 160 segment(s) (max_audio_ms=30000, model='...SenseVoiceSmall-Q8_0.gguf')
Transcription completed in 87.51s for 4638.12s of audio (53.00x real-time) — result_chars=18941
```

## 产出文件

| 文件 | 说明 |
|------|------|
| `xinyuan-road-9-transcript.txt` | 纯文本转写结果 |
| `xinyuan-road-9-transcribe-result.txt.stdout` | JSON（含 rtf、text、load_ms…） |
| `xinyuan-road-9-transcribe-result.txt.stderr` | 运行日志 |
| `xinyuan-road-9-transcribe-run.log` | 包装脚本汇总 |

## 质量备注

- 口语/ASR 常见糊词仍在（如「制瓷/制丝」「博品/薄片」），属识别误差，**非整段超窗崩溃**。  
- 中文段拼接无额外空格，符合 `join_segment_texts` 设计。  
- 约 4.1 字/秒音频量级，对培训口播合理。  

## 结论

**Phase 0 长音频自动分片在 77 分钟黄金样例上通过验收。**  
可进入 Phase 1（Session / 扩大 overlay / 不自动粘贴）开发。
