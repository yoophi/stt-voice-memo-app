# Data Model: Mobile Tauri App Foundation

## Scope

이 초기화 기능은 영속 비즈니스 엔터티를 만들지 않는다. 오디오, transcript, memo,
recording session, user, credential 및 server cache는 모두 후속 feature의 소유물이다.
따라서 database schema, migration, repository port 또는 Zustand persistence가 없다.

아래 모델은 구성과 검증 계약을 명확히 하기 위한 비영속 값이며 런타임 사용자 데이터로
저장하지 않는다.

## AppIdentity

앱 표시와 native bundle에 동일하게 적용되는 빌드 구성 값이다.

| Field | Type | Value / Rule |
|---|---|---|
| `productName` | string | 정확히 `STT Voice Memo` |
| `identifier` | reverse-DNS string | `com.yoophi.sttvoicememo`; 공백/대문자 금지 |
| `version` | semantic version | 초기 `0.1.0`; package/Cargo/Tauri 간 일치 |
| `devPort` | integer | `1420`; Vite와 Tauri `devUrl` 간 일치 |

**Relationships**: `tauri.conf.json`, `package.json`, `Cargo.toml`, 초기 화면이 동일한 제품
정체성을 소비한다. 별도 runtime store에 복제하지 않는다.

## PlatformTarget

doctor와 수동 device validation이 사용하는 대상 분류다.

| Field | Type | Validation |
|---|---|---|
| `platform` | `ios \| android` | 두 값만 허용 |
| `minimumOs` | string | iOS `15.0`, Android API `24` |
| `hostProject` | path | iOS `src-tauri/gen/apple`, Android `src-tauri/gen/android` |
| `runtimePermissions` | string[] | 이 feature에서는 항상 빈 배열 |
| `deviceRequired` | boolean | 완료 검증에서는 항상 `true` |

**Relationships**: 각 target은 여러 `PrerequisiteCheck`와 하나의 완료된
`DeviceValidationRecord`를 갖는다. 앱 자체에는 이 값을 저장하지 않는다.

## PrerequisiteCheck

`mobile-doctor`가 한 실행에서 계산하고 stdout/stderr로만 노출하는 진단 결과다.

| Field | Type | Validation |
|---|---|---|
| `id` | stable string | 예: `android.java`, `ios.xcode` |
| `platform` | `common \| ios \| android` | target filter와 일치 |
| `status` | `pass \| fail \| warning \| skipped` | 정의된 네 상태만 허용 |
| `summary` | string | 비밀 값이나 사용자 경로 내용을 포함하지 않음 |
| `nextAction` | string? | fail이면 실행 가능한 설치/설정 조치 필수 |
| `documentationUrl` | URL? | 가능하면 공식 문서로 연결 |

### State transitions

```text
not-run -> pass
not-run -> fail -> pass        # 도구 설치/환경 설정 후 재실행
not-run -> warning -> pass     # 선택적 또는 device 연결 보완
not-run -> skipped             # 다른 플랫폼만 선택해 검사
```

결과는 매번 새로 계산하며 이전 결과를 신뢰하거나 영속화하지 않는다. 한 플랫폼의 `fail`은
다른 플랫폼 check를 `fail`로 바꾸지 않는다.

## DeviceValidationRecord

`tests/device/mobile-shell-smoke.md`에 개발자가 기록하는 검증 증거다. 앱 런타임 모델이 아니다.

| Field | Type | Validation |
|---|---|---|
| `platform` | `ios \| android` | 각 플랫폼 한 건 이상 |
| `deviceModel` | string | 실제 기기 모델; simulator/emulator 불가 |
| `osVersion` | string | 최소 OS 이상 |
| `coldLaunchPasses` | integer | 정확히 5회 연속 성공해야 완료 |
| `portraitPass` | boolean | 핵심 콘텐츠 clipping/overlap 없음 |
| `landscapePass` | boolean | 핵심 콘텐츠 clipping/overlap 없음 |
| `lifecyclePass` | boolean | background 후 foreground 복귀 정상 |
| `unexpectedPermissionPrompts` | integer | 반드시 0 |
| `validatedAt` | ISO-8601 datetime | 검증 시점 |

### Completion transition

```text
pending -> running -> passed
                  -> failed -> running
```

두 플랫폼 record가 모두 `passed`이고 자동 검증이 성공해야 feature가 완료된다.
