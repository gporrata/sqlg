let lockedUntil = 0;

export function claimRun(): boolean {
  const now = Date.now();
  if (now < lockedUntil) {
    return false;
  }
  lockedUntil = now + 250;
  return true;
}
