/**
 * AItoType Spatial Controller
 * "Glass Monolith" Edition
 */

const { invoke } = window.__TAURI__.core;
// Safely try to get listen, fallback if not available
const { listen } = window.__TAURI__.event || { listen: () => { } };

const PROVIDER_OPENROUTER = 'openrouter';
const PROVIDER_SILICONFLOW = 'siliconflow';
const DEFAULT_OPENROUTER_MODEL = 'google/gemini-3-flash-preview';
const DEFAULT_SILICONFLOW_MODEL = 'TeleAI/TeleSpeechASR';
const DEFAULT_ENHANCEMENT_OPENROUTER_MODEL = DEFAULT_OPENROUTER_MODEL;
const DEFAULT_ENHANCEMENT_SILICONFLOW_MODEL = 'Qwen/Qwen2.5-7B-Instruct';
const DEFAULT_ENHANCEMENT_PROMPT = '你是程序员语音转文字的润色助手。请按规则处理文本：\n1) 去除口头禅、重复词和无意义停顿词；\n2) 修正技术术语、产品名、代码相关拼写错误；\n3) 保留原意，不扩写、不总结、不补充新信息；\n4) 仅做必要标点与断句优化；\n5) 只输出润色后的最终文本，不要任何解释。\n\n原文：\n{text}';
const API_KEY_STORAGE_KEYS = {
  [PROVIDER_OPENROUTER]: 'aitotype_api_key_openrouter',
  [PROVIDER_SILICONFLOW]: 'aitotype_api_key_siliconflow'
};
const ENHANCEMENT_API_KEY_STORAGE_KEYS = {
  [PROVIDER_OPENROUTER]: 'aitotype_api_key_enhancement_openrouter',
  [PROVIDER_SILICONFLOW]: 'aitotype_api_key_enhancement_siliconflow'
};
const DEFAULT_SHORTCUT = /windows/i.test(navigator.userAgent || navigator.platform || '')
  ? 'Ctrl+Shift+Space'
  : 'Alt+Space';

// ============ State ============
const state = {
  status: 'idle',
  lastResult: '',
  history: [],
  audioLevelTimer: null,
  shortcutUnlisten: null,
  shortcutCaptureActive: false,
  shortcutPluginReady: false,
  pendingShortcutContext: null,
  backgroundSession: false,
  lastShortcutToggleAt: 0,
  sttConfig: null,
  currentProvider: PROVIDER_OPENROUTER,
  enhancementProvider: PROVIDER_OPENROUTER,
  enhancementFallbackHintTimer: null,
  recordMode: 'toggle',
  providerApiKeys: {
    [PROVIDER_OPENROUTER]: '',
    [PROVIDER_SILICONFLOW]: ''
  },
  enhancementProviderApiKeys: {
    [PROVIDER_OPENROUTER]: '',
    [PROVIDER_SILICONFLOW]: ''
  }
};

// ============ Elements ============
const el = {
  // Spotlight
  root: document.documentElement,

  // Navigation
  tabs: document.querySelectorAll('.ornament-tab'),
  views: document.querySelectorAll('.spatial-view'),

  // Recorder
  recordTrigger: document.getElementById('record-trigger'),
  orbWrapper: document.querySelector('.orb-wrapper'),
  statusPill: document.getElementById('status-pill'),
  instructionText: document.getElementById('instruction-text'),

  // Result Sheet
  resultSheet: document.getElementById('result-sheet'),
  resultText: document.getElementById('result-text'),
  enhancementFallbackHint: document.getElementById('enhancement-fallback-hint'),
  closeResultBtn: document.getElementById('reset-result-btn'),
  copyBtn: document.getElementById('copy-btn'),
  autoCopySwitch: document.getElementById('auto-copy-switch'),
  autoWriteSwitch: document.getElementById('auto-write-switch'),
  accessibilityHint: document.getElementById('accessibility-hint'),
  openAccessibilityBtn: document.getElementById('open-accessibility-btn'),

  // Settings
  providerSelect: document.getElementById('provider-select'),
  apiKeyLabel: document.getElementById('api-key-label'),
  apiKeyInput: document.getElementById('api-key-input'),
  modelInput: document.getElementById('model-input'),
  enhancementSwitch: document.getElementById('enhancement-switch'),
  enhancementSettings: document.getElementById('enhancement-settings'),
  enhancementProviderSelect: document.getElementById('enhancement-provider-select'),
  enhancementApiKeyLabel: document.getElementById('enhancement-api-key-label'),
  enhancementApiKeyInput: document.getElementById('enhancement-api-key-input'),
  enhancementModelInput: document.getElementById('enhancement-model-input'),
  enhancementPromptInput: document.getElementById('enhancement-prompt-input'),
  testConnectionBtn: document.getElementById('test-connection-btn'),
  testConnectionResult: document.getElementById('test-connection-result'),
  settingsForm: document.getElementById('settings-form'),
  settingsStatus: document.getElementById('settings-status'),
  settingsSaveBtn: document.querySelector('#settings-form button[type="submit"]'),

  // Shortcut (Placeholder)
  shortcutRecorder: document.getElementById('shortcut-recorder'),
  shortcutLabel: document.getElementById('shortcut-label'),

  // Recording Mode
  recordModeSwitch: document.getElementById('record-mode-switch'),

  // History
  historyContainer: document.getElementById('history-container')
};

// ============ Spotlight Effect ============
window.addEventListener('mousemove', (e) => {
  requestAnimationFrame(() => {
    el.root.style.setProperty('--mouse-x', `${e.clientX}px`);
    el.root.style.setProperty('--mouse-y', `${e.clientY}px`);
  });
});

// ============ Navigation ============
function switchView(viewId) {
  el.tabs.forEach(tab => {
    tab.classList.toggle('active', tab.dataset.view === viewId);
  });

  el.views.forEach(view => {
    if (view.id === viewId) {
      view.classList.add('active');
    } else {
      view.classList.remove('active');
    }
  });

  if (viewId === 'view-settings') {
    checkAccessibility();
  }
}

// ============ Permission Checking ============
async function checkAccessibility() {
  try {
    const trusted = await invoke('check_accessibility_permissions');
    if (el.accessibilityHint) {
      if (trusted) {
        el.accessibilityHint.innerHTML = `
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#30D158" stroke-width="2">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span>Accessibility Permission OK</span>
        `;
        el.accessibilityHint.classList.add('success');
      } else {
        el.accessibilityHint.innerHTML = `
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#FF9F0A" stroke-width="2">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <span>Need Accessibility Permission to Auto-Write</span>
          <button type="button" id="open-accessibility-btn" class="text-link">Settings</button>
        `;
        el.accessibilityHint.classList.remove('success');

        // Re-attach listener since we replaced innerHTML
        const btn = el.accessibilityHint.querySelector('#open-accessibility-btn');
        if (btn) {
          btn.onclick = () => invoke('open_accessibility_settings');
        }
      }
      el.accessibilityHint.classList.toggle('hidden', !el.autoWriteSwitch.checked);
    }
  } catch (e) { console.error('Check accessibility failed', e); }
}

// ============ Status System ============
function updateStatus(newStatus, msg) {
  state.status = newStatus;

  // Reset Orb
  el.orbWrapper.classList.remove('active');
  el.orbWrapper.classList.remove('processing');
  el.recordTrigger.style.animation = '';

  switch (newStatus) {
    case 'idle':
      el.statusPill.textContent = 'Ready';
      el.instructionText.textContent = msg || (state.recordMode === 'hold' ? 'Hold shortcut to speak' : 'Tap orb to capture');
      stopsAudioAnim();
      break;

    case 'recording':
      el.orbWrapper.classList.add('active');
      el.statusPill.textContent = 'Recording';
      el.instructionText.textContent = 'Listening...';
      startAudioAnim();
      break;

    case 'transcribing':
      el.orbWrapper.classList.add('active');
      el.orbWrapper.classList.add('processing');
      el.recordTrigger.style.animation = 'pulse 1s infinite';
      el.statusPill.textContent = 'Processing';
      el.instructionText.textContent = 'Transcribing...';
      stopsAudioAnim();
      break;

    case 'success':
      el.statusPill.textContent = 'Success';
      el.instructionText.textContent = 'Complete';
      if (el.orbWrapper.classList.contains('processing')) {
        el.orbWrapper.classList.remove('processing');
      }
      showResult(msg);
      break;

    case 'error':
      el.statusPill.textContent = 'Error';
      el.instructionText.textContent = msg || 'Failed';
      break;
  }
}

async function copyResultToClipboard(text) {
  if (!text) return false;
  try {
    await invoke('copy_to_clipboard', { text });
    return true;
  } catch (e) {
    console.error('Clipboard copy failed', e);
    return false;
  }
}

function invokeWithTimeout(command, payload, timeoutMs = 800) {
  const call = invoke(command, payload);
  const timeout = new Promise((_, reject) => {
    setTimeout(() => reject(new Error(`${command} timeout`)), timeoutMs);
  });
  return Promise.race([call, timeout]);
}

function safeShowOverlayStatus(status) {
  invokeWithTimeout('show_overlay_status', { status }).catch((e) => {
    console.error('Show overlay failed', e);
  });
}

function safeHideOverlay() {
  invokeWithTimeout('hide_overlay', {}).catch((e) => {
    console.error('Hide overlay failed', e);
  });
}

// ============ Recorder Logic ============
async function toggleRecording() {
  if (state.status === 'transcribing') return;

  if (state.status === 'recording') {
    // Stop & Transcribe
    try {
      updateStatus('transcribing');
      if (state.backgroundSession) {
        safeShowOverlayStatus('transcribing');
      }
      const result = await invoke('stop_and_transcribe');

      state.lastResult = result;
      addToHistory(result);

      // Auto-Copy
      if (el.autoCopySwitch && el.autoCopySwitch.checked) {
        await copyResultToClipboard(result);
      }

      // Auto-Write (Paste) logic
      const isAutoWriteEnabled = state.sttConfig?.auto_write || (el.autoWriteSwitch && el.autoWriteSwitch.checked);
      if (state.backgroundSession || isAutoWriteEnabled) {
        if (state.backgroundSession) {
          safeHideOverlay();
        }

        try {
          await invoke('paste_text', { text: result });
        } catch (e) {
          console.error('Paste failed', e);
          updateStatus('error', 'Paste failed. Check Accessibility permissions.');
        }
      }

      updateStatus('success', result);

      if (state.backgroundSession) {
        safeHideOverlay();
      }
      state.backgroundSession = false;
    } catch (e) {
      console.error(e);
      updateStatus('error', e.toString());
      if (state.backgroundSession) {
        safeHideOverlay();
      }
      state.backgroundSession = false;
    }
  } else {
    // Start Recording
    hideResult();
    try {
      await syncConfigFromUi();
      const fromBackgroundShortcut = Boolean(state.pendingShortcutContext?.background);
      state.backgroundSession = fromBackgroundShortcut;
      state.pendingShortcutContext = null;

      await invoke('start_recording');
      updateStatus('recording');

      if (state.backgroundSession) {
        safeShowOverlayStatus('recording');
      }
    } catch (e) {
      // If anything fails after start attempt, try rollback to avoid stuck recording state.
      try {
        await invoke('stop_recording');
      } catch (_) { }
      updateStatus('error', e.toString());
      if (state.backgroundSession) {
        safeHideOverlay();
      }
      state.backgroundSession = false;
      state.pendingShortcutContext = null;
    }
  }
}

async function startRecordingOnly(background) {
  if (state.status === 'recording' || state.status === 'transcribing') return;

  hideResult();
  try {
    await syncConfigFromUi();
    state.backgroundSession = Boolean(background);
    state.pendingShortcutContext = null;
    state.holdStartedAt = Date.now();

    await invoke('start_recording');
    updateStatus('recording');

    if (state.backgroundSession) {
      safeShowOverlayStatus('recording');
    }
  } catch (e) {
    try { await invoke('stop_recording'); } catch (_) { }
    updateStatus('error', e.toString());
    if (state.backgroundSession) {
      safeHideOverlay();
    }
    state.backgroundSession = false;
  }
}

async function stopAndTranscribeOnly() {
  if (state.status !== 'recording') return;

  const holdDuration = Date.now() - (state.holdStartedAt || 0);
  if (holdDuration < 200) {
    try { await invoke('stop_recording'); } catch (_) { }
    updateStatus('idle');
    if (state.backgroundSession) {
      safeHideOverlay();
    }
    state.backgroundSession = false;
    return;
  }

  try {
    updateStatus('transcribing');
    if (state.backgroundSession) {
      safeShowOverlayStatus('transcribing');
    }
    const result = await invoke('stop_and_transcribe');

    state.lastResult = result;
    addToHistory(result);

    if (el.autoCopySwitch && el.autoCopySwitch.checked) {
      await copyResultToClipboard(result);
    }

    const isAutoWriteEnabled = state.sttConfig?.auto_write || (el.autoWriteSwitch && el.autoWriteSwitch.checked);
    if (state.backgroundSession || isAutoWriteEnabled) {
      if (state.backgroundSession) {
        safeHideOverlay();
      }
      try {
        await invoke('paste_text', { text: result });
      } catch (e) {
        console.error('Paste failed', e);
        updateStatus('error', 'Paste failed. Check Accessibility permissions.');
      }
    }

    updateStatus('success', result);

    if (state.backgroundSession) {
      safeHideOverlay();
    }
    state.backgroundSession = false;
  } catch (e) {
    console.error(e);
    updateStatus('error', e.toString());
    if (state.backgroundSession) {
      safeHideOverlay();
    }
    state.backgroundSession = false;
  }
}

// ============ Audio Animation (Real Level) ============
function startAudioAnim() {
  if (!el.recordTrigger || state.audioLevelTimer) return;

  state.audioLevelTimer = setInterval(async () => {
    if (state.status !== 'recording') {
      stopsAudioAnim();
      return;
    }

    try {
      const level = Number(await invoke('get_audio_level'));
      const safeLevel = Number.isFinite(level) ? Math.max(0, Math.min(1, level)) : 0;
      const scale = 1 + safeLevel * 0.14;
      const glowSize = 18 + safeLevel * 34;
      const glowAlpha = 0.2 + safeLevel * 0.45;

      el.recordTrigger.style.transform = `scale(${scale.toFixed(3)})`;
      el.recordTrigger.style.boxShadow = `
        inset 0 0 ${12 + safeLevel * 24}px rgba(255, 255, 255, ${0.06 + safeLevel * 0.14}),
        inset 0 0 ${4 + safeLevel * 10}px rgba(255, 255, 255, ${0.12 + safeLevel * 0.18}),
        0 18px 40px rgba(0, 0, 0, 0.4),
        0 0 ${glowSize.toFixed(1)}px rgba(10, 132, 255, ${glowAlpha.toFixed(3)})
      `;
    } catch (_) { }
  }, 80);
}
function stopsAudioAnim() {
  if (state.audioLevelTimer) {
    clearInterval(state.audioLevelTimer);
    state.audioLevelTimer = null;
  }

  if (el.recordTrigger) {
    el.recordTrigger.style.transform = '';
    el.recordTrigger.style.boxShadow = '';
  }
}

// ============ Result Sheet ============
function showResult(text) {
  el.resultText.textContent = text;
  el.resultSheet.classList.remove('hidden');
}

function hideResult() {
  el.resultSheet.classList.add('hidden');
}

function showEnhancementFallbackHint(reason) {
  if (!el.enhancementFallbackHint) return;

  const detail = typeof reason === 'string' ? reason.trim() : '';
  const maxDetailLength = 120;
  const safeDetail = detail.length > maxDetailLength
    ? `${detail.slice(0, maxDetailLength)}...`
    : detail;
  const text = safeDetail
    ? `LLM 润色失败，已回退原始转写：${safeDetail}`
    : 'LLM 润色失败，已回退原始转写';

  el.enhancementFallbackHint.textContent = text;
  el.enhancementFallbackHint.classList.remove('hidden');
  el.enhancementFallbackHint.classList.add('show');

  if (state.enhancementFallbackHintTimer) {
    clearTimeout(state.enhancementFallbackHintTimer);
  }
  state.enhancementFallbackHintTimer = setTimeout(() => {
    if (!el.enhancementFallbackHint) return;
    el.enhancementFallbackHint.classList.remove('show');
    el.enhancementFallbackHint.classList.add('hidden');
    state.enhancementFallbackHintTimer = null;
  }, 4200);
}

// ============ History ============
function addToHistory(text) {
  const time = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  state.history.unshift({ time, text });
  if (state.history.length > 20) state.history.pop();
  renderHistory();
}

function renderHistory() {
  if (state.history.length === 0) {
    el.historyContainer.innerHTML = '<div class="empty-state">No recordings yet</div>';
    return;
  }

  el.historyContainer.innerHTML = '';
  const fragment = document.createDocumentFragment();

  state.history.forEach((item) => {
    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'history-card';
    card.title = 'Click to copy';

    const time = document.createElement('div');
    time.className = 'history-time';
    time.textContent = item.time;

    const text = document.createElement('div');
    text.className = 'history-text';
    text.textContent = item.text;

    card.appendChild(time);
    card.appendChild(text);

    card.addEventListener('click', async () => {
      const copied = await copyResultToClipboard(item.text);
      if (!copied) return;
      card.classList.add('copied');
      setTimeout(() => card.classList.remove('copied'), 450);
    });

    fragment.appendChild(card);
  });

  el.historyContainer.appendChild(fragment);

}

// ============ Settings ============
function normalizeProvider(provider) {
  return provider === PROVIDER_SILICONFLOW ? PROVIDER_SILICONFLOW : PROVIDER_OPENROUTER;
}

function defaultModelForProvider(provider) {
  return provider === PROVIDER_SILICONFLOW ? DEFAULT_SILICONFLOW_MODEL : DEFAULT_OPENROUTER_MODEL;
}

function defaultEnhancementModelForProvider(provider) {
  return provider === PROVIDER_SILICONFLOW
    ? DEFAULT_ENHANCEMENT_SILICONFLOW_MODEL
    : DEFAULT_ENHANCEMENT_OPENROUTER_MODEL;
}

function keyStorageKey(provider) {
  return API_KEY_STORAGE_KEYS[normalizeProvider(provider)];
}

function enhancementKeyStorageKey(provider) {
  return ENHANCEMENT_API_KEY_STORAGE_KEYS[normalizeProvider(provider)];
}

function getStoredApiKey(provider) {
  const storageKey = keyStorageKey(provider);
  if (!storageKey) return '';
  return localStorage.getItem(storageKey) || '';
}

function setStoredApiKey(provider, value) {
  const storageKey = keyStorageKey(provider);
  if (!storageKey) return;
  if (!value) {
    localStorage.removeItem(storageKey);
    return;
  }
  localStorage.setItem(storageKey, value);
}

function getStoredEnhancementApiKey(provider) {
  const storageKey = enhancementKeyStorageKey(provider);
  if (!storageKey) return '';
  return localStorage.getItem(storageKey) || '';
}

function setStoredEnhancementApiKey(provider, value) {
  const storageKey = enhancementKeyStorageKey(provider);
  if (!storageKey) return;
  if (!value) {
    localStorage.removeItem(storageKey);
    return;
  }
  localStorage.setItem(storageKey, value);
}

function cacheCurrentProviderApiKey() {
  const provider = normalizeProvider(state.currentProvider);
  if (!provider || !el.apiKeyInput) return;
  const value = el.apiKeyInput.value || '';
  state.providerApiKeys[provider] = value;
  setStoredApiKey(provider, value);
}

function cacheCurrentEnhancementApiKey() {
  const provider = normalizeProvider(state.enhancementProvider);
  if (!provider || !el.enhancementApiKeyInput) return;
  const value = el.enhancementApiKeyInput.value || '';
  state.enhancementProviderApiKeys[provider] = value;
  setStoredEnhancementApiKey(provider, value);
}

function syncProviderUi(provider) {
  if (el.apiKeyLabel) {
    el.apiKeyLabel.textContent = provider === PROVIDER_SILICONFLOW
      ? 'SiliconFlow API Key'
      : 'OpenRouter API Key';
  }

  if (el.apiKeyInput) {
    el.apiKeyInput.placeholder = provider === PROVIDER_SILICONFLOW ? 'sk-...' : 'sk-or-...';
  }
}

function syncEnhancementProviderUi(provider) {
  if (el.enhancementApiKeyLabel) {
    el.enhancementApiKeyLabel.textContent = provider === PROVIDER_SILICONFLOW
      ? 'Enhancement SiliconFlow API Key (留空则复用 STT Key)'
      : 'Enhancement OpenRouter API Key (留空则复用 STT Key)';
  }

  if (el.enhancementApiKeyInput) {
    el.enhancementApiKeyInput.placeholder = provider === PROVIDER_SILICONFLOW ? 'sk-...' : 'sk-or-...';
  }
}

function updateEnhancementSettingsVisibility() {
  if (!el.enhancementSettings || !el.enhancementSwitch) return;
  el.enhancementSettings.classList.toggle('hidden', !el.enhancementSwitch.checked);
}

function onProviderChange() {
  cacheCurrentProviderApiKey();
  const provider = normalizeProvider(el.providerSelect?.value);
  const currentModel = el.modelInput?.value?.trim() || '';
  const shouldResetModel = !currentModel
    || currentModel === DEFAULT_OPENROUTER_MODEL
    || currentModel === DEFAULT_SILICONFLOW_MODEL;

  state.currentProvider = provider;
  syncProviderUi(provider);
  if (el.apiKeyInput) {
    el.apiKeyInput.value = state.providerApiKeys[provider] || '';
  }
  if (shouldResetModel && el.modelInput) {
    el.modelInput.value = defaultModelForProvider(provider);
  }
}

function onEnhancementProviderChange() {
  cacheCurrentEnhancementApiKey();
  const provider = normalizeProvider(el.enhancementProviderSelect?.value);
  const currentModel = el.enhancementModelInput?.value?.trim() || '';
  const shouldResetModel = !currentModel
    || currentModel === DEFAULT_ENHANCEMENT_OPENROUTER_MODEL
    || currentModel === DEFAULT_ENHANCEMENT_SILICONFLOW_MODEL;

  state.enhancementProvider = provider;
  syncEnhancementProviderUi(provider);
  if (el.enhancementApiKeyInput) {
    el.enhancementApiKeyInput.value = state.enhancementProviderApiKeys[provider] || '';
  }
  if (shouldResetModel && el.enhancementModelInput) {
    el.enhancementModelInput.value = defaultEnhancementModelForProvider(provider);
  }
}

function buildSttConfigFromUi() {
  cacheCurrentProviderApiKey();
  cacheCurrentEnhancementApiKey();
  const provider = normalizeProvider(el.providerSelect?.value || state.currentProvider);
  const enhancementProvider = normalizeProvider(el.enhancementProviderSelect?.value || state.enhancementProvider);
  const apiKey = state.providerApiKeys[provider] || '';
  const enhancementApiKey = state.enhancementProviderApiKeys[enhancementProvider] || '';
  const model = (el.modelInput?.value || '').trim() || defaultModelForProvider(provider);
  const enhancementModel =
    (el.enhancementModelInput?.value || '').trim() || defaultEnhancementModelForProvider(enhancementProvider);
  const enhancementPrompt = (el.enhancementPromptInput?.value || '').trim() || DEFAULT_ENHANCEMENT_PROMPT;

  return {
    provider,
    api_key: apiKey,
    model,
    base_url: '',
    auto_write: el.autoWriteSwitch ? el.autoWriteSwitch.checked : false,
    record_mode: state.recordMode || 'toggle',
    enhancement_enabled: el.enhancementSwitch ? el.enhancementSwitch.checked : false,
    enhancement_provider: enhancementProvider,
    enhancement_base_url: '',
    enhancement_api_key: enhancementApiKey,
    enhancement_model: enhancementModel,
    enhancement_prompt: enhancementPrompt
  };
}

function formatProviderName(provider) {
  return normalizeProvider(provider) === PROVIDER_SILICONFLOW ? 'SiliconFlow' : 'OpenRouter';
}

function renderConnectionTestResult(kind, text) {
  if (!el.testConnectionResult) return;
  el.testConnectionResult.classList.remove('hidden', 'loading', 'success', 'error');
  el.testConnectionResult.classList.add(kind);
  el.testConnectionResult.textContent = text;
}

async function testConnection() {
  if (!el.testConnectionBtn) return;

  el.testConnectionBtn.disabled = true;
  const originalText = el.testConnectionBtn.textContent;
  el.testConnectionBtn.textContent = 'Testing...';
  renderConnectionTestResult('loading', '正在测试连接，请稍候...');

  try {
    await syncConfigFromUi();
    const result = await invoke('test_connection');
    const providerName = formatProviderName(result.provider);
    const details = `${providerName} / ${result.model} / ${result.latency_ms}ms`;

    if (result.success) {
      renderConnectionTestResult('success', `✅ ${result.message} (${details})`);
    } else {
      renderConnectionTestResult('error', `❌ ${result.message} (${details})`);
    }
  } catch (e) {
    renderConnectionTestResult('error', `❌ 连接测试失败: ${e.toString()}`);
  } finally {
    el.testConnectionBtn.disabled = false;
    el.testConnectionBtn.textContent = originalText || 'Test Connection';
  }
}

async function syncConfigFromUi() {
  if (!el.providerSelect || !el.apiKeyInput || !el.modelInput) return;
  const config = buildSttConfigFromUi();
  await invoke('save_stt_config', { config });
}

async function loadConfig() {
  try {
    const config = await invoke('get_stt_config');
    const provider = normalizeProvider(config.provider);
    const enhancementProvider = normalizeProvider(config.enhancement_provider || PROVIDER_OPENROUTER);
    state.providerApiKeys[PROVIDER_OPENROUTER] = getStoredApiKey(PROVIDER_OPENROUTER);
    state.providerApiKeys[PROVIDER_SILICONFLOW] = getStoredApiKey(PROVIDER_SILICONFLOW);
    state.enhancementProviderApiKeys[PROVIDER_OPENROUTER] = getStoredEnhancementApiKey(PROVIDER_OPENROUTER);
    state.enhancementProviderApiKeys[PROVIDER_SILICONFLOW] = getStoredEnhancementApiKey(PROVIDER_SILICONFLOW);

    if (config.api_key && !state.providerApiKeys[provider]) {
      state.providerApiKeys[provider] = config.api_key;
      setStoredApiKey(provider, config.api_key);
    }
    if (config.enhancement_api_key && !state.enhancementProviderApiKeys[enhancementProvider]) {
      state.enhancementProviderApiKeys[enhancementProvider] = config.enhancement_api_key;
      setStoredEnhancementApiKey(enhancementProvider, config.enhancement_api_key);
    }

    state.currentProvider = provider;
    state.enhancementProvider = enhancementProvider;
    state.sttConfig = { ...config, provider, enhancement_provider: enhancementProvider };

    if (el.providerSelect) el.providerSelect.value = provider;
    syncProviderUi(provider);
    if (el.apiKeyInput) el.apiKeyInput.value = state.providerApiKeys[provider] || '';
    if (config.model && el.modelInput) {
      el.modelInput.value = config.model;
    } else if (el.modelInput) {
      el.modelInput.value = defaultModelForProvider(provider);
    }

    if (el.autoWriteSwitch) {
      el.autoWriteSwitch.checked = !!config.auto_write;
      checkAccessibility();
    }

    if (el.enhancementSwitch) {
      el.enhancementSwitch.checked = !!config.enhancement_enabled;
      updateEnhancementSettingsVisibility();
    }
    if (el.enhancementProviderSelect) {
      el.enhancementProviderSelect.value = enhancementProvider;
    }
    syncEnhancementProviderUi(enhancementProvider);
    if (el.enhancementApiKeyInput) {
      el.enhancementApiKeyInput.value = state.enhancementProviderApiKeys[enhancementProvider] || '';
    }
    if (el.enhancementModelInput) {
      el.enhancementModelInput.value = (config.enhancement_model || '').trim()
        || defaultEnhancementModelForProvider(enhancementProvider);
    }
    if (el.enhancementPromptInput) {
      el.enhancementPromptInput.value = (config.enhancement_prompt || '').trim() || DEFAULT_ENHANCEMENT_PROMPT;
    }

    const recordMode = config.record_mode || localStorage.getItem('aitotype_record_mode') || 'toggle';
    state.recordMode = recordMode;
    localStorage.setItem('aitotype_record_mode', recordMode);
    if (el.recordModeSwitch) {
      el.recordModeSwitch.checked = recordMode === 'hold';
    }
    updateInstructionText();
  } catch (e) { }
}

async function saveConfig(e) {
  e.preventDefault();
  if (el.settingsSaveBtn) {
    el.settingsSaveBtn.disabled = true;
    el.settingsSaveBtn.textContent = 'Saving...';
  }

  const config = buildSttConfigFromUi();

  try {
    await invoke('save_stt_config', { config });
    await loadConfig();
    if (el.settingsStatus) el.settingsStatus.textContent = '';
    if (el.settingsSaveBtn) {
      el.settingsSaveBtn.textContent = 'Saved';
      window.setTimeout(() => {
        if (el.settingsSaveBtn) {
          el.settingsSaveBtn.textContent = 'Save Changes';
          el.settingsSaveBtn.disabled = false;
        }
      }, 1200);
    }
  } catch (e) {
    if (el.settingsStatus) {
      el.settingsStatus.textContent = 'Save failed';
      el.settingsStatus.style.color = '#FF453A';
    }
    if (el.settingsSaveBtn) {
      el.settingsSaveBtn.textContent = 'Save Changes';
      el.settingsSaveBtn.disabled = false;
    }
  }
}

function onApiKeyInput() {
  const provider = normalizeProvider(el.providerSelect?.value || state.currentProvider);
  if (!provider || !el.apiKeyInput) return;
  const value = el.apiKeyInput.value || '';
  state.providerApiKeys[provider] = value;
}

function onEnhancementApiKeyInput() {
  const provider = normalizeProvider(el.enhancementProviderSelect?.value || state.enhancementProvider);
  if (!provider || !el.enhancementApiKeyInput) return;
  const value = el.enhancementApiKeyInput.value || '';
  state.enhancementProviderApiKeys[provider] = value;
}

// ============ Shortcut Logic ============
function normalizeShortcut(modifiers, rawKey) {
  const ordered = ['Command', 'Ctrl', 'Alt', 'Shift']
    .filter((name) => modifiers.includes(name))
    .map((name) => {
      if (name === 'Command') return 'Cmd';
      if (name === 'Ctrl') return 'Control';
      return name;
    });

  let key = rawKey;
  if (!key) return null;

  const specialMap = {
    ' ': 'Space',
    escape: 'Esc',
    enter: 'Enter',
    tab: 'Tab',
    backspace: 'Backspace',
    delete: 'Delete',
    arrowup: 'Up',
    arrowdown: 'Down',
    arrowleft: 'Left',
    arrowright: 'Right'
  };
  key = specialMap[key.toLowerCase()] || key;

  if (key.length === 1) key = key.toUpperCase();
  return [...ordered, key].join('+');
}

async function setShortcut(shortcut) {
  if (!shortcut || shortcut.trim().length === 0) return;
  const ready = await ensureShortcutPluginReady();
  if (!ready) return;

  try {
    await invoke('update_shortcut', { shortcut });
    localStorage.setItem('aitotype_shortcut', shortcut);
  } catch (e) { console.error('Shortcut update failed', e); }
}

async function disableGlobalShortcut() {
  const ready = await ensureShortcutPluginReady();
  if (!ready) return;

  try {
    await invoke('update_shortcut', { shortcut: '' });
  } catch (e) {
    console.error('Disable shortcut failed', e);
  }
}

function getSavedShortcut() {
  const stored = localStorage.getItem('aitotype_shortcut');
  if (/windows/i.test(navigator.userAgent || navigator.platform || '') && (!stored || stored === 'Alt+Space')) {
    localStorage.setItem('aitotype_shortcut', DEFAULT_SHORTCUT);
    return DEFAULT_SHORTCUT;
  }
  return stored || DEFAULT_SHORTCUT;
}

async function ensureShortcutPluginReady(maxAttempts = 30, delayMs = 100) {
  if (state.shortcutPluginReady) return true;

  for (let i = 0; i < maxAttempts; i++) {
    try {
      const ready = await invoke('is_shortcut_ready');
      if (ready) {
        state.shortcutPluginReady = true;
        return true;
      }
    } catch (e) {
      console.error('Check shortcut plugin ready failed', e);
      break;
    }

    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }

  console.warn('Global shortcut plugin is not ready, skip shortcut registration');
  return false;
}

async function initShortcutRecorder() {
  if (!el.shortcutRecorder) return;

  const saved = getSavedShortcut();
  if (el.shortcutLabel) el.shortcutLabel.textContent = saved;
  await setShortcut(saved);

  if (el.shortcutLabel) el.shortcutLabel.style.opacity = 1;

  const startCapture = async () => {
    if (state.shortcutCaptureActive) return;
    state.shortcutCaptureActive = true;

    const previousShortcut = getSavedShortcut();
    el.shortcutRecorder.classList.add('recording');
    if (el.shortcutLabel) el.shortcutLabel.textContent = 'Press keys...';
    await disableGlobalShortcut();

    const handler = async (e) => {
      e.preventDefault(); e.stopPropagation();

      if (e.key === 'Escape') {
        if (el.shortcutLabel) el.shortcutLabel.textContent = previousShortcut;
        el.shortcutRecorder.classList.remove('recording');
        state.shortcutCaptureActive = false;
        setShortcut(previousShortcut);
        window.removeEventListener('keydown', handler);
        return;
      }

      const modifiers = [];
      if (e.metaKey) modifiers.push('Command');
      if (e.ctrlKey) modifiers.push('Ctrl');
      if (e.altKey) modifiers.push('Alt');
      if (e.shiftKey) modifiers.push('Shift');

      let key = e.key;

      if (['Control', 'Alt', 'Shift', 'Meta', 'Command'].includes(key)) return;

      const s = normalizeShortcut(modifiers, key);
      if (!s) return;
      if (el.shortcutLabel) el.shortcutLabel.textContent = s;
      el.shortcutRecorder.classList.remove('recording');
      state.shortcutCaptureActive = false;
      await setShortcut(s);
      window.removeEventListener('keydown', handler);
    };
    window.addEventListener('keydown', handler);
  };

  el.shortcutRecorder.addEventListener('click', startCapture);

  el.shortcutRecorder.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      startCapture();
    }
  });
}

// ============ Init ============
function updateInstructionText() {
  if (state.status !== 'idle') return;
  if (el.instructionText) {
    el.instructionText.textContent = state.recordMode === 'hold'
      ? 'Hold shortcut to speak'
      : 'Tap orb to capture';
  }
}

async function init() {
  // Global shortcut event from Rust
  if (listen) {
    state.shortcutUnlisten = await listen('toggle-recording-event', (event) => {
      if (state.shortcutCaptureActive) return;
      const payload = event?.payload || {};
      const action = payload.action || 'toggle';
      const background = payload.background ?? false;

      if (action === 'toggle') {
        const now = Date.now();
        if (now - state.lastShortcutToggleAt < 450) return;
        state.lastShortcutToggleAt = now;
        state.pendingShortcutContext = payload;
        toggleRecording();
      } else if (action === 'start') {
        startRecordingOnly(background);
      } else if (action === 'stop') {
        stopAndTranscribeOnly();
      }
    });

    await listen('enhancement-fallback-event', (event) => {
      const payload = event?.payload || {};
      showEnhancementFallbackHint(payload.reason);
    });
  }

  // Shortcut
  await initShortcutRecorder();

  // Navigation
  el.tabs.forEach(tab => {
    tab.addEventListener('click', () => switchView(tab.dataset.view));
  });

  // Recorder
  if (el.recordTrigger) {
    el.recordTrigger.addEventListener('click', toggleRecording);
  }

  // Result
  if (el.closeResultBtn) el.closeResultBtn.addEventListener('click', hideResult);
  if (el.copyBtn) {
    el.copyBtn.addEventListener('click', async () => {
      if (state.lastResult) {
        const copied = await copyResultToClipboard(state.lastResult);
        if (!copied) return;

        el.copyBtn.classList.add('copied');
        setTimeout(() => el.copyBtn.classList.remove('copied'), 600);
      }
    });
  }

  // Settings
  if (el.settingsForm) el.settingsForm.addEventListener('submit', saveConfig);
  if (el.providerSelect) el.providerSelect.addEventListener('change', onProviderChange);
  if (el.apiKeyInput) el.apiKeyInput.addEventListener('input', onApiKeyInput);
  if (el.enhancementSwitch) {
    el.enhancementSwitch.addEventListener('change', () => {
      updateEnhancementSettingsVisibility();
    });
  }
  if (el.enhancementProviderSelect) {
    el.enhancementProviderSelect.addEventListener('change', onEnhancementProviderChange);
  }
  if (el.enhancementApiKeyInput) {
    el.enhancementApiKeyInput.addEventListener('input', onEnhancementApiKeyInput);
  }
  if (el.testConnectionBtn) {
    el.testConnectionBtn.addEventListener('click', testConnection);
  }

  if (el.recordModeSwitch) {
    el.recordModeSwitch.addEventListener('change', async () => {
      state.recordMode = el.recordModeSwitch.checked ? 'hold' : 'toggle';
      localStorage.setItem('aitotype_record_mode', state.recordMode);
      updateInstructionText();
      try {
        await syncConfigFromUi();
      } catch (e) {
        console.error('Sync record mode failed', e);
      }
    });
  }

  // Load Config
  loadConfig();

  // Load Auto Copy
  const savedAutoCopy = localStorage.getItem('aitotype_autocopy');
  if (el.autoCopySwitch) {
    el.autoCopySwitch.checked = savedAutoCopy === null ? true : savedAutoCopy === 'true';
  }

  if (el.autoCopySwitch) {
    el.autoCopySwitch.addEventListener('change', (e) => {
      localStorage.setItem('aitotype_autocopy', e.target.checked);
    });
  }

  if (el.autoWriteSwitch) {
    el.autoWriteSwitch.addEventListener('change', async (e) => {
      if (e.target.checked) {
        // Proactively request if not trusted
        const isTrusted = await invoke('check_accessibility_permissions');
        if (!isTrusted) {
          await invoke('request_accessibility_permissions');
        }
      }
      syncConfigFromUi();
      checkAccessibility();
    });
  }

  document.body.classList.add('loaded'); // Fade in
}

document.addEventListener('DOMContentLoaded', init);
