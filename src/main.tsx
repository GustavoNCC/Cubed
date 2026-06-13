import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Cyberpunk theme is always dark — ensure the class is set before first render
document.documentElement.classList.add("dark");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
