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

### 本地脚本命令

在 `/Users/robin/Work/sideproject/AItoType/src/aitotype` 下可用：

1. `npm run dev`：启动开发模式
2. `npm run build`：本机打包当前平台
3. `npm run build:ci`：CI 模式打包（用于 GitHub Actions）

## 📦 下载与安装（给普通用户）

### 下载安装

1. 前往项目 GitHub Releases 页面下载最新 `.dmg`。
2. 双击打开 `.dmg`，将 `AitoType.app` 拖入 `Applications`。
3. 从“应用程序”中启动 AitoType。

### 首次打开被 macOS 拦截时

如果提示“无法验证开发者”或“已损坏”：

1. 在 Finder 里右键应用，选择“打开（Open）”，再确认一次。
2. 或到 `系统设置 -> 隐私与安全性`，点击“仍要打开”。

若仍被 Gatekeeper 拦截（仅限你确认来源可信时）：

```bash
xattr -dr com.apple.quarantine /Applications/AitoType.app
```

## 🧱 打包与发布（给维护者）

### 本地打包

```bash
cd src/aitotype
npm install
npm run tauri build
```

主要产物目录：

- `.app`: `src/aitotype/src-tauri/target/release/bundle/macos/`
- `.dmg`: `src/aitotype/src-tauri/target/release/bundle/dmg/`

### 发布建议流程

1. 先在本机全新安装测试 `.dmg`（不是开发环境直接运行）。
2. 确认权限请求流程（麦克风、辅助功能）正常。
3. 将 `.dmg` 上传到 GitHub Release，并附版本更新说明。

## 🤖 GitHub Actions 自动打包（Win / macOS / Linux）

仓库已提供 CI Pipeline：`/Users/robin/Work/sideproject/AItoType/.github/workflows/build-release.yml`

触发方式：

1. **手动触发构建（不发 Release）**
   - GitHub 仓库页面 -> `Actions` -> `Build And Release` -> `Run workflow`
   - 构建完成后，在该次 workflow 的 `Artifacts` 下载三端安装包。
2. **打 Tag 自动发布 Release（推荐）**
   - 推送 `v*` 标签（如 `v0.1.0`）后，workflow 会自动：
     - 在 `macos-latest` 打包 `.dmg`
     - 在 `windows-latest` 打包 `.msi/.exe`
     - 在 `ubuntu-22.04` 打包 `.deb/.AppImage/.rpm`
     - 自动创建 GitHub Release 并上传全部产物

> 注意：当前 workflow 默认只做“构建与上传”，不包含 macOS 证书签名和公证。  
> 若要消除用户首次安装安全拦截，请按下文“macOS 证书、签名与公证”流程处理后再发布。

## 🚀 一键发布步骤（建议流程）

### Step 1: 更新版本号

至少保持这两处一致：

- `/Users/robin/Work/sideproject/AItoType/src/aitotype/package.json`
- `/Users/robin/Work/sideproject/AItoType/src/aitotype/src-tauri/tauri.conf.json`

### Step 2: 提交代码并推送主分支

```bash
git add .
git commit -m "release: v0.1.0"
git push origin main
```

### Step 3: 打标签触发自动发布

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Step 4: 等待 Pipeline 完成

在 GitHub `Actions` 查看 `Build And Release`，确认 3 个平台 Job 都成功。

### Step 5: 检查 Release 页面

进入 `Releases`，确认自动生成的 Release 中包含：

1. macOS: `.dmg`
2. Windows: `.msi`（可能还会有 `.exe`）
3. Linux: `.deb` / `.AppImage` / `.rpm`

### Step 6: 补充发布说明

编辑 Release Notes，建议包含：

1. 本次新增功能
2. 兼容平台与系统要求
3. 已知问题与绕过方式（如未签名时的 Gatekeeper 提示）

## 🔐 macOS 证书、签名与公证

未签名/未公证的应用在其他 macOS 设备上通常会被安全机制拦截。要获得更顺畅安装体验，建议做 **Developer ID 签名 + Notarization 公证**。

### 1) 准备条件

- Apple Developer Program 账号
- `Developer ID Application` 证书（安装在钥匙串）
- Xcode Command Line Tools（含 `codesign`、`notarytool`、`stapler`）

### 2) 对 `.app` 签名（示例）

```bash
codesign --force --deep --options runtime \
  --sign "Developer ID Application: YOUR_NAME (TEAM_ID)" \
  "src/aitotype/src-tauri/target/release/bundle/macos/AitoType.app"
```

### 3) 提交公证并等待结果（示例）

```bash
xcrun notarytool submit \
  "src/aitotype/src-tauri/target/release/bundle/dmg/AitoType_0.1.0_x64.dmg" \
  --apple-id "YOUR_APPLE_ID" \
  --team-id "YOUR_TEAM_ID" \
  --password "YOUR_APP_SPECIFIC_PASSWORD" \
  --wait
```

### 4) 装订公证票据（Staple）

```bash
xcrun stapler staple "src/aitotype/src-tauri/target/release/bundle/dmg/AitoType_0.1.0_x64.dmg"
```

### 5) 发布前自检

```bash
spctl -a -vv "src/aitotype/src-tauri/target/release/bundle/macos/AitoType.app"
codesign --verify --deep --strict --verbose=2 "src/aitotype/src-tauri/target/release/bundle/macos/AitoType.app"
```

## ⚙️ 配置指南

首次启动后，请点击主界面底部的 **设置图标 (⚙️)** 进行配置：

1. **OpenRouter API Key**: 填入你的 Key（前往 [OpenRouter.ai](https://openrouter.ai/) 获取）。
2. **Model**: 默认为 `google/gemini-3-flash-preview`，你也可以填入其他已订阅的模型 ID。
3. **Global Shortcut**: 点击录制你习惯的快捷键（如 `Cmd+Shift+M` 或 `F1`）。
4. **Auto-Copy**: 开启后，识别结果会自动进入剪贴板。

**注意**：配置会自动保存到本地，重启应用无需重新输入。

## 🔑 API Key 安全说明

- 你的 OpenRouter API Key 会保存在**本机应用配置目录**，不会写入本仓库代码。
- 只要你不手动把该配置文件提交到 GitHub，发布仓库不会泄露你的 Key。
- 当前实现为本地明文保存，建议你：
  - 使用低权限/限额的 OpenRouter Key；
  - 定期轮换 Key；
  - 后续升级为 Keychain 存储（更安全）。

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

本项目采用 **CC BY-NC-SA 4.0** 协议（署名-非商业性使用-相同方式共享）。

- ✅ **允许**：复制、分发、修改、二次创作（需署名，并以相同协议共享）。
- ❌ **禁止**：将本项目或其衍生作品用于商业用途。

协议详情：

- License summary: https://creativecommons.org/licenses/by-nc-sa/4.0/
- Legal code: https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode

---

Made with ❤️ by Robin. Enjoy typing with your voice!
