const PALETTE = ["green", "red", "blue", "orange", "purple", "teal"] as const;

export type BadgeTone = (typeof PALETTE)[number] | string;

export function badgeTone(name: string, color: string | null | undefined): BadgeTone {
  if (color && color.trim()) {
    return color.trim();
  }
  const lower = name.toLowerCase();
  if (lower.includes("prod")) {
    return "red";
  }
  if (lower.includes("dev")) {
    return "green";
  }
  if (lower.includes("preview")) {
    return "blue";
  }
  if (lower.includes("test")) {
    return "orange";
  }
  let hash = 0;
  for (const ch of name) {
    hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  }
  return PALETTE[hash % PALETTE.length];
}
