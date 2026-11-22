/** Named design tokens adapted from Qoin's dark navy and lime palette. */
export const axonTheme = {
  color: {
    canvas: "#10141d",
    canvasGlow: "#1d2c3b",
    surface: "#171e28",
    surfaceRaised: "#202a36",
    surfaceInput: "#10151d",
    border: "#2a3542",
    borderStrong: "#3b4a5a",
    text: "#edf2f8",
    textMuted: "#9aa8b8",
    textSubtle: "#8190a2",
    accent: "#b8f36a",
    accentText: "#172011",
    success: "#b8f36a",
    warning: "#ffd58a",
    danger: "#ff999f",
  },
  radius: { small: "8px", medium: "12px", large: "17px", panel: "14px" },
  spacing: { xs: "4px", sm: "8px", md: "12px", lg: "16px", xl: "24px" },
} as const;

export type AxonTheme = typeof axonTheme;
