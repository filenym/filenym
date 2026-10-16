// UX feedback only. Rust remains authoritative; never normalize user spelling.
export function aliasIssue(name: string): string | null {
  if (/\p{Surrogate}/u.test(name)) return "The name contains an invalid Unicode character.";
  if (name.length === 0) return "Enter an alias name.";
  if (/\p{Cc}/u.test(name)) return "The name contains a forbidden character.";
  // Rust char::is_whitespace includes NEL and excludes BOM.
  const edgeWhitespace = (character: string) => {
    const point = character.codePointAt(0) ?? 0;
    return (
      (point >= 9 && point <= 13) ||
      (point >= 0x2000 && point <= 0x200a) ||
      [0x20, 0x85, 0xa0, 0x1680, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000].includes(point)
    );
  };
  if (edgeWhitespace(name[0]) || edgeWhitespace(name[name.length - 1]))
    return "Remove whitespace at the beginning or end.";
  if (name === "." || name === ".." || /[\\/]/.test(name)) return "Use a name, without a path.";
  if (/[<>:"|?*\p{Cc}]/u.test(name)) return "The name contains a forbidden character.";
  if (name.endsWith(".")) return "The name cannot end with a period.";
  if (new TextEncoder().encode(name).length > 255) return "Use at most 255 UTF-8 bytes.";
  if (/^(con|prn|aux|nul|com[1-9¹²³]|lpt[1-9¹²³])(?:\.|$)/i.test(name))
    return "This name is reserved by Windows.";
  return null;
}
