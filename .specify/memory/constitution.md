<!--
Sync Impact Report
- Version change: template (unratified) -> 1.0.0
- Modified principles:
  - Placeholder Principle 1 -> I. Mobile First, Proven on Devices
  - Placeholder Principle 2 -> II. Hexagonal Rust Boundaries
  - Placeholder Principle 3 -> III. Feature-Sliced React and Explicit State Ownership
  - Placeholder Principle 4 -> IV. Secure and Private Transcription
  - Placeholder Principle 5 -> V. Resilient, Verifiable Voice Workflows
- Added sections:
  - Product and Technology Constraints
  - Development Workflow and Quality Gates
- Removed sections: none
- Templates synchronized:
  - ✅ .specify/templates/plan-template.md
  - ✅ .specify/templates/spec-template.md
  - ✅ .specify/templates/tasks-template.md
  - ✅ .specify/templates/commands/ (directory not present in this installation)
- Runtime guidance synchronized:
  - ✅ README.md
  - ✅ docs/tauri-mobile-voice-memo.md
- Deferred follow-ups: none
-->
# STT Voice Memo Constitution

## Core Principles

### I. Mobile First, Proven on Devices

iOS와 Android가 제품 및 개발의 최우선 대상이다. 모든 기능 명세와 구현 계획은
모바일 권한, 오디오 세션, 앱 수명 주기, 터치 상호작용 및 네트워크 변동을 먼저
정의해야 한다. 녹음 관련 기능은 시뮬레이터만으로 완료 처리할 수 없으며 실제 iOS와
Android 기기에서 핵심 흐름 및 실패 흐름을 검증해야 한다. 첫 릴리스는 foreground
녹음에 집중하며 데스크톱, 백그라운드 녹음 및 실시간 변환은 모바일 핵심 경험이
검증되거나 별도 명세에서 우선순위 변경이 승인된 뒤 진행한다.

이 원칙은 공통 기술 선택이 모바일 플랫폼의 실제 제약을 숨기지 않게 하고, 후순위
플랫폼 때문에 모바일 출시가 지연되는 것을 막는다.

### II. Hexagonal Rust Boundaries

Rust 코드는 Hexagonal Architecture를 따라야 한다. `domain`은 순수 모델과 규칙만,
`application`은 유스케이스만 소유하며 Tauri, 운영체제 API, 파일 시스템, 데이터베이스,
HTTP 클라이언트 또는 OpenAI SDK에 직접 의존해서는 안 된다. 외부 기능은 `ports`로
정의하고 iOS/Android 녹음, 저장소, 백엔드 통신 및 Tauri command는 inbound 또는
infrastructure adapter로 구현해야 한다. Tauri command는 입력 검증과 유스케이스 호출만
담당하며 비즈니스 규칙을 포함해서는 안 된다.

새 외부 의존성 또는 플랫폼 분기는 반드시 port와 adapter 경계를 통해 도입하고,
domain/application 단위 테스트에서 실제 플랫폼이나 네트워크 없이 검증 가능해야 한다.

### III. Feature-Sliced React and Explicit State Ownership

React 코드는 `app -> pages -> widgets -> features -> entities -> shared` 순서의
Feature-Sliced Design 계층을 따라야 하며, 하위 계층은 상위 계층을 import해서는 안 된다.
사용자 행동은 `features`, 도메인 표현은 `entities`, 플랫폼 및 API client와 범용 UI는
`shared`에 둔다. 예외적인 cross-import는 계획의 Complexity Tracking에 이유와 제거
조건을 기록해야 한다.

TanStack Query는 원격 비동기 상태, 캐시, mutation 및 재시도를 소유한다. Zustand는
녹음 세션과 일시적인 UI처럼 클라이언트에만 존재하는 상태를 소유한다. 동일 데이터를
두 저장소에 복제하거나 서버 상태를 Zustand의 별도 진실 공급원으로 만들어서는 안 된다.

### IV. Secure and Private Transcription

OpenAI API 키와 백엔드 자격 증명은 Tauri 번들, 프런트엔드 코드, 클라이언트 저장소 및
로그에 포함해서는 안 된다. 클라이언트의 음성 변환 요청은 애플리케이션이 관리하는
백엔드를 거쳐야 하며, Tauri capability와 플랫폼 권한은 필요한 최소 범위만 허용해야 한다.

각 기능 명세는 오디오와 transcript의 생성 위치, 전송 대상, 보관 기간, 삭제 시점 및
사용자 제어를 명시해야 한다. 원본 오디오 보관은 사용자에게 보이는 선택이어야 하며,
오디오·transcript·자격 증명은 기본 로그 또는 분석 이벤트에 기록해서는 안 된다. 외부
서비스로 전송되는 데이터와 실패 후 남는 임시 파일은 사용자와 개발자가 추적 가능해야 한다.

### V. Resilient, Verifiable Voice Workflows

녹음, 파일 확정, 업로드, 변환 및 메모 저장은 명시적인 상태와 전이로 모델링해야 한다.
권한 거부, 오디오 인터럽트, 앱 background/termination, 중복 요청, 오프라인 상태, 저속
네트워크 및 부분 실패에 대한 결과와 복구 경로를 각 관련 명세에 포함해야 한다. 재시도는
중복 메모나 중복 과금을 만들지 않도록 식별자와 idempotency 경계를 가져야 한다.

도메인 규칙과 port 계약은 자동화된 테스트로 검증해야 한다. 녹음 또는 플랫폼 수명 주기를
변경하는 작업은 실제 iOS 및 Android 기기 검증 항목을 포함해야 하며, 기능 완료 증거에는
성공 흐름과 최소 하나의 관련 실패/복구 흐름이 모두 포함되어야 한다. 측정 불가능한 성공
기준이나 검증 계획이 없는 기능은 구현 준비 상태가 아니다.

## Product and Technology Constraints

- 애플리케이션은 Tauri 2, Rust, React, TypeScript 및 shadcn/ui를 기본 기술로 사용한다.
- 모바일 recorder는 공통 port 뒤의 Swift iOS adapter와 Kotlin Android adapter로 우선
  구현한다. 웹뷰 `MediaRecorder`만을 모바일 녹음의 유일한 구현으로 사용할 수 없다.
- GPT Transcribe API에는 애플리케이션 백엔드만 직접 접근한다.
- transcript는 메모의 기본 데이터다. 원본 오디오와 파생 오디오는 서로 다른 보관 정책을
  가질 수 있으며, VAD 산출물을 원본 녹음으로 취급해서는 안 된다.
- 첫 릴리스 범위는 foreground 음성 메모 녹음, post-recording 변환, 편집 및 로컬 저장이다.
- Handy는 기능과 UX 및 선택적 오디오 알고리즘의 참고 자료다. 코드를 재사용할 때는 모바일
  적합성과 라이선스를 검토하고 현재 port/adapter 경계 안으로 추출해야 한다.
- `~/project/agentic-workspace`의 Rust Hexagonal Architecture와 React Feature-Sliced
  Design을 구조적 기준으로 삼되, 이 프로젝트의 모바일 요구가 우선한다.

## Development Workflow and Quality Gates

1. 각 기능은 우선순위가 지정되고 독립 검증 가능한 모바일 사용자 여정으로 명세해야 한다.
2. 명세는 iOS/Android 동작, 권한 및 수명 주기 edge case, 데이터 보관과 실패 복구를
   포함해야 한다.
3. 구현 계획은 코드 작성 전에 다섯 Core Principle을 모두 통과해야 하며, 위반은
   Complexity Tracking에 필요성, 대안 및 제거 조건을 기록해야 한다.
4. 작업 목록은 domain/port 계약 테스트, adapter 구현, 보안·개인정보 처리 및 실제 기기
   검증을 누락해서는 안 된다.
5. 변경 검토 시 아키텍처 의존 방향, 상태 소유권, secret 노출, 오디오 데이터 수명 주기 및
   실패 복구 증거를 확인해야 한다.
6. 새로운 플랫폼 또는 후순위 기능은 모바일 P1 흐름의 회귀를 방지하는 자동 테스트와 실제
   기기 회귀 검증 계획을 포함해야 한다.

## Governance

이 constitution은 프로젝트의 다른 관행과 문서보다 우선한다. 명세, 계획 또는 구현이 이
문서와 충돌하면 구현을 진행하기 전에 충돌을 해소하거나 명시적인 constitution 개정을
승인해야 한다.

개정안은 변경 이유, 영향을 받는 원칙과 템플릿, 마이그레이션 또는 후속 작업을 함께
기록해야 한다. 버전은 Semantic Versioning을 따른다. 원칙 제거 또는 호환되지 않는 재정의는
MAJOR, 새 원칙이나 실질적인 의무 확장은 MINOR, 의미를 바꾸지 않는 명확화는 PATCH로 올린다.

모든 기능 계획은 Phase 0 이전과 Phase 1 설계 이후 Constitution Check를 수행해야 한다.
모든 변경 검토는 적용되는 원칙의 준수 여부를 확인해야 하며, 예외는 계획의 Complexity
Tracking에 근거와 종료 조건이 있어야 한다. README와 `docs/`는 운영 지침을 제공하지만,
충돌 시 이 constitution을 기준으로 수정한다.

**Version**: 1.0.0 | **Ratified**: 2026-08-15 | **Last Amended**: 2026-08-15
