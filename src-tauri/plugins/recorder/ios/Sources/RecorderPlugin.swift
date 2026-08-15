import Foundation
import Tauri

final class RecorderPlugin: Plugin {
    private var coordinator: RecorderCoordinator?

    @MainActor
    private func recorder() -> RecorderCoordinator {
        if let coordinator { return coordinator }
        let coordinator = RecorderCoordinator { [weak self] event in
            try? self?.trigger("recorderEvent", data: event)
        }
        self.coordinator = coordinator
        return coordinator
    }

    @objc public func permission_status(_ invoke: Invoke) {
        Task { @MainActor in invoke.resolve(recorder().permissionStatus()) }
    }

    @objc public func request_permission(_ invoke: Invoke) {
        Task { @MainActor in invoke.resolve(await recorder().requestPermission()) }
    }

    @objc public func recorder_status(_ invoke: Invoke) {
        execute(invoke) {
            let args = try invoke.parseArgs(StatusArgs.self)
            return try self.recorder().status(sessionId: args.sessionId)
        }
    }

    @objc public func start(_ invoke: Invoke) {
        Task { @MainActor in
            do {
                let args = try invoke.parseArgs(SessionArgs.self)
                invoke.resolve(try await recorder().start(sessionId: args.sessionId))
            } catch {
                reject(invoke, error: error)
            }
        }
    }

    @objc public func pause(_ invoke: Invoke) {
        execute(invoke) {
            let args = try invoke.parseArgs(SessionArgs.self)
            return try self.recorder().pause(sessionId: args.sessionId)
        }
    }

    @objc public func resume(_ invoke: Invoke) {
        execute(invoke) {
            let args = try invoke.parseArgs(SessionArgs.self)
            return try self.recorder().resume(sessionId: args.sessionId)
        }
    }

    @objc public func stop(_ invoke: Invoke) {
        execute(invoke) {
            let args = try invoke.parseArgs(StopArgs.self)
            return try self.recorder().stop(sessionId: args.sessionId, reason: args.reason)
        }
    }

    @objc public func cancel(_ invoke: Invoke) {
        execute(invoke) {
            let args = try invoke.parseArgs(SessionArgs.self)
            return try self.recorder().cancel(sessionId: args.sessionId)
        }
    }

    private func execute<T: Encodable>(
        _ invoke: Invoke,
        operation: @escaping @MainActor () throws -> T
    ) {
        Task { @MainActor in
            do {
                invoke.resolve(try operation())
            } catch {
                reject(invoke, error: error)
            }
        }
    }

    private func reject(_ invoke: Invoke, error: Error) {
        let recorderError = error as? RecorderPluginError
            ?? RecorderPluginError(code: .recorderFailure, retryable: true)
        let code = if let cleanup = recorderError.cleanup {
            "\(recorderError.code.rawValue):\(cleanup.rawValue)"
        } else {
            recorderError.code.rawValue
        }
        invoke.reject(recorderError.publicMessage, code: code)
    }
}

@_cdecl("init_plugin_recorder")
func initPlugin() -> Plugin {
    RecorderPlugin()
}
