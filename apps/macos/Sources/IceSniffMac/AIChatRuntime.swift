import Foundation
import Security

enum AIChatProvider: String, CaseIterable, Identifiable {
    case openAI
    case codex
    case anthropic
    case claudeCode
    case google

    var id: String { rawValue }

    var title: String {
        switch self {
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

    static let `default` = catalog.first(where: { $0.id == "openai-gpt-4.1" }) ?? catalog[0]
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
    }

    func saveAPIKey(_ value: String, for provider: AIChatProvider) throws {
        let data = Data(value.utf8)
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: provider.rawValue
        ]
        let attributes: [CFString: Any] = [
            kSecValueData: data
        ]

        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecItemNotFound {
            var addQuery = query
            addQuery[kSecValueData] = data
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
            kSecAttrAccount: provider.rawValue
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

    init(keychain: AIKeychainStore = AIKeychainStore(), session: URLSession = .shared) {
        self.keychain = keychain
        self.session = session
    }

    func sendConversation(
        messages: [AIChatMessage],
        using model: AIChatModelPreset,
        systemPrompt: String
    ) async throws -> String {
        switch model.provider {
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
        let endpoint = "https://generativelanguage.googleapis.com/v1beta/models/\(model.remoteID):generateContent?key=\(apiKey)"
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
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
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
                continuation.resume(throwing: AIChatServiceError.requestFailed("Failed to launch \(provider.title): \(error.localizedDescription)"))
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

        return "\(provider.title) failed (exit \(exitCode)): \(rawFailure)"
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
            let body = String(data: data, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .prefix(240) ?? ""
            throw AIChatServiceError.requestFailed("\(provider.title) request failed (\(httpResponse.statusCode)): \(body)")
        }
    }
}

@MainActor
final class AIChatController: ObservableObject {
    @Published var panelCollapsed = true
    @Published var settingsVisible = false
    @Published var selectedModelID = AIChatModelPreset.default.id
    @Published var draftMessage = ""
    @Published var messages: [AIChatMessage] = [
        AIChatMessage(
            role: .assistant,
            content: "Ask about packets, protocols, filters, or what you are seeing in the capture. Add your OpenAI, Anthropic, or Google API key in settings to use live models."
        )
    ]
    @Published private(set) var isSending = false
    @Published private(set) var statusMessage = "Add your OpenAI, Anthropic, or Google API key in settings to use your own model account."
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
    }

    var availableModels: [AIChatModelPreset] {
        AIChatModelPreset.catalog
    }

    var selectedModel: AIChatModelPreset {
        availableModels.first(where: { $0.id == selectedModelID }) ?? .default
    }

    func togglePanel() {
        panelCollapsed.toggle()
    }

    func toggleSettings() {
        settingsVisible.toggle()
    }

    func setSelectedModel(_ modelID: String) {
        selectedModelID = modelID
        switch selectedModel.access {
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
    }

    func saveAPIKeyDraft(for provider: AIChatProvider) {
        let value: String
        switch provider {
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
            statusMessage = "\(provider.title) API key saved to Keychain."
        } catch {
            statusMessage = error.localizedDescription
        }
    }

    func removeAPIKey(for provider: AIChatProvider) {
        do {
            try keychain.removeAPIKey(for: provider)
            clearDraft(for: provider)
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
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
