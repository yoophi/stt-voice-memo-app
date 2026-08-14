# UI Contract: Mobile App Shell

## Purpose

iOS와 Android에서 동일하게 보이는 초기 `STT Voice Memo` shell의 관찰 가능한 계약이다.
이 화면은 제품 기반이 실행되었음을 확인하기 위한 것이며 녹음 기능의 mock이 아니다.

## Required content

- 화면의 primary heading은 `STT Voice Memo`다.
- 앱 기반이 준비되었고 녹음 기능은 아직 제공되지 않음을 오해 없이 알리는 짧은 설명을
  표시한다.
- 녹음 시작, transcript 생성, memo 저장 또는 로그인처럼 동작할 것으로 보이는 enabled
  control을 표시하지 않는다.
- shadcn/ui 기반의 최소 presentation component를 사용하되 platform별 제품 정체성은 같다.

## Layout and accessibility

- viewport는 `100svh` 기반이며 상하좌우 `env(safe-area-inset-*)`를 고려한다.
- 지원 최소 viewport는 CSS pixel 기준 `320 × 568`이며 portrait와 landscape에서 heading과
  상태 설명이 잘리거나 겹치지 않는다.
- 본문은 확대 가능한 semantic HTML을 사용하고 색만으로 준비 상태를 전달하지 않는다.
- touch target이 생길 경우 최소 44 CSS px를 적용하지만 이 feature에는 필수 action이 없다.

## Lifecycle behavior

- cold launch마다 같은 ready shell을 표시한다.
- 시작 중 또는 표시 후 background로 이동했다가 foreground로 돌아오면 별도 복구 action
  없이 같은 shell을 표시한다.
- 시작/복귀 시 network, recording, file access 또는 transcription side effect를 실행하지 않는다.

## Permission and privacy behavior

- iOS와 Android 모두 microphone, photo, location, notification 등 민감한 runtime permission
  prompt를 표시하지 않는다.
- 오디오, transcript, memo 또는 credential을 생성·읽기·전송·저장하지 않는다.
- 개발 로그는 lifecycle 및 tooling 진단에 한정하며 환경 변수 값이나 개인 데이터를 출력하지
  않는다.

## Automated assertions

- heading과 ready 설명이 render된다.
- recording/transcription action이 존재하지 않는다.
- app composition root가 `QueryClientProvider`를 정확히 한 번 제공한다.
- 초기 render에서 Tauri invoke 또는 network request가 발생하지 않는다.

## Physical-device acceptance

각 플랫폼에서 다음을 모두 만족해야 한다.

1. 실제 기기에 설치하고 5회 연속 cold launch한다.
2. portrait/landscape에서 주요 콘텐츠를 확인한다.
3. background로 이동한 뒤 foreground로 복귀한다.
4. 예상하지 않은 permission prompt가 0회인지 확인한다.

증거는 `tests/device/mobile-shell-smoke.md` 형식으로 기록한다.
