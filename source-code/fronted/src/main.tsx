import { render } from "solid-js/web";
import App from "./App";
import "./styles/global.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Nie znaleziono elementu #root w index.html");
}

render(() => <App />, root);
