import { useEffect, useState } from "react";
import AppShell from "./components/AppShell";
import { getVersion, isDesktop } from "./lib/tauri";

export default function App() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    if (!isDesktop()) return;

    let active = true;
    getVersion()
      .then((value) => {
        if (active) setVersion(value);
      })
      .catch(() => {
        if (active) setVersion(null);
      });

    return () => {
      active = false;
    };
  }, []);

  return <AppShell version={version} />;
}
