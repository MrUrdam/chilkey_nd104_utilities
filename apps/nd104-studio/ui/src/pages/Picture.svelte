<script lang="ts">
  // Own picture or GIF for the screen: pick, check, preview, send.
  import { onDestroy } from "svelte";
  import { api, onPictureProgress, type Fit, type PictureCheck, type Status } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import Icon from "../components/Icon.svelte";
  import UsbRequired from "../components/UsbRequired.svelte";

  let { status }: { status: Status } = $props();

  let fit = $state<Fit>("fill");
  let check = $state<PictureCheck | null>(null);
  let picking = $state(false);
  let sending = $state(false);
  let progress = $state(0);
  let result = $state<"ok" | string | null>(null);
  let error = $state<string | null>(null);

  // the picture goes straight to the screen controller: USB only
  let ready = $derived(status.device.kind === "ready" && status.device.connection === "usb");
  let usable = $derived(!!check && !check.problem);

  const kb = (bytes: number) => `${Math.ceil(bytes / 1024).toLocaleString()} KB`;

  // Plays a GIF in the preview with the frame times the screen will use.
  let frame = $state(0);
  $effect(() => {
    const c = check;
    frame = 0;
    if (!c || c.preview_frames.length < 2) return;
    let timer: ReturnType<typeof setTimeout>;
    let i = 0;
    const step = () => {
      timer = setTimeout(() => {
        i = (i + 1) % c.preview_frames.length;
        frame = i;
        step();
      }, Math.max(c.delays_ms[i] ?? 100, 20));
    };
    step();
    return () => clearTimeout(timer);
  });

  const unlisten = onPictureProgress((p) => (progress = p));
  onDestroy(() => {
    unlisten.then((f) => f());
  });

  async function pick() {
    picking = true;
    error = null;
    result = null;
    try {
      const c = await api.picturePick(fit);
      if (c) check = c;
    } catch (e) {
      error = String(e);
    } finally {
      picking = false;
    }
  }

  async function setFit(value: Fit) {
    fit = value;
    if (!check || check.problem?.code === "file_too_large" || check.problem?.code === "unsupported_format") return;
    try {
      check = await api.pictureRefit(fit);
      result = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function send() {
    sending = true;
    progress = 0;
    result = null;
    error = null;
    try {
      await api.pictureUpload();
      result = "ok";
    } catch (e) {
      result = String(e);
    } finally {
      sending = false;
    }
  }

  function problemText(c: PictureCheck): string {
    const p = c.problem!;
    switch (p.code) {
      case "file_too_large":
        return t("ui.pic.problem.file_too_large", { size: kb(p.bytes), max: kb(p.max) });
      case "too_many_frames":
        return t("ui.pic.problem.too_many_frames", { frames: p.frames, max: p.max });
      case "too_large_after_conversion":
        return t("ui.pic.problem.too_large_after_conversion", { size: kb(p.bytes), max: kb(p.max) });
      case "damaged":
        return t("ui.pic.problem.damaged", { detail: p.detail });
      default:
        return t("ui.pic.problem.unsupported_format");
    }
  }

  // One line per check: ok / warn (converted) / err
  type Row = { label: string; value: string; state: "ok" | "warn" | "err" };
  let rows = $derived.by<Row[]>(() => {
    if (!check) return [];
    const c = check;
    const p = c.problem?.code;
    const out: Row[] = [
      {
        label: t("ui.pic.check.type"),
        value: c.format ?? (p === "unsupported_format" ? t("ui.pic.unsupported") : "–"),
        state: p === "unsupported_format" || p === "damaged" ? "err" : "ok",
      },
      {
        label: t("ui.pic.check.file_size"),
        value: `${kb(c.file_bytes)} / ${kb(c.spec.max_file_bytes)}`,
        state: p === "file_too_large" ? "err" : "ok",
      },
    ];
    if (c.format || p === "too_many_frames") {
      out.push({
        label: t("ui.pic.check.frames"),
        value:
          p === "too_many_frames" && c.problem?.code === "too_many_frames"
            ? `${c.problem.frames} / ${c.spec.max_frames}`
            : `${c.frames} / ${c.spec.max_frames}`,
        state: p === "too_many_frames" ? "err" : "ok",
      });
    }
    if (c.format) {
      const exact = c.width === c.spec.width && c.height === c.spec.height;
      out.push({
        label: t("ui.pic.check.resolution"),
        value: exact
          ? `${c.width} × ${c.height}`
          : t("ui.pic.scaled", { from: `${c.width} × ${c.height}`, to: `${c.spec.width} × ${c.spec.height}` }),
        state: exact ? "ok" : "warn",
      });
      out.push({
        label: t("ui.pic.check.converted"),
        value: `${kb(c.payload_bytes)} / ${kb(c.spec.max_bytes)}`,
        state: "ok",
      });
    } else if (p === "too_large_after_conversion" && c.problem?.code === "too_large_after_conversion") {
      out.push({
        label: t("ui.pic.check.converted"),
        value: `${kb(c.problem.bytes)} / ${kb(c.spec.max_bytes)}`,
        state: "err",
      });
    }
    return out;
  });

  let notes = $derived(
    (check?.notes ?? []).filter((n) => n.code !== "scaled").map((n) => t(`ui.pic.note.${n.code}`)),
  );
</script>

<div class="stack">
  <h1>{t("ui.nav.picture")}</h1>
  <p class="muted intro">{t("ui.pic.intro")}</p>

  <div class="card stack">
    <div class="head">
      <h2>{t("ui.pic.choose_title")}</h2>
      <div class="segmented" role="group" aria-label={t("ui.pic.fit")}>
        <button aria-pressed={fit === "fill"} onclick={() => setFit("fill")}>{t("ui.pic.fit_fill")}</button>
        <button aria-pressed={fit === "contain"} onclick={() => setFit("contain")}>{t("ui.pic.fit_contain")}</button>
      </div>
    </div>

    <div class="row">
      <button class="key" onclick={pick} disabled={picking || sending}>
        {picking ? "…" : t("ui.pic.choose")}
      </button>
      <span class="muted small">{t("ui.pic.requirements", { max: 59 })}</span>
    </div>

    {#if error}<span class="error-text selectable">{error}</span>{/if}

    {#if check}
      <div class="result">
        <div class="preview">
          <div class="bezel">
            {#if check.preview_frames.length}
              <img src={check.preview_frames[frame]} alt={t("ui.pic.preview")} width="320" height="172" />
            {:else}
              <div class="empty"><Icon name="x" size={32} /></div>
            {/if}
          </div>
          <span class="muted small name selectable">{check.name}</span>
          {#if check.frames > 1}<span class="muted small">{t("ui.pic.animation", { frames: check.frames })}</span>{/if}
        </div>

        <dl class="checks">
          {#each rows as r (r.label)}
            <dt><span class="led {r.state}"></span>{r.label}</dt>
            <dd class:err={r.state === "err"}>{r.value}</dd>
          {/each}
        </dl>
      </div>

      {#if check.problem}
        <div class="notice err">{problemText(check)}</div>
      {/if}
      {#each notes as n (n)}
        <div class="notice">{n}</div>
      {/each}
    {/if}
  </div>

  <div class="card stack">
    <h2>{t("ui.pic.send_title")}</h2>
    <p class="muted small">{t("ui.pic.send_hint")}</p>
    {#if !ready}
      <UsbRequired feature="picture" via={status.device.kind === "ready" ? status.device.connection : null} />
    {/if}
    <div class="row">
      <button class="key primary" onclick={send} disabled={!usable || !ready || sending}>
        {sending ? t("ui.pic.sending") : t("ui.pic.send")}
      </button>
      {#if sending || result === "ok"}
        <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress * 100)}>
          <div class="fill" style="width: {Math.max(progress, 0.02) * 100}%"></div>
        </div>
        <span class="mono small">{Math.round(progress * 100)} %</span>
      {/if}
    </div>
    {#if result === "ok"}
      <div class="notice"><span class="led ok"></span>{t("ui.pic.done")}</div>
    {:else if result}
      <span class="error-text selectable">{result}</span>
    {/if}
  </div>
</div>

<style>
  .intro {
    margin: -6px 0 0;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .small {
    font-size: 12px;
  }

  p.small {
    margin: 0;
  }

  .result {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 20px;
    align-items: start;
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bezel {
    background: linear-gradient(180deg, #2b313b, #14171c);
    border: 1px solid #3b4350;
    border-radius: 14px;
    padding: 8px;
    width: fit-content;
  }

  .bezel img,
  .empty {
    display: block;
    width: 320px;
    height: 172px;
    border-radius: 8px;
    background: #000;
  }

  .empty {
    display: grid;
    place-items: center;
    color: var(--err);
    font-size: 32px;
  }

  .name {
    max-width: 336px;
    overflow-wrap: anywhere;
  }

  .checks {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 16px;
    margin: 0;
    font-size: 13px;
  }

  .checks dt {
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .checks dd {
    margin: 0;
  }

  .checks dd.err {
    color: var(--err);
  }

  .bar {
    flex: 1;
    min-width: 160px;
    height: 8px;
    border-radius: 999px;
    background: var(--panel-3);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease-out;
  }

  @media (max-width: 900px) {
    .result {
      grid-template-columns: 1fr;
    }
  }
</style>
