import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import Overlay from "./overlay/Overlay";
import Layout from "./settings/Layout";
import Home from "./settings/Home";
import Vocabulary from "./settings/Vocabulary";
import Configuration from "./settings/Configuration";
import History from "./settings/History";
import Setup from "./settings/Setup";

function AppRoutes() {
  const [ready, setReady] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(false);

  useEffect(() => {
    // Overlay window doesn't need the setup check
    if (window.location.hash.startsWith("#/overlay")) {
      setReady(true);
      return;
    }
    invoke<[string, string][]>("get_settings")
      .then((pairs) => {
        const map: Record<string, string> = {};
        pairs.forEach(([k, v]) => (map[k] = v));
        setNeedsSetup(!map.hotkey || map.hotkey.length === 0);
        setReady(true);
      })
      .catch(() => setReady(true));
  }, []);

  if (!ready) return null;

  return (
    <Routes>
      <Route path="/overlay" element={<Overlay />} />
      {needsSetup ? (
        <Route path="*" element={<Setup onDone={() => setNeedsSetup(false)} />} />
      ) : (
        <>
          <Route path="/settings" element={<Layout />}>
            <Route index element={<Home />} />
            <Route path="vocabulary" element={<Vocabulary />} />
            <Route path="configuration" element={<Configuration />} />
            <Route path="history" element={<History />} />
          </Route>
          <Route path="*" element={<Navigate to="/settings" replace />} />
        </>
      )}
    </Routes>
  );
}

export default function App() {
  return (
    <HashRouter>
      <AppRoutes />
    </HashRouter>
  );
}
