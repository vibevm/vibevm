// spec://vibeterm/PROP-046#overview (the Solid chrome — one projection of the render-free engine)
import { render } from "solid-js/web";
import { App } from "./App";

const root = document.getElementById("app");
if (root) render(() => <App />, root);
