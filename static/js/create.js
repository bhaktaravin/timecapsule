function defaultUnlockAt() {
  const date = new Date();
  date.setFullYear(date.getFullYear() + 1);
  date.setMinutes(date.getMinutes() - date.getTimezoneOffset());
  return date.toISOString().slice(0, 16);
}

function formatDate(iso) {
  return new Date(iso).toLocaleString(undefined, {
    dateStyle: "full",
    timeStyle: "short",
  });
}

function hideAllResults() {
  document.getElementById("success-card").classList.add("hidden");
  document.getElementById("error-card").classList.add("hidden");
  stopCountdown();
}

let lastUnlockLink = "";
let lastUnlockToken = "";
let lastUnlockAt = null;
let countdownTimer = null;

function stopCountdown() {
  if (countdownTimer) {
    clearInterval(countdownTimer);
    countdownTimer = null;
  }
  document.getElementById("countdown-panel").classList.add("hidden");
  document.getElementById("open-test-btn").classList.add("hidden");
}

function showSuccess(data, { isTest = false } = {}) {
  lastUnlockToken = data.unlock_token;
  lastUnlockLink = `${window.location.origin}/unlock/?token=${encodeURIComponent(data.unlock_token)}`;
  lastUnlockAt = new Date(data.unlock_at);

  document.getElementById("result-recipient").textContent = data.recipient_email;
  document.getElementById("result-unlock-at").textContent = formatDate(data.unlock_at);
  document.getElementById("result-link").textContent = lastUnlockLink;
  document.getElementById("result-token").textContent = lastUnlockToken;

  document.getElementById("create-form-card").classList.add("hidden");
  document.getElementById("success-card").classList.remove("hidden");

  if (isTest) {
    startCountdown(lastUnlockAt);
    document.getElementById("open-test-btn").classList.remove("hidden");
  } else {
    stopCountdown();
  }
}

function startCountdown(unlockAt) {
  stopCountdown();

  const panel = document.getElementById("countdown-panel");
  const text = document.getElementById("countdown-text");
  panel.classList.remove("hidden");

  const tick = () => {
    const secondsLeft = Math.max(0, Math.ceil((unlockAt.getTime() - Date.now()) / 1000));

    if (secondsLeft === 0) {
      text.textContent = "Ready to open — click “Open test capsule” or use the unlock link.";
      stopCountdown();
      panel.classList.remove("hidden");
      document.getElementById("open-test-btn").classList.remove("hidden");
      return;
    }

    text.textContent = `Unlocks in ${secondsLeft} second${secondsLeft === 1 ? "" : "s"}…`;
  };

  tick();
  countdownTimer = setInterval(tick, 500);
}

async function createCapsule(payload) {
  const response = await fetch("/api/capsules", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  const data = await response.json();
  if (!response.ok) {
    throw new Error(data.error || "Request failed");
  }
  return data;
}

async function createTestCapsule() {
  const response = await fetch("/api/dev/test-capsule", { method: "POST" });
  const data = await response.json();
  if (!response.ok) {
    throw new Error(data.error || "Request failed");
  }
  return data;
}

document.getElementById("unlock_at").value = defaultUnlockAt();

document.getElementById("create-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  hideAllResults();

  const submitBtn = document.getElementById("submit-btn");
  submitBtn.disabled = true;
  submitBtn.textContent = "Sealing...";

  try {
    const data = await createCapsule({
      message: document.getElementById("message").value.trim(),
      recipient_email: document.getElementById("recipient_email").value.trim(),
      unlock_at: new Date(document.getElementById("unlock_at").value).toISOString(),
    });
    showSuccess(data);
  } catch (error) {
    document.getElementById("error-text").textContent = error.message;
    document.getElementById("error-card").classList.remove("hidden");
  } finally {
    submitBtn.disabled = false;
    submitBtn.textContent = "Seal message";
  }
});

document.getElementById("test-btn").addEventListener("click", async () => {
  hideAllResults();

  const testBtn = document.getElementById("test-btn");
  testBtn.disabled = true;
  testBtn.textContent = "Creating test...";

  try {
    const data = await createTestCapsule();
    showSuccess(data, { isTest: true });
  } catch (error) {
    document.getElementById("error-text").textContent = error.message;
    document.getElementById("error-card").classList.remove("hidden");
  } finally {
    testBtn.disabled = false;
    testBtn.textContent = "Create test unlock";
  }
});

async function copyText(text) {
  await navigator.clipboard.writeText(text);
}

document.getElementById("copy-link-btn").addEventListener("click", () => copyText(lastUnlockLink));
document.getElementById("copy-token-btn").addEventListener("click", () => copyText(lastUnlockToken));
document.getElementById("open-test-btn").addEventListener("click", () => {
  if (lastUnlockLink) {
    window.location.href = lastUnlockLink;
  }
});

document.getElementById("new-btn").addEventListener("click", () => {
  document.getElementById("create-form").reset();
  document.getElementById("unlock_at").value = defaultUnlockAt();
  document.getElementById("create-form-card").classList.remove("hidden");
  document.getElementById("success-card").classList.add("hidden");
  document.getElementById("error-card").classList.add("hidden");
  stopCountdown();
});

async function loadDevPanel() {
  try {
    const response = await fetch("/api/dev/enabled");
    const data = await response.json();
    if (data.enabled) {
      document.getElementById("dev-panel").classList.remove("hidden");
    }
  } catch {
    // dev panel stays hidden
  }
}

loadDevPanel();
