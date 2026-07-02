import { writable } from "svelte/store";

export type Route = "library" | "store" | "settings";

export const currentRoute = writable<Route>("library");
export const hackerMenuOpen = writable(false);
