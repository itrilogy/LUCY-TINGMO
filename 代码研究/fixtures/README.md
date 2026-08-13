# 长音频黄金样例（fixtures）

本目录存放**本地回归用**录音，体积大，默认已被 `.gitignore` 忽略（仅保留本 README）。

## 鑫苑路9号（Voice Memo）

| 项 | 值 |
|----|-----|
| 来源 | Apple 语音备忘录导出 `鑫苑路9号.m4a` |
| 原始路径（可能临时） | `~/Library/Containers/com.apple.VoiceMemos/.../鑫苑路9号.m4a` |
| 时长 | **≈ 4638 s（约 77.3 分钟）** |
| 原始格式 | AAC mono 48 kHz，约 37 MB |
| 工程内副本 | `xinyuan-road-9.m4a` / `鑫苑路9号.m4a` |
| Handy 用 WAV | `xinyuan-road-9-16k.wav`（16 kHz mono PCM s16，约 142 MB） |

### 为何用它

- 远超 SenseVoice ~30 s 训练窗（约 **155×**），整段推理必触发历史问题。  
- 比安装版 history 里的 30 分钟样例更长，更能验证 **自动分片** 是否稳定。  
- 内容为真实现场长讲，适合听感/可读性抽检。

### 分片预期（max_audio = 30 s，无重叠上界）

| 指标 | 约值 |
|------|------|
| 总样本 @16 kHz | \(4638 × 16000 ≈ 7.42×10^7\) |
| 单段上限 | 480_000 samples |
| 段数（下界） | **≥ 155** 段 |

### 复现 / 回归命令

1. **准备 WAV**（若尚未转换）：

```bash
ffmpeg -y -i "代码研究/fixtures/xinyuan-road-9.m4a" \
  -ac 1 -ar 16000 -c:a pcm_s16le \
  "代码研究/fixtures/xinyuan-road-9-16k.wav"
```

2. **Headless 转写**（需已下载模型，例如 SenseVoice）：

```bash
# 开发构建出的 binary 路径因平台而异；也可对安装版 Handy 试：
/Applications/Handy.app/Contents/MacOS/Handy \
  -f "/Users/kwangwah/Project/Handy/代码研究/fixtures/xinyuan-road-9-16k.wav" \
  --model 'handy-computer/SenseVoiceSmall-gguf/SenseVoiceSmall-Q8_0.gguf' \
  --debug
```

**修复前期望：** 日志出现 `beyond the ~30 s window`，输出极短/乱码。  
**修复后期望：** 日志出现 `Auto-splitting ... into N segment(s) (max_audio_ms=30000)`，全文可读且长度与 77 分钟讲稿同量级。

3. **仅验证分片规划（不跑 ASR）**：`cargo test --lib audio_split`（单元测试不依赖本文件；本文件用于端到端）。

### 注意

- Voice Memo 的 `tmp/.com.apple.uikit.itemprovider...` 路径可能被系统清理；以本目录副本为准。  
- 全量 77 分钟 ASR 在本机可能需 **数十分钟**，请预留时间与电量。  
