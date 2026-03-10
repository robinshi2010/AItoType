const { invoke } = window.__TAURI__?.core || { invoke: async () => '' };
const { listen } = window.__TAURI__?.event || { listen: async () => () => { } };

const pill = document.getElementById('overlay-pill');
const statusText = document.getElementById('status-text');
const statusSubtext = document.getElementById('status-subtext');
const waveformBars = Array.from(document.querySelectorAll('.overlay-waveform .bar'));

let overlayLevelPollTimer = null;
let overlayRenderFrame = null;
let currentStatus = 'recording';
let currentDeviceName = '';
let targetAudioLevel = 0;
let visualAudioLevel = 0;

function normalizeWaveLevel(level) {
  const safeLevel = Number.isFinite(level) ? Math.max(0, Math.min(1, level)) : 0;
  const boosted = Math.min(1, Math.pow(safeLevel, 0.58) * 1.45);
  const threshold = 0.12;

  if (boosted <= threshold) {
    return 0;
  }

  return Math.min(1, (boosted - threshold) / (1 - threshold));
}

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
  if ((overlayLevelPollTimer || overlayRenderFrame) || waveformBars.length === 0) return;

  overlayLevelPollTimer = setInterval(async () => {
    try {
      const level = Number(await invoke('get_audio_level'));
      targetAudioLevel = normalizeWaveLevel(level);
    } catch (_) { }
  }, 60);

  const render = () => {
    const desiredLevel = currentStatus === 'recording' ? targetAudioLevel : 0;
    const smoothing = desiredLevel > visualAudioLevel ? 0.34 : 0.14;
    visualAudioLevel += (desiredLevel - visualAudioLevel) * smoothing;

    const tick = performance.now() / 150;
    const barCount = Math.max(1, waveformBars.length - 1);
    const isSilent = visualAudioLevel < 0.018;

    waveformBars.forEach((bar, index) => {
      if (isSilent) {
        bar.style.height = '6px';
        bar.style.opacity = '0.3';
        return;
      }

      const centerBias = 1 - Math.abs(index - barCount / 2) / Math.max(1, barCount / 2);
      const envelope = 0.7 + centerBias * 0.55;
      const ripple =
        Math.sin(tick + index * 0.72) * 0.42 +
        Math.sin(tick * 1.9 + index * 1.37) * 0.33 +
        Math.sin(tick * 3.1 - index * 0.58) * 0.14;
      const pulse = Math.max(0, Math.sin(tick * 2.4 + index * 0.64));
      const motion = 0.58 + ripple * 0.38 + pulse * 0.22;
      const intensity = visualAudioLevel;
      const height = 6 + envelope * 5 + intensity * (10 + motion * 22);
      const opacity = 0.34 + intensity * 0.66;

      bar.style.height = `${height.toFixed(1)}px`;
      bar.style.opacity = `${opacity.toFixed(3)}`;
    });

    overlayRenderFrame = requestAnimationFrame(render);
  };

  overlayRenderFrame = requestAnimationFrame(render);
}

function stopOverlayAnim() {
  if (overlayLevelPollTimer) {
    clearInterval(overlayLevelPollTimer);
    overlayLevelPollTimer = null;
  }

  if (overlayRenderFrame) {
    cancelAnimationFrame(overlayRenderFrame);
    overlayRenderFrame = null;
  }

  targetAudioLevel = 0;
  visualAudioLevel = 0;

  waveformBars.forEach((bar) => {
    bar.style.height = '6px';
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
