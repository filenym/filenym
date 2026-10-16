import { useTheme, type Theme } from "../hooks/useTheme";
export default function ThemeControl() {
  const { theme, setTheme } = useTheme();
  return (
    <label className="theme-control">
      Theme{" "}
      <select aria-label="Theme" value={theme} onChange={(e) => setTheme(e.target.value as Theme)}>
        <option value="system">System</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
  );
}
