import "leaflet/dist/leaflet.css";
import "@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css";
// Order matters: ../lib/leaflet sets window.L before Geoman evaluates.
import L from "../lib/leaflet";
import "@geoman-io/leaflet-geoman-free";

import type { GeoJsonPolygon, LatLng } from "@vigo/types";
import { useEffect, useLayoutEffect, useRef } from "react";
import { CircleMarker, MapContainer, TileLayer, useMap, useMapEvents } from "react-leaflet";

const INDIA_CENTER: L.LatLngTuple = [20.59, 78.96];

export interface PolygonEditorProps {
  location: LatLng | null;
  area: GeoJsonPolygon | null;
  onLocationChange: (location: LatLng) => void;
  onAreaChange: (area: GeoJsonPolygon | null) => void;
}

/**
 * Map with the store pin (click to move) and its service-area polygon
 * (draw / edit / delete with the toolbar on the left).
 */
export function PolygonEditor(props: PolygonEditorProps) {
  const center: L.LatLngTuple = props.location ? [props.location.lat, props.location.lng] : INDIA_CENTER;
  return (
    <div className="map" data-testid="store-map">
      <MapContainer center={center} zoom={props.location ? 13 : 5} scrollWheelZoom style={{ height: "100%" }}>
        {/* TODO(prod): OSM's public tiles are for light dev use; switch to a tile provider for production. */}
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
          url="https://tile.openstreetmap.org/{z}/{x}/{y}.png"
        />
        {props.location ? (
          <CircleMarker
            center={[props.location.lat, props.location.lng]}
            radius={8}
            pathOptions={{ color: "#07542d", fillColor: "#0c8346", fillOpacity: 1 }}
          />
        ) : null}
        <AreaLayer {...props} />
      </MapContainer>
    </div>
  );
}

function toGeoJson(layer: L.Polygon): GeoJsonPolygon {
  const geometry = layer.toGeoJSON(6).geometry as { coordinates: number[][][] };
  return {
    type: "Polygon",
    coordinates: geometry.coordinates.map((ring) => ring.map(([lng, lat]) => [lng!, lat!] as [number, number])),
  };
}

function AreaLayer({ area, onAreaChange, onLocationChange }: PolygonEditorProps) {
  const map = useMap();
  const layerRef = useRef<L.Polygon | null>(null);
  const onAreaRef = useRef(onAreaChange);
  useLayoutEffect(() => {
    onAreaRef.current = onAreaChange;
  }, [onAreaChange]);

  useMapEvents({
    click(e) {
      if (!map.pm.globalDrawModeEnabled() && !map.pm.globalEditModeEnabled()) {
        onLocationChange({ lat: Number(e.latlng.lat.toFixed(6)), lng: Number(e.latlng.lng.toFixed(6)) });
      }
    },
  });

  // Toolbar + draw/edit/remove events, once per map.
  useEffect(() => {
    map.pm.addControls({
      position: "topleft",
      drawPolygon: true,
      editMode: true,
      removalMode: true,
      drawMarker: false,
      drawCircleMarker: false,
      drawPolyline: false,
      drawRectangle: false,
      drawCircle: false,
      drawText: false,
      cutPolygon: false,
      rotateMode: false,
      dragMode: false,
    });
    const watch = (layer: L.Polygon) => layer.on("pm:edit", () => onAreaRef.current(toGeoJson(layer)));
    const onCreate = (e: { shape: string; layer: L.Layer }) => {
      if (e.shape !== "Polygon" || !(e.layer instanceof L.Polygon)) return;
      layerRef.current?.remove();
      layerRef.current = e.layer;
      watch(e.layer);
      onAreaRef.current(toGeoJson(e.layer));
    };
    const onRemove = (e: { layer: L.Layer }) => {
      if (e.layer === layerRef.current) {
        layerRef.current = null;
        onAreaRef.current(null);
      }
    };
    map.on("pm:create", onCreate);
    map.on("pm:remove", onRemove);
    return () => {
      map.off("pm:create", onCreate);
      map.off("pm:remove", onRemove);
      map.pm.removeControls();
    };
  }, [map]);

  // Reflect `area` changes made outside the map (loaded store, generated hexagon).
  useEffect(() => {
    const current = layerRef.current;
    if (area === null) {
      current?.remove();
      layerRef.current = null;
      return;
    }
    if (current && JSON.stringify(toGeoJson(current)) === JSON.stringify(area)) return;
    current?.remove();
    const ring = area.coordinates[0] ?? [];
    const layer = L.polygon(
      ring.slice(0, -1).map(([lng, lat]) => [lat, lng] as L.LatLngTuple),
      { color: "#0c8346" },
    ).addTo(map);
    layer.on("pm:edit", () => onAreaRef.current(toGeoJson(layer)));
    layerRef.current = layer;
    map.fitBounds(layer.getBounds(), { padding: [24, 24] });
  }, [area, map]);

  return null;
}

/** A regular hexagon of radius `km` around a point, as GeoJSON. */
export function hexagon({ lat, lng }: LatLng, km: number): GeoJsonPolygon {
  const ring: [number, number][] = [];
  for (let i = 0; i <= 6; i++) {
    const a = (((i % 6) * 60 + 30) * Math.PI) / 180;
    ring.push([
      Number((lng + (km * Math.cos(a)) / (111.32 * Math.cos((lat * Math.PI) / 180))).toFixed(6)),
      Number((lat + (km * Math.sin(a)) / 110.57).toFixed(6)),
    ]);
  }
  return { type: "Polygon", coordinates: [ring] };
}
