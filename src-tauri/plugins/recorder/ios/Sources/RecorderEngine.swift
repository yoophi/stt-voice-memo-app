import AVFAudio
import CryptoKit
import Foundation
import UIKit

@MainActor
protocol AudioSessionControlling: AnyObject {
    func permissionStatus() -> RecorderPermissionState
    func requestPermission() async -> RecorderPermissionState
    func activate() throws
    func deactivate()
}

@MainActor
protocol AudioCapturing: AnyObject {
    var currentTime: TimeInterval { get }
    var isRecording: Bool { get }
    func prepareToRecord() -> Bool
    func record() -> Bool
    func pause()
    func stop()
}

@MainActor
protocol RecorderCreating: AnyObject {
    func makeRecorder(destination: URL) throws -> AudioCapturing
}

@MainActor
protocol RecordingFileManaging {
    func prepareDestination(sessionId: String) throws -> URL
    func finalize(url: URL, durationMs: UInt64, reason: FinalizationReason) throws
        -> NativeFinalizedRecording
    func remove(url: URL) -> CleanupOutcome
}

extension AVAudioRecorder: AudioCapturing {}

@MainActor
final class SystemAudioSession: AudioSessionControlling {
    private let session = AVAudioSession.sharedInstance()

    func permissionStatus() -> RecorderPermissionState {
        if #available(iOS 17.0, *) {
            switch AVAudioApplication.shared.recordPermission {
            case .undetermined: .undetermined
            case .granted: .granted
            case .denied: .denied
            @unknown default: .restricted
            }
        } else {
            switch session.recordPermission {
            case .undetermined: .undetermined
            case .granted: .granted
            case .denied: .denied
            @unknown default: .restricted
            }
        }
    }

    func requestPermission() async -> RecorderPermissionState {
        let granted = await withCheckedContinuation { continuation in
            if #available(iOS 17.0, *) {
                AVAudioApplication.requestRecordPermission { allowed in
                    continuation.resume(returning: allowed)
                }
            } else {
                session.requestRecordPermission { allowed in
                    continuation.resume(returning: allowed)
                }
            }
        }
        return granted ? .granted : .denied
    }

    func activate() throws {
        do {
            try session.setCategory(.record, mode: .default, options: [])
            try session.setActive(true)
        } catch {
            throw RecorderPluginError(code: .audioSessionFailure, retryable: true)
        }
    }

    func deactivate() {
        try? session.setActive(false, options: .notifyOthersOnDeactivation)
    }
}

@MainActor
final class SystemRecorderFactory: RecorderCreating {
    func makeRecorder(destination: URL) throws -> AudioCapturing {
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatMPEG4AAC,
            AVSampleRateKey: 44_100,
            AVNumberOfChannelsKey: 1,
            AVEncoderBitRateKey: 96_000,
            AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue,
        ]
        do {
            return try AVAudioRecorder(url: destination, settings: settings)
        } catch {
            throw RecorderPluginError(code: .recorderFailure, retryable: true)
        }
    }
}

@MainActor
struct SystemRecordingFiles: RecordingFileManaging {
    private let fileManager = FileManager.default

    func prepareDestination(sessionId: String) throws -> URL {
        guard Self.isCanonicalUUID(sessionId) else {
            throw RecorderPluginError(code: .invalidSessionId)
        }
        do {
            let base = try fileManager.url(
                for: .applicationSupportDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
            let directory = base.appendingPathComponent("Recordings", isDirectory: true)
            try fileManager.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.protectionKey: FileProtectionType.complete]
            )
            let destination = directory.appendingPathComponent("\(sessionId).m4a")
            if fileManager.fileExists(atPath: destination.path) {
                try fileManager.removeItem(at: destination)
            }
            return destination
        } catch let error as RecorderPluginError {
            throw error
        } catch {
            throw RecorderPluginError(code: .storageUnavailable, retryable: true)
        }
    }

    func finalize(url: URL, durationMs: UInt64, reason: FinalizationReason) throws
        -> NativeFinalizedRecording
    {
        guard durationMs > 0, url.pathExtension == "m4a" else {
            throw RecorderPluginError(code: .invalidArtifact)
        }
        do {
            let attributes = try fileManager.attributesOfItem(atPath: url.path)
            guard let bytes = attributes[.size] as? NSNumber, bytes.uint64Value > 0 else {
                throw RecorderPluginError(code: .invalidArtifact)
            }
            try fileManager.setAttributes(
                [.protectionKey: FileProtectionType.complete],
                ofItemAtPath: url.path
            )
            let audioFile = try AVAudioFile(forReading: url)
            let format = audioFile.fileFormat
            guard format.sampleRate > 0, format.channelCount > 0 else {
                throw RecorderPluginError(code: .invalidArtifact)
            }
            return NativeFinalizedRecording(
                artifactId: UUID().uuidString.lowercased(),
                fileUri: url.absoluteString,
                durationMs: durationMs,
                sampleRateHz: UInt32(format.sampleRate.rounded()),
                channelCount: UInt16(format.channelCount),
                sha256: try Self.sha256(url: url),
                finalizationReason: reason
            )
        } catch let error as RecorderPluginError {
            throw error
        } catch {
            throw RecorderPluginError(code: .finalizationFailure)
        }
    }

    func remove(url: URL) -> CleanupOutcome {
        guard fileManager.fileExists(atPath: url.path) else { return .notFound }
        do {
            try fileManager.removeItem(at: url)
            return .removed
        } catch {
            return .pending
        }
    }

    private static func sha256(url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var digest = SHA256()
        while true {
            let data = try handle.read(upToCount: 64 * 1024) ?? Data()
            if data.isEmpty { break }
            digest.update(data: data)
        }
        return digest.finalize().map { String(format: "%02x", $0) }.joined()
    }

    fileprivate static func isCanonicalUUID(_ value: String) -> Bool {
        UUID(uuidString: value)?.uuidString.lowercased() == value.lowercased()
            && value == value.lowercased()
    }
}

private struct ActiveRecording {
    let sessionId: String
    let startedAtMs: UInt64
    let destination: URL
    let capture: AudioCapturing
    var state: RecordingState
}

private enum TerminalResult {
    case finalized(NativeFinalizedRecording)
    case cancelled(CleanupOutcome)
    case failed(RecorderPluginError)
}

@MainActor
final class RecorderCoordinator {
    private let audioSession: AudioSessionControlling
    private let recorderFactory: RecorderCreating
    private let files: RecordingFileManaging
    private let notifications: NotificationCenter
    private let eventSink: (RecorderEvent) -> Void
    private var active: ActiveRecording?
    private var terminals: [String: TerminalResult] = [:]
    private var sequences: [String: UInt64] = [:]
    private var observerTokens: [NSObjectProtocol] = []

    convenience init(eventSink: @escaping (RecorderEvent) -> Void = { _ in }) {
        self.init(
            audioSession: SystemAudioSession(),
            recorderFactory: SystemRecorderFactory(),
            files: SystemRecordingFiles(),
            notifications: .default,
            eventSink: eventSink
        )
    }

    init(
        audioSession: AudioSessionControlling,
        recorderFactory: RecorderCreating,
        files: RecordingFileManaging,
        notifications: NotificationCenter,
        eventSink: @escaping (RecorderEvent) -> Void = { _ in }
    ) {
        self.audioSession = audioSession
        self.recorderFactory = recorderFactory
        self.files = files
        self.notifications = notifications
        self.eventSink = eventSink
        observeLifecycle()
    }

    func permissionStatus() -> PermissionOutcome {
        PermissionOutcome.from(audioSession.permissionStatus())
    }

    func requestPermission() async -> PermissionOutcome {
        PermissionOutcome.from(await audioSession.requestPermission())
    }

    func status(sessionId: String?) throws -> RecordingSession {
        if let sessionId, !Self.validSessionId(sessionId) {
            throw RecorderPluginError(code: .invalidSessionId)
        }
        if let active {
            guard sessionId == nil || active.sessionId == sessionId else {
                throw RecorderPluginError(code: .staleSession)
            }
            return snapshot(active)
        }
        if let sessionId, let terminal = terminals[sessionId] {
            return terminalSnapshot(sessionId: sessionId, terminal: terminal)
        }
        return .idle
    }

    func start(sessionId: String) async throws -> RecordingSession {
        guard Self.validSessionId(sessionId) else {
            throw RecorderPluginError(code: .invalidSessionId)
        }
        guard active == nil else {
            throw RecorderPluginError(code: .activeSessionExists)
        }
        var permission = audioSession.permissionStatus()
        if permission == .undetermined {
            permission = await audioSession.requestPermission()
        }
        switch permission {
        case .granted: break
        case .denied: throw RecorderPluginError(code: .permissionDenied)
        case .restricted: throw RecorderPluginError(code: .permissionRestricted)
        case .undetermined: throw RecorderPluginError(code: .permissionRequestUnavailable)
        }

        let destination = try files.prepareDestination(sessionId: sessionId)
        do {
            try audioSession.activate()
            let capture = try recorderFactory.makeRecorder(destination: destination)
            guard capture.prepareToRecord(), capture.record() else {
                audioSession.deactivate()
                _ = files.remove(url: destination)
                throw RecorderPluginError(code: .recorderFailure, retryable: true)
            }
            let recording = ActiveRecording(
                sessionId: sessionId,
                startedAtMs: Self.nowMs(),
                destination: destination,
                capture: capture,
                state: .recording
            )
            active = recording
            let result = snapshot(recording)
            emit(sessionId: sessionId, state: .recording)
            return result
        } catch let error as RecorderPluginError {
            throw error
        } catch {
            audioSession.deactivate()
            _ = files.remove(url: destination)
            throw RecorderPluginError(code: .recorderFailure, retryable: true)
        }
    }

    func pause(sessionId: String) throws -> RecordingSession {
        var recording = try requireActive(sessionId: sessionId)
        if recording.state == .paused { return snapshot(recording) }
        guard recording.state == .recording else {
            throw RecorderPluginError(code: .invalidTransition)
        }
        recording.capture.pause()
        recording.state = .paused
        active = recording
        emit(sessionId: sessionId, state: .paused)
        return snapshot(recording)
    }

    func resume(sessionId: String) throws -> RecordingSession {
        var recording = try requireActive(sessionId: sessionId)
        if recording.state == .recording { return snapshot(recording) }
        guard recording.state == .paused, recording.capture.record() else {
            throw RecorderPluginError(code: .recorderFailure, retryable: true)
        }
        recording.state = .recording
        active = recording
        emit(sessionId: sessionId, state: .recording)
        return snapshot(recording)
    }

    func stop(sessionId: String, reason: FinalizationReason) throws -> NativeFinalizedRecording {
        if let terminal = terminals[sessionId] {
            switch terminal {
            case .finalized(let recording): return recording
            case .failed(let error): throw error
            case .cancelled: throw RecorderPluginError(code: .terminalConflict)
            }
        }
        var recording = try requireActive(sessionId: sessionId)
        guard recording.state == .recording || recording.state == .paused else {
            throw RecorderPluginError(code: .invalidTransition)
        }
        recording.state = .finalizing
        active = recording
        let durationMs = UInt64(max(0, recording.capture.currentTime) * 1_000)
        recording.capture.stop()
        audioSession.deactivate()
        do {
            let finalized = try files.finalize(
                url: recording.destination,
                durationMs: durationMs,
                reason: reason
            )
            active = nil
            terminals[sessionId] = .finalized(finalized)
            emit(
                sessionId: sessionId,
                state: .finalized,
                reason: reason,
                recording: finalized.eventRecording
            )
            return finalized
        } catch let error as RecorderPluginError {
            let cleanup = files.remove(url: recording.destination)
            let finalError = RecorderPluginError(
                code: error.code,
                retryable: error.retryable,
                cleanup: cleanup
            )
            active = nil
            terminals[sessionId] = .failed(finalError)
            emit(sessionId: sessionId, state: .failed, reason: reason, cleanup: cleanup)
            throw finalError
        } catch {
            let cleanup = files.remove(url: recording.destination)
            let finalError = RecorderPluginError(
                code: .finalizationFailure,
                cleanup: cleanup
            )
            active = nil
            terminals[sessionId] = .failed(finalError)
            emit(sessionId: sessionId, state: .failed, reason: reason, cleanup: cleanup)
            throw finalError
        }
    }

    func cancel(sessionId: String) throws -> CleanupOutcome {
        if let terminal = terminals[sessionId] {
            switch terminal {
            case .cancelled(let cleanup): return cleanup
            case .failed(let error): throw error
            case .finalized: throw RecorderPluginError(code: .terminalConflict)
            }
        }
        let recording = try requireActive(sessionId: sessionId)
        recording.capture.stop()
        audioSession.deactivate()
        let cleanup = files.remove(url: recording.destination)
        active = nil
        if cleanup == .pending || cleanup == .failed {
            let error = RecorderPluginError(
                code: .cleanupFailure,
                retryable: true,
                cleanup: cleanup
            )
            terminals[sessionId] = .failed(error)
            emit(sessionId: sessionId, state: .failed, cleanup: cleanup)
            throw error
        }
        terminals[sessionId] = .cancelled(cleanup)
        emit(sessionId: sessionId, state: .cancelled, cleanup: cleanup)
        return cleanup
    }

    func handleSystemStop(reason: FinalizationReason) {
        guard let sessionId = active?.sessionId else { return }
        _ = try? stop(sessionId: sessionId, reason: reason)
    }

    private func observeLifecycle() {
        observerTokens.append(
            notifications.addObserver(
                forName: AVAudioSession.interruptionNotification,
                object: nil,
                queue: .main
            ) { [weak self] notification in
                guard
                    let raw = notification.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt,
                    AVAudioSession.InterruptionType(rawValue: raw) == .began
                else { return }
                MainActor.assumeIsolated {
                    self?.handleSystemStop(reason: .interruption)
                }
            }
        )
        observerTokens.append(
            notifications.addObserver(
                forName: AVAudioSession.routeChangeNotification,
                object: nil,
                queue: .main
            ) { [weak self] notification in
                guard
                    let raw = notification.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
                    let reason = AVAudioSession.RouteChangeReason(rawValue: raw),
                    reason == .oldDeviceUnavailable || reason == .routeConfigurationChange
                else { return }
                MainActor.assumeIsolated {
                    self?.handleSystemStop(reason: .routeChange)
                }
            }
        )
        observerTokens.append(
            notifications.addObserver(
                forName: AVAudioSession.mediaServicesWereResetNotification,
                object: nil,
                queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated {
                    self?.handleSystemStop(reason: .mediaServicesReset)
                }
            }
        )
        observerTokens.append(
            notifications.addObserver(
                forName: UIApplication.didEnterBackgroundNotification,
                object: nil,
                queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated {
                    self?.handleSystemStop(reason: .foregroundExit)
                }
            }
        )
    }

    private func requireActive(sessionId: String) throws -> ActiveRecording {
        guard Self.validSessionId(sessionId) else {
            throw RecorderPluginError(code: .invalidSessionId)
        }
        guard let active, active.sessionId == sessionId else {
            throw RecorderPluginError(code: .staleSession)
        }
        return active
    }

    private func snapshot(_ recording: ActiveRecording) -> RecordingSession {
        RecordingSession(
            sessionId: recording.sessionId,
            state: recording.state,
            startedAtMs: recording.startedAtMs,
            durationMs: UInt64(max(0, recording.capture.currentTime) * 1_000),
            terminalReason: nil
        )
    }

    private func terminalSnapshot(sessionId: String, terminal: TerminalResult) -> RecordingSession {
        switch terminal {
        case .finalized(let recording):
            RecordingSession(
                sessionId: sessionId,
                state: .finalized,
                startedAtMs: nil,
                durationMs: recording.durationMs,
                terminalReason: recording.finalizationReason
            )
        case .cancelled:
            RecordingSession(
                sessionId: sessionId,
                state: .cancelled,
                startedAtMs: nil,
                durationMs: 0,
                terminalReason: nil
            )
        case .failed:
            RecordingSession(
                sessionId: sessionId,
                state: .failed,
                startedAtMs: nil,
                durationMs: 0,
                terminalReason: nil
            )
        }
    }

    private func emit(
        sessionId: String,
        state: RecordingState,
        reason: FinalizationReason? = nil,
        recording: EventRecording? = nil,
        cleanup: CleanupOutcome? = nil
    ) {
        let sequence = (sequences[sessionId] ?? 0) + 1
        sequences[sessionId] = sequence
        eventSink(
            RecorderEvent(
                eventId: UUID().uuidString.lowercased(),
                sessionId: sessionId,
                sequence: sequence,
                state: state,
                reason: reason,
                recording: recording,
                cleanup: cleanup
            )
        )
    }

    private static func validSessionId(_ value: String) -> Bool {
        SystemRecordingFiles.isCanonicalUUID(value)
    }

    private static func nowMs() -> UInt64 {
        UInt64(Date().timeIntervalSince1970 * 1_000)
    }
}
