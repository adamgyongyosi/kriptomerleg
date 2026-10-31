import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { DialogHost } from "./Dialogs";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <DialogHost>
      <App />
    </DialogHost>
  </React.StrictMode>,
);
