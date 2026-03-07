/**
 * @license
 * copyright 2026 a7mddra
 * spdx-license-identifier: apache-2.0
 */

import React from "react";
import ReactDOM from "react-dom/client";
import App from "@/App";

import "@/styles/fonts.css";
import "@/styles/variables.css";
import "@/styles/animations.css";
import "@/styles/globals.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
