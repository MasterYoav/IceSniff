import Foundation
import Security

enum AIChatProvider: String, CaseIterable, Identifiable {
    case offline
    case openAI
    case codex
    case anthropic
    case claudeCode
    case google

    var id: String { rawValue }

    var title: String {
        switch self {
        case .offline:
            return "Offline"
        case .openAI:
            return "OpenAI API"
        case .codex:
            return "Codex"
        case .anthropic:
            return "Anthropic API"
        case .claudeCode:
            return "Claude Code"
        case .google:
            return "Google"
        }
    }

    var symbolName: String {
        switch self {
        case .offline:
            return "bolt.shield"
        case .openAI:
            return "bubble.left.and.bubble.right"
        case .codex:
            return "sparkles"
        case .anthropic:
            return "brain.head.profile"
        case .claudeCode:
            return "curlybraces.square"
        case .google:
            return "globe"
        }
    }

    var executableName: String? {
        switch self {
        case .offline:
            return nil
        case .codex:
            return "codex"
        case .claudeCode:
            return "claude"
        case .openAI, .anthropic, .google:
            return nil
        }
    }

    var providerBrandTitle: String {
        switch self {
        case .offline:
            return "Offline"
        case .openAI, .codex:
            return "OpenAI"
        case .anthropic, .claudeCode:
            return "Anthropic"
        case .google:
            return "Google"
        }
    }
}

enum AIChatModelAccess {
    case offline
    case bringYourOwnKey
    case localSubscription
}

struct AIChatModelPreset: Identifiable, Equatable {
    let id: String
    let provider: AIChatProvider
    let title: String
    let subtitle: String
    let remoteID: String
    let access: AIChatModelAccess

    var pickerLabel: String {
        "\(provider.providerBrandTitle) · \(title)"
    }

    static let catalog: [AIChatModelPreset] = [
        AIChatModelPreset(
            id: "offline-assistant",
            provider: .offline,
            title: "Offline Assistant",
            subtitle: "Built-in local fallback for packet explanation and guidance",
            remoteID: "offline",
            access: .offline
        ),
        AIChatModelPreset(
            id: "openai-gpt-4.1",
            provider: .openAI,
            title: "GPT-4.1",
            subtitle: "OpenAI frontier text model",
            remoteID: "gpt-4.1",
            access: .bringYourOwnKey
        ),
        AIChatModelPreset(
            id: "codex-chatgpt",
            provider: .codex,
            title: "Codex",
            subtitle: "Uses the local Codex CLI session tied to a ChatGPT plan",
            remoteID: "codex",
            access: .localSubscription
        ),
        AIChatModelPreset(
            id: "anthropic-claude-sonnet-4",
            provider: .anthropic,
            title: "Claude Sonnet 4",
            subtitle: "Anthropic high-performance reasoning model",
            remoteID: "claude-sonnet-4-20250514",
            access: .bringYourOwnKey
        ),
        AIChatModelPreset(
            id: "claude-code-subscription",
            provider: .claudeCode,
            title: "Claude Code",
            subtitle: "Uses the local Claude Code CLI session tied to a Claude plan",
            remoteID: "claude",
            access: .localSubscription
        ),
        AIChatModelPreset(
            id: "google-gemini-2.5-pro",
            provider: .google,
            title: "Gemini 2.5 Pro",
            subtitle: "Google flagship Gemini API model",
            remoteID: "gemini-2.5-pro",
            access: .bringYourOwnKey
        )
    ]

    static let offlineDefault = catalog.first(where: { $0.id == "offline-assistant" }) ?? catalog[0]
    static let `default` = catalog.first(where: { $0.id == "openai-gpt-4.1" }) ?? offlineDefault
}

enum AIChatMessageRole: String {
    case user
    case assistant
}

struct AIChatMessage: Identifiable, Equatable {
    let id: UUID
    let role: AIChatMessageRole
    let content: String
    let createdAt: Date

    init(id: UUID = UUID(), role: AIChatMessageRole, content: String, createdAt: Date = .now) {
        self.id = id
        self.role = role
        self.content = content
        self.createdAt = createdAt
    }
}

enum AIChatServiceError: LocalizedError {
    case missingAPIKey(AIChatProvider)
    case localRuntimeUnavailable(AIChatProvider)
    case requestFailed(String)
    case emptyResponse(AIChatProvider)

    var errorDescription: String? {
        switch self {
        case let .missingAPIKey(provider):
            return "\(provider.title) API key missing. Add it in chat settings first."
        case let .localRuntimeUnavailable(provider):
            return "\(provider.title) is not available locally. Install the CLI and sign in once in Terminal first."
        case let .requestFailed(message):
            return message
        case let .emptyResponse(provider):
            return "\(provider.title) returned an empty response."
        }
    }
}

final class AIKeychainStore: @unchecked Sendable {
    private enum Constants {
        static let service = "io.icesniff.macos.ai-chat"
        static let useDataProtection = true
    }

    func saveAPIKey(_ value: String, for provider: AIChatProvider) throws {
        let data = Data(value.utf8)
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: provider.rawValue,
            kSecUseDataProtectionKeychain: Constants.useDataProtection
        ]
        let attributes: [CFString: Any] = [
            kSecValueData: data,
            kSecAttrAccessible: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecItemNotFound {
            var addQuery = query
            addQuery[kSecValueData] = data
            addQuery[kSecAttrAccessible] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
            guard addStatus == errSecSuccess else {
                throw AIChatServiceError.requestFailed("Unable to save \(provider.title) API key (\(addStatus)).")
            }
        } else if updateStatus != errSecSuccess {
            throw AIChatServiceError.requestFailed("Unable to update \(provider.title) API key (\(updateStatus)).")
        }
    }

    func loadAPIKey(for provider: AIChatProvider) -> String? {
        var item: CFTypeRef?
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: provider.rawValue,
            kSecUseDataProtectionKeychain: Constants.useDataProtection,
            kSecReturnData: true,
            kSecMatchLimit: kSecMatchLimitOne
        ]

        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            return nil
        }

        return String(data: data, encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
    }

    func removeAPIKey(for provider: AIChatProvider) throws {
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: provider.rawValue,
            kSecUseDataProtectionKeychain: Constants.useDataProtection
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw AIChatServiceError.requestFailed("Unable to remove \(provider.title) API key (\(status)).")
        }
    }

    func hasAPIKey(for provider: AIChatProvider) -> Bool {
        loadAPIKey(for: provider) != nil
    }
}

private struct OpenAIResponsesEnvelope: Decodable {
    struct OutputItem: Decodable {
        struct ContentItem: Decodable {
            let text: String?
            let type: String?
        }

        let content: [ContentItem]?
    }

    let outputText: String?
    let output: [OutputItem]?

    private enum CodingKeys: String, CodingKey {
        case outputText = "output_text"
        case output
    }
}

private struct AnthropicMessagesEnvelope: Decodable {
    struct ContentItem: Decodable {
        let type: String
        let text: String?
    }

    let content: [ContentItem]
}

private struct GeminiGenerateContentEnvelope: Decodable {
    struct Candidate: Decodable {
        struct ResponseContent: Decodable {
            struct Part: Decodable {
                let text: String?
            }

            let parts: [Part]?
        }

        let content: ResponseContent?
    }

    let candidates: [Candidate]?
}

private final class LocalRuntimeOutputAccumulator: @unchecked Sendable {
    private let queue = DispatchQueue(label: "io.icesniff.ai-chat.local-runtime-output")
    private var stdoutData = Data()
    private var stderrData = Data()

    func appendStdout(_ chunk: Data) {
        queue.sync {
            stdoutData.append(chunk)
        }
    }

    func appendStderr(_ chunk: Data) {
        queue.sync {
            stderrData.append(chunk)
        }
    }

    func snapshot() -> (stdout: Data, stderr: Data) {
        queue.sync {
            (stdoutData, stderrData)
        }
    }
}

struct AIChatService: Sendable {
    private let keychain: AIKeychainStore
    private let session: URLSession

    init(keychain: AIKeychainStore = AIKeychainStore(), session: URLSession = AIChatService.makeSecureSession()) {
        self.keychain = keychain
        self.session = session
    }

    private static func makeSecureSession() -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.requestCachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        configuration.urlCache = nil
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.httpCookieAcceptPolicy = .never
        configuration.waitsForConnectivity = false
        return URLSession(configuration: configuration)
    }

    func sendConversation(
        messages: [AIChatMessage],
        using model: AIChatModelPreset,
        systemPrompt: String
    ) async throws -> String {
        switch model.provider {
        case .offline:
            return try await sendOfflineConversation(messages: messages, systemPrompt: systemPrompt)
        case .openAI:
            return try await sendOpenAIConversation(messages: messages, model: model, systemPrompt: systemPrompt)
        case .codex:
            return try await sendCodexConversation(messages: messages, systemPrompt: systemPrompt)
        case .anthropic:
            return try await sendAnthropicConversation(messages: messages, model: model, systemPrompt: systemPrompt)
        case .claudeCode:
            return try await sendClaudeCodeConversation(messages: messages, systemPrompt: systemPrompt)
        case .google:
            return try await sendGeminiConversation(messages: messages, model: model, systemPrompt: systemPrompt)
        }
    }

    func isLocalRuntimeAvailable(for provider: AIChatProvider) -> Bool {
        resolvedExecutableURL(for: provider) != nil
    }

    private func sendOfflineConversation(
        messages: [AIChatMessage],
        systemPrompt: String
    ) async throws -> String {
        guard let latestUserMessage = messages.last(where: { $0.role == .user })?.content.nilIfEmpty else {
            throw AIChatServiceError.emptyResponse(.offline)
        }

        let packetContext = extractPacketContext(from: systemPrompt)
        let loweredPrompt = latestUserMessage.lowercased()

        if let packetContext {
            let summary = offlinePacketSummary(from: packetContext)
            let guidance = offlinePacketGuidance(for: loweredPrompt, packetContext: packetContext)
            return """
            \(summary)

            \(guidance)
            """
            .trimmingCharacters(in: .whitespacesAndNewlines)
        }

        if loweredPrompt.contains("filter") {
            return "Offline mode can still help with filter ideas. Try protocol names like `http`, `dns`, `tls`, `quic`, `ospf`, or port-based filters like `443`, `udp and 53`, or `host=1.1.1.1`."
        }

        if loweredPrompt.contains("protocol") || loweredPrompt.contains("packet") || loweredPrompt.contains("capture") {
            return "Offline mode does not call a hosted model, so it works without keys or CLI tools. I can still help summarize the selected packet, explain common filters, and guide your next inspection steps. Select a packet for more precise offline analysis."
        }

        return "Offline mode is active. Select a packet if you want a local summary of what IceSniff decoded, or ask about filters, protocols, endpoints, or what to inspect next."
    }

    private func sendOpenAIConversation(
        messages: [AIChatMessage],
        model: AIChatModelPreset,
        systemPrompt: String
    ) async throws -> String {
        let apiKey = try requireAPIKey(for: .openAI)
        let inputPayload = [[
            "role": "system",
            "content": [[
                "type": "input_text",
                "text": systemPrompt
            ]]
        ]] + messages.map { message in
            [
                "role": message.role == .assistant ? "assistant" : "user",
                "content": [[
                    "type": "input_text",
                    "text": message.content
                ]]
            ]
        }

        var request = URLRequest(url: URL(string: "https://api.openai.com/v1/responses")!)
        request.httpMethod = "POST"
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        request.timeoutInterval = 60
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.httpBody = try JSONSerialization.data(withJSONObject: [
            "model": model.remoteID,
            "input": inputPayload
        ])

        let (data, response) = try await session.data(for: request)
        try validateHTTP(response, data: data, provider: .openAI)

        let envelope = try JSONDecoder().decode(OpenAIResponsesEnvelope.self, from: data)
        if let outputText = envelope.outputText?.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty {
            return outputText
        }

        let output = envelope.output?
            .flatMap { $0.content ?? [] }
            .compactMap(\.text)
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        guard let output, !output.isEmpty else {
            throw AIChatServiceError.emptyResponse(.openAI)
        }
        return output
    }

    private func sendCodexConversation(
        messages: [AIChatMessage],
        systemPrompt: String
    ) async throws -> String {
        guard isLocalRuntimeAvailable(for: .codex) else {
            throw AIChatServiceError.localRuntimeUnavailable(.codex)
        }

        let prompt = renderedTranscriptPrompt(messages: messages, systemPrompt: systemPrompt)
        return try await runLocalCommand(
            provider: .codex,
            commandURL: try requireExecutableURL(for: .codex),
            arguments: ["exec", "--skip-git-repo-check", prompt]
        )
    }

    private func sendAnthropicConversation(
        messages: [AIChatMessage],
        model: AIChatModelPreset,
        systemPrompt: String
    ) async throws -> String {
        let apiKey = try requireAPIKey(for: .anthropic)
        let anthropicMessages = messages.map { message in
            [
                "role": message.role == .assistant ? "assistant" : "user",
                "content": message.content
            ]
        }

        var request = URLRequest(url: URL(string: "https://api.anthropic.com/v1/messages")!)
        request.httpMethod = "POST"
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        request.timeoutInterval = 60
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
        request.setValue("2023-06-01", forHTTPHeaderField: "anthropic-version")
        request.httpBody = try JSONSerialization.data(withJSONObject: [
            "model": model.remoteID,
            "system": systemPrompt,
            "max_tokens": 1024,
            "messages": anthropicMessages
        ])

        let (data, response) = try await session.data(for: request)
        try validateHTTP(response, data: data, provider: .anthropic)

        let envelope = try JSONDecoder().decode(AnthropicMessagesEnvelope.self, from: data)
        let text = envelope.content
            .filter { $0.type == "text" }
            .compactMap(\.text)
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        guard !text.isEmpty else {
            throw AIChatServiceError.emptyResponse(.anthropic)
        }
        return text
    }

    private func sendClaudeCodeConversation(
        messages: [AIChatMessage],
        systemPrompt: String
    ) async throws -> String {
        guard isLocalRuntimeAvailable(for: .claudeCode) else {
            throw AIChatServiceError.localRuntimeUnavailable(.claudeCode)
        }

        let prompt = renderedTranscriptPrompt(messages: messages, systemPrompt: systemPrompt)
        return try await runLocalCommand(
            provider: .claudeCode,
            commandURL: try requireExecutableURL(for: .claudeCode),
            arguments: ["-p", prompt]
        )
    }

    private func sendGeminiConversation(
        messages: [AIChatMessage],
        model: AIChatModelPreset,
        systemPrompt: String
    ) async throws -> String {
        let apiKey = try requireAPIKey(for: .google)
        let endpoint = "https://generativelanguage.googleapis.com/v1beta/models/\(model.remoteID):generateContent"
        let contents = messages.map { message in
            [
                "role": message.role == .assistant ? "model" : "user",
                "parts": [[
                    "text": message.content
                ]]
            ]
        }

        var request = URLRequest(url: URL(string: endpoint)!)
        request.httpMethod = "POST"
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        request.timeoutInterval = 60
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(apiKey, forHTTPHeaderField: "x-goog-api-key")
        request.httpBody = try JSONSerialization.data(withJSONObject: [
            "systemInstruction": [
                "parts": [[
                    "text": systemPrompt
                ]]
            ],
            "contents": contents
        ])

        let (data, response) = try await session.data(for: request)
        try validateHTTP(response, data: data, provider: .google)

        let envelope = try JSONDecoder().decode(GeminiGenerateContentEnvelope.self, from: data)
        let text = envelope.candidates?
            .compactMap { $0.content?.parts }
            .flatMap { $0 }
            .compactMap(\.text)
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        guard let text, !text.isEmpty else {
            throw AIChatServiceError.emptyResponse(.google)
        }
        return text
    }

    private func requireAPIKey(for provider: AIChatProvider) throws -> String {
        guard let apiKey = keychain.loadAPIKey(for: provider) else {
            throw AIChatServiceError.missingAPIKey(provider)
        }
        return apiKey
    }

    private func requireExecutableURL(for provider: AIChatProvider) throws -> URL {
        guard let executableURL = resolvedExecutableURL(for: provider) else {
            throw AIChatServiceError.localRuntimeUnavailable(provider)
        }
        return executableURL
    }

    private func resolvedExecutableURL(for provider: AIChatProvider) -> URL? {
        guard let executableName = provider.executableName else {
            return nil
        }

        let fileManager = FileManager.default
        let candidateDirectories = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "\(fileManager.homeDirectoryForCurrentUser.path)/.local/bin",
            "\(fileManager.homeDirectoryForCurrentUser.path)/bin"
        ] + (ProcessInfo.processInfo.environment["PATH"]?
            .split(separator: ":")
            .map(String.init) ?? [])

        for directory in candidateDirectories {
            let candidatePath = "\(directory)/\(executableName)"
            if fileManager.isExecutableFile(atPath: candidatePath) {
                return URL(fileURLWithPath: candidatePath)
            }
        }

        return nil
    }

    private func renderedTranscriptPrompt(messages: [AIChatMessage], systemPrompt: String) -> String {
        let transcript = messages.map { message in
            let role = message.role == .assistant ? "Assistant" : "User"
            return "\(role):\n\(message.content)"
        }
        .joined(separator: "\n\n")

        return """
        \(systemPrompt)

        Continue this chat as the assistant. Reply with the next assistant message only. Do not include role labels.

        \(transcript)
        """
    }

    private func extractPacketContext(from systemPrompt: String) -> String? {
        let marker = "Current packet context from the app:"
        guard let range = systemPrompt.range(of: marker) else {
            return nil
        }

        return systemPrompt[range.upperBound...]
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
    }

    private func offlinePacketSummary(from packetContext: String) -> String {
        let packetNumber = captureGroup(in: packetContext, pattern: #"packet #(\d+)"#) ?? "?"
        let source = captureGroup(in: packetContext, pattern: #"- Source: (.+)"#) ?? "unknown source"
        let destination = captureGroup(in: packetContext, pattern: #"- Destination: (.+)"#) ?? "unknown destination"
        let protocolName = captureGroup(in: packetContext, pattern: #"- Protocol: (.+)"#) ?? "unknown protocol"
        let info = captureGroup(in: packetContext, pattern: #"- Info: (.+)"#) ?? "no extra summary"

        return "Selected packet #\(packetNumber) is \(protocolName.lowercased()) traffic from \(source) to \(destination). IceSniff’s current summary is: \(info)."
    }

    private func offlinePacketGuidance(for prompt: String, packetContext: String) -> String {
        if prompt.contains("what") || prompt.contains("summary") || prompt.contains("tell me") {
            return "Offline mode can summarize the selected packet but will not do full hosted-model reasoning. For deeper interpretation, switch to an available API or local subscription model."
        }

        if prompt.contains("suspicious") || prompt.contains("malicious") {
            return "I cannot classify this packet as malicious offline from one packet alone. Check the endpoint pair, protocol behavior, repetition across the capture, and whether the payload or flags match the expected session."
        }

        if prompt.contains("filter") {
            let protocolName = captureGroup(in: packetContext, pattern: #"- Protocol: (.+)"#)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .lowercased() ?? "packet"
            let source = captureGroup(in: packetContext, pattern: #"- Source: (.+)"#) ?? ""
            let destination = captureGroup(in: packetContext, pattern: #"- Destination: (.+)"#) ?? ""
            return "Useful next filters: `\(protocolName)`, `host=\(source)`, `host=\(destination)`, or combine them with ports if the packet summary exposes one."
        }

        return "If you want more detail offline, inspect the right-side packet JSON tree and narrow the packet list with protocol, host, or port filters."
    }

    private func captureGroup(in text: String, pattern: String) -> String? {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.anchorsMatchLines]) else {
            return nil
        }
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        guard let match = regex.firstMatch(in: text, options: [], range: range),
              match.numberOfRanges > 1,
              let groupRange = Range(match.range(at: 1), in: text) else {
            return nil
        }
        return String(text[groupRange]).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func runLocalCommand(
        provider: AIChatProvider,
        commandURL: URL,
        arguments: [String]
    ) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process()
            let stdout = Pipe()
            let stderr = Pipe()
            let outputAccumulator = LocalRuntimeOutputAccumulator()
            process.executableURL = commandURL
            process.arguments = arguments
            process.currentDirectoryURL = FileManager.default.homeDirectoryForCurrentUser
            process.environment = mergedRuntimeEnvironment()
            process.standardOutput = stdout
            process.standardError = stderr

            stdout.fileHandleForReading.readabilityHandler = { handle in
                let chunk = handle.availableData
                guard !chunk.isEmpty else {
                    handle.readabilityHandler = nil
                    return
                }
                outputAccumulator.appendStdout(chunk)
            }

            stderr.fileHandleForReading.readabilityHandler = { handle in
                let chunk = handle.availableData
                guard !chunk.isEmpty else {
                    handle.readabilityHandler = nil
                    return
                }
                outputAccumulator.appendStderr(chunk)
            }

            process.terminationHandler = { process in
                stdout.fileHandleForReading.readabilityHandler = nil
                stderr.fileHandleForReading.readabilityHandler = nil

                outputAccumulator.appendStdout(stdout.fileHandleForReading.readDataToEndOfFile())
                outputAccumulator.appendStderr(stderr.fileHandleForReading.readDataToEndOfFile())
                let snapshot = outputAccumulator.snapshot()

                let stdoutText = normalizedLocalRuntimeOutput(from: snapshot.stdout)
                let stderrText = normalizedLocalRuntimeOutput(from: snapshot.stderr)

                if process.terminationStatus == 0, let output = stdoutText.nilIfEmpty ?? stderrText.nilIfEmpty {
                    continuation.resume(returning: output)
                    return
                }

                let failure = formattedLocalRuntimeFailure(
                    provider: provider,
                    stdoutText: stdoutText,
                    stderrText: stderrText,
                    exitCode: process.terminationStatus
                )
                continuation.resume(throwing: AIChatServiceError.requestFailed(failure))
            }

            do {
                try process.run()
            } catch {
                continuation.resume(throwing: AIChatServiceError.requestFailed("Failed to launch \(provider.title) on this Mac. Verify the local runtime is installed and available, then try again."))
            }
        }
    }

    private func normalizedLocalRuntimeOutput(from data: Data) -> String {
        let text = String(data: data, encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !text.isEmpty else {
            return ""
        }
        if text.count > 1600 {
            return String(text.prefix(1600)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return text
    }

    private func formattedLocalRuntimeFailure(
        provider: AIChatProvider,
        stdoutText: String,
        stderrText: String,
        exitCode: Int32
    ) -> String {
        let rawFailure = stderrText.nilIfEmpty ?? stdoutText.nilIfEmpty ?? "\(provider.title) failed without output."

        if provider == .claudeCode {
            return "Claude Code is not available correctly on this device right now. Please install or set up Claude Code, confirm it works in Terminal, then restart the app and try again."
        }

        if provider == .codex,
           rawFailure.localizedCaseInsensitiveContains("login") || rawFailure.localizedCaseInsensitiveContains("auth") {
            return "Codex is installed, but the local CLI is not authenticated for app use yet. Open Terminal, confirm Codex works there, then try again."
        }

        if provider == .codex {
            return "Codex is installed but could not complete this request. Confirm Codex works in Terminal on this Mac, then try again."
        }

        return "\(provider.title) could not complete this request on this Mac. Exit code \(exitCode)."
    }

    private func mergedRuntimeEnvironment() -> [String: String] {
        var environment = ProcessInfo.processInfo.environment
        let fileManager = FileManager.default
        let preferredPaths = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "\(fileManager.homeDirectoryForCurrentUser.path)/.local/bin",
            "\(fileManager.homeDirectoryForCurrentUser.path)/bin"
        ]
        let existingPaths = environment["PATH"]?
            .split(separator: ":")
            .map(String.init) ?? []
        environment["PATH"] = Array(Set(preferredPaths + existingPaths)).joined(separator: ":")
        return environment
    }

    private func validateHTTP(_ response: URLResponse, data: Data, provider: AIChatProvider) throws {
        guard let httpResponse = response as? HTTPURLResponse else {
            throw AIChatServiceError.requestFailed("\(provider.title) returned an invalid network response.")
        }

        guard (200..<300).contains(httpResponse.statusCode) else {
            throw AIChatServiceError.requestFailed(sanitizedHTTPFailure(provider: provider, statusCode: httpResponse.statusCode, body: data))
        }
    }

    private func sanitizedHTTPFailure(provider: AIChatProvider, statusCode: Int, body: Data) -> String {
        let text = String(data: body, encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let normalized = text.lowercased()

        if statusCode == 401 || statusCode == 403 || normalized.contains("invalid api key") || normalized.contains("authentication") || normalized.contains("unauthorized") {
            return "\(provider.title) rejected the current credentials. Check the saved API key and try again."
        }

        if statusCode == 429 || normalized.contains("rate limit") || normalized.contains("quota") {
            return "\(provider.title) is rate-limiting or quota-limiting this request right now. Wait and try again later."
        }

        if (500...599).contains(statusCode) {
            return "\(provider.title) is having a server-side problem right now. Try again later."
        }

        return "\(provider.title) request failed with status \(statusCode). Check your provider configuration and try again."
    }
}

@MainActor
final class AIChatController: ObservableObject {
    @Published var panelCollapsed = true
    @Published var settingsVisible = false
    @Published var selectedModelID = AIChatModelPreset.offlineDefault.id
    @Published var draftMessage = ""
    @Published var messages: [AIChatMessage] = [
        AIChatMessage(
            role: .assistant,
            content: "Ask about packets, protocols, filters, or what you are seeing in the capture. IceSniff can use its built-in offline assistant, your saved API keys, or local Codex and Claude Code sessions when available."
        )
    ]
    @Published private(set) var isSending = false
    @Published private(set) var statusMessage = ""
    @Published var openAIApiKeyDraft = ""
    @Published var anthropicApiKeyDraft = ""
    @Published var googleApiKeyDraft = ""
    @Published private(set) var localRuntimeAvailability: [AIChatProvider: Bool] = [:]

    private let keychain: AIKeychainStore
    private let service: AIChatService
    private let systemPrompt = """
    You are the embedded AI assistant inside IceSniff, a native macOS packet capture browser.
    Help the user understand network captures, protocols, filters, traffic patterns, suspicious activity, and app workflows.
    Be concise, technical, and actionable. If the question asks for facts not present in the current conversation, say so plainly.
    """

    init(
        keychain: AIKeychainStore = AIKeychainStore(),
        service: AIChatService? = nil
    ) {
        self.keychain = keychain
        self.service = service ?? AIChatService(keychain: keychain)
        refreshLocalRuntimeAvailability()
        refreshAvailableModels()
    }

    var availableModels: [AIChatModelPreset] {
        AIChatModelPreset.catalog.filter { preset in
            switch preset.access {
            case .offline:
                return true
            case .bringYourOwnKey:
                return keychain.hasAPIKey(for: preset.provider)
            case .localSubscription:
                return localRuntimeAvailability[preset.provider] == true
            }
        }
    }

    var selectedModel: AIChatModelPreset {
        availableModels.first(where: { $0.id == selectedModelID }) ?? bestAvailableModel()
    }

    func togglePanel() {
        panelCollapsed.toggle()
    }

    func toggleSettings() {
        settingsVisible.toggle()
    }

    func setSelectedModel(_ modelID: String) {
        guard availableModels.contains(where: { $0.id == modelID }) else {
            selectedModelID = bestAvailableModel().id
            statusMessage = "That model is not currently available on this Mac."
            return
        }

        selectedModelID = modelID
        switch selectedModel.access {
        case .offline:
            statusMessage = "Using the built-in offline assistant."
        case .bringYourOwnKey:
            statusMessage = "Using \(selectedModel.title) via \(selectedModel.provider.title)."
        case .localSubscription:
            statusMessage = "Using \(selectedModel.title) through the local \(selectedModel.provider.title) session."
        }
    }

    func apiKeyConfigured(for provider: AIChatProvider) -> Bool {
        keychain.hasAPIKey(for: provider)
    }

    func localRuntimeConfigured(for provider: AIChatProvider) -> Bool {
        localRuntimeAvailability[provider] == true
    }

    func refreshLocalRuntimeAvailability() {
        localRuntimeAvailability[.codex] = service.isLocalRuntimeAvailable(for: .codex)
        localRuntimeAvailability[.claudeCode] = service.isLocalRuntimeAvailable(for: .claudeCode)
        refreshAvailableModels()
    }

    func saveAPIKeyDraft(for provider: AIChatProvider) {
        let value: String
        switch provider {
        case .offline:
            statusMessage = "The offline assistant does not use an API key."
            return
        case .openAI:
            value = openAIApiKeyDraft
        case .anthropic:
            value = anthropicApiKeyDraft
        case .google:
            value = googleApiKeyDraft
        case .codex, .claudeCode:
            statusMessage = "Local subscription providers do not use API keys here."
            return
        }

        let trimmedValue = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedValue.isEmpty else {
            statusMessage = "Enter an API key before saving \(provider.title)."
            return
        }

        do {
            try keychain.saveAPIKey(trimmedValue, for: provider)
            clearDraft(for: provider)
            refreshAvailableModels()
            statusMessage = "\(provider.title) API key saved to Keychain."
        } catch {
            statusMessage = error.localizedDescription
        }
    }

    func removeAPIKey(for provider: AIChatProvider) {
        do {
            try keychain.removeAPIKey(for: provider)
            clearDraft(for: provider)
            refreshAvailableModels()
            statusMessage = "\(provider.title) API key removed."
        } catch {
            statusMessage = error.localizedDescription
        }
    }

    func sendDraftMessage(packetContext: String? = nil) {
        let trimmedDraft = draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedDraft.isEmpty else {
            statusMessage = "Type a prompt before sending."
            return
        }
        guard !isSending else { return }

        let userMessage = AIChatMessage(role: .user, content: trimmedDraft)
        messages.append(userMessage)
        draftMessage = ""
        isSending = true
        statusMessage = "Sending to \(selectedModel.title)..."

        let conversation = messages
        let selectedModel = self.selectedModel
        let effectiveSystemPrompt: String
        if let packetContext, !packetContext.isEmpty {
            effectiveSystemPrompt = """
            \(systemPrompt)

            Current packet context from the app:
            \(packetContext)
            """
        } else {
            effectiveSystemPrompt = systemPrompt
        }
        Task {
            do {
                let reply = try await service.sendConversation(
                    messages: conversation,
                    using: selectedModel,
                    systemPrompt: effectiveSystemPrompt
                )
                await MainActor.run {
                    messages.append(AIChatMessage(role: .assistant, content: sanitizeAssistantReply(reply)))
                    isSending = false
                    statusMessage = "Reply received from \(selectedModel.title)."
                }
            } catch {
                await MainActor.run {
                    messages.append(
                        AIChatMessage(
                            role: .assistant,
                            content: "Request failed: \(error.localizedDescription)"
                        )
                    )
                    isSending = false
                    statusMessage = error.localizedDescription
                }
            }
        }
    }

    private func clearDraft(for provider: AIChatProvider) {
        switch provider {
        case .offline:
            break
        case .openAI:
            openAIApiKeyDraft = ""
        case .anthropic:
            anthropicApiKeyDraft = ""
        case .google:
            googleApiKeyDraft = ""
        case .codex, .claudeCode:
            break
        }
    }

    private func sanitizeAssistantReply(_ reply: String) -> String {
        reply
            .replacingOccurrences(of: "**", with: "")
            .replacingOccurrences(of: "__", with: "")
    }

    private func refreshAvailableModels() {
        let bestModel = bestAvailableModel()
        if !availableModels.contains(where: { $0.id == selectedModelID }) {
            selectedModelID = bestModel.id
        }

        if statusMessage.isEmpty {
            switch bestModel.access {
            case .offline:
                statusMessage = "Using the built-in offline assistant."
            case .bringYourOwnKey:
                statusMessage = "\(bestModel.title) is available with a saved API key."
            case .localSubscription:
                statusMessage = "\(bestModel.title) is available through a local signed-in CLI session."
            }
        }
    }

    private func bestAvailableModel() -> AIChatModelPreset {
        let preferredIDs = [
            "openai-gpt-4.1",
            "codex-chatgpt",
            "anthropic-claude-sonnet-4",
            "claude-code-subscription",
            "google-gemini-2.5-pro",
            "offline-assistant"
        ]

        for id in preferredIDs {
            if let model = availableModels.first(where: { $0.id == id }) {
                return model
            }
        }

        return AIChatModelPreset.offlineDefault
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
