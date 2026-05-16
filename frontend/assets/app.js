// Firefly Indexer — single-page app with top-nav pages.

// ── i18n ───────────────────────────────────────────────────────────────
const I18N = {
  ko: {
    "status.checking": "확인 중…",
    "status.ready": "준비됨",
    "status.ready_manual": "준비됨 (수동)",
    "status.need_login": "Setup 필요 — 로그인",
    "status.need_install": "Setup 필요 — 설치",
    "banner.title_install": "Claude Code 셋업이 필요합니다.",
    "banner.title_login": "Claude Code 로그인이 필요합니다.",
    "banner.sub": "한 번만 설치·로그인하면 다음부터 자동으로 작동합니다.",
    "banner.btn": "Setup으로 →",
    "index.hero.sub": "사진별 설명과 편집 프롬프트(지우기·옮기기 등)를 자동 생성 → Adobe Firefly 학습 데이터 Excel.",
    "field.input_folder": "입력 폴더",
    "field.input_placeholder": "사진들이 들어있는 폴더 경로",
    "field.output_xlsx": "출력 Excel",
    "field.output_placeholder": "자동 제안됨",
    "btn.pick": "찾기…",
    "scan.hint_default": "— 폴더를 선택하면 자동 스캔합니다.",
    "advanced": "고급 설정",
    "field.scene_size": "씬당 사진 수",
    "field.model": "모델",
    "model.haiku": "Haiku 4.5 · 빠름 (추천)",
    "model.sonnet": "Sonnet 4.6 · 정확",
    "model.opus": "Opus 4.7 · 최강",
    "field.timeout": "씬당 타임아웃 (s)",
    "field.resume": "이어 실행 (옛 중단 batch 자동 재개)",
    "field.output_lang": "Excel 출력 언어",
    "lang.en_firefly": "영어 (Adobe Firefly 표준)",
    "lang.ko": "한국어",
    "btn.start": "인덱싱 시작",
    "btn.stop": "중단",
    "btn.recheck": "Recheck →",
    "btn.open_xlsx": "결과 Excel 열기",
    "btn.open_folder": "출력 폴더 열기",
    "recheck.hero.title": "Recheck",
    "recheck.hero.sub": "생성된 프롬프트가 정확한지 자동 재확인.",
    "recheck.field.xlsx": "검증할 Excel",
    "recheck.placeholder.xlsx": "인덱싱 결과 Excel",
    "recheck.btn.quick": "빠른 재확인 (양식·표현)",
    "recheck.card.title": "사진 vs 묘사 재확인 (진짜 round-trip)",
    "recheck.card.body": "Excel의 caption을 원본 사진과 비교 = 일치도 0~100점 자체 평가.",
    "recheck.field.source": "원본 사진 폴더 (Excel filename ↔ 사진 매핑)",
    "recheck.placeholder.source": "ASCII 경로 · 인덱싱 입력 폴더와 같으면 OK",
    "recheck.field.mode": "모드",
    "recheck.mode.quick": "빠른 (10% 무작위 샘플)",
    "recheck.mode.all": "전체 (모든 row)",
    "recheck.field.estimate": "예상 시간",
    "recheck.btn.start": "사진 vs 묘사 재확인 시작",
    "badge.slow": "시간 ↑",
    "report.form_errors": "양식 오류",
    "report.slop_hits": "표현 반복 감지",
    "rename.hero.title": "Rename",
    "rename.hero.sub": "표준 패턴(<code>26Q2_CK_M3_3_00001.jpg</code>)이 아닌 파일을 일괄로 사본 폴더에 정정합니다. 원본은 절대 수정되지 않습니다.",
    "rename.field.folder": "검사할 폴더",
    "rename.only_non": "비표준 파일만 정정 (표준 파일은 그대로 두기)",
    "rename.field.output": "사본 폴더 (출력)",
    "rename.btn.run": "사본 폴더로 일괄 정정",
    "rename.btn.open": "결과 폴더 열기",
    "setup.hero.title": "Setup",
    "setup.hero.sub": "본 도구는 본인 PC의 Claude Code CLI로 작동합니다.",
    "setup.install.title": "Claude Code CLI 설치",
    "setup.install.body": "아래 \"터미널 자동 실행\" 버튼을 누르세요. Node.js 설치 불필요 — 공식 native 설치 스크립트입니다.",
    "setup.login.title": "Pro / Max 로그인",
    "setup.login.body": "새 터미널 창에서 아래 명령 → 브라우저 OAuth가 자동으로 열립니다.",
    "setup.btn.run_terminal": "터미널 자동 실행",
    "setup.btn.run_terminal_login": "터미널 자동 실행 (브라우저 OAuth 자동 열림)",
    "setup.btn.recheck": "다시 체크",
    "setup.btn.recheck_login": "로그인 상태 다시 체크",
    "setup.done.title": "✓ 셋업 완료",
    "setup.done.body": "모두 준비됐습니다. Index 페이지에서 인덱싱을 시작하세요.",
    "setup.done.btn": "Index로 이동 →",
    "setup.manual.title": "이미 설치·로그인 했는데 위 체크가 실패한다면",
    "setup.manual.body": "PATH 미반영·자동 감지 실패일 수 있습니다. 본인이 이미 셋업을 마친 게 확실하면 아래 버튼으로 자동 체크를 무시하고 Index로 진행하세요. 다음 실행부터 영구 적용됩니다.",
    "setup.manual.btn": "자동 체크 무시하고 진행",
    "setup.manual.reset": "무시 해제",
  },
  en: {
    "status.checking": "checking…",
    "status.ready": "Ready",
    "status.ready_manual": "Ready (manual)",
    "status.need_login": "Setup needed — sign in",
    "status.need_install": "Setup needed — install",
    "banner.title_install": "Claude Code setup required.",
    "banner.title_login": "Claude Code login required.",
    "banner.sub": "Install and sign in once — then it just works.",
    "banner.btn": "Go to Setup →",
    "index.hero.sub": "Auto-generate per-photo descriptions and edit prompts (remove, move, etc.) → Adobe Firefly training-data Excel.",
    "field.input_folder": "Input folder",
    "field.input_placeholder": "Path to a folder containing photos",
    "field.output_xlsx": "Output Excel",
    "field.output_placeholder": "Auto-suggested",
    "btn.pick": "Browse…",
    "scan.hint_default": "— pick a folder and we'll scan it.",
    "advanced": "Advanced",
    "field.scene_size": "Photos per scene",
    "field.model": "Model",
    "model.haiku": "Haiku 4.5 · fast (recommended)",
    "model.sonnet": "Sonnet 4.6 · accurate",
    "model.opus": "Opus 4.7 · strongest",
    "field.timeout": "Per-scene timeout (s)",
    "field.resume": "Resume (continue an interrupted batch)",
    "field.output_lang": "Excel output language",
    "lang.en_firefly": "English (Adobe Firefly standard)",
    "lang.ko": "Korean",
    "btn.start": "Start indexing",
    "btn.stop": "Stop",
    "btn.recheck": "Recheck →",
    "btn.open_xlsx": "Open Excel",
    "btn.open_folder": "Open folder",
    "recheck.hero.title": "Recheck",
    "recheck.hero.sub": "Auto-recheck whether the generated prompts are accurate.",
    "recheck.field.xlsx": "Excel to check",
    "recheck.placeholder.xlsx": "Generated indexing Excel",
    "recheck.btn.quick": "Quick recheck (form · phrasing)",
    "recheck.card.title": "Photo vs description recheck (true round-trip)",
    "recheck.card.body": "Compare each caption back against the original photo — 0~100 self-assessed match score.",
    "recheck.field.source": "Original photo folder (maps Excel filename → photo)",
    "recheck.placeholder.source": "ASCII path · same as the indexing input folder is fine",
    "recheck.field.mode": "Mode",
    "recheck.mode.quick": "Quick (random 10% sample)",
    "recheck.mode.all": "All (every row)",
    "recheck.field.estimate": "Estimated time",
    "recheck.btn.start": "Start photo-vs-description recheck",
    "badge.slow": "slow",
    "report.form_errors": "Form errors",
    "report.slop_hits": "Repetition / slop",
    "rename.hero.title": "Rename",
    "rename.hero.sub": "Bulk-fix non-conforming filenames (vs. <code>26Q2_CK_M3_3_00001.jpg</code>) into a sidecar copy folder. Originals are never modified.",
    "rename.field.folder": "Folder to inspect",
    "rename.only_non": "Only rename non-conforming files (keep conforming as-is)",
    "rename.field.output": "Output copy folder",
    "rename.btn.run": "Bulk rename to copy folder",
    "rename.btn.open": "Open result folder",
    "setup.hero.title": "Setup",
    "setup.hero.sub": "This tool runs on the Claude Code CLI installed on your PC.",
    "setup.install.title": "Install Claude Code CLI",
    "setup.install.body": "Click the \"Run in terminal\" button below. No Node.js needed — this is the official native install script.",
    "setup.login.title": "Pro / Max sign in",
    "setup.login.body": "Run this in a new terminal — the browser OAuth opens automatically.",
    "setup.btn.run_terminal": "Run in terminal",
    "setup.btn.run_terminal_login": "Run in terminal (browser OAuth opens automatically)",
    "setup.btn.recheck": "Recheck",
    "setup.btn.recheck_login": "Recheck login status",
    "setup.done.title": "✓ Setup complete",
    "setup.done.body": "All set. Go to the Index page and start indexing.",
    "setup.done.btn": "Go to Index →",
    "setup.manual.title": "Already installed and signed in but the check above fails?",
    "setup.manual.body": "PATH might not be refreshed, or auto-detection might have missed it. If you're sure setup is complete, override the auto-check below. The choice persists for next launches.",
    "setup.manual.btn": "Override auto-check",
    "setup.manual.reset": "Reset override",
  },
};
const LANG_KEY = "firefly_indexer_lang";
let CURRENT_LANG = "ko";
function applyLang(lang) {
  const dict = I18N[lang] || I18N.en;
  CURRENT_LANG = lang;
  document.documentElement.lang = lang;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    const val = dict[key];
    if (val == null) return;
    if (el.hasAttribute("data-i18n-html") || /<\w+/.test(val)) el.innerHTML = val;
    else el.textContent = val;
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    const key = el.getAttribute("data-i18n-placeholder");
    const val = dict[key];
    if (val != null) el.setAttribute("placeholder", val);
  });
  document.querySelectorAll(".lang-btn").forEach((b) =>
    b.classList.toggle("lang-btn--active", b.dataset.lang === lang));
  try { localStorage.setItem(LANG_KEY, lang); } catch {}
}
function t(key) {
  const dict = I18N[CURRENT_LANG] || I18N.en;
  return dict[key] ?? key;
}
function initLang() {
  let lang = "ko";
  try {
    const saved = localStorage.getItem(LANG_KEY);
    if (saved) lang = saved;
    else if (!(navigator.language || "").toLowerCase().startsWith("ko")) lang = "en";
  } catch {}
  applyLang(lang);
}

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
    title.textContent = t("banner.title_login");
    sub.textContent = t("banner.sub");
  } else {
    banner.removeAttribute("hidden");
    title.textContent = t("banner.title_install");
    sub.textContent = t("banner.sub");
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
  setStatusDot("busy", t("status.checking"));
  try {
    const s = await invoke("check_claude_status");
    setBadge($("#badge-install"), s.installed ? "ok" : "err", s.installed ? "INSTALLED" : "NOT FOUND");
    setBadge($("#badge-login"),   s.logged_in ? "ok" : "warn", s.logged_in ? "LOGGED IN" : "NOT LOGGED IN");

    const override = isManuallyOverridden();
    if (override)                     setStatusDot("ok",   t("status.ready_manual"));
    else if (s.installed && s.logged_in) setStatusDot("ok",   t("status.ready"));
    else if (s.installed)            setStatusDot("warn", t("status.need_login"));
    else                              setStatusDot("err",  t("status.need_install"));

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
    output_lang:  ($("#output-lang") && $("#output-lang").value) || "en",
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
function applyOsInstallCmd() {
  // Show the right native-installer command for the user's OS.
  const ua = (navigator.userAgent || "").toLowerCase();
  const isWin = ua.includes("windows");
  const el = document.getElementById("install-cmd");
  if (el) {
    el.textContent = isWin
      ? "irm https://claude.ai/install.ps1 | iex"
      : "curl -fsSL https://claude.ai/install.sh | bash";
  }
}

async function init() {
  // i18n first — paints UI in the user's preferred language before checks
  initLang();
  applyOsInstallCmd();
  $$(".lang-btn").forEach((b) => {
    b.addEventListener("click", () => {
      applyLang(b.dataset.lang);
      // Refresh dynamic strings that aren't bound via data-i18n
      checkCliStatus();
    });
  });

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
