import AuthenticationServices
import Foundation
import Security
import Supabase

struct SupabaseConfiguration: Equatable, Sendable {
    static let defaultRedirectURL = URL(string: "icesniff://auth/callback")!

    private static func configuredValue(
        for key: String,
        environment: [String: String],
        bundle: Bundle = .main
    ) -> String? {
        if let environmentValue = environment[key]?.trimmingCharacters(in: .whitespacesAndNewlines).nonEmpty {
            return environmentValue
        }

        return (bundle.object(forInfoDictionaryKey: key) as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nonEmpty
    }

    let url: URL
    let publishableKey: String
    let redirectURL: URL

    init?(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        bundle: Bundle = .main
    ) {
        guard
            let urlString = Self.configuredValue(for: "ICESNIFF_SUPABASE_URL", environment: environment, bundle: bundle),
            let publishableKey = Self.configuredValue(for: "ICESNIFF_SUPABASE_PUBLISHABLE_KEY", environment: environment, bundle: bundle),
            let url = URL(string: urlString)
        else {
            return nil
        }

        self.url = url
        self.publishableKey = publishableKey
        redirectURL = Self.configuredValue(for: "ICESNIFF_SUPABASE_REDIRECT_URL", environment: environment, bundle: bundle)
            .flatMap(URL.init(string:))
            ?? Self.defaultRedirectURL
    }

    static func diagnosticMessage(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        bundle: Bundle = .main
    ) -> String {
        let requiredKeys = [
            "ICESNIFF_SUPABASE_URL",
            "ICESNIFF_SUPABASE_PUBLISHABLE_KEY"
        ]

        let missingKeys = requiredKeys.filter { key in
            configuredValue(for: key, environment: environment, bundle: bundle) == nil
        }

        if missingKeys.isEmpty {
            let redirectURL = configuredValue(for: "ICESNIFF_SUPABASE_REDIRECT_URL", environment: environment, bundle: bundle)
                ?? Self.defaultRedirectURL.absoluteString
            return "Auth config detected. Redirect URL: \(redirectURL)."
        }

        return "Sign-in unavailable. Missing config: \(missingKeys.joined(separator: ", "))"
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
                throw CloudProfileConfigurationError.missingConfiguration("Unable to save session data (\(addStatus)).")
            }
        } else if updateStatus != errSecSuccess {
            throw CloudProfileConfigurationError.missingConfiguration("Unable to update session data (\(updateStatus)).")
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
            throw CloudProfileConfigurationError.missingConfiguration("Unable to load session data (\(status)).")
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
            throw CloudProfileConfigurationError.missingConfiguration("Unable to remove session data (\(status)).")
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

private enum SupabaseRuntime {
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

enum CloudProfilesFeature {
    static let unavailableMessage = "Cloud profile sync is temporarily unavailable. Sign-in is still available, but preferences stay local on this Mac."
}

private extension String {
    var nonEmpty: String? {
        isEmpty ? nil : self
    }
}
