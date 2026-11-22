# Firebase Auth Setup

Axon uses Firebase Authentication for end-user identity. Firebase credentials are entered directly into the Firebase SDK clients; Axon's API never receives passwords, SMS codes, OAuth secrets, or email-link action codes.

## Firebase project

Use one dedicated Firebase project for Axon.

1. Register a Web app and copy its public SDK configuration.
2. Register the Android app with package name `co.axon.mobile`.
3. Register the iOS app with bundle identifier `co.axon.mobile`.
4. In Firebase Authentication, enable the Email/Password provider.
5. Add the deployed web hostname to Firebase Authentication's authorized domains.
6. Configure the email templates and project-level abuse protections before production use.

## Web client

Create `frontend/apps/web/.env.local` from `.env.example`:

```dotenv
VITE_FIREBASE_API_KEY=
VITE_FIREBASE_AUTH_DOMAIN=
VITE_FIREBASE_PROJECT_ID=
VITE_FIREBASE_APP_ID=
```

These are Firebase client SDK configuration values, not service-account credentials. Never put a Firebase Admin SDK private key or service-account JSON in a Vite variable.

## Mobile client

Create `frontend/apps/mobile/.env` from `.env.example`:

```dotenv
EXPO_PUBLIC_AXON_API_URL=http://127.0.0.1:8080
EXPO_PUBLIC_FIREBASE_API_KEY=
EXPO_PUBLIC_FIREBASE_AUTH_DOMAIN=
EXPO_PUBLIC_FIREBASE_PROJECT_ID=
EXPO_PUBLIC_FIREBASE_APP_ID=
```

For an Android emulator, use `http://10.0.2.2:8080`. For a physical device, use a reachable HTTPS API endpoint or the documented USB reverse setup. Expo public variables are embedded in the client bundle and must never contain private keys.

## API

Set the same Firebase project ID on the Axon API:

```dotenv
FIREBASE_PROJECT_ID=
AXON_DATABASE_URL=sqlite://axon.db
AXON_ENABLE_SIMULATOR=false
```

The API verifies Firebase ID tokens against Google's Secure Token JWKS and checks issuer, audience, signature, and expiry. It provisions an internal user, personal workspace, owner membership, and audit event transactionally on first authenticated request.

Only `GET /health` is public. Every registered `/api/v1` route requires `Authorization: Bearer <Firebase ID token>`. Workspace and task identifiers are checked against the authenticated user's memberships; cross-user access returns 404.

## Current provider scope

The current client implementation supports Email/Password sign-in and account creation. Google, email-link, and phone-first sign-in are not yet implemented; do not advertise those providers as available. Invitations and workspace role management are also outside this authentication slice.
