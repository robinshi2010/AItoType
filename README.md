# AitoType

<p align="center">
  <img src="docs/screenshots/app-icon.png" alt="AitoType Icon" width="128" height="128" />
</p>

AitoType 是一个基于 Tauri 的桌面语音转文字工具（当前主要面向 macOS），支持全局快捷键录音、实时状态悬浮提示、识别后自动粘贴回当前输入框。

## 功能特性

- 全局快捷键录音（默认 `Alt+Space`，可在设置里自定义）
- 二段式操作：
  - 第一次触发：开始录音
  - 第二次触发：停止录音并发起识别
- 后台触发时显示底部悬浮状态条（录制中 / 识别中）
- 识别完成后自动粘贴到当前应用输入框（适合 chatbox 场景）
- 可选自动复制识别结果到系统剪贴板
- 历史记录查看
- 托盘常驻（可从托盘重新打开窗口）

## 截图

> 你可以后续把截图放到 `docs/screenshots/`，并替换下面占位。

- 主界面（录音状态）  
  `docs/screenshots/main-record.png`
- 后台悬浮条（录制中）  
  `docs/screenshots/overlay-recording.png`
- 后台悬浮条（识别中）  
  `docs/screenshots/overlay-transcribing.png`
- 设置页（快捷键 / API Key / Model）  
  `docs/screenshots/settings.png`

示例 Markdown（补图后直接取消注释）：

```md
<!--
![Main](docs/screenshots/main-record.png)
![Overlay Recording](docs/screenshots/overlay-recording.png)
![Overlay Transcribing](docs/screenshots/overlay-transcribing.png)
![Settings](docs/screenshots/settings.png)
-->
```

## 技术栈

- Tauri v2
- Rust（音频录制、STT 调用、键盘输入）
- Vanilla HTML / CSS / JavaScript（前端界面）
- OpenRouter API（模型路由）

## 项目结构

```text
AItoType/
├─ README.md
├─ docs/
│  └─ screenshots/
└─ src/
   └─ aitotype/
      ├─ package.json
      ├─ src/
      │  ├─ index.html
      │  ├─ main.js
      │  ├─ styles.css
      │  ├─ overlay.html
      │  ├─ overlay.css
      │  └─ overlay.js
      └─ src-tauri/
         ├─ tauri.conf.json
         ├─ icons/
         └─ src/
            ├─ lib.rs
            ├─ audio.rs
            ├─ stt.rs
            └─ keyboard.rs
```

## 环境要求

- macOS（当前主要验证平台）
- Node.js 18+
- Rust stable（建议使用 rustup）
- Xcode Command Line Tools（macOS）

## 快速开始（开发）

在项目根目录执行：

```bash
cd /Users/robin/Work/sideproject/AItoType/src/aitotype
npm install
npm run tauri dev
```

如果你在仓库根目录执行，可使用：

```bash
npm --prefix /Users/robin/Work/sideproject/AItoType/src/aitotype run tauri dev
```

## 配置说明

打开应用 `Settings` 页面配置：

- `OpenRouter API Key`：你的 OpenRouter Key
- `Model`：默认 `google/gemini-3-flash-preview`（可改）
- `Global Shortcut`：设置全局快捷键
- `Auto-Copy Result`：识别后自动复制到剪贴板

### 推荐 Model

- 默认：`google/gemini-3-flash-preview`
- 如遇地区或 provider 限制，可在 OpenRouter 侧调整路由策略或更换可用模型

## 使用方式

1. 在任意应用输入框中，将光标放到目标位置
2. 按一次全局快捷键开始录音
3. 再按一次全局快捷键结束并识别
4. 识别结果会自动粘贴回当前输入框（后台触发场景）

## 打包构建

在 `src/aitotype` 目录执行：

```bash
npm run tauri build
```

常见产物路径（macOS）：

- `.app`：`src/aitotype/src-tauri/target/release/bundle/macos/`
- `.dmg`：`src/aitotype/src-tauri/target/release/bundle/dmg/`

## 更新应用图标

项目已支持通过单张源图批量生成图标资源：

```bash
cd /Users/robin/Work/sideproject/AItoType/src/aitotype
npm run tauri icon /Users/robin/Work/sideproject/AItoType/icon.png
```

## 常见问题

### 1) `npm ERR! enoent Could not read package.json`

原因：执行目录不在 `src/aitotype`。  
解决：切换到 `src/aitotype` 再运行命令，或使用 `npm --prefix ...`。

### 2) OpenRouter 返回 400（地区限制）

如出现类似 `User location is not supported for the API use.`：

- 这是上游 provider 的地区限制，不是录音功能故障
- 可在 OpenRouter 控制台调整该模型的 provider 路由
- 或更换当前可用的模型 / 网络区域

### 3) 全局快捷键无效

- 检查是否被系统或其他应用占用
- 在设置页重新录制快捷键
- macOS 下确保应用具备麦克风/辅助功能权限

## 隐私说明

- 本地采集音频后发送到你配置的 STT API 进行转写
- 识别文本仅用于展示、复制与自动粘贴
- 请遵守你所在地区的隐私与合规要求

## Roadmap（可选）

- 多 provider 切换与可视化路由策略
- 结果后处理（标点、格式化、摘要）
- 自定义快捷命令（翻译、润色、改写）

## License

你可以在发布前补充 License（例如 MIT）。
