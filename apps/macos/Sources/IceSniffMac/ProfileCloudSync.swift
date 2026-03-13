import AuthenticationServices
import Foundation
import Security
import Supabase

struct SupabaseConfiguration: Equatable, Sendable {
    static let defaultRedirectURL = URL(string: "icesniff://auth/callback")!

    let url: URL
    let publishableKey: String
    let profilesTable: String
    let redirectURL: URL

    init?(environment: [String: String] = ProcessInfo.processInfo.environment) {
        guard
            let urlString = environment["ICESNIFF_SUPABASE_URL"]?.trimmingCharacters(in: .whitespacesAndNewlines),
            let publishableKey = environment["ICESNIFF_SUPABASE_PUBLISHABLE_KEY"]?.trimmingCharacters(in: .whitespacesAndNewlines),
            !urlString.isEmpty,
            !publishableKey.isEmpty,
            let url = URL(string: urlString)
        else {
            return nil
        }

        self.url = url
        self.publishableKey = publishableKey
        profilesTable = environment["ICESNIFF_SUPABASE_PROFILES_TABLE"]?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nonEmpty ?? "profiles"
        redirectURL = environment["ICESNIFF_SUPABASE_REDIRECT_URL"]
            .flatMap { URL(string: $0.trimmingCharacters(in: .whitespacesAndNewlines)) }
            ?? Self.defaultRedirectURL
    }

    static func diagnosticMessage(environment: [String: String] = ProcessInfo.processInfo.environment) -> String {
        let requiredKeys = [
            "ICESNIFF_SUPABASE_URL",
            "ICESNIFF_SUPABASE_PUBLISHABLE_KEY"
        ]

        let missingKeys = requiredKeys.filter { key in
            environment[key]?.trimmingCharacters(in: .whitespacesAndNewlines).nonEmpty == nil
        }

        if missingKeys.isEmpty {
            let table = environment["ICESNIFF_SUPABASE_PROFILES_TABLE"]?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .nonEmpty ?? "profiles"
            let redirectURL = environment["ICESNIFF_SUPABASE_REDIRECT_URL"]?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .nonEmpty ?? Self.defaultRedirectURL.absoluteString
            return "Supabase config detected. Profiles table: \(table). Redirect URL: \(redirectURL)."
        }

        return "Supabase config missing: \(missingKeys.joined(separator: ", "))"
    }
}

final class KeychainSessionStore: @unchecked Sendable, AuthLocalStorage {
    private enum Constants {
        static let service = "io.icesniff.macos.profile"
        static let storageKey = "io.icesniff.macos.profile.supabase-session"
    }

    private let decoder = JSONDecoder()

    func store(key: String, value: Data) throws {
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: key
        ]
        let attributes: [CFString: Any] = [
            kSecValueData: value
        ]

        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecItemNotFound {
            var addQuery = query
            addQuery[kSecValueData] = value
            let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
            guard addStatus == errSecSuccess else {
                throw SupabaseProfileError.keychainFailure("Unable to save session data (\(addStatus)).")
            }
        } else if updateStatus != errSecSuccess {
            throw SupabaseProfileError.keychainFailure("Unable to update session data (\(updateStatus)).")
        }
    }

    func retrieve(key: String) throws -> Data? {
        var item: CFTypeRef?
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: key,
            kSecReturnData: true,
            kSecMatchLimit: kSecMatchLimitOne
        ]

        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw SupabaseProfileError.keychainFailure("Unable to load session data (\(status)).")
        }
        return item as? Data
    }

    func remove(key: String) throws {
        let query: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: Constants.service,
            kSecAttrAccount: key
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw SupabaseProfileError.keychainFailure("Unable to remove session data (\(status)).")
        }
    }

    func clear() {
        try? remove(key: Constants.storageKey)
    }

    func loadSupabaseSession() -> Session? {
        guard
            let data = try? retrieve(key: Constants.storageKey),
            let session = try? decoder.decode(Session.self, from: data)
        else {
            return nil
        }

        return session
    }

    static let storageKey = Constants.storageKey
}

enum SupabaseProfileError: LocalizedError {
    case notConfigured
    case keychainFailure(String)
    case invalidUserIdentifier
    case invalidPreferencesPayload

    var errorDescription: String? {
        switch self {
        case .notConfigured:
            return "Cloud profiles are not configured for this build yet."
        case let .keychainFailure(message):
            return message
        case .invalidUserIdentifier:
            return "Supabase returned an unexpected user identifier."
        case .invalidPreferencesPayload:
            return "The saved cloud profile contains invalid preference data."
        }
    }
}

private struct SupabaseProfileRow: Codable, Sendable {
    let id: UUID
    let preferences: String
    let updatedAt: String

    enum CodingKeys: String, CodingKey {
        case id
        case preferences
        case updatedAt = "updated_at"
    }
}

private enum SupabaseRuntime {
    static func timestampFormatter() -> ISO8601DateFormatter {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }

    static func authProvider(for provider: AuthProvider) -> Provider {
        switch provider {
        case .google:
            return .google
        case .github:
            return .github
        }
    }

    static func scopes(for provider: AuthProvider) -> String? {
        switch provider {
        case .github:
            return "read:user user:email"
        case .google:
            return "email profile"
        }
    }

    static func authProvider(from session: Session, fallback: AuthProvider? = nil) -> AuthProvider {
        if let provider = fallback {
            return provider
        }

        let providerName = session.user.identities?.first?.provider
            ?? session.user.appMetadata["provider"]?.stringValue
            ?? session.user.appMetadata["providers"]?.arrayValue?.first?.stringValue

        switch providerName?.lowercased() {
        case "google":
            return .google
        case "github":
            return .github
        default:
            return .github
        }
    }

    static func displayName(from user: User) -> String? {
        user.userMetadata["full_name"]?.stringValue
            ?? user.userMetadata["name"]?.stringValue
            ?? user.identities?.first?.identityData?["name"]?.stringValue
            ?? user.identities?.first?.identityData?["full_name"]?.stringValue
    }

    static func avatarURL(from user: User) -> URL? {
        let rawURL = user.userMetadata["avatar_url"]?.stringValue
            ?? user.userMetadata["picture"]?.stringValue
            ?? user.identities?.first?.identityData?["avatar_url"]?.stringValue
            ?? user.identities?.first?.identityData?["picture"]?.stringValue

        guard let rawURL, let url = URL(string: rawURL) else {
            return nil
        }

        return url
    }

    static func authSession(from session: Session, fallbackProvider: AuthProvider? = nil) -> AuthSession {
        AuthSession(
            userID: session.user.id.uuidString,
            email: session.user.email,
            displayName: displayName(from: session.user),
            avatarURL: avatarURL(from: session.user),
            provider: authProvider(from: session, fallback: fallbackProvider)
        )
    }
}

final class SupabaseAuthService: AuthService, @unchecked Sendable {
    var currentSession: AuthSession? {
        if let session = client.auth.currentSession ?? sessionStore.loadSupabaseSession() {
            return SupabaseRuntime.authSession(from: session)
        }

        return nil
    }

    private let client: SupabaseClient
    private let sessionStore: KeychainSessionStore
    private let configuration: SupabaseConfiguration

    init(configuration: SupabaseConfiguration, sessionStore: KeychainSessionStore) {
        self.configuration = configuration
        self.sessionStore = sessionStore
        client = SupabaseClient(
            supabaseURL: configuration.url,
            supabaseKey: configuration.publishableKey,
            options: SupabaseClientOptions(
                auth: .init(
                    storage: sessionStore,
                    redirectToURL: configuration.redirectURL,
                    storageKey: KeychainSessionStore.storageKey,
                    emitLocalSessionAsInitialSession: true
                )
            )
        )
    }

    func signIn(with provider: AuthProvider) async throws -> AuthSession {
        let session = try await client.auth.signInWithOAuth(
            provider: SupabaseRuntime.authProvider(for: provider),
            redirectTo: configuration.redirectURL,
            scopes: SupabaseRuntime.scopes(for: provider)
        )
        return SupabaseRuntime.authSession(from: session, fallbackProvider: provider)
    }

    func signOut() async throws {
        defer { sessionStore.clear() }
        try await client.auth.signOut()
    }
}

final class SupabaseProfileSyncService: ProfileSyncService, @unchecked Sendable {
    private let client: SupabaseClient

    init(configuration: SupabaseConfiguration, sessionStore: KeychainSessionStore) {
        client = SupabaseClient(
            supabaseURL: configuration.url,
            supabaseKey: configuration.publishableKey,
            options: SupabaseClientOptions(
                auth: .init(
                    storage: sessionStore,
                    redirectToURL: configuration.redirectURL,
                    storageKey: KeychainSessionStore.storageKey,
                    emitLocalSessionAsInitialSession: true
                )
            )
        )
        profilesTable = configuration.profilesTable
    }

    private let profilesTable: String
    private let encoder: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }()
    private let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }()

    func pullPreferences(for session: AuthSession) async throws -> UserPreferences? {
        let response: PostgrestResponse<[SupabaseProfileRow]> = try await client
            .from(profilesTable)
            .select()
            .eq("id", value: session.userID)
            .execute()
        let rows = response.value
        guard let row = rows.first else {
            return nil
        }

        guard let data = row.preferences.data(using: String.Encoding.utf8) else {
            throw SupabaseProfileError.invalidPreferencesPayload
        }

        var preferences = try decoder.decode(UserPreferences.self, from: data)
        if let updatedAt = SupabaseRuntime.timestampFormatter().date(from: row.updatedAt) {
            preferences.updatedAt = updatedAt
        }
        return preferences
    }

    func pushPreferences(_ preferences: UserPreferences, for session: AuthSession) async throws {
        guard let userID = UUID(uuidString: session.userID) else {
            throw SupabaseProfileError.invalidUserIdentifier
        }

        let preferencesData = try encoder.encode(preferences)
        let row = SupabaseProfileRow(
            id: userID,
            preferences: String(decoding: preferencesData, as: UTF8.self),
            updatedAt: SupabaseRuntime.timestampFormatter().string(from: preferences.updatedAt)
        )

        try await client
            .from(profilesTable)
            .upsert(row, onConflict: "id", returning: .minimal)
            .execute()
    }
}

private extension String {
    var nonEmpty: String? {
        isEmpty ? nil : self
    }
}
