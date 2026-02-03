import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { log } from './Logic'

import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { notify } from "./Notifications";

let globalUpdateChecked = false;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

export const Initializer = () => {
  const isStarted = useRef(false);

  useEffect(() => {
    if (isStarted.current || globalUpdateChecked) return;
    isStarted.current = true;
    globalUpdateChecked = true;

    const runChecks = async () => {
      try {
        const wasUpdated = await invoke<boolean>("check_winws_update");
        if (wasUpdated) {
          notify("Zapret обновлен до последней версии!", "success");
        }
      } catch (e) {
        log("update err " + e);
      }
    };

    runChecks();
  }, []);
  return null;
};