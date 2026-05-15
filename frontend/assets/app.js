// Firefly Indexer — single-page app with top-nav pages.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog || window.__TAURI_PLUGIN_DIALOG__;

const $  = (s) => document.querySelector(s);
const $$ = (s) => Array.from(document.querySelectorAll(s));

// ── Page switching ─────────────────────────────────────────────────────
function switchPage(name) {
  $$(".page").forEach((p) => p.setAttribute("hidden", ""));
  const p = document.getElementById("page-" + name);
  if (p) p.removeAttribute("hidden");
  $$(".nav-btn").forEach((b) => b.classList.toggle("nav-btn--active", b.dataset.page === name));
}

// ── Status dot ─────────────────────────────────────────────────────────
function setStatusDot(state, text) {
  const dot = $("#cli-status .status__dot");
  dot.dataset.state = state;
  $("#cli-status .status__text").textContent = text;
}
function setBadge(el, state, text) {
  el.dataset.state = state;
  el.textContent = text;
}

// ── Preflight ──────────────────────────────────────────────────────────
const MANUAL_OVERRIDE_KEY = "firefly_indexer_manual_override";

function isManuallyOverridden() {
  try { return localStorage.getItem(MANUAL_OVERRIDE_KEY) === "true"; } catch { return false; }
}
function setManualOverride(v) {
  try { localStorage.setItem(MANUAL_OVERRIDE_KEY, v ? "true" : "false"); } catch {}
}

function updateSetupBanner(s) {
  const banner = $("#setup-banner");
  const title  = $("#setup-banner-title");
  const sub    = $("#setup-banner-sub");
  if (!banner) return;
  const override = isManuallyOverridden();
  if (override) {
    banner.setAttribute("hidden", "");
    return;
  }
  if (s && s.installed && s.logged_in) {
    banner.setAttribute("hidden", "");
  } else if (s && s.installed) {
    banner.removeAttribute("hidden");
    title.textContent = "Claude Code 로그인이 필요합니다.";
    sub.textContent = "Setup 페이지에서 한 번만 로그인하면 다음부터 자동으로 작동합니다.";
  } else {
    banner.removeAttribute("hidden");
    title.textContent = "Claude Code 셋업이 필요합니다.";
    sub.textContent = "한 번만 설치·로그인하면 다음부터 자동으로 작동합니다.";
  }
}

function updateSetupDoneCard(s) {
  const doneCard = $("#setup-done-card");
  const manualReset = $("#manual-reset");
  if (!doneCard) return;
  const override = isManuallyOverridden();
  const ok = override || (s && s.installed && s.logged_in);
  doneCard.toggleAttribute("hidden", !ok);
  if (manualReset) manualReset.toggleAttribute("hidden", !override);
}

async function checkCliStatus() {
  setStatusDot("busy", "checking…");
  try {
    const s = await invoke("check_claude_status");
    setBadge($("#badge-install"), s.installed ? "ok" : "err", s.installed ? "INSTALLED" : "NOT FOUND");
    setBadge($("#badge-login"),   s.logged_in ? "ok" : "warn", s.logged_in ? "LOGGED IN" : "NOT LOGGED IN");

    const override = isManuallyOverridden();
    if (override)                     setStatusDot("ok",   "준비됨 (수동)");
    else if (s.installed && s.logged_in) setStatusDot("ok",   "준비됨");
    else if (s.installed)            setStatusDot("warn", "Setup 필요 — 로그인");
    else                              setStatusDot("err",  "Setup 필요 — 설치");

    updateSetupBanner(s);
    updateSetupDoneCard(s);
    updateStartButton();
    return s;
  } catch (e) {
    setStatusDot("err", String(e));
    return null;
  }
}

// ── Folder pickers ─────────────────────────────────────────────────────
async function pickFolder(initial) {
  if (!dialog || !dialog.open) return null;
  const sel = await dialog.open({ directory: true, multiple: false, defaultPath: initial || undefined });
  if (!sel) return null;
  return Array.isArray(sel) ? sel[0] : sel;
}
async function pickSaveXlsx(suggested) {
  if (!dialog || !dialog.save) return null;
  return (await dialog.save({
    title: "출력 Excel 저장",
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
    defaultPath: suggested || undefined,
  })) || null;
}

// ── Indexer page ───────────────────────────────────────────────────────
let indexerRunning = false;

function updateStartButton() {
  const cliOK = $("#cli-status .status__dot").dataset.state === "ok" || isManuallyOverridden();
  const input = $("#input-folder").value.trim();
  const output = $("#output-xlsx").value.trim();
  $("#start-indexing").disabled = !(cliOK && input && output) || indexerRunning;
}

async function scanInputFolderInline() {
  const folder = $("#input-folder").value.trim();
  if (!folder) {
    $("#scan-hint").textContent = "— 폴더를 선택하면 자동 스캔합니다.";
    return;
  }
  try {
    const r = await invoke("scan_folder", { folder });
    if (r.total === 0) {
      $("#scan-hint").textContent = "이 폴더에 jpg/png 사진이 없습니다.";
    } else if (r.nonconforming.length > 0) {
      $("#scan-hint").innerHTML =
        `${r.total}장 발견 — 표준 ${r.conforming.length} / <span style="color:var(--warning)">비표준 ${r.nonconforming.length}</span>. ` +
        `<a href="#" id="goto-rename" style="color:var(--accent)">Rename으로 이동 →</a>`;
      const link = $("#goto-rename");
      if (link) link.addEventListener("click", (e) => {
        e.preventDefault();
        $("#rn-folder").value = folder;
        switchPage("rename");
        scanRenameFolder();
      });
    } else {
      $("#scan-hint").textContent = `${r.total}장 — 모두 표준 패턴 ✓`;
    }
  } catch (e) {
    $("#scan-hint").textContent = "스캔 실패: " + e;
  }
}

async function fillSuggestedOutput() {
  const input = $("#input-folder").value.trim();
  if (!input || $("#output-xlsx").value.trim()) return;
  try {
    const sug = await invoke("suggest_output_path", { inputFolder: input });
    $("#output-xlsx").value = sug;
  } catch {}
}

function appendLog(logEl, text, cls = "") {
  const span = document.createElement("span");
  if (cls) span.className = cls;
  span.textContent = text;
  logEl.appendChild(span);
  logEl.scrollTop = logEl.scrollHeight;
}

// ── Validate page ──────────────────────────────────────────────────────
function setVal(elId, txt) { const e = $(elId); if (e) e.textContent = txt; }
function refreshValidateButton() {
  $("#run-validate").disabled = !$("#val-xlsx").value.trim();
}
function renderReport(rep) {
  $("#eval-report").removeAttribute("hidden");
  // Scores (only form + slop)
  setVal("#score-form",  rep.form_total > 0 ? `${rep.form_score_pct.toFixed(1)}%` : "—");
  setVal("#score-form-detail", rep.form_total > 0 ? `${rep.form_pass} / ${rep.form_total} rows` : "");
  setVal("#score-slop", String(rep.slop_hits.length));

  // Form issues
  const fl = $("#report-form-list");
  fl.innerHTML = rep.form_issues.length === 0
    ? '<div class="item"><span class="row-id">—</span><span class="body" style="color:var(--success)">사고 없음 — 100% 양식 통과</span></div>'
    : rep.form_issues.map((i) =>
        `<div class="item"><span class="row-id">row ${i.row}</span><span class="body">${escape(i.field)} = <em>${escape(i.value)}</em><br/>${escape(i.reason)}</span></div>`
      ).join("");

  // Slop
  const sl = $("#report-slop-list");
  sl.innerHTML = rep.slop_hits.length === 0
    ? '<div class="item"><span class="row-id">—</span><span class="body" style="color:var(--success)">슬롭 0건</span></div>'
    : rep.slop_hits.map((s) =>
        `<div class="item"><span class="row-id">row ${s.row}</span><span class="body"><em>${escape(s.phrase)}</em><br/>${escape(s.caption)}</span></div>`
      ).join("");
}
function escape(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
}

// L3 vision round-trip helpers
function updateVisionEstimate() {
  const mode = $("#ve-mode").value;
  const rowsHint = parseInt($("#val-xlsx").dataset.rows || "0", 10);
  if (!rowsHint) { $("#ve-estimate").value = "Excel 먼저 선택"; return; }
  const factor = mode === "quick" ? 0.10 : 1.0;
  const evalRows = Math.max(1, Math.round(rowsHint * factor));
  const sec = evalRows * 8;   // ~8s per row average
  const mm = Math.ceil(sec / 60);
  $("#ve-estimate").value = `${evalRows} rows · ~${mm}분`;
}

async function runVisionEval() {
  const req = {
    output_xlsx:  $("#val-xlsx").value.trim(),
    source_folder: $("#ve-folder").value.trim(),
    model:        $("#ve-model").value || "haiku",
    mode:         $("#ve-mode").value || "quick",
    timeout_secs: 600,
  };
  if (!req.output_xlsx || !req.source_folder) { alert("Excel과 원본 사진 폴더를 모두 선택해주세요."); return; }
  $("#ve-status").removeAttribute("hidden");
  $("#ve-log").textContent = "";
  $("#ve-progress-bar").style.width = "2%";
  $("#run-vision-eval").disabled = true;
  $("#stop-vision-eval").disabled = false;
  try {
    const rep = await invoke("run_vision_evaluation", { req });
    appendLog($("#ve-log"),
      `\n결과: ${rep.total_evaluated} rows · caption ${rep.avg_caption_score.toFixed(1)} · edit ${rep.avg_edit_score.toFixed(1)} · overall ${rep.avg_score.toFixed(1)}\n`,
      "ok");
    // top-up the dashboard with 3 vision score boxes
    $("#eval-report").removeAttribute("hidden");
    addScoreBox("CAPTION (사진 묘사)",    rep.avg_caption_score.toFixed(1), `${rep.total_evaluated} rows`, "caption");
    addScoreBox("EDIT (편집 프롬프트)",  rep.avg_edit_score.toFixed(1),    `${rep.total_evaluated} rows`, "edit");
    addScoreBox("OVERALL (종합)",        rep.avg_score.toFixed(1),         `${rep.total_evaluated} rows`, "overall");
  } catch (e) {
    appendLog($("#ve-log"), "\n실패: " + String(e) + "\n", "err");
  } finally {
    $("#run-vision-eval").disabled = false;
    $("#stop-vision-eval").disabled = true;
  }
}

function addScoreBox(label, value, detail, key) {
  const grid = $("#eval-report .score-grid");
  if (!grid) return;
  const dataKey = key || "vision";
  let box = grid.querySelector(`[data-score="${dataKey}"]`);
  if (!box) {
    box = document.createElement("div");
    box.className = "score";
    box.dataset.score = dataKey;
    box.innerHTML = `<span class="mono mono--dim">${label}</span><span class="score__val">—</span><span class="mono mono--dim score__detail"></span>`;
    grid.appendChild(box);
  }
  box.querySelector(".score__val").textContent = value;
  box.querySelector(".score__detail").textContent = detail;
}

listen("vision-eval-progress", (event) => {
  const p = event.payload;
  const cls = p.level === "fail" ? "err" : p.level === "done" ? "ok" : p.level === "cancelled" ? "warn" : "dim";
  appendLog($("#ve-log"), p.message + "\n", cls);
  $("#ve-counter").textContent = `${p.done} / ${p.total}`;
  $("#ve-avg").textContent = `avg ${p.avg_score.toFixed(1)}`;
  if (p.total > 0) {
    const pct = Math.max(2, Math.min(100, (p.done / p.total) * 100));
    $("#ve-progress-bar").style.width = pct + "%";
  }
});

async function runValidate() {
  const req = {
    output_xlsx: $("#val-xlsx").value.trim(),
    answer_key_xlsx: null,
    run_form:  true,
    run_match: false,
    run_slop:  true,
  };
  try {
    const rep = await invoke("run_evaluation", { req });
    renderReport(rep);
    // store row count for vision estimate
    $("#val-xlsx").dataset.rows = String(rep.total_rows);
    $("#run-vision-eval").disabled = !$("#ve-folder").value.trim();
    updateVisionEstimate();
  } catch (e) {
    alert("검증 실패: " + e);
  }
}

async function startIndexing() {
  const req = {
    input_folder: $("#input-folder").value.trim(),
    output_xlsx:  $("#output-xlsx").value.trim(),
    scene_size:   parseInt($("#scene-size").value, 10) || 6,
    model:        $("#model").value || "haiku",
    timeout_secs: parseInt($("#timeout").value, 10) || 600,
    resume:       $("#resume").checked,
  };
  if (!req.input_folder || !req.output_xlsx) return;

  indexerRunning = true;
  $("#run-status").removeAttribute("hidden");
  $("#log").removeAttribute("hidden");
  $("#log").textContent = "";
  $("#progress-bar").style.width = "2%";
  $("#run-counter").textContent = "준비 중…";
  $("#run-cost").textContent = "$0.00";
  $("#stop-indexing").disabled = false;
  $("#open-xlsx").disabled = true;
  $("#open-folder").disabled = true;
  updateStartButton();

  appendLog($("#log"), `$ run_indexing  scene_size=${req.scene_size}  model=${req.model}\n`, "dim");
  try {
    const summary = await invoke("run_indexing", { req });
    $("#progress-bar").style.width = "100%";
    appendLog($("#log"), `\n[DONE] ${summary.scenes_done}/${summary.scenes_total} 씬 — ${summary.output_path}\n`, "ok");
    $("#open-xlsx").disabled = false;
    $("#open-xlsx").dataset.path = summary.output_path;
    $("#open-folder").disabled = false;
    $("#open-folder").dataset.path = summary.output_path;
    $("#goto-validate").disabled = false;
    $("#goto-validate").dataset.path = summary.output_path;
  } catch (e) {
    appendLog($("#log"), "\n[FAIL] " + String(e) + "\n", "err");
  } finally {
    indexerRunning = false;
    $("#stop-indexing").disabled = true;
    updateStartButton();
  }
}

// Sub-step state per scene
let currentSceneId = 0;
let substepStartTimes = new Map();   // index -> timestamp

function ensureSubstepList(total, sceneLabel) {
  const wrap = $("#substep");
  wrap.removeAttribute("hidden");
  $("#substep-scene").textContent = sceneLabel;
  $("#substep-counter").textContent = `0 / ${total + 2}`;
  const ul = $("#substep-list");
  ul.innerHTML = "";
  // Static plan: prompt build → N x photo analysis → JSON integration
  const items = [
    { kind: "prompt_built", label: "프롬프트 빌드 — Claude 호출" },
  ];
  for (let i = 1; i <= total; i++) {
    items.push({ kind: `photo_${i}`, label: `사진 ${i}/${total} 분석` });
  }
  items.push({ kind: "final_text", label: "씬 통합 JSON 응답" });
  items.push({ kind: "saved", label: "Excel 저장 + 진행 기록" });

  for (const it of items) {
    const li = document.createElement("li");
    li.dataset.kind = it.kind;
    li.dataset.state = "pending";
    li.innerHTML = `<span class="substep__icon"></span><span>${it.label}</span><span class="substep__time"></span>`;
    ul.appendChild(li);
  }
  substepStartTimes.clear();
}

function markSubstep(kind, state, totalCount = null) {
  const ul = $("#substep-list");
  if (!ul) return;
  const li = ul.querySelector(`li[data-kind="${kind}"]`);
  if (!li) return;
  const now = performance.now();
  if (state === "active") substepStartTimes.set(kind, now);
  if (state === "done" && substepStartTimes.has(kind)) {
    const dt = (now - substepStartTimes.get(kind)) / 1000;
    li.querySelector(".substep__time").textContent = dt < 60 ? `${dt.toFixed(1)}s` : `${(dt / 60).toFixed(1)}m`;
  }
  li.dataset.state = state;
  // counter
  const doneCount = ul.querySelectorAll('li[data-state="done"]').length;
  const totalLi = totalCount ?? ul.querySelectorAll("li").length;
  $("#substep-counter").textContent = `${doneCount} / ${totalLi}`;
}

listen("indexing-progress", (event) => {
  const p = event.payload;
  // Always log
  const cls = p.level === "fail" ? "err" : p.level === "done" ? "ok" : p.level === "scene" ? "ok" : "dim";
  appendLog($("#log"), p.message + "\n", cls);
  $("#run-cost").textContent = `$${p.total_cost_usd.toFixed(4)}`;

  if (p.scene_total > 0) {
    $("#run-counter").textContent = `${p.scene_done} / ${p.scene_total} 씬 · 현재 scene ${p.current_scene_id} (${p.current_scene_prefix})`;
    const pct = Math.max(2, Math.min(100, ((p.scene_done + (p.substep_total > 0 ? p.substep_done / p.substep_total : 0)) / p.scene_total) * 100));
    $("#progress-bar").style.width = pct + "%";
  }

  // Scene start: build the sub-step list
  if (p.level === "info" && p.substep_kind === "scene_start" && p.current_scene_id !== currentSceneId) {
    currentSceneId = p.current_scene_id;
    ensureSubstepList(p.substep_total, `scene ${p.current_scene_id} · ${p.current_scene_prefix}`);
  }

  // Sub-step events
  if (p.level === "substep") {
    if (p.substep_kind === "prompt_built") {
      markSubstep("prompt_built", "done");
    } else if (p.substep_kind === "read_start") {
      markSubstep(`photo_${p.substep_done}`, "active");
    } else if (p.substep_kind === "read_done") {
      markSubstep(`photo_${p.substep_done}`, "done");
    } else if (p.substep_kind === "final_text") {
      markSubstep("final_text", "done");
      markSubstep("saved", "active");
    } else if (p.substep_kind === "result_err") {
      markSubstep("final_text", "error");
    }
  }

  // Scene complete
  if (p.level === "scene") {
    markSubstep("saved", "done");
  }

  // Scene fail
  if (p.level === "fail") {
    const ul = $("#substep-list");
    ul.querySelectorAll('li[data-state="active"]').forEach((li) => li.dataset.state = "error");
  }
});

// ── Rename page ────────────────────────────────────────────────────────
async function scanRenameFolder() {
  const folder = $("#rn-folder").value.trim();
  if (!folder) return;
  try {
    const r = await invoke("scan_folder", { folder });
    $("#rn-total").textContent = String(r.total);
    $("#rn-conform").textContent = String(r.conforming.length);
    $("#rn-noncon").textContent = String(r.nonconforming.length);
    $("#rn-report").removeAttribute("hidden");
    if (r.nonconforming.length > 0) {
      $("#rn-row-noncon").removeAttribute("hidden");
    } else {
      $("#rn-row-noncon").setAttribute("hidden", "");
    }
    if (r.deduced_project) $("#rn-project").value = r.deduced_project;
    if (r.deduced_seq)     $("#rn-seq").value     = r.deduced_seq;
    if (r.deduced_module)  $("#rn-module").value  = r.deduced_module;
    if (r.deduced_ver)     $("#rn-ver").value     = r.deduced_ver;
    if (!$("#rn-output").value) {
      const sep = folder.includes("\\") ? "\\" : "/";
      const parts = folder.split(sep);
      const name = parts.pop();
      $("#rn-output").value = parts.join(sep) + sep + name + "_renamed";
    }
    $("#run-rename").disabled = r.total === 0;
  } catch (e) {
    alert("스캔 실패: " + e);
  }
}

async function runRename() {
  const req = {
    folder:             $("#rn-folder").value.trim(),
    output_folder:      $("#rn-output").value.trim(),
    project:            $("#rn-project").value.trim() || "26Q2",
    seq:                $("#rn-seq").value.trim() || "XX",
    module:             parseInt($("#rn-module").value, 10) || 1,
    ver:                parseInt($("#rn-ver").value, 10) || 1,
    start_index:        parseInt($("#rn-start").value, 10) || 1,
    num_width:          parseInt($("#rn-width").value, 10) || 5,
    only_nonconforming: $("#rn-only-non").checked,
  };
  if (!req.output_folder) { alert("사본 폴더 경로가 비어있습니다."); return; }
  $("#rn-log").removeAttribute("hidden");
  $("#rn-log").textContent = "";
  appendLog($("#rn-log"), `정정 시작…\n`, "dim");
  try {
    const r = await invoke("apply_rename", { req });
    appendLog($("#rn-log"), `${r.copied}장 사본 복사 완료.\n매핑 표: ${r.mapping_csv}\n`, "ok");
    $("#rn-open-result").disabled = false;
    $("#rn-open-result").dataset.path = req.output_folder;
  } catch (e) {
    appendLog($("#rn-log"), "실패: " + String(e) + "\n", "err");
  }
}

// ── Wire ───────────────────────────────────────────────────────────────
async function init() {
  // Nav
  $$(".nav-btn").forEach((b) => {
    b.addEventListener("click", () => switchPage(b.dataset.page));
  });

  // Indexer page
  $("#pick-input").addEventListener("click", async () => {
    const f = await pickFolder($("#input-folder").value || undefined);
    if (f) { $("#input-folder").value = f; await scanInputFolderInline(); await fillSuggestedOutput(); updateStartButton(); }
  });
  $("#input-folder").addEventListener("input", () => { updateStartButton(); });
  $("#input-folder").addEventListener("change", async () => { await scanInputFolderInline(); await fillSuggestedOutput(); updateStartButton(); });
  $("#pick-output").addEventListener("click", async () => {
    const f = await pickSaveXlsx($("#output-xlsx").value || undefined);
    if (f) { $("#output-xlsx").value = f; updateStartButton(); }
  });
  $("#output-xlsx").addEventListener("input", updateStartButton);
  $("#start-indexing").addEventListener("click", startIndexing);
  $("#open-xlsx").addEventListener("click", async () => {
    const p = $("#open-xlsx").dataset.path;
    if (p) await invoke("open_path", { path: p });
  });
  $("#open-folder").addEventListener("click", async () => {
    const p = $("#open-folder").dataset.path;
    if (!p) return;
    const sep = p.includes("\\") ? "\\" : "/";
    const idx = p.lastIndexOf(sep);
    const folder = idx > 0 ? p.slice(0, idx) : p;
    await invoke("open_path", { path: folder });
  });

  // Goto Validate from Index done
  $("#goto-validate").addEventListener("click", () => {
    const p = $("#goto-validate").dataset.path;
    if (p) {
      $("#val-xlsx").value = p;
      refreshValidateButton();
    }
    switchPage("validate");
  });

  // Validate page
  $("#val-xlsx").addEventListener("input", refreshValidateButton);
  $("#val-pick-xlsx").addEventListener("click", async () => {
    if (!dialog || !dialog.open) return;
    const f = await dialog.open({ multiple: false, filters: [{ name: "Excel", extensions: ["xlsx"] }] });
    if (f) { $("#val-xlsx").value = Array.isArray(f) ? f[0] : f; refreshValidateButton(); }
  });
  $("#run-validate").addEventListener("click", runValidate);

  // L3 vision round-trip wiring
  $("#ve-pick-folder").addEventListener("click", async () => {
    const f = await pickFolder($("#ve-folder").value || undefined);
    if (f) {
      $("#ve-folder").value = f;
      $("#run-vision-eval").disabled = !($("#val-xlsx").value.trim());
    }
  });
  $("#ve-mode").addEventListener("change", updateVisionEstimate);
  $("#run-vision-eval").addEventListener("click", runVisionEval);
  $("#stop-vision-eval").addEventListener("click", async () => {
    try { await invoke("cancel_vision_evaluation"); } catch {}
  });

  // Rename page
  $("#rn-pick-folder").addEventListener("click", async () => {
    const f = await pickFolder($("#rn-folder").value || undefined);
    if (f) { $("#rn-folder").value = f; await scanRenameFolder(); }
  });
  $("#rn-folder").addEventListener("change", scanRenameFolder);
  $("#rn-pick-output").addEventListener("click", async () => {
    const f = await pickFolder($("#rn-output").value || undefined);
    if (f) $("#rn-output").value = f;
  });
  $("#run-rename").addEventListener("click", runRename);
  $("#rn-open-result").addEventListener("click", async () => {
    const p = $("#rn-open-result").dataset.path;
    if (p) await invoke("open_path", { path: p });
  });

  // Setup page
  $("#recheck-install").addEventListener("click", checkCliStatus);
  $("#recheck-login").addEventListener("click", checkCliStatus);
  $("#run-install")?.addEventListener("click", async () => {
    try { await invoke("open_setup_terminal", { command: "install" }); }
    catch (e) { alert("터미널 실행 실패: " + e); }
  });
  $("#run-login")?.addEventListener("click", async () => {
    try { await invoke("open_setup_terminal", { command: "login" }); }
    catch (e) { alert("터미널 실행 실패: " + e); }
  });
  $("#setup-go-index")?.addEventListener("click", () => switchPage("indexer"));
  $("#goto-setup-btn")?.addEventListener("click", () => switchPage("setup"));
  $("#manual-override")?.addEventListener("click", () => {
    setManualOverride(true);
    checkCliStatus().then(() => switchPage("indexer"));
  });
  $("#manual-reset")?.addEventListener("click", () => {
    setManualOverride(false);
    checkCliStatus();
  });

  await checkCliStatus();
  // default page = indexer
  switchPage("indexer");
}

init().catch((e) => {
  console.error(e);
  alert("init failed: " + e);
});
