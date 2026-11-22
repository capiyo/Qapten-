import AsyncStorage from "@react-native-async-storage/async-storage";
import { getApp, getApps, initializeApp } from "firebase/app";
import * as FirebaseAuth from "firebase/auth";
import { getAuth, initializeAuth } from "firebase/auth";

const config = {
  apiKey: process.env.EXPO_PUBLIC_FIREBASE_API_KEY,
  authDomain: process.env.EXPO_PUBLIC_FIREBASE_AUTH_DOMAIN,
  projectId: process.env.EXPO_PUBLIC_FIREBASE_PROJECT_ID,
  appId: process.env.EXPO_PUBLIC_FIREBASE_APP_ID
};

const configured = Object.values(config).every(value => typeof value === "string" && value.length > 0);
const app = configured ? (getApps().length ? getApp() : initializeApp(config)) : null;

const getReactNativePersistence = (FirebaseAuth as unknown as {
  getReactNativePersistence: (storage: typeof AsyncStorage) => NonNullable<Parameters<typeof initializeAuth>[1]>["persistence"];
}).getReactNativePersistence;

function createAuth() {
  if (!app) return null;
  try {
    return initializeAuth(app, { persistence: getReactNativePersistence(AsyncStorage) });
  } catch {
    return getAuth(app);
  }
}

export const auth = createAuth();
