function App() {
  return (
    <div className="flex h-screen flex-col">
      <div
        data-tauri-drag-region
        className="flex h-10 shrink-0 items-center justify-between bg-surface-1 px-4"
      >
        <span className="text-sm font-medium text-text-primary">
          App Launcher
        </span>
        <div className="text-xs text-text-secondary">placeholder</div>
      </div>
      <main className="flex-1 p-6">
        <p className="text-text-secondary">Ready to build.</p>
      </main>
    </div>
  );
}

export default App;
