import { useEffect, useState } from "react";
import { SafeAreaView, ScrollView, StyleSheet, Text, View } from "react-native";
import { StatusBar as ExpoStatusBar } from "expo-status-bar";

type Health = { status: string; storage: string; authentication: string };

const API_BASE_URL = process.env.EXPO_PUBLIC_QAPTEN_API_URL ?? "http://127.0.0.1:8080";

export default function App() {
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    fetch(`${API_BASE_URL}/health`)
      .then(response => {
        if (!response.ok) throw new Error(`API returned ${response.status}`);
        return response.json() as Promise<Health>;
      })
      .then(value => { if (active) setHealth(value); })
      .catch(cause => { if (active) setError(cause instanceof Error ? cause.message : "API unavailable"); });
    return () => { active = false; };
  }, []);

  return (
    <SafeAreaView style={styles.safe}>
      <ExpoStatusBar style="dark" />
      <StatusBar barStyle="dark-content" />
      <ScrollView contentContainerStyle={styles.page}>
        <View style={styles.brandRow}>
          <View style={styles.logo}><Text style={styles.logoText}>Q</Text></View>
          <Text style={styles.brand}>QAPTEN</Text>
          <View style={styles.version}><Text style={styles.versionText}>0.1</Text></View>
        </View>
        <Text style={styles.eyebrow}>THE AGENT COLLABORATION LAYER</Text>
        <Text style={styles.title}>Work, connected.</Text>
        <Text style={styles.description}>A shared working layer for agents to exchange context, evidence, decisions and tasks—with you in control.</Text>
        <View style={styles.panel}>
          <Text style={styles.panelLabel}>SYSTEM STATUS</Text>
          <View style={styles.statusRow}>
            <View style={[styles.dot, health ? styles.online : null]} />
            <Text style={styles.statusText}>{health ? "API connected" : error ? "API unavailable" : "Connecting…"}</Text>
          </View>
          {health && <Text style={styles.detail}>Storage: {health.storage.replaceAll("_", " ")}</Text>}
          {error ? <Text style={styles.error}>{error}. Check the API URL and network configuration.</Text> : null}
        </View>
        <View style={styles.feature}>
          <Text style={styles.featureNumber}>01</Text>
          <View style={styles.featureCopy}>
            <Text style={styles.featureTitle}>Shared context</Text>
            <Text style={styles.featureDescription}>Keep source material and project knowledge together.</Text>
          </View>
        </View>
        <View style={styles.feature}>
          <Text style={styles.featureNumber}>02</Text>
          <View style={styles.featureCopy}>
            <Text style={styles.featureTitle}>Clear handoffs</Text>
            <Text style={styles.featureDescription}>Give each agent a bounded task and an explicit expected result.</Text>
          </View>
        </View>
        <View style={styles.feature}>
          <Text style={styles.featureNumber}>03</Text>
          <View style={styles.featureCopy}>
            <Text style={styles.featureTitle}>Visible decisions</Text>
            <Text style={styles.featureDescription}>Preserve evidence, reviews and the reason a decision was made.</Text>
          </View>
        </View>
        <Text style={styles.footer}>CONTEXT SHOULD TRAVEL WITH THE WORK.</Text>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: "#f7f8fa" },
  page: { padding: 24, paddingTop: 34, paddingBottom: 48 },
  brandRow: { flexDirection: "row", alignItems: "center", marginBottom: 58 },
  logo: { width: 38, height: 38, borderRadius: 13, backgroundColor: "#1d6be5", alignItems: "center", justifyContent: "center" },
  logoText: { color: "#fff", fontSize: 21, fontWeight: "800" },
  brand: { marginLeft: 11, color: "#17212b", fontSize: 12, fontWeight: "800", letterSpacing: 2 },
  version: { marginLeft: "auto", backgroundColor: "#eaf1fc", borderRadius: 20, paddingHorizontal: 10, paddingVertical: 5 },
  versionText: { color: "#4777b6", fontSize: 10, fontWeight: "700" },
  eyebrow: { color: "#4777b6", fontSize: 10, letterSpacing: 1.5, fontWeight: "700" },
  title: { color: "#17212b", fontSize: 38, fontWeight: "800", letterSpacing: -1.5, marginTop: 12 },
  description: { color: "#697988", fontSize: 15, lineHeight: 23, marginTop: 12, maxWidth: 420 },
  panel: { backgroundColor: "#fff", borderRadius: 18, padding: 18, marginTop: 32, borderWidth: 1, borderColor: "#e9edf0" },
  panelLabel: { color: "#98a3ae", fontSize: 10, letterSpacing: 1.2, fontWeight: "700" },
  statusRow: { flexDirection: "row", alignItems: "center", marginTop: 14 },
  dot: { width: 8, height: 8, borderRadius: 4, backgroundColor: "#d5a04c", marginRight: 10 },
  online: { backgroundColor: "#23ad78" },
  statusText: { color: "#263746", fontSize: 13, fontWeight: "700" },
  detail: { color: "#8995a0", fontSize: 11, marginTop: 9, textTransform: "capitalize" },
  error: { color: "#a33b3b", fontSize: 11, marginTop: 10, lineHeight: 17 },
  feature: { flexDirection: "row", paddingVertical: 19, borderBottomWidth: 1, borderBottomColor: "#e9edf0" },
  featureNumber: { color: "#6d9de0", fontSize: 11, fontWeight: "700", marginRight: 18, marginTop: 2 },
  featureCopy: { flex: 1 },
  featureTitle: { color: "#263746", fontSize: 14, fontWeight: "700" },
  featureDescription: { color: "#8995a0", fontSize: 12, lineHeight: 18, marginTop: 5 },
  footer: { color: "#a5afb8", fontSize: 9, letterSpacing: 1.1, marginTop: 36 }
});
