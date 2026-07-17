# pinplay

`pinplay`는 로컬 영상 파일이나 웹 영상 주소를 터미널에서 바로 작고 항상
위에 표시되는 창으로 재생하는 CLI입니다. 운영체제의 제한적인 네이티브
PiP 대신 mpv의 테두리 없는 floating window를 사용하므로 96x54 같은 매우
작은 초기 크기와 자유로운 리사이즈를 지원합니다.

```bash
pinplay "https://youtu.be/..." --start 1:30 --mute --speed 1.5 --size tiny
pinplay "./movie file.mkv" --size 240x135 --position top-left
```

## 요구 사항

- [mpv](https://mpv.io/) — 영상 재생과 floating window
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — YouTube 등 웹사이트 주소 처리
- 최신 YouTube 지원을 위한 JavaScript runtime. Homebrew의 yt-dlp 설치 시
  Deno가 함께 설치됩니다.

macOS에서는 다음 명령으로 설치할 수 있습니다.

```bash
brew install mpv yt-dlp
```

## 설치

Rust 1.85 이상이 설치되어 있다면 저장소에서 바로 설치할 수 있습니다.

```bash
cd pinplay
./scripts/install.sh
pinplay doctor
```

의존성까지 한 번에 설치하려면 macOS에서 다음 옵션을 사용할 수 있습니다.

```bash
./scripts/install.sh --with-deps
```

또는 Cargo 기본 경로에 설치할 수 있습니다.

```bash
cargo install --path . --locked
```

## 사용법

```text
pinplay [OPTIONS] <SOURCE>
pinplay doctor [--json]
```

주요 옵션:

| 옵션 | 설명 |
| --- | --- |
| `-s, --start <TIME>` | 시작 위치: `90`, `1:30`, `01:02:03.5`, `25%`, `#3` |
| `-m, --mute` | 음소거 상태로 시작 |
| `-r, --speed <RATE>` | 재생 속도 `0.01..=100` |
| `--volume <0-100>` | 시작 볼륨 |
| `--size <SIZE>` | `nano`, `tiny`, `small`, `medium`, `large`, `WIDTHxHEIGHT` |
| `--position <CORNER>` | `top-left`, `top-right`, `bottom-left`, `bottom-right` |
| `--margin <PIXELS>` | 선택한 화면 가장자리와의 간격 |
| `--loop` | 무한 반복 |
| `--controls` | mpv의 화면 컨트롤 표시 |
| `--border` | 운영체제 창 테두리 표시 |
| `--detach` | 플레이어를 실행하고 터미널로 즉시 복귀 |
| `--dry-run` | mpv를 실행하지 않고 전달할 인자만 출력 |

크기 프리셋:

| 프리셋 | 크기 |
| --- | --- |
| `nano` | 96x54 |
| `tiny` | 160x90 |
| `small` | 320x180 (기본값) |
| `medium` | 480x270 |
| `large` | 640x360 |

창은 기본적으로 오른쪽 아래에 24px 간격을 두고 표시됩니다. 테두리가 없는
상태에서도 창을 드래그하거나 가장자리를 잡아 크기를 변경할 수 있습니다.

## 예제

```bash
# YouTube 영상을 1분 30초부터 1.5배속으로 재생
pinplay "https://www.youtube.com/watch?v=..." --start 1:30 --speed 1.5

# 화면 왼쪽 위에 매우 작은 음소거 창 표시
pinplay "https://example.com/live.m3u8" --size nano --position top-left --mute

# 로컬 MKV 파일 반복 재생
pinplay "./videos/demo.mkv" --loop --volume 30

# 터미널을 점유하지 않고 실행
pinplay "https://youtu.be/..." --detach
```

셸이 `&`, `?`, 공백 등을 먼저 해석하지 않도록 URL과 공백이 있는 경로는
항상 따옴표로 감싸는 것을 권장합니다.

## 진단

`doctor`는 네트워크 요청 없이 로컬 실행 파일과 기능 준비 상태를 확인합니다.

```bash
pinplay doctor
pinplay doctor --json
```

`--json` 출력은 패키지 설치 검사나 자동화에서 사용할 수 있는 버전이 지정된
스키마를 제공합니다. `mpv` 또는 `yt-dlp`가 없으면 종료 코드 `3`을 반환합니다.

## 설계와 안전성

Pinplay은 입력을 셸 명령 문자열로 합치지 않습니다. 검증된 옵션은 각각 별도
인자로 만들고, 영상 주소나 파일 경로는 mpv의 `--` 구분자 뒤에 정확히 하나의
프로세스 인자로 전달합니다. 로컬 파일은 실행 전에 존재 여부를 확인하고
절대 경로로 변환합니다.

기본적으로 `--no-config`로 mpv를 실행해 사용자 설정이나 스크립트가 Pinplay의
창 동작을 바꾸지 않게 합니다. 임의의 mpv/yt-dlp 옵션을 전달하는 escape hatch는
의도적으로 제공하지 않습니다.

## 지원 범위

- mpv/FFmpeg가 해석할 수 있는 로컬 파일과 직접 미디어 URL
- 최신 yt-dlp가 지원하는 웹사이트 주소
- macOS를 우선 지원합니다.
- Windows와 X11 기반 Linux는 소스 빌드 및 CI 대상이며, 별도 설치 패키지는
  아직 제공하지 않습니다.

DRM, 지역 제한, 로그인 또는 쿠키가 필요한 콘텐츠는 보장하지 않습니다.
Wayland에서는 compositor 정책 때문에 창 위치나 always-on-top 요청이 무시될 수
있습니다. 실제 최소 리사이즈 크기도 운영체제와 창 관리자에 따라 달라집니다.

YouTube, mpv, yt-dlp와 공식 제휴한 프로젝트가 아닙니다. 접근 및 재생 권한이
있는 콘텐츠에만 사용하세요.

## 개발

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked

# Homebrew cargo-llvm-cov를 설치한 경우
LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov \
LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata \
cargo llvm-cov --all-targets --locked --summary-only --fail-under-lines 80
```

## 라이선스

Pinplay은 [MIT 라이선스](LICENSE)로 배포됩니다. mpv와 yt-dlp는 번들되지 않는
별도 프로그램이며 각 프로젝트의 라이선스가 적용됩니다. 자세한 내용은
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)를 참고하세요.
