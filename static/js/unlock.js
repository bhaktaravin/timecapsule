function formatDate(iso) {
  return new Date(iso).toLocaleString(undefined, {
    dateStyle: "full",
    timeStyle: "short",
  });
}

function tokenFromQuery() {
  const params = new URLSearchParams(window.location.search);
  return params.get("token")?.trim() || "";
}

function hideResults() {
  document.getElementById("sealed-card").classList.add("hidden");
  document.getElementById("opened-card").classList.add("hidden");
  document.getElementById("error-card").classList.add("hidden");
}

async function openCapsule(token) {
  hideResults();

  const unlockBtn = document.getElementById("unlock-btn");
  unlockBtn.disabled = true;
  unlockBtn.textContent = "Opening...";

  try {
    const response = await fetch(`/api/unlock/${encodeURIComponent(token)}`);
    const data = await response.json();

    if (response.status === 403 && data.error?.includes("sealed until")) {
      const match = data.error.match(/sealed until (.+)$/);
      const when = match ? formatDate(match[1]) : "the scheduled date";
      document.getElementById("sealed-text").textContent =
        `This capsule is still sealed. It unlocks on ${when}.`;
      document.getElementById("sealed-card").classList.remove("hidden");
      return;
    }

    if (!response.ok) {
      throw new Error(data.error || "Request failed");
    }

    document.getElementById("opened-meta").textContent =
      `For ${data.recipient_email} · Opened ${formatDate(data.opened_at)}`;
    document.getElementById("message-body").textContent = data.message;
    document.getElementById("opened-card").classList.remove("hidden");
  } catch (error) {
    document.getElementById("error-text").textContent = error.message;
    document.getElementById("error-card").classList.remove("hidden");
  } finally {
    unlockBtn.disabled = false;
    unlockBtn.textContent = "Open capsule";
  }
}

const prefilled = tokenFromQuery();
if (prefilled) {
  document.getElementById("token").value = prefilled;
}

document.getElementById("unlock-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const token = document.getElementById("token").value.trim();
  if (!token) return;
  await openCapsule(token);
});

if (prefilled) {
  openCapsule(prefilled);
}
