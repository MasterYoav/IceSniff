# Profile Cloud Sync Plan

This document defines the first implementation plan for making the `Profile` section real.

The goal is simple:

- a user signs in with Google or GitHub
- the app syncs the user’s preferences to the cloud
- the same preferences appear on another Mac after sign-in

This is intentionally a v1 plan.
It should solve the real product need without overcomplicating the first release.

## Product Goal

The `Profile` section should become the home for:

- sign in
- sign out
- sync status
- cloud-backed preferences
- future account-level settings

## Scope For V1

V1 should sync only user preferences that already exist in the app:

- theme
- font family
- font size step

Later versions can add:

- saved filters
- capture defaults
- section-specific preferences
- account-level usage data
- user-created presets

## Recommended Backend

Recommended choice: Supabase.

Reasons:

- Google OAuth support
- GitHub OAuth support
- straightforward hosted workflow for early rollout
- widely-used auth + Postgres stack
- good fit for a simple v1 cloud-sync feature

## Why Not A Hosted-Only Dependency

This feature should not assume a permanently hosted third-party dependency is acceptable.

The preferred direction is:

- use Supabase APIs
- keep the app-side auth/sync layer provider-agnostic
- keep the option to swap or self-host later without rewriting the app

## Architecture

The feature should be split into four local responsibilities.

### 1. Preferences Model

Create a single codable preferences type that represents the synced state.

Suggested shape:

```swift
struct UserPreferences: Codable, Equatable {
    var theme: AppTheme
    var fontChoice: AppFontChoice
    var fontSizeStep: AppFontSizeStep
    var schemaVersion: Int
}
```

This model should become the source of truth for synced settings.

## 2. Local Preferences Store

Create a small store responsible for reading and writing local preferences.

Responsibilities:

- load from `UserDefaults`
- save to `UserDefaults`
- expose/apply a `UserPreferences` value

This allows the app to work fully when signed out.

## 3. Auth Session Layer

Create an authentication abstraction that hides provider details from SwiftUI views.

Suggested shape:

```swift
struct AuthSession: Equatable {
    let userID: String
    let email: String?
    let displayName: String?
    let provider: AuthProvider
}
```

The service should expose:

- current session
- sign in with Google
- sign in with GitHub
- sign out

Important rule:

- auth tokens should not be stored in `UserDefaults`
- use Keychain-backed secure storage
- Supabase access and refresh tokens should be treated as credentials and stored securely

## 4. Profile Sync Service

Create a cloud sync abstraction that uploads and downloads `UserPreferences`.

Suggested responsibilities:

- fetch remote preferences for signed-in user
- upload current local preferences
- resolve simple conflicts
- expose last sync status and timestamp

For Supabase-backed v1, this should map to:

- account identity from Supabase auth/session APIs
- one `profiles` row per user
- update timestamps managed in an `updated_at` column

## Sync Strategy For V1

Use `latest-write-wins`.

That means:

- the preference payload contains `updatedAt`
- on sign-in, the newer version wins
- after sign-in, preference changes push automatically

This is enough for v1 and avoids unnecessary merge logic.

## Signed-Out Behavior

Signed-out behavior should remain exactly as the app works today:

- local preferences still load
- local preferences still save
- no network dependency

That keeps the feature low-risk.

## Signed-In Behavior

After sign-in:

1. fetch remote preferences
2. compare timestamps
3. apply the newer preferences
4. continue saving locally
5. push updated preferences after local changes

## UI Expectations

The `Profile` section should eventually contain:

- signed-out state
- provider sign-in buttons
- signed-in summary
- sync state
- last sync timestamp
- manual `Sync Now` action
- sign-out action

V1 should avoid adding too much profile UI beyond these basics.

## Suggested Swift Types

These types are the expected first extraction layer:

- `UserPreferences`
- `PreferencesStore`
- `AuthProvider`
- `AuthSession`
- `AuthService`
- `ProfileSyncService`
- `SyncStatus`

## Suggested Phased Implementation

### Phase 1

Extract preferences from `AppModel` into a codable type and local store.

Deliverables:

- `UserPreferences`
- `PreferencesStore`
- tests for encode/decode and apply/load behavior

### Phase 2

Add auth and profile-sync abstractions with mocked implementations.

Deliverables:

- `AuthSession`
- `AuthService` protocol
- `ProfileSyncService` protocol
- placeholder `Profile` UI states

### Phase 3

Integrate real Supabase auth and storage.

Deliverables:

- Google sign-in
- GitHub sign-in
- preference upload/download
- Keychain-backed token handling
- Supabase session restoration on relaunch

### Phase 4

Add CI/CD enforcement for the feature branch workflow.

Deliverables:

- mac build in CI
- mac tests in CI
- CLI tests in CI
- branch protection and required checks

## CI/CD Learning Goals For This Feature

This feature is a good place to practice:

- feature branches
- small incremental commits
- test-first or test-alongside development
- secret management
- PR validation
- required status checks
- merge discipline

## Definition Of Done For V1

V1 is done when:

- a user can sign in with Google or GitHub
- the current theme/font/font size sync to the cloud
- a second Mac restores those settings after sign-in
- local-only usage still works while signed out
- tests cover preference serialization and sync decisions
- CI validates build and tests on every PR

## Non-Goals For V1

Do not include these yet:

- team/shared profiles
- multi-device merge UIs
- audit history
- full account management
- cross-platform profile sync beyond the mac app

Those can come later.

## Supabase-Specific Interface Notes

The app should keep these protocols provider-neutral even though Supabase is the chosen backend.

That means:

- `AuthProvider` should include `.google`, `.apple`, and `.github`
- `AuthService` should not expose raw Supabase SDK types to views
- `ProfileSyncService` should work in terms of `UserPreferences`, not backend JSON payloads

Suggested Supabase-backed implementations:

- `SupabaseAuthService`
- `SupabaseProfileSyncService`
- `KeychainSessionStore`

Suggested supporting types:

```swift
struct RemoteProfileDocument: Codable, Equatable {
    var id: String
    var preferences: UserPreferences
    var updatedAt: Date
}
```

```swift
struct StoredSession: Codable, Equatable {
    var sessionID: String
    var userID: String
    var provider: AuthProvider
}
```

The SwiftUI layer should never need to know whether the backend is Supabase-hosted or self-hosted.
