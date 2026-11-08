# Frontend

- `apps/web`: React + TypeScript + Vite web workspace.
- `apps/mobile`: React Native + Expo mobile foundation.

## Web

```sh
cd frontend/apps/web
npm install
npm run dev
```

The Vite development server proxies `/api` and `/health` to the local Axum API at `127.0.0.1:8080`.

## Mobile

```sh
cd frontend/apps/mobile
npm install
npx expo start
```

Set `EXPO_PUBLIC_QAPTEN_API_URL` to an address reachable from the device. On a physical phone, `127.0.0.1` points to the phone itself, not the development computer. Configure the computer's reachable LAN address for local device testing.

Both clients are early foundations. Authentication, workspace membership, task handoffs, and artifact review UI are not implemented yet.
