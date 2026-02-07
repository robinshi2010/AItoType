const { listen } = window.__TAURI__.event || { listen: async () => () => {} };

const pill = document.getElementById('overlay-pill');
const statusText = document.getElementById('status-text');
const statusSubtext = document.getElementById('status-subtext');

function setStatus(status) {
  const isTranscribing = status === 'transcribing';
  pill.classList.toggle('transcribing', isTranscribing);
  pill.classList.toggle('recording', !isTranscribing);
  statusText.textContent = isTranscribing ? 'Transcribing' : 'Recording';
  statusSubtext.textContent = isTranscribing ? 'Processing speech...' : 'Listening...';
}

async function init() {
  await listen('overlay-status', (event) => {
    const status = event?.payload?.status || 'recording';
    setStatus(status);
  });

  setStatus('recording');
}

init();
