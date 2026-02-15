const { invoke } = window.__TAURI__.core || { invoke: async () => '' };
const { listen } = window.__TAURI__.event || { listen: async () => () => { } };

const pill = document.getElementById('overlay-pill');
const statusText = document.getElementById('status-text');
const statusSubtext = document.getElementById('status-subtext');
const waveformBars = Array.from(document.querySelectorAll('.overlay-waveform .bar'));

let overlayLevelTimer = null;
let currentStatus = 'recording';
let currentDeviceName = '';

function updateSubtext() {
  if (!statusSubtext) return;

  if (currentStatus === 'transcribing') {
    statusSubtext.textContent = currentDeviceName
      ? `🎙 ${currentDeviceName} · Processing speech...`
      : 'Processing speech...';
    return;
  }

  statusSubtext.textContent = currentDeviceName
    ? `🎙 ${currentDeviceName}`
    : 'Listening...';
}

async function queryAndShowDevice() {
  try {
    const name = String(await invoke('get_input_device_name') || '').trim();
    currentDeviceName = name;
  } catch (_) {
    currentDeviceName = '';
  }
  updateSubtext();
}

function startOverlayAnim() {
  if (overlayLevelTimer || waveformBars.length === 0) return;

  overlayLevelTimer = setInterval(async () => {
    try {
      const level = Number(await invoke('get_audio_level'));
      const safeLevel = Number.isFinite(level) ? Math.max(0, Math.min(1, level)) : 0;
      const tick = Date.now() / 160;

      waveformBars.forEach((bar, index) => {
        const wave = 0.82 + Math.sin(tick + index * 1.1) * 0.18;
        const height = Math.max(4, Math.min(24, 4 + safeLevel * 20 * wave));
        bar.style.height = `${height.toFixed(1)}px`;
        bar.style.opacity = `${(0.35 + safeLevel * 0.65).toFixed(3)}`;
      });
    } catch (_) { }
  }, 80);
}

function stopOverlayAnim() {
  if (overlayLevelTimer) {
    clearInterval(overlayLevelTimer);
    overlayLevelTimer = null;
  }

  waveformBars.forEach((bar) => {
    bar.style.height = '4px';
    bar.style.opacity = '';
  });
}

document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    stopOverlayAnim();
    return;
  }
  if (currentStatus === 'recording') {
    startOverlayAnim();
  }
});

function setStatus(status) {
  currentStatus = status === 'transcribing' ? 'transcribing' : 'recording';
  const isTranscribing = currentStatus === 'transcribing';

  pill.classList.toggle('transcribing', isTranscribing);
  pill.classList.toggle('recording', !isTranscribing);
  statusText.textContent = isTranscribing ? 'Transcribing' : 'Recording';

  if (isTranscribing) {
    stopOverlayAnim();
  } else {
    startOverlayAnim();
    queryAndShowDevice();
  }

  updateSubtext();
}

async function init() {
  await listen('overlay-status', (event) => {
    const status = String(event?.payload?.status || 'recording').trim().toLowerCase();
    setStatus(status);
  });

  setStatus('recording');
  queryAndShowDevice();
}

init();
