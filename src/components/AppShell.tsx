type AppShellProps = {
  version: string | null;
};

export default function AppShell({ version }: AppShellProps) {
  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="app-brand">
          <img className="app-logo" src="/filenym-logo.png" alt="" />
          <span className="app-name">FileNym</span>
        </div>
        {version && <span className="app-version">v{version}</span>}
      </header>
      <main className="app-main">
        <img className="hero-logo" src="/filenym-logo.png" alt="FileNym logo" />
        <h1>FileNym</h1>
        <p>One file. Many names.</p>
      </main>
    </div>
  );
}
