export function executionSummary(total: number, executed: number, failed: number): string {
  if (total === 0) {
    return "Nothing to run";
  }
  if (failed === 0) {
    return total === 1 ? "1 statement executed" : `${total} statements executed`;
  }
  const fail = failed === 1 ? "1 failed" : `${failed} failed`;
  return `${executed} of ${total} statements executed · ${fail}`;
}

export function formatElapsed(ms: number | null): string {
  if (ms == null) {
    return "";
  }
  if (ms < 1000) {
    return `${ms} ms`;
  }
  return `${(ms / 1000).toFixed(2)} s`;
}
