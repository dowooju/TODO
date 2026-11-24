# 내 달력 — 캘린더 + 체크리스트

날짜가 정해진 할 일과 **날짜가 정해지지 않은 "언젠가" 할 일**을 한 화면에서 함께 보고 관리하는 개인용 Windows 데스크톱 앱입니다.
요구사항은 [PRD.md](PRD.md), 화면 흐름은 [design/ui-flow.webp](design/ui-flow.webp) 를 참고하세요.

## 주요 기능

| 기능 | 설명 |
|------|------|
| 월간 달력 | 날짜 칸마다 할 일 1개당 태그 색 네모 1개, 완료하면 회색. 오늘은 핑크로 강조 |
| 언젠가 할 일 | 날짜 없는 할 일을 오른쪽 패널에 태그별로 모아 표시. "오늘 하기" 버튼(마우스를 올리면 보임) |
| 끌어오기 | 언젠가 항목을 달력 날짜로 드래그하면 그 날짜로 지정, 체크리스트 항목을 패널로 드래그하면 날짜 해제 |
| 주간 체크리스트 | 날짜를 누르면 주간 바 + 그날 체크리스트. 완료해도 자리 그대로(회색 + 취소선) |
| 상세 패널 | 체크리스트 항목을 누르면 오른쪽에 태그·날짜·마감·설명과 완료/수정/삭제 |
| 태그 | ••• 메뉴에서 태그 관리(이름·8가지 색), 태그로 보기(필터) |
| 마감 알림 | 날짜 + 마감 시간이 있는 할 일을 **마감 10분 전**에 Windows 알림. 설정에서 켜기/끄기 |
| 트레이 상주 | 창을 닫아도 트레이에 남아 알림을 보냄. 완전히 끄려면 트레이 아이콘 오른쪽 클릭 → 종료 |
| 자동 실행 | **Windows 시작 시 자동 실행이 기본으로 켜짐** (트레이에서 조용히 시작). 설정에서 끌 수 있음 |
| 구글 캘린더 | 날짜 있는 할 일을 구글 기본 캘린더와 양방향 동기화, 구글 일정도 달력에 함께 표시 |

## 개발 환경

- Windows 10/11
- [Node.js](https://nodejs.org/) LTS
- [Rust](https://rustup.rs/) (MSVC 툴체인)
- Visual Studio Build Tools — "C++를 사용한 데스크톱 개발"
- WebView2 런타임 (Windows 11 에는 기본 포함)

```powershell
npm install
```

## 실행

```powershell
npm run tauri dev
```

화면만 빠르게 보고 싶으면 `npm run dev` 후 브라우저에서 http://localhost:1420 을 열면 됩니다.
이때는 Rust 백엔드 대신 localStorage 를 쓰는 mock 백엔드로 동작합니다.

## 설치 파일 만들기

```powershell
npm run tauri build
```

`src-tauri\target\release\bundle\nsis\내 달력_0.1.0_x64-setup.exe` 가 만들어집니다.
설치한 앱을 처음 실행하면 Windows 시작 시 자동 실행이 등록됩니다 (개발 빌드는 등록하지 않음).

## 구글 캘린더 연동 설정

구글 로그인에는 Google Cloud 의 OAuth 클라이언트 ID 가 필요합니다. 한 번만 하면 됩니다.

1. [Google Cloud Console](https://console.cloud.google.com/) 에서 새 프로젝트를 만듭니다.
2. **API 및 서비스 → 라이브러리** 에서 **Google Calendar API** 를 사용 설정합니다.
3. **API 및 서비스 → OAuth 동의 화면** 을 설정합니다.
   - 사용자 유형: 외부, 범위: `.../auth/calendar.events`
   - **게시 상태를 "프로덕션"으로 바꿔 주세요.** "테스트" 상태로 두면 리프레시 토큰이 7일 뒤 만료되어 매주 다시 로그인해야 합니다.
   - 검증을 받지 않았으므로 로그인할 때 "확인되지 않은 앱" 경고가 뜹니다. 본인만 쓰는 앱이므로 **고급 → 계속** 으로 진행하면 됩니다.
4. **API 및 서비스 → 사용자 인증 정보 → 사용자 인증 정보 만들기 → OAuth 클라이언트 ID** 를 누르고 유형을 **데스크톱 앱** 으로 고릅니다.
5. 프로젝트 루트에 `.env` 파일을 만들고 값을 넣습니다 (`.env.example` 참고, `.env` 는 git 에 올라가지 않습니다).

   ```
   GOOGLE_CLIENT_ID=xxxxxxxx.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=GOCSPX-xxxxxxxx
   ```

6. 앱을 다시 빌드합니다 (`npm run tauri dev` 또는 `npm run tauri build`).

클라이언트 ID 없이 빌드하면 로그인 화면에 "구글 연동 없이 시작하기 (개발용)" 버튼이 보입니다.

### 동기화 규칙

- 날짜가 있는 할 일만 구글 캘린더에 올라갑니다. 날짜를 지우면 구글에서도 지워집니다.
- 날짜만 있으면 종일 일정, 마감 시간이 있으면 "마감 30분 전 ~ 마감" 일정이 됩니다.
- 완료한 할 일은 구글 일정 제목 앞에 ✅ 가 붙습니다.
- 앱에서 바꾸면 바로 보내고, 구글 쪽 변경은 앱 실행 시와 5분마다 받아옵니다 (설정 → 지금 동기화).
- 구글 캘린더에서 일정을 지우면 앱의 할 일도 지워집니다.
- 양쪽에서 같은 할 일을 바꾸면 나중에 바꾼 쪽이 이깁니다.
- 인터넷이 끊겨도 앱은 그대로 쓸 수 있고, 연결되면 이어서 동기화합니다.

## 데이터 위치

| 내용 | 위치 |
|------|------|
| 할 일·태그·설정 (SQLite) | `%APPDATA%\com.todocalendar.app\todo.db` |
| 로그 (최대 약 5MB 순환) | `%LOCALAPPDATA%\com.todocalendar.app\logs\` |
| 구글 리프레시 토큰 | Windows 자격 증명 관리자 → `com.todocalendar.app` |

앱을 업데이트하거나 재설치해도 데이터는 유지됩니다.

## 테스트

```powershell
npm test
```

```powershell
cd src-tauri; cargo test
```

## 구조

```
src/                    React 화면
  api/                  Rust 명령 호출 (+ 브라우저용 mock)
  components/           달력, 주간 체크리스트, 언젠가 패널, 상세, 메뉴, 설정 등
  lib/                  날짜 계산, 태그 색 팔레트
  store.ts              Zustand 상태
src-tauri/src/          Rust 백엔드
  db.rs                 SQLite 연결과 마이그레이션
  repo.rs               할 일 / 태그 / 설정 저장소
  commands.rs           화면에서 부르는 명령
  notifier.rs           마감 10분 전 알림 스케줄러
  tray.rs               트레이, 창 수명 관리
  google/               구글 로그인(OAuth PKCE), Calendar API
  sync/                 양방향 동기화 워커, outbox, 일정 변환
```
