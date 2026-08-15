# iOS Foreground Recorder Physical-Device Evidence

**Feature**: GitHub Issue #4 / `specs/003-ios-foreground-recorder/`

## Test environment

- Device model: Unavailable (only an iPhone 17 Pro simulator was connected)
- iOS version: Unavailable
- Build commit: `a5a8a45`
- Date: 2026-08-15
- Tester: Codex automated validation; physical tester pending

Do not include recorded phrases, audio, absolute file paths, native error text,
or credentials in this document or its linked evidence.

## Acceptance matrix

| Scenario                                      | Required repetitions | Status  | Content-safe evidence / notes               |
| --------------------------------------------- | -------------------: | ------- | ------------------------------------------- |
| First permission grant after Record           |      1 fresh install | Not run | Physical iPhone required                    |
| Permission denial and repeated Record         |                    2 | Not run | Physical iPhone required                    |
| Normal start/pause/resume/stop                |                   20 | Not run | Physical iPhone required                    |
| User cancel from recording                    |                    3 | Not run | Physical iPhone required                    |
| User cancel from paused                       |                    3 | Not run | Physical iPhone required                    |
| Incoming call/system interruption             |                    2 | Not run | Physical iPhone required                    |
| Wired input removal                           |     2 when available | Not run | Physical iPhone/accessory required          |
| Bluetooth input removal                       |     2 when available | Not run | Physical iPhone/accessory required          |
| Home/app switch while recording               |                    3 | Not run | Physical iPhone required                    |
| Media-services reset                          |                    1 | Not run | Physical iPhone Developer settings required |
| Five consecutive cold launches and recordings |                    5 | Not run | Physical iPhone required                    |
| Repeated stop/cancel taps                     |               3 each | Not run | Physical iPhone required                    |

## Automated and build evidence

| Check                          | Status  | Evidence / notes                                |
| ------------------------------ | ------- | ----------------------------------------------- |
| Rust workspace tests           | Passed  | 23 tests: recorder core 19, plugin boundary 4   |
| Rust clippy and formatting     | Passed  | Workspace clean; vendored `swift-rs` warns only |
| TypeScript tests/build/lint    | Passed  | 37 tests, TypeScript build, and ESLint passed   |
| Swift coordinator tests        | Passed  | 17 tests on the connected iOS simulator         |
| iOS simulator target compile   | Passed  | Rust/Swift plugin compiled for iOS simulator    |
| Android Rust plugin compile    | Passed  | Safe unsupported adapter compiled for arm64     |
| Physical Android app startup   | Not run | `adb` unavailable; physical API 24+ required    |
| Unsigned iOS arm64 debug build | Passed  | Tauri produced one unsigned iOS bundle          |
| Physical iPhone install/build  | Not run | `devicectl` reported simulated devices only     |

## Completion statement

Issue #4 must not be marked physical-device complete until every required iPhone
row above has actual physical-device evidence and the physical Android startup
regression passes. Simulator, mocked, and compile-only tests are supplementary.

Automated implementation validation is complete. T017, T024, T029, and T033
remain open because their acceptance evidence explicitly requires physical
devices; the absence of a device is not treated as a passing or simulated
result.
