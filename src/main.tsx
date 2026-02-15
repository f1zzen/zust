import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

const rootElement = document.getElementById("root") as HTMLElement;
const root = ReactDOM.createRoot(rootElement);

root.render(
  <App />
);