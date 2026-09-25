// Leaflet-Geoman reads the global `L` when it loads, but ES-module Leaflet
// doesn't create one. Importing this module first provides it.
import L from "leaflet";

declare global {
  interface Window {
    L: typeof L;
  }
}

window.L = L;

export default L;
