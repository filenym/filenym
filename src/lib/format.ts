import type { TimestampDto } from "./transport";
export function size(value: string): string {
  return `${BigInt(value).toLocaleString()} bytes`;
}
export function time(value: TimestampDto | null): string {
  if (value === null) return "Not recorded";
  const milliseconds = BigInt(value.seconds) * 1000n + BigInt(Math.floor(value.nanoseconds / 1e6));
  if (milliseconds > 8640000000000000n || milliseconds < -8640000000000000n)
    return `${value.seconds}s + ${value.nanoseconds}ns since Unix epoch`;
  return new Date(Number(milliseconds))
    .toISOString()
    .replace(/\.\d{3}Z$/, `.${String(value.nanoseconds).padStart(9, "0")}Z`);
}
