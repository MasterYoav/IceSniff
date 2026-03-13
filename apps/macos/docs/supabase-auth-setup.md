# Supabase Auth Setup

This document records the exact setup used to make cloud-backed `Profile` auth and preference sync work in the mac app.

It is written as an operator runbook, not as a design note.

## What This Feature Does

The mac app can now:

- sign in with GitHub
- sign in with Google
- store the authenticated session securely in Keychain
- sync UI preferences through Supabase
- restore those preferences on another Mac after sign-in

Current synced preferences:

- theme
- font family
- font size step

## What Is Not Included

- Apple sign-in
- team/shared profiles
- account management UI
- production CI secrets wiring

Apple sign-in was intentionally removed because it requires an Apple Developer account and is not part of this branch.

## Required Supabase Configuration

The app expects these environment variables at runtime:

- `ICESNIFF_SUPABASE_URL`
- `ICESNIFF_SUPABASE_PUBLISHABLE_KEY`
- `ICESNIFF_SUPABASE_PROFILES_TABLE`

Recommended value for the table variable:

```text
profiles
```

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
ICESNIFF_SUPABASE_PROFILES_TABLE = profiles
```

Important:

- type the variable names manually in Xcode
- do not paste names copied from chat or formatted text
- zero-width Unicode characters in the variable name will make the app treat the value as missing

## Required Supabase Table

Create this table in the Supabase SQL editor:

```sql
create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  preferences jsonb not null default '{}'::jsonb,
  updated_at text not null default now()::text
);
```

Then enable row-level security and add policies:

```sql
alter table public.profiles enable row level security;

create policy "users can read own profile"
on public.profiles
for select
to authenticated
using (auth.uid() = id);

create policy "users can insert own profile"
on public.profiles
for insert
to authenticated
with check (auth.uid() = id);

create policy "users can update own profile"
on public.profiles
for update
to authenticated
using (auth.uid() = id)
with check (auth.uid() = id);
```

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

- the app falls back to mock auth/sync
- `Profile Status` explains what is missing

When the env vars are present:

- the app uses the real Supabase runtime
- browser-based OAuth opens for Google or GitHub
- sessions persist in Keychain

## Expected Login Flow

1. Launch the app from Xcode
2. Open `Profile`
3. Click `Continue with GitHub` or `Continue with Google`
4. Complete auth in the browser
5. The browser redirects back to `icesniff://auth/callback`
6. The app receives the session
7. The app pulls or creates the `profiles` row

## Common Failure Modes

### 1. App signs in instantly with a fake profile

Cause:

- the app is still using `MockAuthService`

Most likely reasons:

- missing Supabase env vars
- wrong env var names
- wrong Xcode scheme

### 2. App says the publishable key is missing even though it was entered

Cause:

- the Xcode env var name contains hidden Unicode characters

Fix:

- delete the variable row and type the env var name manually

### 3. GitHub says the redirect URI is not associated with the application

Cause:

- the GitHub OAuth app callback URL was set to the app callback instead of the Supabase callback

Fix:

- set the GitHub callback URL to `https://<project-ref>.supabase.co/auth/v1/callback`

### 4. The app opens the browser but never comes back signed in

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
- local preference sync push/pull through Supabase

Not implemented on this branch:

- Apple sign-in
- CI secrets/configuration for hosted integration tests
