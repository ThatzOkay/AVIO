import {
  argbFromHex,
  type DynamicScheme,
  Hct,
  hexFromArgb,
  MaterialDynamicColors,
  SchemeContent,
} from "@material/material-color-utilities";

// MD3 color roles → Vuetify theme key mappings
const getRoles = (scheme: DynamicScheme) => ({
  primary: hexFromArgb(MaterialDynamicColors.primary.getArgb(scheme)),
  onPrimary: hexFromArgb(MaterialDynamicColors.onPrimary.getArgb(scheme)),
  primaryContainer: hexFromArgb(
    MaterialDynamicColors.primaryContainer.getArgb(scheme),
  ),
  onPrimaryContainer: hexFromArgb(
    MaterialDynamicColors.onPrimaryContainer.getArgb(scheme),
  ),
  secondary: hexFromArgb(MaterialDynamicColors.secondary.getArgb(scheme)),
  onSecondary: hexFromArgb(MaterialDynamicColors.onSecondary.getArgb(scheme)),
  secondaryContainer: hexFromArgb(
    MaterialDynamicColors.secondaryContainer.getArgb(scheme),
  ),
  onSecondaryContainer: hexFromArgb(
    MaterialDynamicColors.onSecondaryContainer.getArgb(scheme),
  ),
  tertiary: hexFromArgb(MaterialDynamicColors.tertiary.getArgb(scheme)),
  tertiaryContainer: hexFromArgb(
    MaterialDynamicColors.tertiaryContainer.getArgb(scheme),
  ),
  onTertiaryContainer: hexFromArgb(
    MaterialDynamicColors.onTertiaryContainer.getArgb(scheme),
  ),
  error: hexFromArgb(MaterialDynamicColors.error.getArgb(scheme)),
  errorContainer: hexFromArgb(
    MaterialDynamicColors.errorContainer.getArgb(scheme),
  ),
  surface: hexFromArgb(MaterialDynamicColors.surface.getArgb(scheme)),
  onSurface: hexFromArgb(MaterialDynamicColors.onSurface.getArgb(scheme)),
  surfaceVariant: hexFromArgb(
    MaterialDynamicColors.surfaceVariant.getArgb(scheme),
  ),
  onSurfaceVariant: hexFromArgb(
    MaterialDynamicColors.onSurfaceVariant.getArgb(scheme),
  ),
  surfaceContainer: hexFromArgb(
    MaterialDynamicColors.surfaceContainer.getArgb(scheme),
  ),
  surfaceContainerHigh: hexFromArgb(
    MaterialDynamicColors.surfaceContainerHigh.getArgb(scheme),
  ),
  surfaceContainerHighest: hexFromArgb(
    MaterialDynamicColors.surfaceContainerHighest.getArgb(scheme),
  ),
  surfaceContainerLow: hexFromArgb(
    MaterialDynamicColors.surfaceContainerLow.getArgb(scheme),
  ),
  outline: hexFromArgb(MaterialDynamicColors.outline.getArgb(scheme)),
  outlineVariant: hexFromArgb(
    MaterialDynamicColors.outlineVariant.getArgb(scheme),
  ),
  inverseSurface: hexFromArgb(
    MaterialDynamicColors.inverseSurface.getArgb(scheme),
  ),
  inverseOnSurface: hexFromArgb(
    MaterialDynamicColors.inverseOnSurface.getArgb(scheme),
  ),
  inversePrimary: hexFromArgb(
    MaterialDynamicColors.inversePrimary.getArgb(scheme),
  ),
  scrim: hexFromArgb(MaterialDynamicColors.scrim.getArgb(scheme)),
});

export const buildM3Theme = (seedHex: string, dark = false) => {
  const hct = Hct.fromInt(argbFromHex(seedHex));
  const scheme = new SchemeContent(hct, dark, 0.0);
  const roles = getRoles(scheme);

  // Map MD3 roles → Vuetify theme colors
  return {
    dark,
    colors: {
      // Vuetify semantic → MD3 role
      primary: roles.primary,
      "on-primary": roles.onPrimary,
      "primary-container": roles.primaryContainer,
      "on-primary-container": roles.onPrimaryContainer,
      secondary: roles.secondary,
      "on-secondary": roles.onSecondary,
      "secondary-container": roles.secondaryContainer,
      "on-secondary-container": roles.onSecondaryContainer,
      tertiary: roles.tertiary,
      "tertiary-container": roles.tertiaryContainer,
      "on-tertiary-container": roles.onTertiaryContainer,
      error: roles.error,
      "error-container": roles.errorContainer,
      background: roles.surface,
      "on-background": roles.onSurface,
      surface: roles.surface,
      "on-surface": roles.onSurface,
      "surface-variant": roles.surfaceVariant,
      "on-surface-variant": roles.onSurfaceVariant,
      "surface-container": roles.surfaceContainer,
      "surface-container-high": roles.surfaceContainerHigh,
      "surface-container-highest": roles.surfaceContainerHighest,
      "surface-container-low": roles.surfaceContainerLow,
      outline: roles.outline,
      "outline-variant": roles.outlineVariant,
      "inverse-surface": roles.inverseSurface,
      "inverse-on-surface": roles.inverseOnSurface,
      "inverse-primary": roles.inversePrimary,
      scrim: roles.scrim,
    },
  };
};

// Injects --md-sys-color-* CSS vars onto :root for use outside Vuetify
export const injectM3CssVars = (seedHex: string, dark = false) => {
  const hct = Hct.fromInt(argbFromHex(seedHex));
  const scheme = new SchemeContent(hct, dark, 0.0);
  console.log("injectM3CssVars", seedHex, dark, scheme);
  const roles = getRoles(scheme);
  const root = document.documentElement;
  for (const [key, val] of Object.entries(roles)) {
    // camelCase → kebab-case
    const varName = `--md-sys-color-${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
    root.style.setProperty(varName, val);
  }
};
