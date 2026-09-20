// GameWache Dashboard Client App
document.addEventListener('DOMContentLoaded', () => {
  let activeTab = 'crashes';
  let currentProject = document.getElementById('projectSelect').value;
  let ws = null;
  let currentCrashes = [];
  let registeredTweakables = {};

  // TAB NAVIGATION
  const navTabs = document.querySelectorAll('.nav-tab');
  navTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      navTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      activeTab = tab.dataset.tab;

      document.querySelectorAll('.tab-pane').forEach(pane => {
        pane.classList.add('hidden');
        pane.classList.remove('active');
      });

      const activePane = document.getElementById(`tab-${activeTab}`);
      if (activePane) {
        activePane.classList.remove('hidden');
        activePane.classList.add('active');
      }

      onTabChanged(activeTab);
    });
  });

  // PROJECT SELECT
  document.getElementById('projectSelect').addEventListener('change', (e) => {
    currentProject = e.target.value;
    refreshData();
  });

  document.getElementById('refreshBtn').addEventListener('click', () => {
    refreshData();
  });

  function onTabChanged(tab) {
    if (tab === 'crashes') loadCrashes();
    if (tab === 'replay') loadReplays();
    if (tab === 'spatial') loadSpatial();
    if (tab === 'shaders') loadShaders();
    if (tab === 'assets') loadAssetStats();
    if (tab === 'feedback') loadFeedback();
  }

  function refreshData() {
    onTabChanged(activeTab);
  }

  // WEBSOCKET FOR LIVE TUNING
  function connectWebSocket() {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${proto}//${window.location.host}/ws/live_tuning`;
    ws = new WebSocket(wsUrl);

    const statusBadge = document.getElementById('wsStatus');

    ws.onopen = () => {
      statusBadge.innerHTML = '<span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span><span>WebSocket Live</span>';
      statusBadge.className = 'flex items-center gap-1.5 text-xs font-mono text-emerald-400 bg-emerald-500/10 px-2.5 py-1 rounded-full border border-emerald-500/20';
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        handleLiveMessage(msg);
      } catch (err) {
        console.error('Error parsing live WS message:', err);
      }
    };

    ws.onclose = () => {
      statusBadge.innerHTML = '<span class="w-2 h-2 rounded-full bg-red-400"></span><span>Disconnected</span>';
      statusBadge.className = 'flex items-center gap-1.5 text-xs font-mono text-red-400 bg-red-500/10 px-2.5 py-1 rounded-full border border-red-500/20';
      setTimeout(connectWebSocket, 3000);
    };
  }

  function handleLiveMessage(msg) {
    if (msg.msg_type === 'register') {
      registeredTweakables[msg.param_path] = {
        value: msg.value,
        meta: msg.meta || {}
      };
      renderTweakSliders();
    } else if (msg.msg_type === 'tweak') {
      if (registeredTweakables[msg.param_path]) {
        registeredTweakables[msg.param_path].value = msg.value;
        updateSliderUI(msg.param_path, msg.value);
      }
    }
  }

  function renderTweakSliders() {
    const grid = document.getElementById('tweakerSlidersGrid');
    const paths = Object.keys(registeredTweakables);

    if (paths.length === 0) {
      grid.innerHTML = `<div class="col-span-full bg-gray-900 border border-gray-800 rounded-xl p-8 text-center text-gray-500 font-mono text-xs">
        Brak aktywnych zmiennych w grze. Oczekuję na wywołanie <code>GameWache.register_tweak()</code>...
      </div>`;
      return;
    }

    grid.innerHTML = '';
    paths.forEach(path => {
      const item = registeredTweakables[path];
      const min = item.meta.min !== undefined ? item.meta.min : 0;
      const max = item.meta.max !== undefined ? item.meta.max : 100;
      const step = item.meta.step !== undefined ? item.meta.step : 0.1;

      const card = document.createElement('div');
      card.className = 'bg-gray-900 border border-gray-800 rounded-xl p-4 font-mono space-y-2';
      card.innerHTML = `
        <div class="flex items-center justify-between text-xs">
          <span class="font-bold text-gray-200 truncate">${path}</span>
          <span id="val_${cleanId(path)}" class="text-amber-400 font-bold">${item.value}</span>
        </div>
        <input type="range" id="slider_${cleanId(path)}" min="${min}" max="${max}" step="${step}" value="${item.value}"
          class="w-full h-2 bg-gray-950 rounded-lg appearance-none cursor-pointer accent-amber-500">
        <div class="flex justify-between text-[10px] text-gray-500">
          <span>Min: ${min}</span>
          <span>Max: ${max}</span>
        </div>
      `;

      const slider = card.querySelector(`#slider_${cleanId(path)}`);
      slider.addEventListener('input', (e) => {
        const val = parseFloat(e.target.value);
        document.getElementById(`val_${cleanId(path)}`).textContent = val;
        sendTweak(path, val);
      });

      grid.appendChild(card);
    });
  }

  function updateSliderUI(path, val) {
    const slider = document.getElementById(`slider_${cleanId(path)}`);
    const label = document.getElementById(`val_${cleanId(path)}`);
    if (slider) slider.value = val;
    if (label) label.textContent = val;
  }

  function cleanId(str) {
    return str.replace(/[^a-zA-Z0-9]/g, '_');
  }

  function sendTweak(path, value) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        msg_type: 'tweak',
        param_path: path,
        value: value
      }));
    }
  }

  document.getElementById('btnSendCmd').addEventListener('click', () => {
    const input = document.getElementById('consoleCmdInput');
    const cmd = input.value.trim();
    if (!cmd) return;

    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        msg_type: 'command',
        cmd: cmd
      }));
      input.value = '';
    }
  });

  // CRASHES MANAGEMENT
  async function loadCrashes() {
    try {
      const res = await fetch(`/api/v1/projects/${currentProject}/crashes?limit=50`);
      if (!res.ok) throw new Error('Failed to load crashes');
      currentCrashes = await res.json();
      document.getElementById('crashCountBadge').textContent = currentCrashes.length;
      renderCrashesTable(currentCrashes);
    } catch (err) {
      console.error('Error fetching crashes:', err);
    }
  }

  function renderCrashesTable(crashes) {
    const tbody = document.getElementById('crashesTableBody');
    tbody.innerHTML = '';

    if (crashes.length === 0) {
      tbody.innerHTML = `<tr><td colspan="8" class="p-8 text-center text-gray-500 italic">Brak zarejestrowanych incydentów dla projektu ${currentProject}. Silnik działa stabilnie!</td></tr>`;
      return;
    }

    crashes.forEach(c => {
      const tr = document.createElement('tr');
      tr.className = 'hover:bg-gray-800/40 transition cursor-pointer';

      const loc = c.source_file ? `${c.source_file}:${c.line_no || 0}` : 'Natywny rdzeń';
      const scene = c.scene_name ? c.scene_name.replace('res://scenes/', '') : 'Brak';
      const timeAgo = formatTimeAgo(new Date(c.last_seen));

      tr.innerHTML = `
        <td class="p-3 text-gray-500">#${c.id}</td>
        <td class="p-3">
          <div class="font-bold text-red-400 truncate max-w-sm">${escapeHtml(c.exception_type)}</div>
          <div class="text-[11px] text-gray-400 truncate max-w-md">${escapeHtml(c.message)}</div>
        </td>
        <td class="p-3 text-gray-300 truncate max-w-xs text-[11px]">${escapeHtml(loc)}</td>
        <td class="p-3 text-gray-400 truncate max-w-xs text-[11px]">${escapeHtml(scene)}</td>
        <td class="p-3 text-center"><span class="bg-gray-800 text-amber-400 font-bold px-2 py-0.5 rounded border border-gray-700">${c.occurrences}</span></td>
        <td class="p-3 text-gray-400 uppercase text-[11px]">${escapeHtml(c.platform)}</td>
        <td class="p-3 text-gray-500 text-[11px]">${timeAgo}</td>
        <td class="p-3 text-right">
          <button class="bg-gray-800 hover:bg-gray-700 text-gray-200 px-2 py-1 rounded text-[11px] border border-gray-700 btn-details" data-id="${c.id}">Szczegóły</button>
        </td>
      `;

      tr.querySelector('.btn-details').addEventListener('click', (e) => {
        e.stopPropagation();
        openCrashModal(c);
      });
      tr.addEventListener('click', () => openCrashModal(c));

      tbody.appendChild(tr);
    });
  }

  function openCrashModal(c) {
    document.getElementById('modalCrashTitle').textContent = `Crash #${c.id}: ${c.exception_type} - ${c.message}`;
    const meta = document.getElementById('modalCrashMetadata');
    meta.innerHTML = `
      <div><span class="text-gray-500">Silnik:</span> ${c.engine} ${c.engine_version}</div>
      <div><span class="text-gray-500">Platforma:</span> ${c.platform}</div>
      <div><span class="text-gray-500">Scena:</span> ${c.scene_name || 'N/A'}</div>
      <div><span class="text-gray-500">Wystąpienia:</span> ${c.occurrences}</div>
      <div><span class="text-gray-500">Pozycja XYZ:</span> ${c.spatial_x !== null ? `(${c.spatial_x.toFixed(1)}, ${c.spatial_y.toFixed(1)}, ${c.spatial_z.toFixed(1)})` : 'N/A'}</div>
      <div><span class="text-gray-500">Powiązany Replay:</span> ${c.replay_id ? `<a href="#replay" class="text-amber-400 underline">${c.replay_id}</a>` : 'Brak'}</div>
    `;

    // Stacktrace
    const stackPre = document.getElementById('modalStacktrace');
    try {
      const frames = JSON.parse(c.stacktrace_json);
      if (Array.isArray(frames) && frames.length > 0) {
        stackPre.textContent = frames.map(f => `  at ${f.function} (${f.filename}:${f.lineno})`).join('\n');
      } else {
        stackPre.textContent = c.message;
      }
    } catch {
      stackPre.textContent = c.stacktrace_json || c.message;
    }

    // Breadcrumbs
    const crumbsDiv = document.getElementById('modalBreadcrumbs');
    crumbsDiv.innerHTML = '';
    try {
      const crumbs = JSON.parse(c.breadcrumbs_json);
      if (Array.isArray(crumbs) && crumbs.length > 0) {
        crumbs.slice(-15).forEach(b => {
          const item = document.createElement('div');
          item.className = 'flex items-center gap-2 p-1.5 border-b border-gray-900 text-[11px]';
          item.innerHTML = `
            <span class="text-gray-500 w-16">${b.engine_tick ? `[T:${b.engine_tick}]` : ''}</span>
            <span class="bg-gray-800 text-amber-300 px-1 rounded uppercase text-[9px]">${b.category}</span>
            <span class="text-gray-300 truncate">${escapeHtml(b.message)}</span>
          `;
          crumbsDiv.appendChild(item);
        });
      } else {
        crumbsDiv.innerHTML = '<div class="p-2 text-gray-500">Brak zarejestrowanych okruchów chleba.</div>';
      }
    } catch {
      crumbsDiv.innerHTML = '<div class="p-2 text-gray-500">Brak breadcrumbs.</div>';
    }

    document.getElementById('crashModal').classList.remove('hidden');
  }

  document.getElementById('btnModalClose').addEventListener('click', () => {
    document.getElementById('crashModal').classList.add('hidden');
  });

  // SPATIAL MAP CANVAS
  async function loadSpatial() {
    const sceneInput = document.getElementById('spatialSceneInput').value.trim();
    if (!sceneInput) return;

    try {
      const res = await fetch(`/api/v1/projects/${currentProject}/spatial_hotspots?scene=${encodeURIComponent(sceneInput)}`);
      const clusters = await res.json();
      renderSpatialCanvas(clusters);
    } catch (err) {
      console.error('Error fetching spatial clusters:', err);
    }
  }

  document.getElementById('btnLoadSpatial').addEventListener('click', loadSpatial);

  function renderSpatialCanvas(clusters) {
    const canvas = document.getElementById('spatialCanvas');
    const ctx = canvas.getContext('2d');
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#030712';
    ctx.fillRect(0, 0, w, h);

    // Siatka pomocnicza
    ctx.strokeStyle = '#111827';
    ctx.lineWidth = 1;
    for (let x = 0; x < w; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, h);
      ctx.stroke();
    }
    for (let y = 0; y < h; y += 40) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    // Centruj układ współrzędnych
    const cx = w / 2;
    const cy = h / 2;
    const scale = 3.5;

    ctx.strokeStyle = '#374151';
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();

    if (!Array.isArray(clusters) || clusters.length === 0) {
      ctx.fillStyle = '#6b7280';
      ctx.font = '12px "JetBrains Mono"';
      ctx.textAlign = 'center';
      ctx.fillText('Brak punktów awarii z koordynatami XYZ dla wybranej sceny.', cx, cy);
      return;
    }

    clusters.forEach(c => {
      const px = cx + c.centroid[0] * scale;
      const py = cy - c.centroid[2] * scale; // w 3D zazwyczaj Z to góra/dół na płaszczyźnie poziomu

      const radius = Math.min(6 + c.count * 3, 24);
      let color = 'rgba(16, 185, 129, 0.8)'; // emerald
      if (c.count >= 2) color = 'rgba(245, 158, 11, 0.8)'; // amber
      if (c.count >= 5) color = 'rgba(239, 68, 68, 0.9)'; // red

      ctx.beginPath();
      ctx.arc(px, py, radius, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.fill();
      ctx.lineWidth = 2;
      ctx.strokeStyle = '#fff';
      ctx.stroke();

      ctx.fillStyle = '#fff';
      ctx.font = '10px "JetBrains Mono"';
      ctx.textAlign = 'center';
      ctx.fillText(c.count, px, py + 3);
    });
  }

  // SHADERS PREWARM
  async function loadShaders() {
    try {
      const res = await fetch(`/api/v1/projects/${currentProject}/shaders`);
      const hitches = await res.json();
      renderShadersTable(hitches);
    } catch (err) {
      console.error('Error fetching shaders:', err);
    }
  }

  function renderShadersTable(hitches) {
    const tbody = document.getElementById('shadersTableBody');
    tbody.innerHTML = '';

    if (!Array.isArray(hitches) || hitches.length === 0) {
      tbody.innerHTML = `<tr><td colspan="6" class="p-8 text-center text-gray-500 italic">Brak wykrytych zacięć kompilacji shaderów. Potoki graficzne graczy działają płynnie.</td></tr>`;
      return;
    }

    hitches.forEach(h => {
      const tr = document.createElement('tr');
      tr.className = 'hover:bg-gray-800/40';
      tr.innerHTML = `
        <td class="p-3 text-amber-300 truncate max-w-md">${escapeHtml(h.shader_path)}</td>
        <td class="p-3 text-center font-bold text-red-400">${h.max_hitch_ms.toFixed(1)} ms</td>
        <td class="p-3 text-center text-gray-300">${h.avg_hitch_ms.toFixed(1)} ms</td>
        <td class="p-3 text-center text-gray-400">${h.occurrences}</td>
        <td class="p-3 text-gray-400 truncate max-w-xs">${escapeHtml(h.gpu_renderer)}</td>
        <td class="p-3 text-gray-500 text-[11px]">${formatTimeAgo(new Date(h.last_seen))}</td>
      `;
      tbody.appendChild(tr);
    });
  }

  document.getElementById('btnGeneratePrewarmGodot').addEventListener('click', async () => {
    try {
      const res = await fetch(`/api/v1/shader_prewarm/godot?project_id=${currentProject}`);
      const code = await res.text();
      showCodeModal('Wygenerowany skrypt: res://addons/gamewache/generated/shader_prewarm.gd', code);
    } catch (err) {
      alert('Błąd generowania skryptu: ' + err);
    }
  });

  document.getElementById('btnGeneratePrewarmUnity').addEventListener('click', async () => {
    try {
      const res = await fetch(`/api/v1/shader_prewarm/unity?project_id=${currentProject}`);
      const code = await res.text();
      showCodeModal('Wygenerowany skrypt: ShaderPrewarmer.cs (Unity)', code);
    } catch (err) {
      alert('Błąd generowania skryptu: ' + err);
    }
  });

  function showCodeModal(title, code) {
    document.getElementById('modalCrashTitle').textContent = title;
    document.getElementById('modalCrashMetadata').innerHTML = '';
    document.getElementById('modalStacktrace').textContent = code;
    document.getElementById('modalBreadcrumbs').innerHTML = '<div class="p-2 text-gray-500">Skopiuj powyższy kod lub użyj narzędzia MCP gw_generate_shader_prewarm, aby zapisać go bezpośrednio w projekcie.</div>';
    document.getElementById('crashModal').classList.remove('hidden');
  }

  // ASSETS & CACHE
  async function loadAssetStats() {
    try {
      const res = await fetch('/api/v1/cas/stats');
      const stats = await res.json();
      document.getElementById('statCasSize').textContent = `${(stats.total_compressed_bytes / (1024 * 1024)).toFixed(1)} MB`;
      document.getElementById('statCasRatio').textContent = `${((1.0 - stats.compression_ratio) * 100).toFixed(1)}% Kompresji`;
    } catch (err) {
      console.error('Error fetching CAS stats:', err);
    }
  }

  document.getElementById('btnRunAudit').addEventListener('click', async () => {
    const path = document.getElementById('auditPathInput').value.trim();
    const resultsDiv = document.getElementById('auditResults');
    resultsDiv.innerHTML = '<p class="text-amber-400 animate-pulse">Skanowanie grafu scen i assetów...</p>';

    try {
      const res = await fetch(`/api/v1/audit?path=${encodeURIComponent(path)}`);
      const rep = await res.json();

      document.getElementById('statOrphanBytes').textContent = `${(rep.total_orphan_bytes / (1024 * 1024)).toFixed(1)} MB`;

      let html = `
        <div class="mb-4 text-xs space-y-1">
          <div>Przeskanowane pliki: <span class="text-white font-bold">${rep.total_files}</span> | Osiągalne w scenach: <span class="text-emerald-400 font-bold">${rep.reachable_files}</span></div>
          <div>Wykryte sieroce pliki (Bloat): <span class="text-amber-400 font-bold">${rep.orphan_files.length}</span> (Łącznie: ${(rep.total_orphan_bytes / (1024 * 1024)).toFixed(2)} MB)</div>
        </div>
      `;

      if (rep.budget_violations.length > 0) {
        html += `<div class="mb-3 p-3 bg-red-950/40 border border-red-800 rounded">
          <div class="text-red-400 font-bold mb-1">Naruszenia Budżetów Jakościowych (${rep.budget_violations.length}):</div>
          <ul class="list-disc pl-4 space-y-1">
            ${rep.budget_violations.map(b => `<li><span class="text-gray-300">${b.path}</span> — <span class="text-red-300 font-bold">${b.actual}</span> (Limit: ${b.limit})</li>`).join('')}
          </ul>
        </div>`;
      }

      if (rep.orphan_files.length > 0) {
        html += `
          <div class="text-gray-400 font-bold mb-2">Najcięższe Niepowiązane Zasoby do Wycięcia:</div>
          <div class="space-y-1 max-h-60 overflow-y-auto">
            ${rep.orphan_files.slice(0, 20).map(o => `
              <div class="flex justify-between p-1.5 bg-gray-950 border border-gray-800 rounded">
                <span class="text-gray-300 truncate">${o.path}</span>
                <span class="text-amber-400 font-bold">${(o.size_bytes / (1024 * 1024)).toFixed(2)} MB</span>
              </div>
            `).join('')}
          </div>
        `;
      }

      resultsDiv.innerHTML = html;
    } catch (err) {
      resultsDiv.innerHTML = `<p class="text-red-400">Błąd podczas audytu: ${err}</p>`;
    }
  });

  // PLAYTEST FEEDBACK
  async function loadFeedback() {
    try {
      const res = await fetch(`/api/v1/projects/${currentProject}/feedback`);
      const feedback = await res.json();
      renderFeedback(feedback);
    } catch (err) {
      console.error('Error fetching feedback:', err);
    }
  }

  function renderFeedback(items) {
    const grid = document.getElementById('feedbackGrid');
    grid.innerHTML = '';

    if (!Array.isArray(items) || items.length === 0) {
      grid.innerHTML = `<div class="col-span-full p-8 text-center text-gray-500 italic">Brak zgłoszeń od playtesterów. Nakładka F11 oczekuje na nadesłanie opinii z gry.</div>`;
      return;
    }

    items.forEach(fb => {
      const card = document.createElement('div');
      card.className = 'bg-gray-900 border border-gray-800 rounded-xl p-4 space-y-2 flex flex-col justify-between';

      let imgHtml = '';
      if (fb.screenshot_base64) {
        imgHtml = `<img src="data:image/webp;base64,${fb.screenshot_base64}" class="w-full h-32 object-cover rounded border border-gray-800 mb-2 cursor-pointer" alt="Screenshot">`;
      }

      card.innerHTML = `
        <div>
          ${imgHtml}
          <div class="flex items-center justify-between text-[11px] mb-1">
            <span class="bg-gray-800 text-amber-300 px-1.5 py-0.5 rounded uppercase font-bold">${escapeHtml(fb.category)}</span>
            <span class="text-gray-500">${formatTimeAgo(new Date(fb.created_at))}</span>
          </div>
          <p class="text-gray-200 text-xs mt-1 leading-relaxed">${escapeHtml(fb.user_comment)}</p>
        </div>
        <div class="pt-2 border-t border-gray-800 text-[10px] text-gray-500 flex justify-between">
          <span>Scena: ${fb.scene_name || 'N/A'}</span>
          <span>${fb.replay_id ? `<span class="text-amber-400">Replay załączony</span>` : ''}</span>
        </div>
      `;
      grid.appendChild(card);
    });
  }

  // UTILS
  function escapeHtml(str) {
    if (!str) return '';
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function formatTimeAgo(date) {
    const now = new Date();
    const diffSec = Math.floor((now - date) / 1000);
    if (diffSec < 60) return `${diffSec}s temu`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m temu`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h temu`;
    return `${Math.floor(diffHours / 24)}d temu`;
  }

  // START
  connectWebSocket();
  loadCrashes();
});
