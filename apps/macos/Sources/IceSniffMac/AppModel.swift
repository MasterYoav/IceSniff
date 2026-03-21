import Foundation

enum AppSection: String, CaseIterable, Identifiable {
    case packets
    case stats
    case conversations
    case streams
    case transactions
    case profile
    case settings

    var id: String { rawValue }

    var title: String {
        switch self {
        case .packets: return "Packets"
        case .stats: return "Stats"
        case .conversations: return "Conversations"
        case .streams: return "Streams"
        case .transactions: return "Transactions"
        case .profile: return "Profile"
        case .settings: return "Settings"
        }
    }

    var iconSystemName: String {
        switch self {
        case .packets: return "list.bullet.rectangle.portrait"
        case .stats: return "chart.bar.xaxis"
        case .conversations: return "arrow.left.arrow.right.circle"
        case .streams: return "waveform.path.ecg"
        case .transactions: return "point.topleft.down.curvedto.point.bottomright.up"
        case .profile: return "person.crop.circle"
        case .settings: return "gearshape"
        }
    }

    static let primarySections: [AppSection] = [.packets, .stats, .conversations, .streams, .transactions]
    static let footerSections: [AppSection] = [.profile, .settings]
}

enum AppTheme: String, CaseIterable, Identifiable, Codable {
    case defaultDark
    case defaultLight
    case ocean
    case ember
    case forest

    var id: String { rawValue }

    var title: String {
        switch self {
        case .defaultDark: return "Default Dark"
        case .defaultLight: return "Default Light"
        case .ocean: return "Ocean"
        case .ember: return "Ember"
        case .forest: return "Forest"
        }
    }

    var isDark: Bool {
        switch self {
        case .defaultDark, .ocean, .ember, .forest:
            return true
        case .defaultLight:
            return false
        }
    }
}

enum AppFontChoice: String, CaseIterable, Identifiable, Codable {
    case system
    case rounded
    case serif
    case monospaced

    var id: String { rawValue }

    var title: String {
        switch self {
        case .system: return "System"
        case .rounded: return "Rounded"
        case .serif: return "Serif"
        case .monospaced: return "Monospaced"
        }
    }
}

enum AppFontSizeStep: String, Codable {
    case extraSmall
    case small
    case medium
    case large
    case extraLarge

    var scale: CGFloat {
        switch self {
        case .extraSmall: return 0.86
        case .small: return 0.93
        case .medium: return 1.0
        case .large: return 1.08
        case .extraLarge: return 1.16
        }
    }
}

struct UserPreferences: Codable, Equatable {
    static let currentSchemaVersion = 1

    private static func codingDateFormatter() -> ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }

    private enum CodingKeys: String, CodingKey {
        case theme
        case fontChoice
        case fontSizeStep
        case schemaVersion
        case updatedAt
    }

    var theme: AppTheme
    var fontChoice: AppFontChoice
    var fontSizeStep: AppFontSizeStep
    var schemaVersion: Int
    var updatedAt: Date

    init(
        theme: AppTheme,
        fontChoice: AppFontChoice,
        fontSizeStep: AppFontSizeStep,
        schemaVersion: Int = UserPreferences.currentSchemaVersion,
        updatedAt: Date = .now
    ) {
        self.theme = theme
        self.fontChoice = fontChoice
        self.fontSizeStep = fontSizeStep
        self.schemaVersion = schemaVersion
        self.updatedAt = updatedAt
    }

    static let `default` = UserPreferences(
        theme: .defaultDark,
        fontChoice: .rounded,
        fontSizeStep: .medium
    )

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let defaults = Self.default

        theme = try container.decodeIfPresent(AppTheme.self, forKey: .theme) ?? defaults.theme
        fontChoice = try container.decodeIfPresent(AppFontChoice.self, forKey: .fontChoice) ?? defaults.fontChoice
        fontSizeStep = try container.decodeIfPresent(AppFontSizeStep.self, forKey: .fontSizeStep) ?? defaults.fontSizeStep
        schemaVersion = try container.decodeIfPresent(Int.self, forKey: .schemaVersion) ?? Self.currentSchemaVersion

        if let timestamp = try container.decodeIfPresent(String.self, forKey: .updatedAt),
           let parsedDate = Self.codingDateFormatter().date(from: timestamp) {
            updatedAt = parsedDate
        } else if let date = try container.decodeIfPresent(Date.self, forKey: .updatedAt) {
            updatedAt = date
        } else {
            updatedAt = defaults.updatedAt
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(theme, forKey: .theme)
        try container.encode(fontChoice, forKey: .fontChoice)
        try container.encode(fontSizeStep, forKey: .fontSizeStep)
        try container.encode(schemaVersion, forKey: .schemaVersion)
        try container.encode(Self.codingDateFormatter().string(from: updatedAt), forKey: .updatedAt)
    }
}

final class PreferencesStore {
    private enum Keys {
        static let preferencesBlob = "icesniff.user_preferences"
        static let appTheme = "icesniff.app_theme"
        static let darkMode = "icesniff.dark_mode"
        static let fontChoice = "icesniff.font_choice"
        static let fontSizeStep = "icesniff.font_size_step"
    }

    private let defaults: UserDefaults
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        encoder.dateEncodingStrategy = .iso8601
        decoder.dateDecodingStrategy = .iso8601
    }

    func loadPersistedPreferences() -> UserPreferences? {
        guard hasPersistedPreferences else {
            return nil
        }

        return load()
    }

    func load() -> UserPreferences {
        if let data = defaults.data(forKey: Keys.preferencesBlob),
           let preferences = try? decoder.decode(UserPreferences.self, from: data) {
            return preferences
        }

        return loadLegacyPreferences()
    }

    private var hasPersistedPreferences: Bool {
        if defaults.data(forKey: Keys.preferencesBlob) != nil {
            return true
        }

        return defaults.object(forKey: Keys.appTheme) != nil
            || defaults.object(forKey: Keys.darkMode) != nil
            || defaults.object(forKey: Keys.fontChoice) != nil
            || defaults.object(forKey: Keys.fontSizeStep) != nil
    }

    func save(_ preferences: UserPreferences) {
        if let data = try? encoder.encode(preferences) {
            defaults.set(data, forKey: Keys.preferencesBlob)
        }

        // Keep legacy keys in sync during the migration so older development builds still read
        // the same UI state when switching branches or binaries.
        defaults.set(preferences.theme.rawValue, forKey: Keys.appTheme)
        defaults.set(preferences.theme.isDark, forKey: Keys.darkMode)
        defaults.set(preferences.fontChoice.rawValue, forKey: Keys.fontChoice)
        defaults.set(preferences.fontSizeStep.rawValue, forKey: Keys.fontSizeStep)
    }

    private func loadLegacyPreferences() -> UserPreferences {
        let theme: AppTheme
        if let rawTheme = defaults.string(forKey: Keys.appTheme),
           let persistedTheme = AppTheme(rawValue: rawTheme) {
            theme = persistedTheme
        } else {
            let legacyDarkMode = defaults.object(forKey: Keys.darkMode) as? Bool ?? true
            theme = legacyDarkMode ? .defaultDark : .defaultLight
        }

        let fontChoice: AppFontChoice
        if let rawFontChoice = defaults.string(forKey: Keys.fontChoice),
           let persistedFontChoice = AppFontChoice(rawValue: rawFontChoice) {
            fontChoice = persistedFontChoice
        } else {
            fontChoice = .rounded
        }

        let fontSizeStep: AppFontSizeStep
        if let rawFontSizeStep = defaults.string(forKey: Keys.fontSizeStep),
           let persistedFontSizeStep = AppFontSizeStep(rawValue: rawFontSizeStep) {
            fontSizeStep = persistedFontSizeStep
        } else {
            fontSizeStep = .medium
        }

        return UserPreferences(
            theme: theme,
            fontChoice: fontChoice,
            fontSizeStep: fontSizeStep
        )
    }
}

enum AuthProvider: String, CaseIterable, Identifiable, Codable {
    case google
    case github

    var id: String { rawValue }

    var title: String {
        switch self {
        case .google:
            return "Google"
        case .github:
            return "GitHub"
        }
    }

    var symbolName: String {
        switch self {
        case .google:
            return "globe"
        case .github:
            return "chevron.left.forwardslash.chevron.right"
        }
    }

}

struct AuthSession: Equatable, Sendable {
    let userID: String
    let email: String?
    let displayName: String?
    let avatarURL: URL?
    let provider: AuthProvider
}

enum SyncStatus: Equatable, Sendable {
    case idle
    case syncing
    case synced(Date?)
    case failed(String)

    var title: String {
        switch self {
        case .idle:
            return "Not synced yet"
        case .syncing:
            return "Syncing..."
        case let .synced(date):
            if let date {
                return "Synced \(RelativeDateTimeFormatter().localizedString(for: date, relativeTo: .now))"
            }
            return "Synced"
        case let .failed(message):
            return "Sync failed: \(message)"
        }
    }
}

protocol AuthService: Sendable {
    var currentSession: AuthSession? { get }
    func signIn(with provider: AuthProvider) async throws -> AuthSession
    func signOut() async throws
}

protocol ProfileSyncService: Sendable {
    func pullPreferences(for session: AuthSession) async throws -> UserPreferences?
    func pushPreferences(_ preferences: UserPreferences, for session: AuthSession) async throws
}

enum CloudProfileConfigurationError: LocalizedError {
    case missingConfiguration(String)

    var errorDescription: String? {
        switch self {
        case let .missingConfiguration(message):
            return message
        }
    }
}

struct DisabledAuthService: AuthService {
    let diagnosticMessage: String

    var currentSession: AuthSession?

    func signIn(with provider: AuthProvider) async throws -> AuthSession {
        throw CloudProfileConfigurationError.missingConfiguration(diagnosticMessage)
    }

    func signOut() async throws {}
}

struct DisabledProfileSyncService: ProfileSyncService {
    func pullPreferences(for session: AuthSession) async throws -> UserPreferences? {
        nil
    }

    func pushPreferences(_ preferences: UserPreferences, for session: AuthSession) async throws {}
}

struct MockAuthService: AuthService {
    var currentSession: AuthSession?

    func signIn(with provider: AuthProvider) async throws -> AuthSession {
        switch provider {
        case .google:
            return AuthSession(
                userID: "mock-google-user",
                email: "google-user@example.com",
                displayName: "Google User",
                avatarURL: nil,
                provider: .google
            )
        case .github:
            return AuthSession(
                userID: "mock-github-user",
                email: "github-user@example.com",
                displayName: "GitHub User",
                avatarURL: URL(string: "https://avatars.githubusercontent.com/u/9919?v=4"),
                provider: .github
            )
        }
    }

    func signOut() async throws {}
}

struct MockProfileSyncService: ProfileSyncService {
    func pullPreferences(for session: AuthSession) async throws -> UserPreferences? {
        nil
    }

    func pushPreferences(_ preferences: UserPreferences, for session: AuthSession) async throws {}
}

enum CaptureSaveScope {
    case filteredOnly
    case wholeCapture
}

enum FilterExpressionNormalizer {
    static func normalize(_ expression: String) -> String? {
        let trimmed = expression.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        let lowercased = trimmed.lowercased()
        let canonical = lowercased
            .replacingOccurrences(of: #"(?i)\s+and\s+"#, with: " && ", options: .regularExpression)
            .replacingOccurrences(of: #"(?<!&)&(?!&)"#, with: " && ", options: .regularExpression)
        let explicitOperators = CharacterSet(charactersIn: "=!<>&|()")

        if canonical.rangeOfCharacter(from: explicitOperators) != nil {
            return normalizeExplicit(canonical)
        }

        let tokens = canonical
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
            .filter { !$0.isEmpty }

        guard !tokens.isEmpty else { return nil }
        return tokens.map(shorthandClause(for:)).joined(separator: " && ")
    }

    static func saveFilter(for scope: CaptureSaveScope, expression: String) -> String? {
        switch scope {
        case .filteredOnly:
            return normalize(expression)
        case .wholeCapture:
            return nil
        }
    }

    private static func normalizeExplicit(_ filter: String) -> String {
        let rawClauses = filter
            .components(separatedBy: "&&")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }

        guard !rawClauses.isEmpty else { return filter }

        return rawClauses.map { clause in
            if let value = clause.split(separator: "=", maxSplits: 1).last.map(String.init)?
                .trimmingCharacters(in: .whitespacesAndNewlines),
               clause.hasPrefix("protocol=") {
                return "protocol=\(value.lowercased())"
            }

            if let value = clause.split(separator: "=", maxSplits: 1).last.map(String.init)?
                .trimmingCharacters(in: .whitespacesAndNewlines),
               clause.hasPrefix("port=") {
                return "port=\(value)"
            }

            let containsAdvancedOperators = clause.range(of: #"[!<>\|\(\)]"#, options: .regularExpression) != nil
            if containsAdvancedOperators {
                return clause
            }

            return shorthandClause(for: clause)
        }.joined(separator: " && ")
    }

    private static func shorthandClause(for token: String) -> String {
        if Int(token) != nil {
            return "port=\(token)"
        }

        switch token {
        case "http", "https", "tcp", "udp", "dns", "tls", "quic", "icmp", "arp", "dhcp", "ssh", "ftp":
            return "protocol=\(token)"
        default:
            return "protocol=\(token)"
        }
    }
}

enum LiveCaptureErrorMapper {
    static func message(for error: Error) -> String {
        let description = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
        let normalized = description.lowercased()

        if normalized.contains("user canceled")
            || normalized.contains("user cancelled")
            || normalized.contains("cancelled")
            || normalized.contains("canceled") {
            return "Administrator authentication was canceled. Live capture did not start."
        }

        if normalized.contains("operation not permitted")
            || normalized.contains("permission denied")
            || normalized.contains("/dev/bpf") {
            return "Live capture requires administrator approval on macOS. Approve the prompt and try again."
        }

        if normalized.contains("no such file")
            || normalized.contains("not found")
            || normalized.contains("capture backend unavailable") {
            return "No packet capture backend is available on this Mac."
        }

        if normalized.contains("privileged live capture failed to launch") {
            return "Live capture failed to launch after administrator approval."
        }

        if normalized.contains("did not record a process identifier") {
            return "Live capture started unreliably and could not be managed. Try starting it again."
        }

        return description
    }
}

enum PrivilegedCaptureCommandBuilder {
    static func launchCommand(
        executablePath: String,
        arguments: [String],
        pidFile: String,
        errorFile: String
    ) -> String {
        let shellCommand = ([executablePath] + arguments).map(shellQuoted).joined(separator: " ")
        return "umask 077; nohup \(shellCommand) >/dev/null 2>\(shellQuoted(errorFile)) < /dev/null & echo $! > \(shellQuoted(pidFile))"
    }

    static func stopCommand(pidFile: String, errorFile: String) -> String {
        """
        if [ -f \(shellQuoted(pidFile)) ]; then
          pid=$(cat \(shellQuoted(pidFile)))
          if [ -n "$pid" ]; then
            kill -TERM "$pid" 2>/dev/null || true
            for _ in 1 2 3 4 5 6 7 8 9 10; do
              kill -0 "$pid" 2>/dev/null || break
              sleep 0.1
            done
            kill -0 "$pid" 2>/dev/null && kill -KILL "$pid" 2>/dev/null || true
          fi
        fi
        rm -f \(shellQuoted(pidFile)) \(shellQuoted(errorFile))
        """
    }

    static func shellQuoted(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}

enum CapturePrivilegeSetup {
    static let supportDirectory = "/Library/Application Support/IceSniff"
    static let scriptPath = "/Library/Application Support/IceSniff/chmod-bpf.sh"
    static let plistPath = "/Library/LaunchDaemons/com.icesniff.chmodbpf.plist"
    static let label = "com.icesniff.chmodbpf"

    static func firstExistingBPFDevicePath() -> String? {
        for index in 0..<256 {
            let path = "/dev/bpf\(index)"
            if FileManager.default.fileExists(atPath: path) {
                return path
            }
        }
        return nil
    }

    static func isCaptureAccessReady() -> Bool {
        guard let path = firstExistingBPFDevicePath() else { return false }
        return FileManager.default.isWritableFile(atPath: path)
    }

    static func isInstalled() -> Bool {
        FileManager.default.fileExists(atPath: scriptPath) && FileManager.default.fileExists(atPath: plistPath)
    }

    static func installCommand() -> String {
        let script = """
        #!/bin/sh
        for dev in /dev/bpf*; do
          [ -e "$dev" ] || continue
          /usr/bin/chgrp admin "$dev" 2>/dev/null || true
          /bin/chmod 660 "$dev" 2>/dev/null || true
        done
        """

        let plist = """
        <?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
        <plist version=\"1.0\">
        <dict>
            <key>Label</key>
            <string>\(label)</string>
            <key>ProgramArguments</key>
            <array>
                <string>/bin/sh</string>
                <string>\(scriptPath)</string>
            </array>
            <key>RunAtLoad</key>
            <true/>
        </dict>
        </plist>
        """

        return """
        /bin/mkdir -p \(PrivilegedCaptureCommandBuilder.shellQuoted(supportDirectory))
        /usr/bin/install -m 755 /dev/null \(PrivilegedCaptureCommandBuilder.shellQuoted(scriptPath))
        /bin/cat > \(PrivilegedCaptureCommandBuilder.shellQuoted(scriptPath)) <<'ICESNIFF_BPF_SCRIPT'
        \(script)
        ICESNIFF_BPF_SCRIPT
        /usr/sbin/chown root:wheel \(PrivilegedCaptureCommandBuilder.shellQuoted(scriptPath))
        /bin/chmod 755 \(PrivilegedCaptureCommandBuilder.shellQuoted(scriptPath))
        /usr/bin/install -m 644 /dev/null \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath))
        /bin/cat > \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath)) <<'ICESNIFF_BPF_PLIST'
        \(plist)
        ICESNIFF_BPF_PLIST
        /usr/sbin/chown root:wheel \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath))
        /bin/chmod 644 \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath))
        /bin/launchctl bootout system \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath)) >/dev/null 2>&1 || true
        /bin/launchctl bootstrap system \(PrivilegedCaptureCommandBuilder.shellQuoted(plistPath))
        /bin/launchctl enable system/\(label) >/dev/null 2>&1 || true
        /bin/sh \(PrivilegedCaptureCommandBuilder.shellQuoted(scriptPath))
        """
    }
}

struct EngineCapabilities {
    let version: String
    let supportsInspect: Bool
    let supportsPacketList: Bool
    let supportsPacketDetail: Bool
    let supportsStats: Bool
    let supportsConversations: Bool
    let supportsStreams: Bool
    let supportsTransactions: Bool
    let supportsSave: Bool
    let supportsLiveCapture: Bool
    let capture: EngineCaptureSupport
    let filters: EngineFilterSupport
    let export: EngineExportSupport
    let supportedProtocols: [String]

    static let fallback = EngineCapabilities(
        version: "v1",
        supportsInspect: true,
        supportsPacketList: true,
        supportsPacketDetail: true,
        supportsStats: true,
        supportsConversations: true,
        supportsStreams: true,
        supportsTransactions: true,
        supportsSave: true,
        supportsLiveCapture: true,
        capture: .fallback,
        filters: .fallback,
        export: .fallback,
        supportedProtocols: ["arp", "dns", "ethernet", "http", "ipv4", "ipv6", "tcp", "tls", "udp"]
    )
}

struct EngineCaptureSupport {
    let bundledBackend: Bool
    let builtInTcpdump: Bool
    let interfaceDiscovery: Bool
    let requiresAdminForLiveCapture: Bool

    static let fallback = EngineCaptureSupport(
        bundledBackend: true,
        builtInTcpdump: true,
        interfaceDiscovery: true,
        requiresAdminForLiveCapture: true
    )
}

struct EngineFilterSupport {
    let packetFilters: Bool
    let streamFilters: Bool
    let transactionFilters: Bool
    let shorthandProtocolTerms: Bool
    let shorthandPortTerms: Bool
    let caseInsensitiveProtocols: Bool
    let alternateAndOperators: [String]

    static let fallback = EngineFilterSupport(
        packetFilters: true,
        streamFilters: true,
        transactionFilters: true,
        shorthandProtocolTerms: true,
        shorthandPortTerms: true,
        caseInsensitiveProtocols: true,
        alternateAndOperators: ["&&", "&", "and"]
    )
}

struct EngineExportSupport {
    let saveCapture: Bool
    let filteredSave: Bool
    let wholeCaptureSave: Bool

    static let fallback = EngineExportSupport(
        saveCapture: true,
        filteredSave: true,
        wholeCaptureSave: true
    )
}

enum EngineCapabilitiesParser {
    static func parse(_ response: [String: Any]) -> EngineCapabilities {
        let capabilities = response["capabilities"] as? [String: Any]
        let capture = response["capture"] as? [String: Any]
        let filters = response["filters"] as? [String: Any]
        let export = response["export"] as? [String: Any]
        let dissectors = response["dissectors"] as? [String: Any]

        return EngineCapabilities(
            version: stringValue(response["engine_version"]).isEmpty ? stringValue(response["schema_version"]) : stringValue(response["engine_version"]),
            supportsInspect: boolValue(capabilities?["inspect"]),
            supportsPacketList: boolValue(capabilities?["packet_list"]),
            supportsPacketDetail: boolValue(capabilities?["packet_detail"]),
            supportsStats: boolValue(capabilities?["stats"]),
            supportsConversations: boolValue(capabilities?["conversations"]),
            supportsStreams: boolValue(capabilities?["streams"]),
            supportsTransactions: boolValue(capabilities?["transactions"]),
            supportsSave: boolValue(capabilities?["save"]),
            supportsLiveCapture: boolValue(capabilities?["live_capture"]),
            capture: EngineCaptureSupport(
                bundledBackend: boolValue(capture?["bundled_backend"]),
                builtInTcpdump: boolValue(capture?["built_in_tcpdump"]),
                interfaceDiscovery: boolValue(capture?["interface_discovery"]),
                requiresAdminForLiveCapture: boolValue(capture?["requires_admin_for_live_capture"])
            ),
            filters: EngineFilterSupport(
                packetFilters: boolValue(filters?["packet_filters"]),
                streamFilters: boolValue(filters?["stream_filters"]),
                transactionFilters: boolValue(filters?["transaction_filters"]),
                shorthandProtocolTerms: boolValue(filters?["shorthand_protocol_terms"]),
                shorthandPortTerms: boolValue(filters?["shorthand_port_terms"]),
                caseInsensitiveProtocols: boolValue(filters?["case_insensitive_protocols"]),
                alternateAndOperators: arrayValue(filters?["alternate_and_operators"]).compactMap { $0 as? String }
            ),
            export: EngineExportSupport(
                saveCapture: boolValue(export?["save_capture"]),
                filteredSave: boolValue(export?["filtered_save"]),
                wholeCaptureSave: boolValue(export?["whole_capture_save"])
            ),
            supportedProtocols: arrayValue(dissectors?["protocols"]).compactMap { $0 as? String }
        )
    }

    private static func arrayValue(_ value: Any?) -> [Any] {
        value as? [Any] ?? []
    }

    private static func stringValue(_ value: Any?) -> String {
        if let string = value as? String {
            return string
        }
        if let number = value as? NSNumber {
            return number.stringValue
        }
        return ""
    }

    private static func boolValue(_ value: Any?) -> Bool {
        if let bool = value as? Bool {
            return bool
        }
        if let number = value as? NSNumber {
            return number.boolValue
        }
        if let string = value as? String {
            return NSString(string: string).boolValue
        }
        return false
    }
}

enum EngineCommand {
    case engineInfo
    case inspect(path: String)
    case list(path: String, limit: String, filter: String?)
    case showPacket(path: String, index: Int)
    case stats(path: String, filter: String?)
    case conversations(path: String, filter: String?)
    case streams(path: String, filter: String?)
    case transactions(path: String, filter: String?)
    case save(sourcePath: String, outputPath: String, filter: String?)

    var args: [String] {
        switch self {
        case .engineInfo:
            return ["engine-info"]
        case let .inspect(path):
            return ["inspect", path]
        case let .list(path, limit, filter):
            return ["list", path, limit] + Self.filterArgs(filter)
        case let .showPacket(path, index):
            return ["show-packet", path, String(index)]
        case let .stats(path, filter):
            return ["stats", path] + Self.filterArgs(filter)
        case let .conversations(path, filter):
            return ["conversations", path] + Self.filterArgs(filter)
        case let .streams(path, filter):
            return ["streams", path] + Self.filterArgs(filter)
        case let .transactions(path, filter):
            return ["transactions", path] + Self.filterArgs(filter)
        case let .save(sourcePath, outputPath, filter):
            return ["save", sourcePath, outputPath] + Self.filterArgs(filter)
        }
    }

    private static func filterArgs(_ filter: String?) -> [String] {
        guard let filter, !filter.isEmpty else { return [] }
        return ["--filter", filter]
    }
}

struct PacketRow: Identifiable, Sendable {
    let id: Int
    let index: Int
    let timestamp: String
    let source: String
    let destination: String
    let protocolName: String
    let info: String
}

struct NamedCountRow: Identifiable {
    let id = UUID()
    let bucket: String
    let name: String
    let count: Int
}

struct ConversationRow: Identifiable {
    let id = UUID()
    let service: String
    let protocolName: String
    let endpointA: String
    let endpointB: String
    let packets: Int
}

struct StreamRow: Identifiable {
    let id = UUID()
    let service: String
    let protocolName: String
    let client: String
    let server: String
    let state: String
    let packets: Int
}

struct TransactionRow: Identifiable {
    let id = UUID()
    let service: String
    let protocolName: String
    let client: String
    let server: String
    let requestSummary: String
    let responseSummary: String
    let state: String
}

struct ByteRangeMetadata: Identifiable, Hashable {
    let start: Int
    let end: Int

    var id: String { "\(start)-\(end)" }
    var count: Int { max(0, end - start) }
}

struct PacketFieldNode: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let value: String
    let byteRange: ByteRangeMetadata?
    let children: [PacketFieldNode]
}

struct PacketFieldListItem: Identifiable, Hashable {
    let node: PacketFieldNode
    let depth: Int

    var id: UUID { node.id }
}

struct PacketInspectorState: Identifiable {
    let id = UUID()
    let packetNumber: Int
    let timestamp: String
    let capturedLength: Int
    let originalLength: Int
    let linkSummary: String
    let networkSummary: String
    let transportSummary: String
    let applicationSummary: String
    let fields: [PacketFieldNode]
    let flatFields: [PacketFieldListItem]
    let rawBytes: [UInt8]
}

enum CliBridgeError: LocalizedError {
    case commandFailed(String)
    case invalidJSON(String)

    var errorDescription: String? {
        switch self {
        case let .commandFailed(message):
            return message
        case let .invalidJSON(message):
            return message
        }
    }
}

private final class PipeDataBox: @unchecked Sendable {
    var data = Data()
}

struct CliBridge {
    static func makeProcess(repoRoot: URL, args: [String]) -> Process {
        let process = Process()
        process.currentDirectoryURL = repoRoot

        let environment = ProcessInfo.processInfo.environment
        let fileManager = FileManager.default
        process.environment = mergedEnvironment(from: environment)

        if let tsharkRuntime = resolveBundledTShark(fileManager: fileManager) ?? resolveSystemTShark(fileManager: fileManager) {
            process.environment?["ICESNIFF_TSHARK_BIN"] = tsharkRuntime.path
        }

        if let explicitCLI = environment["ICESNIFF_CLI_BIN"], fileManager.fileExists(atPath: explicitCLI) {
            process.executableURL = URL(fileURLWithPath: explicitCLI)
            process.arguments = args
            return process
        }

        if let localCLI = resolveLocalCLI(repoRoot: repoRoot, fileManager: fileManager) {
            process.executableURL = localCLI
            process.arguments = args
            return process
        }

        if let bundledCLI = resolveBundledCLI(fileManager: fileManager) {
            process.executableURL = bundledCLI
            process.arguments = args
            return process
        }

        if let cargoExecutable = resolveCargoExecutable(environment: environment, fileManager: fileManager) {
            process.executableURL = cargoExecutable
            process.arguments = ["run", "-q", "-p", "icesniff-cli", "--"] + args
            return process
        }

        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["cargo", "run", "-q", "-p", "icesniff-cli", "--"] + args

        return process
    }

    private static func mergedEnvironment(from environment: [String: String]) -> [String: String] {
        var merged = environment
        let existingPATH = environment["PATH"] ?? ""
        let preferredPaths = [
            "/Users/yoavperetz/.cargo/bin",
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin"
        ]
        let additions = preferredPaths.filter { !existingPATH.split(separator: ":").contains(Substring($0)) }
        if additions.isEmpty {
            merged["PATH"] = existingPATH
        } else if existingPATH.isEmpty {
            merged["PATH"] = additions.joined(separator: ":")
        } else {
            merged["PATH"] = ([existingPATH] + additions).joined(separator: ":")
        }
        return merged
    }

    private static func resolveLocalCLI(repoRoot: URL, fileManager: FileManager) -> URL? {
        let relativeCandidates = [
            "target/debug/icesniff-cli",
            "target/release/icesniff-cli"
        ]
        let environment = ProcessInfo.processInfo.environment

        var probe = repoRoot
        for _ in 0..<3 {
            for relativePath in relativeCandidates {
                let candidate = probe.appendingPathComponent(relativePath)
                if fileManager.fileExists(atPath: candidate.path) {
                    return candidate
                }
            }

            let parent = probe.deletingLastPathComponent()
            if parent.path == probe.path {
                break
            }
            probe = parent
        }

        let tempTargetRoots: [URL]
        if let explicitTarget = environment["ICESNIFF_CARGO_TARGET_DIR"], !explicitTarget.isEmpty {
            tempTargetRoots = [URL(fileURLWithPath: explicitTarget, isDirectory: true)]
        } else {
            tempTargetRoots = [
                URL(fileURLWithPath: "/tmp/icesniff-ipv6-target", isDirectory: true),
                URL(fileURLWithPath: "/tmp/icesniff-macos-release-target", isDirectory: true),
                FileManager.default.temporaryDirectory.appendingPathComponent("icesniff-ipv6-target", isDirectory: true),
                FileManager.default.temporaryDirectory.appendingPathComponent("icesniff-macos-release-target", isDirectory: true)
            ]
        }

        for targetRoot in tempTargetRoots {
            let debugCLI = targetRoot.appendingPathComponent("debug/icesniff-cli")
            if fileManager.isExecutableFile(atPath: debugCLI.path) {
                return debugCLI
            }

            let releaseCLI = targetRoot.appendingPathComponent("release/icesniff-cli")
            if fileManager.isExecutableFile(atPath: releaseCLI.path) {
                return releaseCLI
            }
        }

        return nil
    }

    private static func resolveBundledCLI(fileManager: FileManager) -> URL? {
        let candidateURLs: [URL?] = [
            Bundle.main.url(forResource: "icesniff-cli", withExtension: nil, subdirectory: "BundledCLI"),
            Bundle.module.url(forResource: "icesniff-cli", withExtension: nil, subdirectory: "BundledCLI"),
            Bundle.main.resourceURL?.appendingPathComponent("BundledCLI/icesniff-cli"),
            Bundle.module.resourceURL?.appendingPathComponent("BundledCLI/icesniff-cli")
        ]

        for candidate in candidateURLs.compactMap({ $0 }) where fileManager.isExecutableFile(atPath: candidate.path) {
            return candidate
        }

        return nil
    }

    private static func resolveBundledTShark(fileManager: FileManager) -> URL? {
        let candidateURLs: [URL?] = [
            Bundle.main.resourceURL?.appendingPathComponent("BundledTShark/Wireshark.app/Contents/MacOS/tshark"),
            Bundle.module.resourceURL?.appendingPathComponent("BundledTShark/Wireshark.app/Contents/MacOS/tshark")
        ]

        return candidateURLs
            .compactMap { $0 }
            .first(where: { fileManager.isExecutableFile(atPath: $0.path) })
    }

    private static func resolveSystemTShark(fileManager: FileManager) -> URL? {
        let environment = ProcessInfo.processInfo.environment
        if let explicit = environment["ICESNIFF_TSHARK_BIN"], fileManager.isExecutableFile(atPath: explicit) {
            return URL(fileURLWithPath: explicit)
        }

        let candidates = [
            "/Applications/Wireshark.app/Contents/MacOS/tshark",
            "/opt/homebrew/bin/tshark",
            "/usr/local/bin/tshark",
            "/usr/bin/tshark"
        ].map(URL.init(fileURLWithPath:))

        return candidates.first(where: { fileManager.isExecutableFile(atPath: $0.path) })
    }

    private static func resolveCargoExecutable(environment: [String: String], fileManager: FileManager) -> URL? {
        if let cargoHome = environment["CARGO_HOME"] {
            let candidate = URL(fileURLWithPath: cargoHome, isDirectory: true).appendingPathComponent("bin/cargo")
            if fileManager.fileExists(atPath: candidate.path) {
                return candidate
            }
        }

        let homeDirectory = FileManager.default.homeDirectoryForCurrentUser
        let candidates = [
            homeDirectory.appendingPathComponent(".cargo/bin/cargo"),
            URL(fileURLWithPath: "/opt/homebrew/bin/cargo"),
            URL(fileURLWithPath: "/usr/local/bin/cargo")
        ]

        return candidates.first { fileManager.fileExists(atPath: $0.path) }
    }

    static func runJSONData(repoRoot: URL, args: [String]) throws -> Data {
        let process = makeProcess(repoRoot: repoRoot, args: ["--json"] + args)
        let stdout = Pipe()
        let stderr = Pipe()

        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        let stdoutHandle = stdout.fileHandleForReading
        let stderrHandle = stderr.fileHandleForReading
        let stdoutQueue = DispatchQueue(label: "icesniff.cli.stdout")
        let stderrQueue = DispatchQueue(label: "icesniff.cli.stderr")
        let stdoutGroup = DispatchGroup()
        let stderrGroup = DispatchGroup()
        let stdoutBox = PipeDataBox()
        let stderrBox = PipeDataBox()

        stdoutGroup.enter()
        stdoutQueue.async {
            stdoutBox.data = stdoutHandle.readDataToEndOfFile()
            stdoutGroup.leave()
        }

        stderrGroup.enter()
        stderrQueue.async {
            stderrBox.data = stderrHandle.readDataToEndOfFile()
            stderrGroup.leave()
        }

        process.waitUntilExit()
        stdoutGroup.wait()
        stderrGroup.wait()

        let stdoutText = String(data: stdoutBox.data, encoding: .utf8) ?? ""
        let stderrText = String(data: stderrBox.data, encoding: .utf8) ?? ""

        guard process.terminationStatus == 0 else {
            let message = stderrText.isEmpty ? stdoutText : stderrText
            throw CliBridgeError.commandFailed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }

        guard let jsonData = extractJSONObjectData(from: stdoutText) else {
            throw CliBridgeError.invalidJSON("CLI returned non-JSON output: \(stdoutText)")
        }

        return jsonData
    }

    static func runText(repoRoot: URL, args: [String]) throws -> String {
        let process = makeProcess(repoRoot: repoRoot, args: args)
        let stdout = Pipe()
        let stderr = Pipe()

        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        let stdoutHandle = stdout.fileHandleForReading
        let stderrHandle = stderr.fileHandleForReading
        let stdoutQueue = DispatchQueue(label: "icesniff.cli.text.stdout")
        let stderrQueue = DispatchQueue(label: "icesniff.cli.text.stderr")
        let stdoutGroup = DispatchGroup()
        let stderrGroup = DispatchGroup()
        let stdoutBox = PipeDataBox()
        let stderrBox = PipeDataBox()

        stdoutGroup.enter()
        stdoutQueue.async {
            stdoutBox.data = stdoutHandle.readDataToEndOfFile()
            stdoutGroup.leave()
        }

        stderrGroup.enter()
        stderrQueue.async {
            stderrBox.data = stderrHandle.readDataToEndOfFile()
            stderrGroup.leave()
        }

        process.waitUntilExit()
        stdoutGroup.wait()
        stderrGroup.wait()

        let stdoutText = String(data: stdoutBox.data, encoding: .utf8) ?? ""
        let stderrText = String(data: stderrBox.data, encoding: .utf8) ?? ""

        guard process.terminationStatus == 0 else {
            let message = stderrText.isEmpty ? stdoutText : stderrText
            throw CliBridgeError.commandFailed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }

        return stdoutText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func extractJSONObjectData(from output: String) -> Data? {
        guard let start = output.firstIndex(of: "{"), let end = output.lastIndex(of: "}") else {
            return nil
        }
        let jsonSubstring = output[start ... end]
        return String(jsonSubstring).data(using: .utf8)
    }
}

struct LiveCaptureBridge {
    enum BackendKind: String {
        case iceSniffHelper

        var displayName: String {
            switch self {
            case .iceSniffHelper:
                return "IceSniff Capture"
            }
        }
    }

    struct Runtime {
        let backendKind: BackendKind
        let executableURL: URL
        let interfaceListArguments: [String]
        let startArguments: (_ interface: String, _ outputPath: String, _ stopFile: String?) -> [String]
        let prefersPrivilegedLaunch: Bool
        let dropsPrivilegesAfterLaunch: Bool
        let environmentOverrides: [String: String]
    }

    enum Resolution {
        case available(Runtime)
        case unavailable(String)
    }

    static func resolveRuntime() -> Resolution {
        let fileManager = FileManager.default
        let environment = ProcessInfo.processInfo.environment

        let candidates = explicitHelperURL(from: environment)
            .map { [$0] } ?? bundledAndLocalHelperCandidates()

        for candidate in candidates where fileManager.isExecutableFile(atPath: candidate.path) {
            return .available(Runtime(
                backendKind: .iceSniffHelper,
                executableURL: candidate,
                interfaceListArguments: ["list-interfaces"],
                startArguments: { interface, outputPath, stopFile in
                    var arguments = ["start", "--interface", interface, "--output", outputPath]
                    if let stopFile {
                        arguments.append(contentsOf: ["--stop-file", stopFile])
                    }
                    return arguments
                },
                prefersPrivilegedLaunch: false,
                dropsPrivilegesAfterLaunch: false,
                environmentOverrides: [:]
            ))
        }

        return .unavailable("Live capture helper unavailable. Build or bundle icesniff-capture-helper to enable libpcap-based live capture.")
    }

    static func tempCapturePath() -> String {
        let nanos = UInt64(Date().timeIntervalSince1970 * 1_000_000_000)
        return FileManager.default.temporaryDirectory
            .appendingPathComponent("icesniff-live-\(nanos).pcap")
            .path
    }

    static func parseInterfaceLines(_ output: String) -> [String] {
        output
            .split(separator: "\n")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private static func explicitHelperURL(from environment: [String: String]) -> URL? {
        guard let explicit = environment["ICESNIFF_CAPTURE_HELPER_BIN"], !explicit.isEmpty else {
            return nil
        }
        return URL(fileURLWithPath: explicit)
    }

    private static func bundledAndLocalHelperCandidates() -> [URL] {
        var candidates: [URL] = []
        let environment = ProcessInfo.processInfo.environment

        if let explicitWorkspace = environment["ICESNIFF_RUST_WORKSPACE_ROOT"], !explicitWorkspace.isEmpty {
            candidates.append(contentsOf: helperCandidates(workspaceRoot: URL(fileURLWithPath: explicitWorkspace, isDirectory: true)))
        }

        for root in inferredWorkspaceRoots() {
            candidates.append(contentsOf: helperCandidates(workspaceRoot: root))
        }

        let tempTargetRoots: [URL]
        if let explicitTarget = environment["ICESNIFF_CARGO_TARGET_DIR"], !explicitTarget.isEmpty {
            tempTargetRoots = [URL(fileURLWithPath: explicitTarget, isDirectory: true)]
        } else {
            tempTargetRoots = [
                URL(fileURLWithPath: "/tmp/icesniff-macos-release-target", isDirectory: true),
                FileManager.default.temporaryDirectory.appendingPathComponent("icesniff-macos-release-target", isDirectory: true)
            ]
        }
        for targetRoot in tempTargetRoots {
            candidates.append(targetRoot.appendingPathComponent("debug/icesniff-capture-helper"))
            candidates.append(targetRoot.appendingPathComponent("release/icesniff-capture-helper"))
        }

        let bundledCandidates: [URL?] = [
            Bundle.main.url(forResource: "icesniff-capture-helper", withExtension: nil, subdirectory: "BundledCLI"),
            Bundle.module.url(forResource: "icesniff-capture-helper", withExtension: nil, subdirectory: "BundledCLI"),
            Bundle.main.resourceURL?.appendingPathComponent("BundledCLI/icesniff-capture-helper"),
            Bundle.module.resourceURL?.appendingPathComponent("BundledCLI/icesniff-capture-helper")
        ]
        candidates.append(contentsOf: bundledCandidates.compactMap { $0 })

        return candidates
    }

    private static func helperCandidates(workspaceRoot: URL) -> [URL] {
        let environment = ProcessInfo.processInfo.environment
        let targetRoot: URL
        if let explicitTarget = environment["ICESNIFF_CARGO_TARGET_DIR"], !explicitTarget.isEmpty {
            targetRoot = URL(fileURLWithPath: explicitTarget, isDirectory: true)
        } else {
            targetRoot = workspaceRoot.appendingPathComponent("target", isDirectory: true)
        }

        return [
            targetRoot.appendingPathComponent("debug/icesniff-capture-helper"),
            targetRoot.appendingPathComponent("release/icesniff-capture-helper")
        ]
    }

    private static func inferredWorkspaceRoots() -> [URL] {
        let fileManager = FileManager.default
        let sourceFileDirectory = URL(fileURLWithPath: #filePath, isDirectory: false).deletingLastPathComponent()
        let roots = [
            URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true),
            sourceFileDirectory,
            Bundle.main.bundleURL,
            Bundle.module.bundleURL
        ]

        return roots.compactMap { candidate in
            findWorkspaceRoot(startingAt: candidate, fileManager: fileManager)
        }
    }

    private static func findWorkspaceRoot(startingAt candidate: URL, fileManager: FileManager) -> URL? {
        var probe = candidate

        for _ in 0..<16 {
            let cargo = probe.appendingPathComponent("Cargo.toml").path
            let helperCargo = probe.appendingPathComponent("apps/capture-helper/Cargo.toml").path
            if fileManager.fileExists(atPath: cargo), fileManager.fileExists(atPath: helperCargo) {
                return probe
            }

            let parent = probe.deletingLastPathComponent()
            if parent.path == probe.path {
                break
            }
            probe = parent
        }

        return nil
    }
}

private enum LiveCaptureHandle {
    case process(Process, Pipe)
    case privileged(pidFile: String, errorFile: String, prefersUserSpaceStop: Bool, stopFile: String?)
}

@MainActor
final class AppModel: ObservableObject {
    @Published var selectedSection: AppSection = .packets
    @Published var sidebarCollapsed = false
    @Published var appTheme: AppTheme = .defaultDark
    @Published var fontChoice: AppFontChoice = .rounded
    @Published var fontSizeStep: AppFontSizeStep = .medium
    @Published private(set) var authSession: AuthSession?
    @Published private(set) var syncStatus: SyncStatus = .idle
    @Published private(set) var profileStatusMessage = CloudProfilesFeature.unavailableMessage

    @Published var capturePath = ""
    @Published var filterExpression = ""
    @Published var packetLimit = "200"
    @Published var availableCaptureInterfaces: [String] = ["en0", "lo0", "bridge0", "utun0"]
    @Published var selectedCaptureInterface = "en0"
    @Published var isSniffing = false
    @Published private(set) var isCaptureTransitioning = false
    @Published private(set) var captureBackendName = "Unavailable"
    @Published private(set) var captureBackendMessage = "Live capture backend unavailable."

    @Published var statusMessage = "Choose a capture file to begin."
    @Published var isBusy = false

    @Published var schemaVersion = ""
    @Published var captureFormat = ""
    @Published var packetCountHint = 0

    @Published var packetsShown = 0
    @Published var totalPackets = 0
    @Published var packets: [PacketRow] = []

    @Published var statsRows: [NamedCountRow] = []
    @Published var conversations: [ConversationRow] = []
    @Published var streams: [StreamRow] = []
    @Published var transactions: [TransactionRow] = []

    @Published var selectedPacketIndex: Int?
    @Published var selectedPacketJSON = "Select a packet to inspect details."
    @Published var packetInspectorState: PacketInspectorState?
    @Published private(set) var engineCapabilities = EngineCapabilities.fallback

    let repoRoot: URL
    private let preferencesStore: PreferencesStore
    private let authService: AuthService
    private let profileSyncService: ProfileSyncService
    private let cloudProfilesConfigured: Bool
    private let cloudProfilesDiagnosticMessage: String
    private var liveCaptureHandle: LiveCaptureHandle?
    private var liveCapturePollTask: Task<Void, Never>?

    var darkMode: Bool {
        appTheme.isDark
    }

    var fontScale: CGFloat {
        fontSizeStep.scale
    }

    var cloudProfilesAvailable: Bool {
        cloudProfilesConfigured
    }

    var selectedPacketContextForAI: String? {
        guard let selectedPacketIndex,
              let packet = packets.first(where: { $0.index == selectedPacketIndex }) else {
            return nil
        }

        let normalizedJSON = selectedPacketJSON.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalizedJSON.isEmpty,
              normalizedJSON != "Select a packet to inspect details.",
              normalizedJSON != "No packets in current selection.",
              !normalizedJSON.hasPrefix("Request failed:") else {
            return nil
        }

        return """
        The user currently has packet #\(packet.index) selected in IceSniff.

        Packet summary:
        - Timestamp: \(packet.timestamp)
        - Source: \(packet.source)
        - Destination: \(packet.destination)
        - Protocol: \(packet.protocolName)
        - Info: \(packet.info)

        Full selected packet JSON:
        \(normalizedJSON)
        """
    }

    init(
        preferencesStore: PreferencesStore = PreferencesStore(),
        authService: AuthService? = nil,
        profileSyncService: ProfileSyncService? = nil
    ) {
        repoRoot = Self.resolveRepoRoot()
        self.preferencesStore = preferencesStore
        let sessionStore = KeychainSessionStore()
        cloudProfilesDiagnosticMessage = SupabaseConfiguration.diagnosticMessage()
        if let configuration = SupabaseConfiguration() {
            self.authService = authService ?? SupabaseAuthService(configuration: configuration, sessionStore: sessionStore)
            cloudProfilesConfigured = true
        } else {
            self.authService = authService ?? DisabledAuthService(diagnosticMessage: cloudProfilesDiagnosticMessage)
            cloudProfilesConfigured = false
        }
        self.profileSyncService = profileSyncService ?? DisabledProfileSyncService()
        applyPreferences(preferencesStore.load())
        authSession = self.authService.currentSession
        profileStatusMessage = Self.initialProfileStatusMessage(
            authSession: authSession,
            cloudProfilesConfigured: cloudProfilesConfigured,
            cloudProfilesDiagnosticMessage: cloudProfilesDiagnosticMessage
        )

        Task {
            await loadEngineCapabilities()
            await loadCaptureInterfaces()
        }
    }

    func toggleSidebar() {
        sidebarCollapsed.toggle()
    }

    func setTheme(_ theme: AppTheme) {
        appTheme = theme
        persistPreferences()
    }

    func setFontChoice(_ choice: AppFontChoice) {
        fontChoice = choice
        persistPreferences()
    }

    func increaseFontSize() {
        switch fontSizeStep {
        case .extraSmall:
            setFontSizeStep(.small)
        case .small:
            setFontSizeStep(.medium)
        case .medium:
            setFontSizeStep(.large)
        case .large:
            setFontSizeStep(.extraLarge)
        case .extraLarge:
            break
        }
    }

    func decreaseFontSize() {
        switch fontSizeStep {
        case .extraSmall:
            break
        case .small:
            setFontSizeStep(.extraSmall)
        case .medium:
            setFontSizeStep(.small)
        case .large:
            setFontSizeStep(.medium)
        case .extraLarge:
            setFontSizeStep(.large)
        }
    }

    private func setFontSizeStep(_ step: AppFontSizeStep) {
        fontSizeStep = step
        persistPreferences()
    }

    private func applyPreferences(_ preferences: UserPreferences) {
        appTheme = preferences.theme
        fontChoice = preferences.fontChoice
        fontSizeStep = preferences.fontSizeStep
    }

    private func persistPreferences() {
        preferencesStore.save(currentPreferences())
    }

    private func persistPreferences(_ preferences: UserPreferences) {
        preferencesStore.save(preferences)
    }

    private func currentPreferences(updatedAt: Date = .now) -> UserPreferences {
        UserPreferences(
            theme: appTheme,
            fontChoice: fontChoice,
            fontSizeStep: fontSizeStep,
            updatedAt: updatedAt
        )
    }

    func signIn(with provider: AuthProvider) {
        profileStatusMessage = "Signing in with \(provider.title)..."

        Task {
            do {
                let session = try await authService.signIn(with: provider)
                await MainActor.run {
                    authSession = session
                    syncStatus = .idle
                    profileStatusMessage = "Signed in as \(session.displayName ?? session.email ?? session.userID). Preferences stay local on this Mac."
                }
            } catch {
                await MainActor.run {
                    syncStatus = .failed(error.localizedDescription)
                    profileStatusMessage = error.localizedDescription
                }
            }
        }
    }

    func signOutProfile() {
        Task {
            do {
                try await authService.signOut()
                await MainActor.run {
                    authSession = nil
                    syncStatus = .idle
                    profileStatusMessage = "Signed out. Local preferences remain on this Mac."
                }
            } catch {
                await MainActor.run {
                    syncStatus = .failed(error.localizedDescription)
                    profileStatusMessage = error.localizedDescription
                }
            }
        }
    }

    func syncProfileNow() {
        profileStatusMessage = CloudProfilesFeature.unavailableMessage
    }

    private func syncProfileFromCloud() async {
        await MainActor.run {
            syncStatus = .idle
            profileStatusMessage = CloudProfilesFeature.unavailableMessage
        }
    }

    private static func initialProfileStatusMessage(
        authSession: AuthSession?,
        cloudProfilesConfigured: Bool,
        cloudProfilesDiagnosticMessage: String
    ) -> String {
        if let authSession {
            return "Signed in as \(authSession.displayName ?? authSession.email ?? authSession.userID). Preferences stay local on this Mac."
        }

        if cloudProfilesConfigured {
            return "Sign in with Google or GitHub. Preferences stay local on this Mac."
        }

        return cloudProfilesDiagnosticMessage
    }

    func setCapturePath(_ path: String) {
        capturePath = path
    }

    func setStatus(message: String) {
        statusMessage = message
    }

    var hasActiveFilter: Bool {
        normalizedFilterExpression() != nil
    }

    func saveCapture(to outputPath: String, scope: CaptureSaveScope = .filteredOnly) {
        guard !capturePath.isEmpty else {
            statusMessage = "No capture is open to save."
            return
        }

        isBusy = true
        statusMessage = "Saving capture..."

        let sourcePath = capturePath
        let filter = FilterExpressionNormalizer.saveFilter(for: scope, expression: filterExpression)
        let root = repoRoot

        Task {
            do {
                try await Task.detached(priority: .userInitiated) {
                    _ = try CliBridge.runText(
                        repoRoot: root,
                        args: EngineCommand.save(sourcePath: sourcePath, outputPath: outputPath, filter: filter).args
                    )
                }.value
                statusMessage = "Saved capture to \(outputPath)."
            } catch {
                statusMessage = "Save failed: \(error.localizedDescription)"
            }
            isBusy = false
        }
    }

    func toggleSniffing() {
        Task {
            guard !isCaptureTransitioning else { return }
            if isSniffing {
                await stopSniffing()
            } else {
                await startSniffing()
            }
        }
    }

    func refreshAll() {
        guard !capturePath.isEmpty else {
            statusMessage = "Capture path is empty."
            return
        }

        isBusy = true
        statusMessage = "Loading capture data..."

        Task {
            do {
                let inspect = try await requestJSON(.inspect(path: capturePath))
                applyInspect(inspect)

                let list = try await requestJSON(listCommand())
                applyPacketList(list)

                let stats = try await requestJSON(statsCommand())
                applyStats(stats)

                let conv = try await requestJSON(conversationsCommand())
                applyConversations(conv)

                let stream = try await requestJSON(streamsCommand())
                applyStreams(stream)

                let tx = try await requestJSON(transactionsCommand())
                applyTransactions(tx)

                statusMessage = "Loaded capture successfully."
            } catch {
                if isSniffing, isTransientLiveCaptureReadError(error) {
                    statusMessage = "Waiting for packets on \(selectedCaptureInterface)..."
                } else {
                    statusMessage = "Request failed: \(error.localizedDescription)"
                }
            }
            isBusy = false
        }
    }

    func reloadPacketsOnly() {
        guard !capturePath.isEmpty else {
            statusMessage = "Capture path is empty."
            return
        }

        isBusy = true
        statusMessage = "Reloading packets..."

        Task {
            do {
                let list = try await requestJSON(listCommand())
                applyPacketList(list)
                statusMessage = "Packets refreshed."
            } catch {
                if isSniffing, isTransientLiveCaptureReadError(error) {
                    statusMessage = "Waiting for packets on \(selectedCaptureInterface)..."
                } else {
                    statusMessage = "Request failed: \(error.localizedDescription)"
                }
            }
            isBusy = false
        }
    }

    func loadPacketDetails(index: Int) {
        guard !capturePath.isEmpty else {
            return
        }

        Task {
            do {
                let response = try await requestJSON(.showPacket(path: capturePath, index: index))
                let formatted = try JSONSerialization.data(withJSONObject: response, options: [.prettyPrinted, .sortedKeys])
                selectedPacketJSON = String(data: formatted, encoding: .utf8) ?? "Failed to format packet JSON."
            } catch {
                selectedPacketJSON = "Request failed: \(error.localizedDescription)"
            }
        }
    }

    func presentPacketInspector(index: Int) {
        guard !capturePath.isEmpty else { return }

        Task {
            do {
                let response = try await requestJSON(.showPacket(path: capturePath, index: index))
                guard let inspector = parsePacketInspectorState(from: response) else {
                    statusMessage = "Failed to decode packet details for packet \(index)."
                    return
                }
                packetInspectorState = inspector
            } catch {
                statusMessage = "Packet details failed: \(error.localizedDescription)"
            }
        }
    }

    private func loadCaptureInterfaces() async {
        do {
            let interfaces = try await Task.detached(priority: .userInitiated) {
                try Self.listCaptureInterfaces()
            }.value
            if !interfaces.isEmpty {
                availableCaptureInterfaces = interfaces
                if selectedCaptureInterface.isEmpty || !interfaces.contains(selectedCaptureInterface) {
                    selectedCaptureInterface = interfaces[0]
                }
                if case let .available(runtime) = LiveCaptureBridge.resolveRuntime() {
                    captureBackendName = runtime.backendKind.displayName
                    if CapturePrivilegeSetup.isCaptureAccessReady() {
                        captureBackendMessage = "Using \(runtime.backendKind.displayName) for live capture."
                    } else if CapturePrivilegeSetup.isInstalled() {
                        captureBackendMessage = "Capture permissions installed. Log out or restart macOS to activate them."
                    } else {
                        captureBackendMessage = "One-time capture permission setup required. Start sniffing to install it."
                    }
                }
            } else {
                statusMessage = "No capture interfaces reported by the backend. Using fallback interface names."
            }
        } catch {
            if case let .unavailable(message) = LiveCaptureBridge.resolveRuntime() {
                captureBackendName = "Unavailable"
                captureBackendMessage = message
            }
            if availableCaptureInterfaces.isEmpty {
                availableCaptureInterfaces = ["en0"]
                selectedCaptureInterface = "en0"
            }
            statusMessage = "Capture interfaces unavailable: \(error.localizedDescription)"
        }
    }

    private func loadEngineCapabilities() async {
        do {
            let response = try await requestJSON(.engineInfo)
            engineCapabilities = EngineCapabilitiesParser.parse(response)
        } catch {
            engineCapabilities = .fallback
            let message = error.localizedDescription.lowercased()
            if message.contains("unknown command: engine-info") {
                return
            }
            statusMessage = "Engine capabilities unavailable: \(error.localizedDescription)"
        }
    }

    private func startSniffing() async {
        guard !isCaptureTransitioning else { return }
        guard !selectedCaptureInterface.isEmpty else {
            statusMessage = "No capture interface selected."
            return
        }
        guard liveCaptureHandle == nil else {
            statusMessage = "Live capture is already running."
            return
        }
        if case let .unavailable(message) = LiveCaptureBridge.resolveRuntime() {
            statusMessage = message
            return
        }

        if !CapturePrivilegeSetup.isCaptureAccessReady() {
            if CapturePrivilegeSetup.isInstalled() {
                statusMessage = "Capture permissions are installed but not active yet. Log out or restart macOS, then try again."
                return
            }
            do {
                statusMessage = "Installing one-time capture permissions..."
                try ensureCapturePrivilegesInstalled()
                captureBackendMessage = "Using IceSniff Capture for live capture."
            } catch {
                statusMessage = "Live capture setup failed: \(LiveCaptureErrorMapper.message(for: error))"
                return
            }
        }

        isCaptureTransitioning = true
        defer { isCaptureTransitioning = false }

        do {
            let (handle, path) = try startLiveCaptureProcess(interface: selectedCaptureInterface)
            liveCaptureHandle = handle
            capturePath = path
            packetCountHint = 0
            packetsShown = 0
            totalPackets = 0
            packets = []
            isSniffing = true
            statusMessage = "Live capture started on \(selectedCaptureInterface). Waiting for packets..."
            startLiveCapturePolling()
        } catch {
            isSniffing = false
            statusMessage = "Live capture failed: \(LiveCaptureErrorMapper.message(for: error))"
        }
    }

    private func stopSniffing() async {
        guard !isCaptureTransitioning else { return }
        liveCapturePollTask?.cancel()
        liveCapturePollTask = nil

        isCaptureTransitioning = true
        defer { isCaptureTransitioning = false }

        do {
            try stopLiveCaptureProcess()
            isSniffing = false
            statusMessage = "Live capture stopped."
            await finalizeStoppedCapture()
        } catch {
            isSniffing = false
            statusMessage = "Failed to stop live capture: \(LiveCaptureErrorMapper.message(for: error))"
        }
    }

    private func startLiveCapturePolling() {
        liveCapturePollTask?.cancel()
        liveCapturePollTask = Task { [weak self] in
            while let self, !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(100))
                guard !Task.isCancelled else { break }
                await self.reloadLivePacketList()
            }
        }
    }

    private func reloadLivePacketList() async {
        guard isSniffing, !capturePath.isEmpty else { return }
        guard captureFileExists(at: capturePath) else {
            if packets.isEmpty {
                statusMessage = "Waiting for capture file on \(selectedCaptureInterface)..."
            }
            return
        }
        guard captureFileHasReadableData(at: capturePath) else {
            if packets.isEmpty {
                statusMessage = "Waiting for packets on \(selectedCaptureInterface)..."
            }
            return
        }

        do {
            if normalizedFilterExpression() == nil {
                let inspect = try await requestJSON(.inspect(path: capturePath))
                applyInspect(inspect)
                let hintedTotal = max(packetCountHint, totalPackets)
                totalPackets = hintedTotal
                if hintedTotal > UInt64(packets.count) {
                    try await appendLivePackets(upTo: Int(hintedTotal))
                }
                packetsShown = packets.count
            } else {
                let list = try await requestJSON(listCommand())
                applyPacketList(list)
            }
            if !packets.isEmpty {
                statusMessage = "Live capture running on \(selectedCaptureInterface)."
            }
        } catch {
            if isTransientLiveCaptureReadError(error) || packets.isEmpty {
                statusMessage = "Waiting for packets on \(selectedCaptureInterface)..."
            }
        }
    }

    private func appendLivePackets(upTo totalCount: Int) async throws {
        guard totalCount > packets.count else { return }

        let startIndex = packets.count
        let missingIndices = Array(startIndex..<totalCount)
        let batchSize = 24
        var appendedRows: [PacketRow] = []
        appendedRows.reserveCapacity(missingIndices.count)

        for batchStart in stride(from: 0, to: missingIndices.count, by: batchSize) {
            let batchEnd = min(batchStart + batchSize, missingIndices.count)
            let batch = Array(missingIndices[batchStart..<batchEnd])

            let rows = try await withThrowingTaskGroup(of: PacketRow?.self) { group in
                for index in batch {
                    group.addTask { [capturePath, repoRoot] in
                        try Self.fetchLivePacketRow(repoRoot: repoRoot, capturePath: capturePath, index: index)
                    }
                }

                var fetchedRows: [PacketRow] = []
                for try await row in group {
                    if let row {
                        fetchedRows.append(row)
                    }
                }
                return fetchedRows.sorted { $0.index < $1.index }
            }

            appendedRows.append(contentsOf: rows)
        }

        guard !appendedRows.isEmpty else { return }
        packets.append(contentsOf: appendedRows)
    }

    private func finalizeStoppedCapture() async {
        guard !capturePath.isEmpty else { return }

        for attempt in 0..<30 {
            do {
                let inspect = try await requestJSON(.inspect(path: capturePath))
                applyInspect(inspect)

                let list = try await requestJSON(listCommand())
                applyPacketList(list)
                statusMessage = "Live capture stopped."
                return
            } catch {
                if isTransientLiveCaptureReadError(error), attempt < 29 {
                    statusMessage = "Finalizing capture..."
                    try? await Task.sleep(for: .milliseconds(200))
                    continue
                }
                statusMessage = "Request failed: \(error.localizedDescription)"
                return
            }
        }
    }

    private func listCommand() -> EngineCommand {
        .list(path: capturePath, limit: packetListLimitString(), filter: normalizedFilterExpression())
    }

    private func statsCommand() -> EngineCommand {
        .stats(path: capturePath, filter: normalizedFilterExpression())
    }

    private func conversationsCommand() -> EngineCommand {
        .conversations(path: capturePath, filter: normalizedFilterExpression())
    }

    private func streamsCommand() -> EngineCommand {
        .streams(path: capturePath, filter: normalizedFilterExpression())
    }

    private func transactionsCommand() -> EngineCommand {
        .transactions(path: capturePath, filter: normalizedFilterExpression())
    }

    private func normalizedFilterExpression() -> String? {
        FilterExpressionNormalizer.normalize(filterExpression)
    }

    private func packetListLimitString() -> String {
        if isSniffing {
            return "1000000"
        }

        let hardInitialLimit = 20000
        if packetCountHint > 0 {
            return String(min(packetCountHint, hardInitialLimit))
        }

        return String(hardInitialLimit)
    }

    private func captureFileExists(at path: String) -> Bool {
        FileManager.default.fileExists(atPath: path)
    }

    private func captureFileHasReadableData(at path: String) -> Bool {
        guard
            let attributes = try? FileManager.default.attributesOfItem(atPath: path),
            let fileSize = attributes[.size] as? NSNumber
        else {
            return false
        }

        return fileSize.intValue >= 24
    }

    private func isTransientLiveCaptureReadError(_ error: Error) -> Bool {
        let message = ((error as? LocalizedError)?.errorDescription ?? error.localizedDescription).lowercased()
        return message.contains("cannot read an unknown capture container")
            || message.contains("failed to read capture file")
            || message.contains("truncated")
    }

    private func preparePrivilegedCaptureFile(at path: String) throws {
        let fileManager = FileManager.default
        if !fileManager.fileExists(atPath: path) {
            guard fileManager.createFile(atPath: path, contents: Data()) else {
                throw CliBridgeError.commandFailed("Failed to prepare live capture file.")
            }
        }

        try fileManager.setAttributes([.posixPermissions: 0o644], ofItemAtPath: path)
    }

    private static func cleanupCaptureArtifacts(pidFile: String, errorFile: String) {
        try? FileManager.default.removeItem(atPath: pidFile)
        try? FileManager.default.removeItem(atPath: errorFile)
    }

    private static func prepareRestrictedTemporaryFile(at path: String) throws {
        let fileManager = FileManager.default
        if !fileManager.fileExists(atPath: path) {
            guard fileManager.createFile(atPath: path, contents: Data()) else {
                throw CliBridgeError.commandFailed("Failed to prepare secure temporary runtime file.")
            }
        }

        try fileManager.setAttributes([.posixPermissions: 0o600], ofItemAtPath: path)
    }

    private static func readTrimmedFile(at path: String) -> String? {
        guard FileManager.default.fileExists(atPath: path) else { return nil }
        return try? String(contentsOfFile: path, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func sanitizedRuntimeError(stdout: String, stderr: String) -> CliBridgeError {
        let message = (stderr.isEmpty ? stdout : stderr).trimmingCharacters(in: .whitespacesAndNewlines)
        return CliBridgeError.commandFailed(LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed(message)))
    }

    nonisolated private static func mergedCaptureEnvironment(overrides: [String: String]) -> [String: String] {
        var environment = ProcessInfo.processInfo.environment
        for (key, value) in overrides {
            environment[key] = value
        }
        return environment
    }

    nonisolated private static func listCaptureInterfaces() throws -> [String] {
        let runtime: LiveCaptureBridge.Runtime
        switch LiveCaptureBridge.resolveRuntime() {
        case let .available(resolvedRuntime):
            runtime = resolvedRuntime
        case let .unavailable(message):
            throw CliBridgeError.commandFailed(message)
        }
        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = runtime.executableURL
        process.arguments = runtime.interfaceListArguments
        process.environment = mergedCaptureEnvironment(overrides: runtime.environmentOverrides)
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        let outputData = stdout.fileHandleForReading.readDataToEndOfFile()
        let errorData = stderr.fileHandleForReading.readDataToEndOfFile()
        let outputText = String(data: outputData, encoding: .utf8) ?? ""
        let errorText = String(data: errorData, encoding: .utf8) ?? ""

        guard process.terminationStatus == 0 else {
            let message = errorText.isEmpty ? outputText : errorText
            throw CliBridgeError.commandFailed(message.trimmingCharacters(in: .whitespacesAndNewlines))
        }

        return LiveCaptureBridge.parseInterfaceLines(outputText)
    }

    private func startLiveCaptureProcess(interface: String) throws -> (LiveCaptureHandle, String) {
        let runtime: LiveCaptureBridge.Runtime
        switch LiveCaptureBridge.resolveRuntime() {
        case let .available(resolvedRuntime):
            runtime = resolvedRuntime
        case let .unavailable(message):
            throw CliBridgeError.commandFailed(message)
        }
        let path = LiveCaptureBridge.tempCapturePath()
        if runtime.prefersPrivilegedLaunch {
            return try startPrivilegedLiveCapture(runtime: runtime, interface: interface, outputPath: path)
        }

        let process = Process()
        let stderr = Pipe()
        process.executableURL = runtime.executableURL
        process.arguments = runtime.startArguments(interface, path, nil)
        process.environment = Self.mergedCaptureEnvironment(overrides: runtime.environmentOverrides)
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = FileHandle.nullDevice
        process.standardError = stderr

        try process.run()
        Thread.sleep(forTimeInterval: 0.12)

        if !process.isRunning {
            process.waitUntilExit()
            let errorData = stderr.fileHandleForReading.readDataToEndOfFile()
            let errorText = String(data: errorData, encoding: .utf8) ?? ""
            throw Self.sanitizedRuntimeError(stdout: "", stderr: errorText)
        }

        return (.process(process, stderr), path)
    }

    private func stopLiveCaptureProcess() throws {
        guard let handle = liveCaptureHandle else { return }

        switch handle {
        case let .process(process, stderrPipe):
            if process.isRunning {
                process.terminate()
                process.waitUntilExit()
            }
            _ = stderrPipe.fileHandleForReading.readDataToEndOfFile()

        case let .privileged(pidFile, errorFile, prefersUserSpaceStop, stopFile):
            if let stopFile {
                FileManager.default.createFile(atPath: stopFile, contents: Data())
                try? awaitHelperShutdown(pidFile: pidFile)
                try? FileManager.default.removeItem(atPath: stopFile)
                Self.cleanupCaptureArtifacts(pidFile: pidFile, errorFile: errorFile)
                liveCaptureHandle = nil
                return
            }
            guard let pidText = Self.readTrimmedFile(at: pidFile), !pidText.isEmpty else {
                Self.cleanupCaptureArtifacts(pidFile: pidFile, errorFile: errorFile)
                liveCaptureHandle = nil
                return
            }
            if prefersUserSpaceStop, try terminateUserOwnedProcess(pidText: pidText) {
                Self.cleanupCaptureArtifacts(pidFile: pidFile, errorFile: errorFile)
                liveCaptureHandle = nil
                return
            }
            try runPrivilegedShellCommand(PrivilegedCaptureCommandBuilder.stopCommand(pidFile: pidFile, errorFile: errorFile))
            Self.cleanupCaptureArtifacts(pidFile: pidFile, errorFile: errorFile)
        }

        liveCaptureHandle = nil
    }

    private func startPrivilegedLiveCapture(runtime: LiveCaptureBridge.Runtime, interface: String, outputPath: String) throws -> (LiveCaptureHandle, String) {
        let pidFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("icesniff-live-\(UUID().uuidString).pid")
            .path
        let errorFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("icesniff-live-\(UUID().uuidString).err")
            .path
        let stopFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("icesniff-live-\(UUID().uuidString).stop")
            .path

        try preparePrivilegedCaptureFile(at: outputPath)
        try Self.prepareRestrictedTemporaryFile(at: pidFile)
        try Self.prepareRestrictedTemporaryFile(at: errorFile)
        try Self.prepareRestrictedTemporaryFile(at: stopFile)

        let arguments = runtime.startArguments(interface, outputPath, runtime.backendKind == .iceSniffHelper ? stopFile : nil)

        let launchCommand = PrivilegedCaptureCommandBuilder.launchCommand(
            executablePath: runtime.executableURL.path,
            arguments: arguments,
            pidFile: pidFile,
            errorFile: errorFile
        )

        do {
            try runPrivilegedShellCommand(launchCommand)
        } catch {
            let errorText = Self.readTrimmedFile(at: errorFile) ?? ""
            if !errorText.isEmpty {
                throw CliBridgeError.commandFailed(LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed(errorText)))
            }
            throw CliBridgeError.commandFailed(LiveCaptureErrorMapper.message(for: error))
        }

        let deadline = Date().addingTimeInterval(2.0)
        while Date() < deadline {
            if let pidText = Self.readTrimmedFile(at: pidFile),
               !pidText.isEmpty {
                return (.privileged(
                    pidFile: pidFile,
                    errorFile: errorFile,
                    prefersUserSpaceStop: runtime.dropsPrivilegesAfterLaunch,
                    stopFile: runtime.backendKind == .iceSniffHelper ? stopFile : nil
                ), outputPath)
            }
            Thread.sleep(forTimeInterval: 0.05)
        }

        let errorText = Self.readTrimmedFile(at: errorFile) ?? ""
        if !errorText.isEmpty {
            throw CliBridgeError.commandFailed(LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed(errorText)))
        }
        throw CliBridgeError.commandFailed(LiveCaptureErrorMapper.message(for: CliBridgeError.commandFailed("Privileged live capture failed to launch.")))
    }

    private func awaitHelperShutdown(pidFile: String) throws {
        let deadline = Date().addingTimeInterval(2.0)
        while Date() < deadline {
            if !FileManager.default.fileExists(atPath: pidFile) {
                return
            }
            if let pidText = Self.readTrimmedFile(at: pidFile),
               !pidText.isEmpty,
               !isProcessAlive(pidText: pidText) {
                return
            }
            Thread.sleep(forTimeInterval: 0.05)
        }
    }

    private func isProcessAlive(pidText: String) -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/kill")
        process.arguments = ["-0", pidText]
        do {
            try process.run()
            process.waitUntilExit()
            return process.terminationStatus == 0
        } catch {
            return false
        }
    }

    private func terminateUserOwnedProcess(pidText: String) throws -> Bool {
        let process = Process()
        let stderr = Pipe()
        process.executableURL = URL(fileURLWithPath: "/bin/kill")
        process.arguments = ["-TERM", pidText]
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus == 0 {
            return true
        }

        let errorData = stderr.fileHandleForReading.readDataToEndOfFile()
        let errorText = String(data: errorData, encoding: .utf8) ?? ""
        if errorText.lowercased().contains("operation not permitted") {
            return false
        }
        return false
    }

    private func parsePacketInspectorState(from response: [String: Any]) -> PacketInspectorState? {
        guard let packet = response["packet"] as? [String: Any] else {
            return nil
        }

        let index = intValue(packet["index"])
        let timestamp = "\(stringValue(packet["timestamp_seconds"])).\(stringValue(packet["timestamp_fraction"]))"
        let fields = parseFieldNodes(from: packet["fields"])
        let rawBytes = arrayValue(packet["raw_bytes"]).compactMap { value -> UInt8? in
            if let int = value as? Int, int >= 0, int <= 255 {
                return UInt8(int)
            }
            if let number = value as? NSNumber {
                let int = number.intValue
                guard int >= 0, int <= 255 else { return nil }
                return UInt8(int)
            }
            return nil
        }

        return PacketInspectorState(
            packetNumber: index,
            timestamp: timestamp,
            capturedLength: intValue(packet["captured_length"]),
            originalLength: intValue(packet["original_length"]),
            linkSummary: layerSummary(from: packet["link"]) ?? "Unknown",
            networkSummary: layerSummary(from: packet["network"]) ?? "Unknown",
            transportSummary: layerSummary(from: packet["transport"]) ?? "Unknown",
            applicationSummary: layerSummary(from: packet["application"]) ?? "Unknown",
            fields: fields,
            flatFields: flattenFieldNodes(fields),
            rawBytes: rawBytes
        )
    }

    nonisolated private static func livePacketRow(fromShowPacketResponse response: [String: Any]) -> PacketRow? {
        guard let packet = response["packet"] as? [String: Any] else {
            return nil
        }

        let index = liveIntValue(packet["index"])
        let seconds = liveStringValue(packet["timestamp_seconds"])
        let fraction = liveStringValue(packet["timestamp_fraction"])

        let source: String
        let destination: String
        if let network = packet["network"] as? [String: Any] {
            source = liveStringValue(network["source_ip"])
            destination = liveStringValue(network["destination_ip"])
        } else {
            source = ""
            destination = ""
        }

        let protocolName: String
        let info: String
        if let transport = packet["transport"] as? [String: Any] {
            let kind = liveStringValue(transport["kind"])
            let sourcePort = liveStringValue(transport["source_port"])
            let destinationPort = liveStringValue(transport["destination_port"])
            protocolName = kind.isEmpty ? "unknown" : kind
            if !sourcePort.isEmpty, !destinationPort.isEmpty {
                info = "\(sourcePort) -> \(destinationPort)"
            } else {
                info = protocolName.uppercased()
            }
        } else {
            protocolName = livePacketProtocolName(from: packet)
            info = protocolName.uppercased()
        }

        return PacketRow(
            id: index,
            index: index,
            timestamp: "\(seconds).\(fraction)",
            source: source,
            destination: destination,
            protocolName: protocolName,
            info: info
        )
    }

    nonisolated private static func livePacketProtocolName(from packet: [String: Any]) -> String {
        if let transport = packet["transport"] as? [String: Any] {
            let kind = liveStringValue(transport["kind"])
            if !kind.isEmpty {
                return kind
            }
        }

        if let application = packet["application"] as? [String: Any] {
            let kind = liveStringValue(application["kind"])
            if !kind.isEmpty {
                return kind
            }
        }

        if let network = packet["network"] as? [String: Any] {
            let kind = liveStringValue(network["kind"])
            if !kind.isEmpty {
                return kind
            }
        }

        return "unknown"
    }

    nonisolated private static func liveStringValue(_ value: Any?) -> String {
        if let string = value as? String {
            return string
        }
        if let number = value as? NSNumber {
            return number.stringValue
        }
        return ""
    }

    nonisolated private static func liveIntValue(_ value: Any?) -> Int {
        if let int = value as? Int {
            return int
        }
        if let number = value as? NSNumber {
            return number.intValue
        }
        if let string = value as? String, let int = Int(string) {
            return int
        }
        return 0
    }

    nonisolated private static func fetchLivePacketRow(repoRoot: URL, capturePath: String, index: Int) throws -> PacketRow? {
        let data = try CliBridge.runJSONData(
            repoRoot: repoRoot,
            args: EngineCommand.showPacket(path: capturePath, index: index).args
        )
        let object = try JSONSerialization.jsonObject(with: data)
        guard let response = object as? [String: Any] else {
            throw CliBridgeError.invalidJSON("CLI JSON root is not an object.")
        }
        return livePacketRow(fromShowPacketResponse: response)
    }

    private func parseFieldNodes(from value: Any?) -> [PacketFieldNode] {
        arrayValue(value).compactMap { item in
            guard let dict = item as? [String: Any] else { return nil }
            let range: ByteRangeMetadata?
            if let byteRange = dict["byte_range"] as? [String: Any] {
                range = ByteRangeMetadata(
                    start: intValue(byteRange["start"]),
                    end: intValue(byteRange["end"])
                )
            } else {
                range = nil
            }

            return PacketFieldNode(
                name: stringValue(dict["name"]),
                value: stringValue(dict["value"]),
                byteRange: range,
                children: parseFieldNodes(from: dict["children"])
            )
        }
    }

    private func flattenFieldNodes(_ nodes: [PacketFieldNode], depth: Int = 0) -> [PacketFieldListItem] {
        nodes.flatMap { node in
            [PacketFieldListItem(node: node, depth: depth)] + flattenFieldNodes(node.children, depth: depth + 1)
        }
    }

    private func layerSummary(from value: Any?) -> String? {
        guard let value else { return nil }
        if value is NSNull {
            return nil
        }
        if let dict = value as? [String: Any] {
            let parts = dict
                .sorted { $0.key < $1.key }
                .compactMap { key, rawValue -> String? in
                    if rawValue is NSNull { return nil }
                    if let nested = rawValue as? [String: Any] {
                        let nestedSummary = layerSummary(from: nested) ?? ""
                        return nestedSummary.isEmpty ? key : "\(key): \(nestedSummary)"
                    }
                    if let array = rawValue as? [Any], !array.isEmpty {
                        let joined = array.map { scalarDescription(for: $0) }.joined(separator: ", ")
                        return "\(key): \(joined)"
                    }
                    let scalar = scalarDescription(for: rawValue)
                    return scalar.isEmpty ? nil : "\(key): \(scalar)"
                }
            return parts.isEmpty ? nil : parts.joined(separator: " • ")
        }
        return scalarDescription(for: value)
    }

    private func scalarDescription(for value: Any) -> String {
        if let string = value as? String {
            return string
        }
        if let number = value as? NSNumber {
            return number.stringValue
        }
        if let bool = value as? Bool {
            return bool ? "true" : "false"
        }
        return ""
    }

    private func applyInspect(_ response: [String: Any]) {
        schemaVersion = stringValue(response["schema_version"])
        captureFormat = stringValue(response["format"])
        packetCountHint = intValue(response["packet_count_hint"])
    }

    private func applyPacketList(_ response: [String: Any]) {
        packetsShown = intValue(response["packets_shown"])
        totalPackets = intValue(response["total_packets"])

        let rows = arrayValue(response["packets"]).compactMap { item -> PacketRow? in
            guard let dict = item as? [String: Any] else { return nil }
            let index = intValue(dict["index"])
            let seconds = stringValue(dict["timestamp_seconds"])
            let fraction = stringValue(dict["timestamp_fraction"])
            return PacketRow(
                id: index,
                index: index,
                timestamp: "\(seconds).\(fraction)",
                source: stringValue(dict["source"]),
                destination: stringValue(dict["destination"]),
                protocolName: stringValue(dict["protocol"]),
                info: stringValue(dict["info"])
            )
        }
        packets = rows
        if rows.isEmpty {
            selectedPacketIndex = nil
            selectedPacketJSON = "No packets in current selection."
        }
    }

    private func applyStats(_ response: [String: Any]) {
        var rows: [NamedCountRow] = []
        for item in arrayValue(response["link_layer_counts"]) {
            if let dict = item as? [String: Any] {
                rows.append(
                    NamedCountRow(bucket: "Link", name: stringValue(dict["name"]), count: intValue(dict["count"]))
                )
            }
        }
        for item in arrayValue(response["network_layer_counts"]) {
            if let dict = item as? [String: Any] {
                rows.append(
                    NamedCountRow(bucket: "Network", name: stringValue(dict["name"]), count: intValue(dict["count"]))
                )
            }
        }
        for item in arrayValue(response["transport_layer_counts"]) {
            if let dict = item as? [String: Any] {
                rows.append(
                    NamedCountRow(bucket: "Transport", name: stringValue(dict["name"]), count: intValue(dict["count"]))
                )
            }
        }
        statsRows = rows
    }

    private func applyConversations(_ response: [String: Any]) {
        conversations = arrayValue(response["conversations"]).compactMap { item in
            guard let dict = item as? [String: Any] else { return nil }
            return ConversationRow(
                service: stringValue(dict["service"]),
                protocolName: stringValue(dict["protocol"]),
                endpointA: stringValue(dict["endpoint_a"]),
                endpointB: stringValue(dict["endpoint_b"]),
                packets: intValue(dict["packets"])
            )
        }
    }

    private func applyStreams(_ response: [String: Any]) {
        streams = arrayValue(response["streams"]).compactMap { item in
            guard let dict = item as? [String: Any] else { return nil }
            return StreamRow(
                service: stringValue(dict["service"]),
                protocolName: stringValue(dict["protocol"]),
                client: stringValue(dict["client"]),
                server: stringValue(dict["server"]),
                state: stringValue(dict["session_state"]),
                packets: intValue(dict["packets"])
            )
        }
    }

    private func applyTransactions(_ response: [String: Any]) {
        transactions = arrayValue(response["transactions"]).compactMap { item in
            guard let dict = item as? [String: Any] else { return nil }
            return TransactionRow(
                service: stringValue(dict["service"]),
                protocolName: stringValue(dict["protocol"]),
                client: stringValue(dict["client"]),
                server: stringValue(dict["server"]),
                requestSummary: stringValue(dict["request_summary"]),
                responseSummary: stringValue(dict["response_summary"]),
                state: stringValue(dict["state"])
            )
        }
    }

    private func requestJSON(_ command: EngineCommand) async throws -> [String: Any] {
        let root = repoRoot
        let jsonData = try await Task.detached(priority: .userInitiated) {
            try CliBridge.runJSONData(repoRoot: root, args: command.args)
        }.value
        let object = try JSONSerialization.jsonObject(with: jsonData)
        guard let dictionary = object as? [String: Any] else {
            throw CliBridgeError.invalidJSON("CLI JSON root is not an object.")
        }
        return dictionary
    }

    private static func resolveRepoRoot() -> URL {
        let env = ProcessInfo.processInfo.environment
        if let explicitRustWorkspace = env["ICESNIFF_RUST_WORKSPACE_ROOT"], !explicitRustWorkspace.isEmpty {
            return URL(fileURLWithPath: explicitRustWorkspace, isDirectory: true)
        }

        let fileManager = FileManager.default
        let sourceFileDirectory = URL(fileURLWithPath: #filePath, isDirectory: false).deletingLastPathComponent()
        let localMacCandidates = [
            sourceFileDirectory
                .deletingLastPathComponent()
                .deletingLastPathComponent()
                .appendingPathComponent("rust-engine", isDirectory: true),
            URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true)
                .deletingLastPathComponent()
                .appendingPathComponent("rust-engine", isDirectory: true),
            URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true)
                .deletingLastPathComponent()
                .deletingLastPathComponent()
                .appendingPathComponent("rust-engine", isDirectory: true)
        ]

        for candidate in localMacCandidates {
            let cargo = candidate.appendingPathComponent("Cargo.toml").path
            let cliCargo = candidate.appendingPathComponent("apps/cli/Cargo.toml").path
            if fileManager.fileExists(atPath: cargo), fileManager.fileExists(atPath: cliCargo) {
                return candidate
            }
        }

        let candidateRoots = [
            sourceFileDirectory,
            URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true),
            Bundle.main.bundleURL,
            Bundle.module.bundleURL
        ]

        for candidate in candidateRoots {
            if let root = findWorkspaceRoot(startingAt: candidate, fileManager: fileManager) {
                return root
            }
        }

        return URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true)
    }

    private static func findWorkspaceRoot(startingAt candidate: URL, fileManager: FileManager) -> URL? {
        var probe = candidate

        for _ in 0..<16 {
            let localMacWorkspace = probe.appendingPathComponent("rust-engine", isDirectory: true)
            let localCargo = localMacWorkspace.appendingPathComponent("Cargo.toml").path
            let localCliCargo = localMacWorkspace.appendingPathComponent("apps/cli/Cargo.toml").path
            if fileManager.fileExists(atPath: localCargo), fileManager.fileExists(atPath: localCliCargo) {
                return localMacWorkspace
            }

            let cargo = probe.appendingPathComponent("Cargo.toml").path
            let cliCargo = probe.appendingPathComponent("apps/cli/Cargo.toml").path
            if fileManager.fileExists(atPath: cargo), fileManager.fileExists(atPath: cliCargo) {
                return probe
            }

            let parent = probe.deletingLastPathComponent()
            if parent.path == probe.path {
                break
            }
            probe = parent
        }

        return nil
    }

    private func ensureCapturePrivilegesInstalled() throws {
        if CapturePrivilegeSetup.isCaptureAccessReady() {
            return
        }

        try runPrivilegedShellCommand(CapturePrivilegeSetup.installCommand())

        let deadline = Date().addingTimeInterval(2.0)
        while Date() < deadline {
            if CapturePrivilegeSetup.isCaptureAccessReady() {
                return
            }
            Thread.sleep(forTimeInterval: 0.1)
        }

        throw CliBridgeError.commandFailed("IceSniff installed capture permissions, but they are not active yet. Log out or restart macOS and try again.")
    }

    private func runPrivilegedShellCommand(_ command: String) throws {
        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        process.arguments = [
            "-e",
            #"do shell script "\#(appleScriptEscaped(command))" with administrator privileges"#
        ]
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let stderrText = String(data: stderr.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let stdoutText = String(data: stdout.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            throw Self.sanitizedRuntimeError(stdout: stdoutText, stderr: stderrText)
        }
    }

    private func appleScriptEscaped(_ value: String) -> String {
        value
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
    }

    private func arrayValue(_ value: Any?) -> [Any] {
        value as? [Any] ?? []
    }

    private func stringValue(_ value: Any?) -> String {
        if let string = value as? String {
            return string
        }
        if let number = value as? NSNumber {
            return number.stringValue
        }
        return ""
    }

    private func boolValue(_ value: Any?) -> Bool {
        if let bool = value as? Bool {
            return bool
        }
        if let number = value as? NSNumber {
            return number.boolValue
        }
        if let string = value as? String {
            return NSString(string: string).boolValue
        }
        return false
    }

    private func intValue(_ value: Any?) -> Int {
        if let int = value as? Int {
            return int
        }
        if let uint = value as? UInt {
            return Int(uint)
        }
        if let number = value as? NSNumber {
            return number.intValue
        }
        if let string = value as? String, let parsed = Int(string) {
            return parsed
        }
        return 0
    }

}
