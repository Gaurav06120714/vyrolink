// VYRO LINK web chat helpers (ADR-040). Plain browser APIs only; no network
// calls here (the internet mailbox is called from Dart). Nothing is logged.
(() => {
  'use strict';

  // Created before the app starts; vlboot.js resolves it once the core loads.
  window.vlcoreReady = new Promise((resolve, reject) => { window.vlcoreLoaded = [resolve, reject]; });

  // --- Storage: one IndexedDB object store in this site's own browser storage.
  const DB = 'vyrolink', STORE = 'kv';
  let opened;
  function db() {
    opened ??= new Promise((resolve, reject) => {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE);
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    return opened;
  }
  async function run(mode, fn) {
    const d = await db();
    return new Promise((resolve, reject) => {
      const t = d.transaction(STORE, mode);
      const req = fn(t.objectStore(STORE));
      t.oncomplete = () => resolve(req ? req.result : undefined);
      t.onerror = () => reject(t.error);
      t.onabort = () => reject(t.error);
    });
  }
  window.vlStore = {
    get: (key) => run('readonly', (s) => s.get(key)).then((v) => v ?? null),
    put: (key, value) => run('readwrite', (s) => s.put(value, key)),
    clear: () => run('readwrite', (s) => s.clear()),
    // Asks the browser not to evict this site's data under storage pressure.
    persist: () => (navigator.storage && navigator.storage.persist ? navigator.storage.persist() : Promise.resolve(false)),
  };

  // --- Files.
  window.vlPick = (accept) => new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.onchange = async () => {
      const f = input.files && input.files[0];
      if (!f) return resolve(null);
      resolve({ name: f.name.slice(0, 64), size: f.size, data: new Uint8Array(await f.arrayBuffer()) });
    };
    input.oncancel = () => resolve(null);
    input.click();
  });

  // Re-encodes a photo as a JPEG of at most maxBytes. Drawing the pixels to a
  // canvas drops all metadata, including where the photo was taken.
  window.vlShrink = async (bytes, maxBytes) => {
    const bmp = await createImageBitmap(new Blob([bytes]));
    let scale = Math.min(1, 640 / Math.max(bmp.width, bmp.height));
    for (let i = 0; i < 12; i++) {
      const c = document.createElement('canvas');
      c.width = Math.max(1, Math.round(bmp.width * scale));
      c.height = Math.max(1, Math.round(bmp.height * scale));
      c.getContext('2d').drawImage(bmp, 0, 0, c.width, c.height);
      for (const q of [0.7, 0.5, 0.35]) {
        const blob = await new Promise((r) => c.toBlob(r, 'image/jpeg', q));
        if (blob && blob.size <= maxBytes) return new Uint8Array(await blob.arrayBuffer());
      }
      scale *= 0.75;
    }
    return null;
  };

  window.vlSave = (name, bytes) => {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([bytes]));
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 10000);
  };

  // --- One location fix, only when the user taps Share location.
  window.vlLocate = () => new Promise((resolve) => {
    if (!navigator.geolocation) return resolve(null);
    navigator.geolocation.getCurrentPosition(
      (p) => resolve({ lat: p.coords.latitude, lon: p.coords.longitude, acc: p.coords.accuracy, at: p.timestamp }),
      () => resolve(null),
      { enableHighAccuracy: true, timeout: 20000, maximumAge: 0 },
    );
  });

  // --- Camera, only on the scan screen. Frames are decoded in this page and
  // never stored or sent.
  window.vlCameraAllowed = async () => {
    try {
      const s = await navigator.mediaDevices.getUserMedia({ video: true });
      s.getTracks().forEach((t) => t.stop());
      return true;
    } catch (_) {
      return false;
    }
  };

  // Starts scanning into `video`; calls onText(text) for each QR seen.
  // Returns a function that stops the camera.
  window.vlScan = async (video, onText) => {
    const stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } });
    video.srcObject = stream;
    video.muted = true;
    video.setAttribute('playsinline', '');
    await video.play();
    const canvas = document.createElement('canvas');
    const g = canvas.getContext('2d', { willReadFrequently: true });
    let on = true;
    const tick = () => {
      if (!on) return;
      if (video.videoWidth) {
        canvas.width = video.videoWidth;
        canvas.height = video.videoHeight;
        g.drawImage(video, 0, 0);
        const img = g.getImageData(0, 0, canvas.width, canvas.height);
        const code = window.jsQR(img.data, canvas.width, canvas.height, { inversionAttempts: 'attemptBoth' });
        if (code && code.data) onText(code.data);
      }
      setTimeout(tick, 200);
    };
    tick();
    return () => {
      on = false;
      stream.getTracks().forEach((t) => t.stop());
    };
  };
})();
