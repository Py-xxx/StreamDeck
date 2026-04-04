import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Disable context menu globally (except on text inputs)
document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement;
  const isTextInput = 
    target.tagName === "INPUT" && 
    (target.getAttribute("type") === "text" || 
     target.getAttribute("type") === "number" ||
     target.getAttribute("type") === "password") ||
    target.tagName === "TEXTAREA" ||
    target.isContentEditable;
  
  if (!isTextInput) {
    e.preventDefault();
    return false;
  }
});

// Prevent drag and drop
document.addEventListener("dragstart", (e) => {
  e.preventDefault();
  return false;
});

// Disable F5 refresh and Ctrl+R
document.addEventListener("keydown", (e) => {
  if (
    e.key === "F5" ||
    (e.ctrlKey && e.key === "r") ||
    (e.metaKey && e.key === "r")
  ) {
    e.preventDefault();
    return false;
  }
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
