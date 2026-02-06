/**
 * AitoType Spatial Controller
 * "Glass Monolith" Edition
 */

const { invoke } = window.__TAURI__.core;
// Safely try to get listen, fallback if not available
const { listen } = window.__TAURI__.event || { listen: () => { } };

// ============ State ============
const state = {
  status: 'idle',
  lastResult: '',
  history: [],
  audioLevelTimer: null,
  shortcutUnlisten: null,
  shortcutCaptureActive: false,
  pendingShortcutContext: null,
  backgroundSession: false
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
  closeResultBtn: document.getElementById('reset-result-btn'),
  copyBtn: document.getElementById('copy-btn'),
  autoCopySwitch: document.getElementById('auto-copy-switch'),

  // Settings
  providerSelect: document.getElementById('provider-select'),
  apiKeyInput: document.getElementById('api-key-input'),
  modelInput: document.getElementById('model-input'),
  settingsForm: document.getElementById('settings-form'),
  settingsStatus: document.getElementById('settings-status'),
  settingsSaveBtn: document.querySelector('#settings-form button[type="submit"]'),

  // Shortcut (Placeholder)
  shortcutRecorder: document.getElementById('shortcut-recorder'),
  shortcutLabel: document.getElementById('shortcut-label'),

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
      el.instructionText.textContent = msg || 'Tap orb to capture';
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

// ============ Recorder Logic ============
async function toggleRecording() {
  if (state.status === 'transcribing') return;

  if (state.status === 'recording') {
    // Stop & Transcribe
    try {
      updateStatus('transcribing');
      if (state.backgroundSession) {
        await invoke('show_overlay_status', { status: 'transcribing' });
      }
      const result = await invoke('stop_and_transcribe');

      state.lastResult = result;
      addToHistory(result);

      // Auto-Copy
      if (el.autoCopySwitch && el.autoCopySwitch.checked) {
        await copyResultToClipboard(result);
      }

      // Auto-paste back to the active app only for background shortcut sessions
      if (state.backgroundSession) {
        try {
          await invoke('paste_text', { text: result });
        } catch (e) { console.error('Paste failed', e); }
      }

      updateStatus('success', result);

      if (state.backgroundSession) {
        await invoke('hide_overlay');
      }
      state.backgroundSession = false;
    } catch (e) {
      console.error(e);
      updateStatus('error', e.toString());
      if (state.backgroundSession) {
        await invoke('hide_overlay');
      }
      state.backgroundSession = false;
    }
  } else {
    // Start Recording
    hideResult();
    try {
      const fromBackgroundShortcut = Boolean(state.pendingShortcutContext?.background);
      state.backgroundSession = fromBackgroundShortcut;
      state.pendingShortcutContext = null;

      await invoke('start_recording');
      updateStatus('recording');

      if (state.backgroundSession) {
        await invoke('show_overlay_status', { status: 'recording' });
      }
    } catch (e) {
      updateStatus('error', e.toString());
      if (state.backgroundSession) {
        await invoke('hide_overlay');
      }
      state.backgroundSession = false;
      state.pendingShortcutContext = null;
    }
  }
}

// ============ Audio Animation (Simulated) ============
function startAudioAnim() {
  // Can add volume visualization here later
}
function stopsAudioAnim() { }

// ============ Result Sheet ============
function showResult(text) {
  el.resultText.textContent = text;
  el.resultSheet.classList.remove('hidden');
}

function hideResult() {
  el.resultSheet.classList.add('hidden');
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

  el.historyContainer.innerHTML = state.history.map(item => `
    <div class="history-card" style="padding:16px; margin-bottom:10px; background:rgba(255,255,255,0.05); border-radius:12px; font-size:14px; color:rgba(255,255,255,0.8);">
      <div style="font-size:11px; color:rgba(255,255,255,0.4); margin-bottom:4px">${item.time}</div>
      ${item.text}
    </div>
  `).join('');
}

// ============ Settings ============
async function loadConfig() {
  try {
    const config = await invoke('get_stt_config');
    if (config.provider) el.providerSelect.value = config.provider;
    if (config.api_key) el.apiKeyInput.value = config.api_key;
    if (config.model) el.modelInput.value = config.model;
  } catch (e) { }
}

async function saveConfig(e) {
  e.preventDefault();
  if (el.settingsSaveBtn) {
    el.settingsSaveBtn.disabled = true;
    el.settingsSaveBtn.textContent = 'Saving...';
  }

  const config = {
    provider: el.providerSelect.value,
    api_key: el.apiKeyInput.value,
    model: el.modelInput.value,
    base_url: ''
  };

  try {
    await invoke('save_stt_config', { config });
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

  try {
    await invoke('update_shortcut', { shortcut });
    localStorage.setItem('aitotype_shortcut', shortcut);
  } catch (e) { console.error('Shortcut update failed', e); }
}

async function disableGlobalShortcut() {
  try {
    await invoke('update_shortcut', { shortcut: '' });
  } catch (e) {
    console.error('Disable shortcut failed', e);
  }
}

function initShortcutRecorder() {
  if (!el.shortcutRecorder) return;

  const saved = localStorage.getItem('aitotype_shortcut') || 'Alt+Space';
  if (el.shortcutLabel) el.shortcutLabel.textContent = saved;
  setShortcut(saved);

  if (el.shortcutLabel) el.shortcutLabel.style.opacity = 1;

  const startCapture = async () => {
    if (state.shortcutCaptureActive) return;
    state.shortcutCaptureActive = true;

    const previousShortcut = localStorage.getItem('aitotype_shortcut') || 'Alt+Space';
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
async function init() {
  // Global shortcut event from Rust
  if (listen) {
    state.shortcutUnlisten = await listen('toggle-recording-event', (event) => {
      if (state.shortcutCaptureActive) return;
      state.pendingShortcutContext = event?.payload || null;
      toggleRecording();
    });
  }

  // Shortcut
  initShortcutRecorder();

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

  document.body.classList.add('loaded'); // Fade in
}

document.addEventListener('DOMContentLoaded', init);
