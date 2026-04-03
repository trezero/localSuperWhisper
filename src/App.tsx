import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import Overlay from "./overlay/Overlay";
import Layout from "./settings/Layout";
import Home from "./settings/Home";
import Vocabulary from "./settings/Vocabulary";
import Configuration from "./settings/Configuration";
import History from "./settings/History";

export default function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<Overlay />} />
        <Route path="/settings" element={<Layout />}>
          <Route index element={<Home />} />
          <Route path="vocabulary" element={<Vocabulary />} />
          <Route path="configuration" element={<Configuration />} />
          <Route path="history" element={<History />} />
        </Route>
        <Route path="*" element={<Navigate to="/settings" replace />} />
      </Routes>
    </HashRouter>
  );
}
