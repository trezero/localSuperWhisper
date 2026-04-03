import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import Overlay from "./overlay/Overlay";

function SettingsPlaceholder() {
  return <div className="text-white text-center p-4">Settings</div>;
}

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<Overlay />} />
        <Route path="/settings/*" element={<SettingsPlaceholder />} />
        <Route path="*" element={<Navigate to="/settings" replace />} />
      </Routes>
    </HashRouter>
  );
}
