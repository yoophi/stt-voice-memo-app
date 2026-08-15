import Foundation

enum RecorderPermissionState: String, Codable, Equatable {
    case undetermined
    case granted
    case denied
    case restricted
}

struct PermissionOutcome: Codable, Equatable {
    let state: RecorderPermissionState
    let canRequest: Bool
    let canOpenSettings: Bool

    static func from(_ state: RecorderPermissionState) -> PermissionOutcome {
        switch state {
        case .undetermined:
            PermissionOutcome(state: state, canRequest: true, canOpenSettings: false)
        case .granted:
            PermissionOutcome(state: state, canRequest: false, canOpenSettings: false)
        case .denied:
            PermissionOutcome(state: state, canRequest: false, canOpenSettings: true)
        case .restricted:
            PermissionOutcome(state: state, canRequest: false, canOpenSettings: false)
        }
    }
}

enum RecordingState: String, Codable, Equatable {
    case idle
    case recording
    case paused
    case finalizing
    case finalized
    case cancelled
    case failed
}

enum FinalizationReason: String, Codable, Equatable {
    case userStop
    case interruption
    case routeChange
    case foregroundExit
    case mediaServicesReset
}

enum CleanupOutcome: String, Codable, Equatable {
    case removed
    case notFound
    case pending
    case failed
}

struct RecordingSession: Codable, Equatable {
    let sessionId: String?
    let state: RecordingState
    let startedAtMs: UInt64?
    let durationMs: UInt64
    let terminalReason: FinalizationReason?

    static let idle = RecordingSession(
        sessionId: nil,
        state: .idle,
        startedAtMs: nil,
        durationMs: 0,
        terminalReason: nil
    )
}

struct NativeFinalizedRecording: Codable, Equatable {
    let artifactId: String
    let sessionId: String
    let fileUri: String
    let durationMs: UInt64
    let byteLength: UInt64
    let sampleRateHz: UInt32
    let channelCount: UInt16
    let sha256: String
    let finalizationReason: FinalizationReason

    var eventRecording: EventRecording {
        EventRecording(
            artifactId: artifactId,
            sessionId: sessionId,
            mimeType: "audio/mp4",
            fileExtension: "m4a",
            durationMs: durationMs,
            byteLength: byteLength,
            sampleRateHz: sampleRateHz,
            channelCount: channelCount,
            sha256: sha256,
            finalizationReason: finalizationReason
        )
    }
}

struct EventRecording: Codable, Equatable {
    let artifactId: String
    let sessionId: String
    let mimeType: String
    let fileExtension: String
    let durationMs: UInt64
    let byteLength: UInt64
    let sampleRateHz: UInt32
    let channelCount: UInt16
    let sha256: String
    let finalizationReason: FinalizationReason
}

struct RecorderEvent: Codable, Equatable {
    let eventId: String
    let sessionId: String
    let sequence: UInt64
    let state: RecordingState
    let reason: FinalizationReason?
    let recording: EventRecording?
    let cleanup: CleanupOutcome?
}

enum RecorderPluginErrorCode: String, Codable, Equatable {
    case invalidSessionId
    case activeSessionExists
    case invalidTransition
    case staleSession
    case permissionDenied
    case permissionRestricted
    case permissionRequestUnavailable
    case storageUnavailable
    case audioSessionFailure
    case recorderFailure
    case finalizationFailure
    case invalidArtifact
    case cleanupFailure
    case terminalConflict
}

struct RecorderPluginError: Error, Codable, Equatable {
    let code: RecorderPluginErrorCode
    let retryable: Bool
    let cleanup: CleanupOutcome?

    init(
        code: RecorderPluginErrorCode,
        retryable: Bool = false,
        cleanup: CleanupOutcome? = nil
    ) {
        self.code = code
        self.retryable = retryable
        self.cleanup = cleanup
    }

    var publicMessage: String {
        switch code {
        case .invalidSessionId: "invalid recording session identifier"
        case .activeSessionExists: "a recording session is already active"
        case .invalidTransition: "recorder action is invalid in the current state"
        case .staleSession: "recording session is no longer active"
        case .permissionDenied: "microphone permission is denied"
        case .permissionRestricted: "microphone permission is restricted"
        case .permissionRequestUnavailable: "microphone permission request is unavailable"
        case .storageUnavailable: "recording storage is unavailable"
        case .audioSessionFailure: "audio session is unavailable"
        case .recorderFailure: "recorder operation failed"
        case .finalizationFailure: "recorder finalization failed"
        case .invalidArtifact: "finalized recording is invalid"
        case .cleanupFailure: "recording cleanup failed"
        case .terminalConflict: "recording session already ended differently"
        }
    }
}

struct SessionArgs: Decodable {
    let sessionId: String
}

struct StatusArgs: Decodable {
    let sessionId: String?
}

struct StopArgs: Decodable {
    let sessionId: String
    let reason: FinalizationReason
}
