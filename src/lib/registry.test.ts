import { describe, it, expect } from "vitest";
import { aliasIssue } from "./aliasValidation";
import { isFile, isTimestamp, isNativeText, isError } from "./registry";
import { size, time } from "./format";
describe("transport and alias boundaries", () => {
  it("validates every forbidden category without rewriting extensions", () => {
    for (const value of [
      "",
      " ",
      "a ",
      " a",
      ".",
      "..",
      "a/b",
      "a\\b",
      "a?b",
      "CON.txt",
      "COM¹.txt",
      "lpt³.pdf",
      "a.",
      "a\n",
      "\ud800",
      "é".repeat(128),
    ])
      expect(aliasIssue(value), value).not.toBeNull();
    for (const value of [
      "Context.PDF",
      "É.txt",
      "é.txt",
      "e\u0301.txt",
      "a".repeat(255),
      "é".repeat(127) + "a",
    ])
      expect(aliasIssue(value), value).toBeNull();
  });
  it("keeps integer/time precision and rejects unsafe runtime payloads", () => {
    expect(size("18446744073709551615")).toContain("18");
    expect(isTimestamp({ seconds: "-1", nanoseconds: 999999999 })).toBe(true);
    expect(isTimestamp({ seconds: "01", nanoseconds: 0 })).toBe(false);
    expect(isTimestamp({ seconds: "0", nanoseconds: 1e9 })).toBe(false);
    expect(time({ seconds: "-1", nanoseconds: 999999999 })).toBe("1969-12-31T23:59:59.999999999Z");
    expect(
      isNativeText({
        encoding: "windowsUtf16",
        units: [0xd800],
        display: "escaped",
        escaped: true,
      }),
    ).toBe(true);
    expect(
      isNativeText({ encoding: "windowsUtf16", units: [65536], display: "bad", escaped: false }),
    ).toBe(false);
    expect(isFile({ id: "x", revision: 9007199254740992 })).toBe(false);
    expect(isError({ code: "raw SQL error" })).toBe(false);
  });
});
it("accepts the shared Rust transport fixture without numeric conversion", async () => {
  const fixture = await import("../../tests/contracts/registry.json");
  expect(isFile(fixture.default)).toBe(true);
  expect(fixture.default.revision).toBe("9223372036854775807");
  expect(fixture.default.sizeBytes).toBe("18446744073709551615");
});
it("matches Rust whitespace policy without rejecting BOM", () => {
  expect(aliasIssue("\u0085name")).not.toBeNull();
  expect(aliasIssue("name\u0085")).not.toBeNull();
  expect(aliasIssue("\ufeffname")).toBeNull();
  expect(aliasIssue("name\ufeff")).toBeNull();
});
