# Supabase Auth Setup

This document records the exact setup used to make `Profile` auth work in the mac app.

It is written as an operator runbook, not as a design note.

## What This Feature Does

The mac app can now:

- sign in with GitHub
- sign in with Google
- store the authenticated session securely in Keychain
- restore the authenticated session after relaunch

## What Is Not Included

- Apple sign-in
- cloud-backed preference sync in the public build
- AI account hosting or AI conversation sync through Supabase
- team/shared profiles
- account management UI
- production CI secrets wiring

Apple sign-in was intentionally removed because it requires an Apple Developer account and is not part of this branch.

## Required Supabase Configuration

The app expects these environment variables at runtime:

- `ICESNIFF_SUPABASE_URL`
- `ICESNIFF_SUPABASE_PUBLISHABLE_KEY`
- `ICESNIFF_SUPABASE_REDIRECT_URL` (optional)

## Add Environment Variables In Xcode

For local development, add the variables in the Xcode Run scheme.

1. Open `Product` -> `Scheme` -> `Edit Scheme...`
2. Select `Run`
3. Open `Arguments`
4. Add the environment variables under `Environment Variables`

Example:

```text
ICESNIFF_SUPABASE_URL = https://<project-ref>.supabase.co
ICESNIFF_SUPABASE_PUBLISHABLE_KEY = sb_publishable_...
ICESNIFF_SUPABASE_REDIRECT_URL = icesniff://auth/callback
```

Important:

- type the variable names manually in Xcode
- do not paste names copied from chat or formatted text
- zero-width Unicode characters in the variable name will make the app treat the value as missing

## Required Redirect Configuration

In Supabase `Authentication` -> `URL Configuration`, add:

```text
icesniff://auth/callback
```

That custom URL is how the browser auth flow returns to the mac app.

## GitHub Provider Setup

This is the correct callback split:

- GitHub OAuth App callback URL: Supabase HTTPS callback
- app redirect URL: `icesniff://auth/callback`

For a Supabase project at `https://<project-ref>.supabase.co`, configure GitHub like this:

```text
Authorization callback URL = https://<project-ref>.supabase.co/auth/v1/callback
```

Do not set the GitHub OAuth app callback to `icesniff://auth/callback`.

Steps:

1. In Supabase, enable GitHub under `Authentication` -> `Providers`
2. In GitHub, create an OAuth App
3. Set the callback URL to `https://<project-ref>.supabase.co/auth/v1/callback`
4. Copy the GitHub client ID and client secret into Supabase

## Google Provider Setup

Steps:

1. In Supabase, enable Google under `Authentication` -> `Providers`
2. In Google Cloud Console, create a Web OAuth client
3. Set the Google redirect URI to:

```text
https://<project-ref>.supabase.co/auth/v1/callback
```

4. Copy the Google client ID and client secret into Supabase

## Runtime Behavior

When the env vars are missing:

- sign-in is unavailable
- `Profile Status` explains what is missing

When the env vars are present:

- the app uses the real Supabase runtime
- browser-based OAuth opens for Google or GitHub
- sessions persist in Keychain
- preferences still remain local to the current Mac
- AI provider settings remain separate from Supabase auth

## Expected Login Flow

1. Launch the app from Xcode
2. Open `Profile`
3. Click `Continue with GitHub` or `Continue with Google`
4. Complete auth in the browser
5. The browser redirects back to `icesniff://auth/callback`
6. The app receives the session
7. The app restores the signed-in identity on relaunch through the stored session

## Common Failure Modes

### 1. App says the publishable key is missing even though it was entered

Cause:

- the Xcode env var name contains hidden Unicode characters

Fix:

- delete the variable row and type the env var name manually

### 2. GitHub says the redirect URI is not associated with the application

Cause:

- the GitHub OAuth app callback URL was set to the app callback instead of the Supabase callback

Fix:

- set the GitHub callback URL to `https://<project-ref>.supabase.co/auth/v1/callback`

### 3. The app opens the browser but never comes back signed in

Cause:

- `icesniff://auth/callback` is not allowed in Supabase URL configuration

Fix:

- add `icesniff://auth/callback` to the redirect allowlist

## Files Involved In This Feature

- `Sources/IceSniffMac/AppModel.swift`
- `Sources/IceSniffMac/ProfileCloudSync.swift`
- `Sources/IceSniffMac/Views.swift`
- `Tests/IceSniffMacTests/AppModelTests.swift`

## Current Status

Working on this branch:

- GitHub sign-in
- Google sign-in
- avatar loading from provider metadata
- local-only preferences

Not implemented on this branch:

- Apple sign-in
- cloud preference sync
- CI secrets/configuration for hosted integration tests
