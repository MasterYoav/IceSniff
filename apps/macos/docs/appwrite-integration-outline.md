# Supabase Integration Outline

This document narrows the future implementation of cloud-backed profiles to a Supabase-specific runtime design while keeping the app-side interfaces clean.

## Goal

Implement real identity and preference sync using Supabase without leaking backend SDK details into the SwiftUI layer.

## App-Side Service Boundary

The app should continue to depend on these abstractions:

- `AuthProvider`
- `AuthSession`
- `AuthService`
- `ProfileSyncService`
- `UserPreferences`

The SwiftUI layer should interact only with those abstractions.

## Concrete Services To Add

Planned concrete implementations:

- `SupabaseAuthService`
- `SupabaseProfileSyncService`
- `KeychainSessionStore`

## Auth Providers

V1 providers:

- Google
- GitHub

These should map into:

- `.google`
- `.github`

inside the app’s provider enum.

## Session Storage

Session material should not live in `UserDefaults`.

Use Keychain-backed storage for:

- access tokens
- refresh tokens
- any restorable session metadata needed at relaunch

## Cloud Document Shape

The app should treat the remote profile as one row per user in a Supabase table.

Suggested shape:

```swift
struct RemoteProfileDocument: Codable, Equatable {
    var id: String
    var preferences: UserPreferences
    var updatedAt: Date
}
```

That keeps sync simple for v1.

## Sync Policy

Use `latest-write-wins`.

Rules:

1. On login, pull remote profile.
2. Compare remote `updated_at` against local `updatedAt`.
3. Apply the newer one.
4. Push local changes after sign-in.
5. Allow a manual `Sync Now` action.

## Expected AppModel Integration

When real integration starts, `AppModel` should:

1. receive an `AuthService`
2. receive a `ProfileSyncService`
3. receive a `PreferencesStore`
4. remain unaware of Supabase SDK-specific request/response types

## CI/CD Implications

When Supabase integration lands, CI will need:

- build and test on pull requests
- secret injection for any integration or smoke environments
- no hardcoded project IDs, endpoints, or provider secrets in source

## Environment Variables To Expect Later

These names are suggestions, not a final contract:

- `ICESNIFF_SUPABASE_URL`
- `ICESNIFF_SUPABASE_PUBLISHABLE_KEY`
- `ICESNIFF_SUPABASE_PROFILES_TABLE`

Provider secrets should remain in Supabase configuration, not inside the app repo.

## Minimal Table Schema

The cleanest v1 Supabase table uses:

- `id` as the authenticated user ID
- `preferences` as a `jsonb` column
- `updated_at` as a string or timestamp column used for sync comparison

## Non-Goals

This phase does not define:

- production Supabase deployment
- self-hosting topology
- backup strategy
- multi-region setup

Those are operational concerns to decide after the app-side integration works.
