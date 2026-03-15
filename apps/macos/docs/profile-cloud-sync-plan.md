# Profile Cloud Sync Plan

This document is retained as a historical design note.

## Current Status

Profile cloud sync is not part of the public macOS build.

The current shipped behavior is:

- sign in with Google
- sign in with GitHub
- restore the authenticated session from Keychain
- keep theme, font family, and font size local to the current Mac

## Why This Plan Was Shelved

The original direction was to store user preferences in Supabase so they would follow the user across devices.

That is currently disabled for the public repository because:

- it would expose a hosted backend to abuse in an open-source build
- it adds operational cost and moderation work before the product is ready for that investment
- the auth-only experience is enough for the current release phase

## What Still Applies

A few parts of the original design are still relevant:

- provider-neutral auth models are still a good idea
- session credentials should stay in Keychain, not `UserDefaults`
- local preferences should remain usable while signed out or offline

## What Does Not Apply Right Now

These are not active product goals in the public build:

- syncing preferences to Supabase
- downloading a `profiles` row at sign-in
- pushing preference changes automatically to the cloud
- conflict resolution between local and remote preference payloads
- a `Sync Now` action in the profile UI

## If Cloud Sync Returns Later

If the project revisits cloud-backed preferences in the future, this should be treated as a fresh product decision rather than as an automatically resumed roadmap item.

Before reintroducing it, the project should decide:

- whether a hosted backend is acceptable for the public app
- what abuse controls and rate limits are required
- whether preferences should sync only for trusted builds or paid users
- how support and incident ownership will be handled

Until then, this app should be documented and shipped as:

- auth-enabled
- local-preferences only
- no public cloud preference sync
