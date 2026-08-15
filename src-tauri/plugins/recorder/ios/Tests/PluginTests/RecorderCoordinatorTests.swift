import AVFAudio
import Foundation
import XCTest
@testable import tauri_plugin_recorder

@MainActor
final class RecorderCoordinatorTests: XCTestCase {
    func testStartPauseResumeAndStopUseOneRecorderAndDeactivateSession() async throws {
        let audioSession = FakeAudioSession(permission: .granted)
        let capture = FakeCapture()
        let factory = FakeRecorderFactory(capture: capture)
        let files = FakeRecordingFiles()
        let coordinator = RecorderCoordinator(
            audioSession: audioSession,
            recorderFactory: factory,
            files: files,
            notifications: NotificationCenter()
        )

        let started = try await coordinator.start(sessionId: Self.sessionId)
        let paused = try coordinator.pause(sessionId: Self.sessionId)
        let resumed = try coordinator.resume(sessionId: Self.sessionId)
        let finalized = try coordinator.stop(sessionId: Self.sessionId, reason: .userStop)

        XCTAssertEqual(started.state, .recording)
        XCTAssertEqual(paused.state, .paused)
        XCTAssertEqual(resumed.state, .recording)
        XCTAssertEqual(finalized.durationMs, 750)
        XCTAssertEqual(finalized.channelCount, 1)
        XCTAssertEqual(factory.createCount, 1)
        XCTAssertEqual(capture.pauseCount, 1)
        XCTAssertEqual(capture.recordCount, 2)
        XCTAssertEqual(capture.stopCount, 1)
        XCTAssertEqual(audioSession.activateCount, 1)
        XCTAssertEqual(audioSession.deactivateCount, 1)
    }

    func testSecondStartIsRejectedWhileRecording() async throws {
        let coordinator = makeCoordinator()
        _ = try await coordinator.start(sessionId: Self.sessionId)

        do {
            _ = try await coordinator.start(sessionId: "a3bb189e-8bf9-3888-9912-ace4e6543002")
            XCTFail("second active session should fail")
        } catch let error as RecorderPluginError {
            XCTAssertEqual(error.code, .activeSessionExists)
        }
    }

    func testRepeatedPauseResumeAndStopAreIdempotent() async throws {
        let capture = FakeCapture()
        let coordinator = makeCoordinator(capture: capture)
        _ = try await coordinator.start(sessionId: Self.sessionId)

        _ = try coordinator.pause(sessionId: Self.sessionId)
        _ = try coordinator.pause(sessionId: Self.sessionId)
        _ = try coordinator.resume(sessionId: Self.sessionId)
        _ = try coordinator.resume(sessionId: Self.sessionId)
        let first = try coordinator.stop(sessionId: Self.sessionId, reason: .userStop)
        let second = try coordinator.stop(sessionId: Self.sessionId, reason: .userStop)

        XCTAssertEqual(first, second)
        XCTAssertEqual(capture.pauseCount, 1)
        XCTAssertEqual(capture.recordCount, 2)
        XCTAssertEqual(capture.stopCount, 1)
    }

    func testCancelActiveAndPausedSessionsRemovesEachArtifactOnce() async throws {
        for shouldPause in [false, true] {
            let capture = FakeCapture()
            let files = FakeRecordingFiles()
            let audioSession = FakeAudioSession(permission: .granted)
            let coordinator = makeCoordinator(
                capture: capture,
                files: files,
                audioSession: audioSession
            )
            _ = try await coordinator.start(sessionId: Self.sessionId)
            if shouldPause {
                _ = try coordinator.pause(sessionId: Self.sessionId)
            }

            let first = try coordinator.cancel(sessionId: Self.sessionId)
            let second = try coordinator.cancel(sessionId: Self.sessionId)

            XCTAssertEqual(first, .removed)
            XCTAssertEqual(second, .removed)
            XCTAssertEqual(files.removeCount, 1)
            XCTAssertEqual(capture.stopCount, 1)
            XCTAssertEqual(audioSession.deactivateCount, 1)
        }
    }

    func testCancelReportsAndStoresPendingCleanupWithoutDeletingTwice() async throws {
        let files = FakeRecordingFiles(cleanup: .pending)
        let coordinator = makeCoordinator(files: files)
        _ = try await coordinator.start(sessionId: Self.sessionId)

        for _ in 0..<2 {
            do {
                _ = try coordinator.cancel(sessionId: Self.sessionId)
                XCTFail("pending cleanup should be retryable failure")
            } catch let error as RecorderPluginError {
                XCTAssertEqual(error.code, .cleanupFailure)
                XCTAssertTrue(error.retryable)
                XCTAssertEqual(error.cleanup, .pending)
            }
        }

        XCTAssertEqual(files.removeCount, 1)
    }

    func testStopAfterCancelReturnsTerminalConflict() async throws {
        let coordinator = makeCoordinator()
        _ = try await coordinator.start(sessionId: Self.sessionId)
        _ = try coordinator.cancel(sessionId: Self.sessionId)

        XCTAssertThrowsError(
            try coordinator.stop(sessionId: Self.sessionId, reason: .userStop)
        ) { error in
            XCTAssertEqual((error as? RecorderPluginError)?.code, .terminalConflict)
        }
    }

    func testDeniedPermissionNeverCreatesRecorderOrActivatesAudioSession() async {
        let audioSession = FakeAudioSession(permission: .denied)
        let factory = FakeRecorderFactory(capture: FakeCapture())
        let coordinator = RecorderCoordinator(
            audioSession: audioSession,
            recorderFactory: factory,
            files: FakeRecordingFiles(),
            notifications: NotificationCenter()
        )

        do {
            _ = try await coordinator.start(sessionId: Self.sessionId)
            XCTFail("denied permission should fail")
        } catch let error as RecorderPluginError {
            XCTAssertEqual(error.code, .permissionDenied)
            XCTAssertEqual(factory.createCount, 0)
            XCTAssertEqual(audioSession.activateCount, 0)
        } catch {
            XCTFail("unexpected error type")
        }
    }

    func testInterruptionWinsRaceWithUserStopAndEmitsOneTerminalEvent() async throws {
        var events: [RecorderEvent] = []
        let audioSession = FakeAudioSession(permission: .granted)
        let coordinator = RecorderCoordinator(
            audioSession: audioSession,
            recorderFactory: FakeRecorderFactory(capture: FakeCapture()),
            files: FakeRecordingFiles(),
            notifications: NotificationCenter(),
            eventSink: { events.append($0) }
        )
        _ = try await coordinator.start(sessionId: Self.sessionId)

        coordinator.handleSystemStop(reason: .interruption)
        let repeated = try coordinator.stop(sessionId: Self.sessionId, reason: .userStop)

        XCTAssertEqual(repeated.finalizationReason, .interruption)
        XCTAssertEqual(events.filter { $0.state == .finalized }.count, 1)
        XCTAssertEqual(audioSession.deactivateCount, 1)
    }

    func testRouteLossNotificationFinalizesWithoutAutoResume() async throws {
        let notifications = NotificationCenter()
        var events: [RecorderEvent] = []
        let capture = FakeCapture()
        let coordinator = RecorderCoordinator(
            audioSession: FakeAudioSession(permission: .granted),
            recorderFactory: FakeRecorderFactory(capture: capture),
            files: FakeRecordingFiles(),
            notifications: notifications,
            eventSink: { events.append($0) }
        )
        _ = try await coordinator.start(sessionId: Self.sessionId)

        notifications.post(
            name: AVAudioSession.routeChangeNotification,
            object: nil,
            userInfo: [
                AVAudioSessionRouteChangeReasonKey:
                    AVAudioSession.RouteChangeReason.oldDeviceUnavailable.rawValue,
            ]
        )

        XCTAssertEqual(events.last?.reason, .routeChange)
        XCTAssertEqual(capture.recordCount, 1)
        XCTAssertFalse(capture.isRecording)
    }

    func testBackgroundAndMediaResetMapToStableTerminalReasons() async throws {
        for (notificationName, expectedReason) in [
            (UIApplication.didEnterBackgroundNotification, FinalizationReason.foregroundExit),
            (AVAudioSession.mediaServicesWereResetNotification, .mediaServicesReset),
        ] {
            let notifications = NotificationCenter()
            var events: [RecorderEvent] = []
            let coordinator = RecorderCoordinator(
                audioSession: FakeAudioSession(permission: .granted),
                recorderFactory: FakeRecorderFactory(capture: FakeCapture()),
                files: FakeRecordingFiles(),
                notifications: notifications,
                eventSink: { events.append($0) }
            )
            _ = try await coordinator.start(sessionId: Self.sessionId)
            notifications.post(name: notificationName, object: nil)
            XCTAssertEqual(events.last?.reason, expectedReason)
            XCTAssertEqual(events.last?.state, .finalized)
        }
    }

    private func makeCoordinator(
        capture: FakeCapture? = nil,
        files: FakeRecordingFiles? = nil,
        audioSession: FakeAudioSession? = nil
    ) -> RecorderCoordinator {
        RecorderCoordinator(
            audioSession: audioSession ?? FakeAudioSession(permission: .granted),
            recorderFactory: FakeRecorderFactory(capture: capture ?? FakeCapture()),
            files: files ?? FakeRecordingFiles(),
            notifications: NotificationCenter()
        )
    }

    private static let sessionId = "550e8400-e29b-41d4-a716-446655440000"
}

@MainActor
private final class FakeAudioSession: AudioSessionControlling {
    var permission: RecorderPermissionState
    var activateCount = 0
    var deactivateCount = 0

    init(permission: RecorderPermissionState) {
        self.permission = permission
    }

    func permissionStatus() -> RecorderPermissionState { permission }
    func requestPermission() async -> RecorderPermissionState { permission }
    func activate() throws { activateCount += 1 }
    func deactivate() { deactivateCount += 1 }
}

@MainActor
private final class FakeCapture: AudioCapturing {
    var currentTime: TimeInterval = 0.75
    var isRecording = false
    var recordCount = 0
    var pauseCount = 0
    var stopCount = 0

    func prepareToRecord() -> Bool { true }
    func record() -> Bool {
        recordCount += 1
        isRecording = true
        return true
    }
    func pause() {
        pauseCount += 1
        isRecording = false
    }
    func stop() {
        stopCount += 1
        isRecording = false
    }
}

@MainActor
private final class FakeRecorderFactory: RecorderCreating {
    let capture: FakeCapture
    var createCount = 0

    init(capture: FakeCapture) {
        self.capture = capture
    }

    func makeRecorder(destination: URL) throws -> AudioCapturing {
        createCount += 1
        return capture
    }
}

@MainActor
private final class FakeRecordingFiles: RecordingFileManaging {
    let cleanup: CleanupOutcome
    var removeCount = 0

    init(cleanup: CleanupOutcome = .removed) {
        self.cleanup = cleanup
    }

    func prepareDestination(sessionId: String) throws -> URL {
        URL(fileURLWithPath: "/private/app/Library/Application Support/Recordings/\(sessionId).m4a")
    }

    func finalize(url: URL, durationMs: UInt64, reason: FinalizationReason) throws
        -> NativeFinalizedRecording
    {
        NativeFinalizedRecording(
            artifactId: "c56a4180-65aa-42ec-a945-5fd21dec0538",
            fileUri: url.absoluteString,
            durationMs: durationMs,
            sampleRateHz: 44_100,
            channelCount: 1,
            sha256: String(repeating: "a", count: 64),
            finalizationReason: reason
        )
    }

    func remove(url: URL) -> CleanupOutcome {
        removeCount += 1
        return cleanup
    }
}
