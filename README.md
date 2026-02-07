# AitoType 🎙️

<p align="center">
  <img src="docs/screenshots/app-icon.png" alt="AitoType Icon" width="128" height="128" />
</p>

AitoType 是一个开源、免费的桌面端语音转文字工具，基于 **Tauri v2** 构建，旨在提供极致轻量、高效的语音输入体验。

**核心理念**：免费开源、极简设计、高效生产力。

> ⚠️ **声明**：本项目完全开源免费，供大家学习与使用。**严禁用于任何商业盈利行为**。

---

## 📸 界面预览

<p align="center">
  <img src="docs/screenshots/main.png" alt="主界面 - 极简悬浮球" width="45%" border="1" />
  <img src="docs/screenshots/setting.png" alt="设置页 - 自定义模型与快捷键" width="45%" border="1" />
</p>

## ✨ 功能特性

- **全局快捷键唤起**：默认 `Alt+Space` 一键录音，再次按下即可停止并识别。
- **无缝嵌入工作流**：
  - **自动粘贴**：识别完成后，结果自动“打字”到你当前光标所在的输入框（Chatbox、文档、编辑器等）。
  - **自动复制**：可选自动复制结果到剪贴板。
- **极致 UI 设计**：
   - "Glass Monolith" 玻璃拟态设计风格。
   - 沉浸式动效与交互反馈。
   - **后台悬浮条**：在后台录音时，屏幕底部显示灵动胶囊状态条，不干扰视线。
- **灵活的模型支持**：
   - 内置 **OpenRouter** 支持。
   - 默认配置 **Gemini 3.0 Flash**（速度快、精度高、免费额度充裕）。
   - 可自定义 API Key 和 Model（如 GPT-4o, Claude 3.5 Sonnet 等）。
- **隐私安全**：
   - 音频数据仅在您的设备上录制，并直接发送至您配置的 API 服务商。
   - 本地不留存录音音频文件；会保存必要配置（如 API Key、Model、快捷键）以便下次使用。

## 🛠️ 技术栈

- **Core**: [Rust](https://www.rust-lang.org/) (Tauri v2, cpal, hound, arboard, enigo)
- **Frontend**: Vanilla JS + CSS (无框架，追求极致轻量与性能)
- **API**: OpenRouter (兼容 OpenAI 格式)

## 🚀 快速开始

### 前置要求

- macOS (目前主要适配平台)
- Node.js 18+
- Rust 环境 (推荐通过 `rustup` 安装)

### 开发运行

```bash
# 1. 克隆项目
git clone https://github.com/your-username/AitoType.git
cd AitoType/src/aitotype

# 2. 安装依赖
npm install

# 3. 启动开发模式
npm run tauri dev
```

### 打包构建

```bash
# 在 src/aitotype 目录下执行
npm run tauri build
```
构建产物通常位于 `src-tauri/target/release/bundle/dmg/*.dmg`。

## ⚙️ 配置指南

首次启动后，请点击主界面底部的 **设置图标 (⚙️)** 进行配置：

1. **OpenRouter API Key**: 填入你的 Key（前往 [OpenRouter.ai](https://openrouter.ai/) 获取）。
2. **Model**: 默认为 `google/gemini-3-flash-preview`，你也可以填入其他已订阅的模型 ID。
3. **Global Shortcut**: 点击录制你习惯的快捷键（如 `Cmd+Shift+M` 或 `F1`）。
4. **Auto-Copy**: 开启后，识别结果会自动进入剪贴板。

**注意**：配置会自动保存到本地，重启应用无需重新输入。

## 🧭 使用方法

### 首次使用（建议）

1. 打开应用，进入 **Settings** 页面。
2. 填写 `OpenRouter API Key` 与 `Model`，点击 **Save Changes**。
3. 设置你习惯的全局快捷键（默认 `Alt+Space`）。
4. 按需开启 `Auto-Copy Result`。

### 日常使用流程

1. 在任意应用中把光标放到目标输入框（如 Chatbox、微信、飞书、编辑器、文档）。
2. 按一次全局快捷键开始录音。
3. 说完后再次按快捷键结束录音，应用会自动转写。
4. 如在后台触发录音，转写完成后会自动把文本粘贴到当前光标位置。
5. 如需手动处理结果，可在主界面中复制文本或查看历史记录。

### 常见场景

- **聊天回复**：边说边转写，快速发送长消息。
- **会议纪要**：录制关键语句，集中整理到文档。
- **代码注释/文档**：先口述内容，再微调文字。

## 🤝 贡献与反馈

欢迎提交 Issue 或 Pull Request！无论是功能建议、Bug 反馈还是代码贡献，我们都非常欢迎。

## 📄 许可证 (License)

本项目采用 **非商业开源协议**。

- ✅ **允许**：个人免费使用、修改源码用于学习或个人项目、分发给他人免费使用。
- ❌ **禁止**：将本项目或其衍生代码用于**任何形式的商业盈利**（包括但不限于付费软件、广告植入、企业内部收费工具等）。

---

Made with ❤️ by Robin. Enjoy typing with your voice!
