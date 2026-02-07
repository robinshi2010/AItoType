# AitoType 功能需求总结

> 文档版本：v1.0  
> 更新时间：2026-02-06  
> 依据来源：
> - `/Users/robin/Work/sideproject/AItoType/plans/active/plan001_aitotype_development.md`
> - `/Users/robin/Work/sideproject/AItoType/docs/competitive_analysis.md`
> - `/Users/robin/Work/sideproject/AItoType/docs/openrouter_gemini_integration.md`

## 1. 项目目标与定位

AitoType 是一款 macOS 全局语音输入工具，核心路径是“录音 -> 转写 -> 输入到当前应用”。

核心定位：
- BYOK（Bring Your Own Key）：用户自带 API Key，降低成本与使用门槛
- 多模型支持：OpenAI、Groq、阿里云、豆包、可扩展 Custom Endpoint
- 开源透明：代码可审计，提升隐私信任
- 轻量高效：基于 Tauri v2，应用体积小、启动快

## 2. 用户角色与核心流程

主要用户：
- 高频文本输入用户（开发者、运营、内容创作、办公用户）
- 对输入效率有要求的专业用户
- 对模型可控性有需求的 BYOK 用户

核心使用流程（MVP）：
1. 用户按下全局快捷键开始录音
2. 再次按下快捷键停止录音
3. 应用调用所选 STT Provider 执行语音转文字
4. 识别结果按配置输出到当前激活应用（模拟键盘优先，剪贴板为备选）
5. 用户在设置页可配置 Provider、模型、快捷键、输出策略

## 3. MVP 功能清单

### 3.1 P0（必须）

- 全局快捷键监听与录音触发
- 录音模块（开始/停止、音频数据获取）
- STT Provider 统一接口（Provider 抽象）
- OpenAI/Groq（Whisper 兼容）适配
- 文本输出到当前应用（模拟键盘输入）
- 设置页基础能力：
  - Provider 配置（类型、Base URL、API Key、Model）
  - 快捷键配置
  - 输出方式配置
  - 连接测试反馈
- 基础状态反馈：录音状态、连接测试状态、错误提示

### 3.2 P1（增强）

- 剪贴板粘贴作为输出备选方案
- 录音状态悬浮窗（轻量不打断）
- 配置持久化（tauri-plugin-store）
- Provider 扩展：阿里云、豆包

## 4. 非功能需求

### 4.1 性能

- 录音触发与停止响应需快速，避免明显阻塞
- 常规语音段落应在可接受延迟内返回识别结果
- UI 交互需保持轻量，避免复杂动画引发卡顿

### 4.2 权限

- 麦克风权限：用于采集语音
- 辅助功能权限：用于模拟键盘输入到其他应用
- 权限文案需在应用说明中明确用途与边界

### 4.3 隐私与安全

- API Key 本地保存（需掩码展示，避免明文泄露）
- 仅将必要音频数据发送到用户选定 Provider
- 不引入与产品目标无关的数据采集
- 网络错误、鉴权失败、模型不可用需明确反馈

### 4.4 可用性与可访问性

- 关键控件支持键盘访问
- 焦点态可见
- 文本和背景满足对比度可读性要求
- 错误提示应贴近对应字段

## 5. 设置页信息架构

设置页采用单页分区结构，包含以下模块：

1. 顶部应用信息区
- 应用名称、版本、运行状态、隐私说明

2. Provider 配置区
- Provider 类型（openai/groq/aliyun/doubao/custom）
- Base URL
- API Key（密码态）
- Model
- 启用状态

3. 快捷键配置区
- 快捷键显示
- 快捷键录制入口

4. 输出方式区
- 模拟键盘输入
- 剪贴板粘贴（备选）

5. 连接测试与反馈区
- 状态模型：`idle | testing | success | error`
- 每种状态有清晰视觉反馈和文案

6. 高级选项区
- 语言策略（自动/中文/英文）
- 自动标点
- 去填充词（占位，Phase 2）

7. 底部操作区
- 保存配置
- 重置
- 导入/导出（占位）

## 6. 前后端契约草案（本轮仅定义，不实现）

### 6.1 Provider 配置数据结构

```ts
interface ProviderConfig {
  provider_type: 'openai' | 'groq' | 'aliyun' | 'doubao' | 'custom';
  base_url: string;
  api_key: string; // UI 中掩码展示
  model: string;
  enabled: boolean;
}
```

### 6.2 设置分区契约

```ts
type SettingsSections = {
  provider_settings: ProviderConfig;
  hotkey_settings: {
    push_to_talk: string;
  };
  output_settings: {
    mode: 'simulate_keyboard' | 'clipboard_paste';
  };
  advanced_settings: {
    language: 'auto' | 'zh' | 'en';
    auto_punctuation: boolean;
    remove_fillers: boolean;
  };
};
```

### 6.3 连接测试状态契约

```ts
type ConnectionTestState = 'idle' | 'testing' | 'success' | 'error';
```

## 7. 阶段路线图

### MVP（当前阶段）

- 打通主链路：录音 -> STT -> 输入
- 完成设置页核心配置能力
- 支持 OpenAI/Groq

### Phase 2

- 适配阿里云、豆包
- 离线模型探索（whisper.cpp）
- 历史记录、词典热词、AI 改写能力

### Phase 3

- 去填充词与文本清洗增强
- 自动格式化增强
- 多平台扩展（如 Windows）

## 8. 风险与依赖

主要风险：
- macOS 权限获取失败导致录音或输入不可用
- Provider API 兼容性差异（鉴权、入参格式、返回结构）
- 网络波动导致识别失败或延迟上升
- 模型变更或临时不可用导致服务不稳定

关键依赖：
- `tauri-plugin-global-shortcut`
- `tauri-plugin-store`
- Rust 侧录音与输入能力（`cpal`, `enigo`）
- Provider API 的稳定性与可用配额

## 9. 验收标准

功能验收：
- 可完成全局快捷键录音与停止
- 至少一个 Provider 可成功转写并输出到当前应用
- 设置页可保存并读取关键配置
- 连接测试状态可正确反馈

体验验收：
- 关键流程可在短路径内完成
- 错误提示可理解且可定位
- 页面在桌面与移动视口下结构完整无重叠

稳定性验收：
- 常见异常（空 Key、无网络、模型错误）有可预期反馈
- 不因单次请求失败导致应用不可用
