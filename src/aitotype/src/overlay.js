const { listen } = window.__TAURI__.event || { listen: async () => () => {} };

const card = document.getElementById('overlay-card');
const statusText = document.getElementById('status-text');

function setStatus(status) {
  const isTranscribing = status === 'transcribing';
  card.classList.toggle('transcribing', isTranscribing);
  card.classList.toggle('recording', !isTranscribing);
  statusText.textContent = isTranscribing ? 'Transcribing' : 'Recording';
}

async function init() {
  await listen('overlay-status', (event) => {
    const status = event?.payload?.status || 'recording';
    setStatus(status);
  });

  setStatus('recording');
}

init();
