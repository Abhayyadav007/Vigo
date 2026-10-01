import { createContext, useContext, type ReactNode } from "react";
import { colors, type Palette } from "./theme";

// Shared components read JS colours (spinners, placeholders) from here, so an
// app on the light palette gets matching values. Defaults to the dark palette.
const ColorsContext = createContext<Palette>(colors);

export function ColorsProvider({ palette, children }: { palette: Palette; children: ReactNode }) {
  return <ColorsContext.Provider value={palette}>{children}</ColorsContext.Provider>;
}

export const useColors = () => useContext(ColorsContext);
