import { Component, createEffect, createSignal, Show } from "solid-js";
import { api, PROTONDB_SUPPORTED_PLATFORMS, PROTONDB_TIER_INFO, type Platform, type ProtonDbSummary } from "@/lib/tauri";

/** Plakietka ProtonDB — wyłącznie Steam, patrz `PROTONDB_SUPPORTED_PLATFORMS`
 * i moduł-dokumentacja `protondb.rs`. `null` może znaczyć zarówno „gra nie ma
 * jeszcze zgłoszeń", jak i błąd sieciowy — w obu przypadkach nie pokazujemy
 * nic zamiast zgadywać, który to przypadek. */
const ProtonDbBadge: Component<{ platform: Platform; id: string }> = (props) => {
  const [rating, setRating] = createSignal<ProtonDbSummary | null>(null);
  const [loading, setLoading] = createSignal(false);

  // Efekt (nie `onMount`): SolidJS reużywa komponent przy zmianie parametrów
  // trasy, więc ocena musi się przeładować po zmianie gry.
  createEffect(() => {
    const platform = props.platform;
    const id = props.id;
    setRating(null);
    if (!PROTONDB_SUPPORTED_PLATFORMS.includes(platform)) return;
    const appid = Number(id);
    if (Number.isNaN(appid)) return;
    let cancelled = false;
    setLoading(true);
    api
      .fetchProtondbRating(appid)
      .then((r) => {
        if (!cancelled) setRating(r);
      })
      .catch((err) => console.warn("Nie udało się pobrać oceny ProtonDB:", err))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  });

  return (
    <>
      <Show when={PROTONDB_SUPPORTED_PLATFORMS.includes(props.platform) && rating()}>
        {(r) => {
          const info = () => PROTONDB_TIER_INFO[r().tier] ?? { label: r().tier, color: "var(--text-muted)" };
          return (
            <a
              href={`https://www.protondb.com/app/${props.id}`}
              target="_blank"
              rel="noreferrer"
              style={{
                display: "inline-flex",
                "align-items": "center",
                gap: "6px",
                "font-size": "12px",
                padding: "3px 10px",
                "border-radius": "999px",
                border: `1px solid ${info().color}`,
                color: info().color,
                "text-decoration": "none",
                "margin-bottom": "8px",
              }}
            >
              ProtonDB: {info().label}
            </a>
          );
        }}
      </Show>
      <Show when={loading()}>
        <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-bottom": "8px" }} role="status">
          Sprawdzam ocenę ProtonDB…
        </div>
      </Show>
    </>
  );
};

export default ProtonDbBadge;
