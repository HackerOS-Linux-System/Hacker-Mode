import { onCleanup } from "solid-js";

/**
 * Nawigacja padem i klawiaturą dla trybu big-picture / TTY, gdzie myszka
 * często nie jest dostępna (albo niewygodna z kanapy). Elementy oznaczone
 * atrybutem `data-focusable` (patrz `useFocusable`) tworzą siatkę, po
 * której można się poruszać strzałkami/D-padem, aktywować przyciskiem
 * Enter/A, i cofać Escape/B.
 *
 * Nawigacja przestrzenna: dla każdego kierunku szukamy wśród pozostałych
 * elementów tego, który leży "w tamtą stronę" względem aktualnie
 * zaznaczonego (na osi głównej) i jest najbliżej na osi drugorzędnej —
 * to standardowe podejście używane w nawigacji telewizorów/konsol.
 */

type Direction = "up" | "down" | "left" | "right";

function focusableElements(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>("[data-focusable]")).filter(
    (el) => el.offsetParent !== null, // pomijamy niewidoczne (np. w ukrytych zakładkach)
  );
}

function currentFocused(): HTMLElement | null {
  const active = document.activeElement;
  return active instanceof HTMLElement && active.hasAttribute("data-focusable") ? active : null;
}

function moveFocus(direction: Direction) {
  const all = focusableElements();
  if (all.length === 0) return;

  const current = currentFocused();
  if (!current) {
    all[0].focus();
    return;
  }

  const currentRect = current.getBoundingClientRect();
  const cx = currentRect.left + currentRect.width / 2;
  const cy = currentRect.top + currentRect.height / 2;

  let best: HTMLElement | null = null;
  let bestScore = Infinity;

  for (const el of all) {
    if (el === current) continue;
    const rect = el.getBoundingClientRect();
    const ex = rect.left + rect.width / 2;
    const ey = rect.top + rect.height / 2;
    const dx = ex - cx;
    const dy = ey - cy;

    let primary: number;
    let secondary: number;
    switch (direction) {
      case "up":
        if (dy >= -4) continue;
        primary = -dy;
        secondary = Math.abs(dx);
        break;
      case "down":
        if (dy <= 4) continue;
        primary = dy;
        secondary = Math.abs(dx);
        break;
      case "left":
        if (dx >= -4) continue;
        primary = -dx;
        secondary = Math.abs(dy);
        break;
      case "right":
        if (dx <= 4) continue;
        primary = dx;
        secondary = Math.abs(dy);
        break;
    }

    // Waga: głównie odległość na osi ruchu, lekka kara za odchylenie w bok
    // (współczynnik 1.5 dobrany empirycznie — faworyzuje sąsiadów "w linii").
    const score = primary + secondary * 1.5;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }

  (best ?? all[0]).focus();
  best?.scrollIntoView({ block: "nearest", behavior: "smooth" });
}

function activateFocused() {
  const current = currentFocused();
  if (!current) return;
  current.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
}

const KEY_TO_DIRECTION: Record<string, Direction> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

function onKeyDown(e: KeyboardEvent) {
  const direction = KEY_TO_DIRECTION[e.key];
  if (direction) {
    e.preventDefault();
    moveFocus(direction);
    return;
  }
  if (e.key === "Enter" || e.key === " ") {
    // Enter/Space na elemencie, który sam nie ma swojego handlera (np. div
    // karty), aktywuje go — elementy z własnym `onKeyDown`/`<button>`
    // i tak obsłużą to natywnie wcześniej.
  }
}

// --- Obsługa pada (Gamepad API) -----------------------------------------

const AXIS_DEADZONE = 0.5;
const REPEAT_DELAY_MS = 220;
let lastMoveAt = 0;
// Śledzimy poprzedni stan przycisków per-pad (indeks w `navigator.getGamepads()`),
// żeby wykryć "naciśnięcie" (zbocze narastające) niezależnie dla każdego
// podłączonego kontrolera — więcej niż jedna osoba może sterować menu,
// przydatne np. gdy ktoś testuje pada, a hostuje z klawiatury.
const lastButtonsByPad = new Map<number, boolean[]>();
let rafHandle: number | null = null;

function pollGamepad() {
  const pads = navigator.getGamepads?.() ?? [];

  for (const pad of pads) {
    if (!pad) continue;

    const now = performance.now();
    const axisX = pad.axes[0] ?? 0;
    const axisY = pad.axes[1] ?? 0;
    const dpadUp = pad.buttons[12]?.pressed ?? false;
    const dpadDown = pad.buttons[13]?.pressed ?? false;
    const dpadLeft = pad.buttons[14]?.pressed ?? false;
    const dpadRight = pad.buttons[15]?.pressed ?? false;

    if (now - lastMoveAt > REPEAT_DELAY_MS) {
      let direction: Direction | null = null;
      if (dpadUp || axisY < -AXIS_DEADZONE) direction = "up";
      else if (dpadDown || axisY > AXIS_DEADZONE) direction = "down";
      else if (dpadLeft || axisX < -AXIS_DEADZONE) direction = "left";
      else if (dpadRight || axisX > AXIS_DEADZONE) direction = "right";

      if (direction) {
        moveFocus(direction);
        lastMoveAt = now;
      }
    }

    const previous = lastButtonsByPad.get(pad.index) ?? [];
    const aPressed = pad.buttons[0]?.pressed ?? false;
    const bPressed = pad.buttons[1]?.pressed ?? false;
    if (aPressed && !previous[0]) activateFocused();
    if (bPressed && !previous[1]) {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    }
    lastButtonsByPad.set(
      pad.index,
      pad.buttons.map((b) => b.pressed),
    );
  }

  rafHandle = requestAnimationFrame(pollGamepad);
}

let installed = false;

/** Wywołaj raz (np. w `App.tsx`) po zamontowaniu aplikacji. */
export function installNavigation() {
  if (installed) return;
  installed = true;
  window.addEventListener("keydown", onKeyDown);
  rafHandle = requestAnimationFrame(pollGamepad);

  onCleanup(() => {
    window.removeEventListener("keydown", onKeyDown);
    if (rafHandle) cancelAnimationFrame(rafHandle);
    installed = false;
  });
}

/**
 * Hook Solid do rejestrowania elementu jako punktu nawigacji. Podpina
 * `Enter`/`Space` do przekazanego callbacku (klawiatura — gamepad korzysta
 * z tego samego handlera przez syntetyczny `KeyboardEvent`, patrz
 * `activateFocused`).
 */
export function useFocusable<T extends HTMLElement>(onActivate?: () => void) {
  const ref = (el: T) => {
    el.tabIndex = 0;
    el.setAttribute("data-focusable", "true");
    if (onActivate) {
      el.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onActivate();
        }
      });
    }
  };
  return { ref };
}
