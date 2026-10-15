import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/base.css";

async function boot() {
  // Outside Tauri (a plain browser on the Vite dev server), stand in a fake
  // backend so the interface can be worked on and screenshotted without the
  // service or a headset. Never part of a release build.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMock } = await import("./mock");
    installMock(new URLSearchParams(location.search).get("scenario") ?? "ready");
  }
  createApp(App).use(createPinia()).mount("#app");
}

void boot();
