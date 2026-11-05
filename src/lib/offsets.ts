export function utf16ToUtf8Offset(text: string, utf16: number): number {
  const end = Math.max(0, Math.min(utf16, text.length));
  let bytes = 0;
  for (let index = 0; index < end; ) {
    const code = text.codePointAt(index) ?? 0;
    bytes += utf8Len(code);
    index += code > 0xffff ? 2 : 1;
  }
  return bytes;
}

export function utf8ToUtf16Offset(text: string, utf8: number): number {
  const end = Math.max(0, utf8);
  let bytes = 0;
  let units = 0;
  for (let index = 0; index < text.length && bytes < end; ) {
    const code = text.codePointAt(index) ?? 0;
    const size = utf8Len(code);
    if (bytes + size > end) break;
    bytes += size;
    const width = code > 0xffff ? 2 : 1;
    units += width;
    index += width;
  }
  return units;
}

function utf8Len(code: number): number {
  if (code <= 0x7f) return 1;
  if (code <= 0x7ff) return 2;
  if (code <= 0xffff) return 3;
  return 4;
}
