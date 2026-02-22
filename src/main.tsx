import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

import { initParticlesEngine } from "@tsparticles/react";
import { loadSlim } from "@tsparticles/slim";

initParticlesEngine(async (engine) => {
  await loadSlim(engine);
}).then(() => {
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <App />
  );
});