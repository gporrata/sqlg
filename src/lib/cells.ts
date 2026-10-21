import { columnMin } from "./layout";

const cellFont = "12px Menlo, monospace";
const headFont = "600 12px -apple-system, Arial, sans-serif";
const typeFont = "500 10px Menlo, monospace";
const cellPad = 20;
const widthCap = 4000;

export const jsonPreviewLimit = 64;

export function isJsonType(typeName: string): boolean {
  const data = typeName.toLowerCase().trim();
  return data === "json" || data.startsWith("json ");
}

export function jsonPreview(value: string): string {
  const compact = value.replace(/\s+/g, " ").trim();
  if (compact.length <= jsonPreviewLimit) return compact;
  return `${compact.slice(0, jsonPreviewLimit - 1).trimEnd()}…`;
}

export function cellText(typeName: string, value: string | null): string {
  if (value == null) return "";
  if (isJsonType(typeName)) return jsonPreview(value);
  return value;
}

export function prettyJson(value: string | null): string {
  if (value == null) return "null";
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

let measureCanvas: HTMLCanvasElement | null = null;
let measureContext: CanvasRenderingContext2D | null = null;
let measureReady = false;

function textWidth(text: string, font: string): number {
  const fallback = text.length * 7.2;
  if (!text || typeof document === "undefined") return fallback;
  if (typeof navigator !== "undefined" && /jsdom/i.test(navigator.userAgent)) return fallback;
  if (!measureReady) {
    measureReady = true;
    try {
      measureCanvas ??= document.createElement("canvas");
      measureContext = measureCanvas.getContext("2d");
    } catch {
      measureContext = null;
    }
  }
  if (!measureContext) return fallback;
  measureContext.font = font;
  const measured = measureContext.measureText(text).width;
  return measured > 0 ? measured : fallback;
}

export function fitColumnWidth(name: string, typeName: string, values: readonly (string | null)[], showTypes: boolean): number {
  let widest = "";
  for (const value of values) {
    const text = cellText(typeName, value);
    if (text.length > widest.length) widest = text;
  }
  const width = Math.max(
    textWidth(name, headFont),
    showTypes && typeName ? textWidth(typeName, typeFont) : 0,
    textWidth(widest, cellFont),
  );
  return Math.max(columnMin, Math.min(widthCap, Math.ceil(width + cellPad)));
}

export function columnSizeAction(pinned: boolean, current: number, fitted: number): "widen" | "narrow" {
  if (pinned && current + 1 >= fitted) return "narrow";
  return "widen";
}
