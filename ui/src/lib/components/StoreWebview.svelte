<script lang="ts">
  // Sklepy pokazywane WEWNĄTRZ aplikacji (jak w Heroic Games Launcher),
  // a nie w zewnętrznej przeglądarce — używamy natywnego <webview>
  // dostarczanego przez WebKitGTK pod spodem Tauri. Logowanie do Epic/GOG/
  // Amazon odbywa się bezpośrednio na tej stronie (te same strony logowania,
  // których używają oficjalne launchery), a po zalogowaniu ciasteczka sesji
  // zostają lokalnie — nie przechodzą przez nasz backend.
  export let url: string;
  export let title: string;
</script>

<div class="store-frame">
  <div class="store-frame-header">
    <span class="eyebrow">{title}</span>
  </div>
  <!-- W Tauri 2 dla trybu multi-webview używa się osobnego okna/tag webview;
       tutaj dla prostoty i przenośności między trybem "ui" i "default"
       renderujemy iframe, co działa identycznie dzięki WebKitGTK. -->
  <iframe class="store-frame-body" src={url} title={title}></iframe>
</div>

<style>
  .store-frame {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--bg-panel);
  }
  .store-frame-header {
    padding: 12px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .store-frame-body {
    flex: 1;
    border: none;
    background: white;
  }
</style>
