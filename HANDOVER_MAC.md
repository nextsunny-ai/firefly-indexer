# Firefly Indexer — Mac 빌드 인수인계

> 대표님 맥북에서 = 다음 클로드(또는 본인)가 이어서 = **.dmg / .app 빌드 + 의뢰자 전달**까지.

---

## 한 줄 요약

Windows 노트북에서 **코드 전부 작성 완료**. macOS 빌드만 = 맥북에서 = `npm install` + `npm run tauri build` = 한 번이면 끝.

---

## 위치

- **Drive 영구**: `G:/내 드라이브/SUNNY_TEAM/에이전시_외주/어도비_파이어플라이/프로그램_firefly_indexer_tauri/`
- **운영 코드** (Windows): `D:/WORK/firefly_indexer_tauri/` (= Drive에서 받아 ASCII 경로로 작업)

맥북에서 = Drive 받은 후 = 아무 ASCII 경로 폴더에 복사 (= 예: `~/work/firefly_indexer_tauri`).

---

## 사전 준비 (맥북 1회)

```bash
# Xcode Command Line Tools
xcode-select --install

# Rust (1.75+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 18+
brew install node          # 또는 nvm 으로 18+ 설치

# Claude Code CLI (= 빌드 직접 사용은 X, 의뢰자 PC에 필요)
npm install -g @anthropic-ai/claude-code
```

---

## 빌드

```bash
cd ~/work/firefly_indexer_tauri
npm install
npm run tauri build
```

산출물:
- `src-tauri/target/release/bundle/macos/Firefly Indexer.app`
- `src-tauri/target/release/bundle/dmg/Firefly Indexer_0.1.0_aarch64.dmg` (Apple Silicon)
- 또는 `_x64.dmg` (Intel)

★ Universal binary (Apple Silicon + Intel 동시) 원하면:
```bash
npm run tauri build -- --target universal-apple-darwin
```
(= `rustup target add x86_64-apple-darwin aarch64-apple-darwin` 먼저)

---

## ⚠️ 옛 사고 자료 (= 코딩실수모음.md `#tauri_bundle_targets_OS분기`)

빌드 후 검증:
1. `src-tauri/target/release/bundle/macos/*.app` 존재 확인 — 없으면 = artifact 0개 사고
2. `bundle/dmg/*.dmg` 존재 확인
3. `Finished release` 만 보이고 = artifact 0개면 = `tauri.conf.json` `bundle.targets` 확인 (= 우리는 이미 `["nsis","msi","app","dmg"]` 박음 = OK)

---

## 의뢰자 테스트 → 정정 → 출시

1. .dmg를 의뢰자에게 전달 (= 카톡·Slack·Drive 링크)
2. 의뢰자 PC 1회 셋업:
   - Claude Pro/Max 구독 박혀있어야 함
   - 터미널 = `npm install -g @anthropic-ai/claude-code`
   - 터미널 = `claude /login` = 브라우저 OAuth 1회
3. 의뢰자 = .dmg 더블클릭 = .app 실행 = preflight wizard 자동 = "Logged In" 확인 = main UI
4. 사진 폴더 선택 → 인덱싱 → Excel 생성 검증

문제 발생 시 = 의뢰자 화면 캡처 + 로그 → 정정.

---

## 코드 구조

```
firefly_indexer_tauri/
├── package.json                # Tauri CLI + plugins (dialog, fs, opener)
├── frontend/                   # 단일 HTML 앱
│   ├── index.html              # 4 step wizard (preflight → input → output → running → done)
│   └── assets/
│       ├── style.css           # 디자인 시스템 v1 풀 적용 (Pretendard + 모노 라벨 + 토큰)
│       └── app.js              # invoke + listen + 단계 전환
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json         # bundle.targets = ["nsis","msi","app","dmg"]
    │                           # withGlobalTauri: true  (★ Tauri v2 외부 URL 대응)
    ├── build.rs
    ├── capabilities/default.json
    ├── icons/                  # ico + icns + 32/128 png  (스토리메이커에서 복사)
    └── src/
        ├── main.rs             # entry (windows_subsystem = "windows")
        ├── lib.rs              # Tauri command handlers
        ├── claude_cli.rs       # claude binary search + auth status + run_scene subprocess
        ├── prompts.rs          # system prompt + user message builder (양식 명세 풀)
        ├── naming.rs           # scan_folder + apply_rename (원본 보존, 사본 폴더로 정정)
        ├── excel.rs            # rust_xlsxwriter — scenes + characterid 2 시트
        └── indexer.rs          # batch loop + progress events
```

---

## Tauri commands (frontend ↔ rust)

| command | input | output |
|---|---|---|
| `check_claude_status` | — | `CliStatus { installed, logged_in, bin_path, ...hint }` |
| `scan_folder` | `folder: string` | `ScanReport` |
| `apply_rename` | `RenameRequest` | `RenameReport { copied, mapping_csv, rows }` |
| `run_indexing` | `IndexRequest` | `IndexSummary { scenes_done, total_cost_usd, output_path }` (+ `indexing-progress` events) |
| `open_path` | `path: string` | () — OS native open |
| `suggest_output_path` | `inputFolder: string` | string |

---

## 진행 상태 (2026-05-15 야간 시점)

- [x] Tauri 2.0 scaffold + cross-platform 빌드 설정
- [x] Rust backend (claude CLI wrapper + cross-OS auth)
- [x] Rust 모듈: prompts / naming / excel / indexer
- [x] lib.rs + main.rs + 6 Tauri commands
- [x] HTML/CSS/JS frontend (디자인 시스템 v1 풀 적용)
- [x] Windows 빌드 검증 (debug --no-bundle = compile OK 확인)
- [ ] **Mac .app / .dmg 빌드** (= 맥북에서 이어서)
- [ ] 의뢰자 테스트 + 정정 사이클
- [ ] evaluator.rs (= 양식 검증 + 정답지 일치율 자동 측정) — 우선순위 ↑
- [ ] 후속: 라이선스·결제·자동 업데이트 (= 판매 결정 후)

---

## 검증된 옛 사고 → 우리에게 박힌 룰

| 옛 사고 | 우리 코드 위치 | 룰 |
|---|---|---|
| #tauri_oauth_chrome_session_X | (해당 X — 로그인 X 결정) | OAuth = 외부 Chrome 박음 (= 추후 라이선스 추가 시) |
| #tauri_v2_외부_URL + withGlobalTauri_누락 | `tauri.conf.json` | `withGlobalTauri: true` 박힘 |
| #mac_claude_oauth_keychain | `claude_cli.rs is_logged_in()` | `claude auth status` JSON subprocess (= OS-invariant) |
| #tauri_bundle_targets_OS분기 | `tauri.conf.json bundle.targets` | `["nsis","msi","app","dmg"]` 4 타겟 |
| #npm_global_권한사고 | 안내 텍스트 (preflight) | `npm install -g` 안내, EACCES 시 `~/.npm-global` |
| #OAuth_subprocess_사고 | `claude_cli.rs run_scene()` | `claude` CLI subprocess만 |
| #subprocess_context_bleed | `claude_cli.rs child_env() + run_scene()` | env scrub + tempfile cwd + stdin pipe |

---

## 다음 클로드 진입 시

1. Drive `프로그램_firefly_indexer_tauri/` 받기
2. 본 MD read
3. 맥북 사전 준비 (= Rust + Node + claude CLI 1회 설치)
4. `npm install && npm run tauri build` → .dmg 생성
5. 의뢰자에게 전달
6. 정정 사이클
