# Aegis — Brand & Design System

Source of truth for the Aegis visual language. The design Claude (and any
human designers) should read this top-to-bottom before producing UI.

## Direction

**Vibe:** Quiet observatory. Radar console crossed with a financial almanac.
Calm by default, decisive when it matters. *Not* crypto-purple, *not*
Bloomberg-dense, *not* newspaper.

**Tagline / pull-quote:** *"Quiet, until it isn't."*

**Tone of voice:** Plainspoken, technical, never breathless. Numbers do the
talking. Italic serif for emphasis ("watchful by design"), monospace for
metrics, sans for UI chrome.

## Logo — Sonar (variant 1, "OG")

Concentric half-arcs over a horizon line, red dot at the origin, innermost
arc in cobalt. This is the chosen mark — use it everywhere there's a
brand surface (sidebar, favicon, loading states, empty states, splash).

The mark is implemented at `frontend/src/components/SonarLogo.tsx`. Use
`<SonarLogo size={36} />` for the canonical size, `min-size: 16px`.
Construction grid: 8pt.

**Wordmark:** "Aeg*i*s" — set in Fraunces 500, the `i` swapped to Instrument
Serif italic in rust (`#C44536`). Component: `<AegisWord />` (to be built
alongside SonarLogo).

## Color tokens

CSS variables live at `frontend/src/index.css`. Use these names, not raw hex.

```
--ink:        #1A1A1A   primary text, primary stroke
--ink-2:      #2A2520   secondary text
--paper:      #F6F4EE   page background (light surfaces)
--paper-2:    #EFEBE0   raised surfaces, side rails
--sand:       #D9CDB4   muted fills, soft chips
--sand-2:     #C9BB9C   warning fills (less than rust)
--cobalt:     #3B5BDB   primary accent, brand arc, primary CTA fill
--cobalt-ink: #2A3FA8   cobalt hover/border
--rust:       #C44536   alert / critical / italic accent / dot
--rust-ink:   #9B3327   rust hover/border
--moss:       #5A6B47   positive deltas, healthy state
--line:       rgba(26,26,26,.18)   hairlines
--line-soft:  rgba(26,26,26,.10)   row dividers
```

**Palette intent:** rust is *only* for danger and the brand-italic `i`.
Cobalt is *only* for primary action and the inner arc. Moss is *only* for
healthy/positive. Don't decorate with these.

A dark-mode variant ("Graphite Console": carbon `#0E1116` paper, bone
`#F0EDE4` ink, accent `#7C8CFF`, dot `#FF7A59`) is approved but **out of
scope for v1** — ship light first.

## Typography

```
--serif:        'Fraunces', 'Times New Roman', serif       display, headings, stat values
--serif-italic: 'Instrument Serif', serif                  emphasis, taglines, the "i" in Aegis
--sans:         'Geist', system-ui, sans-serif             UI chrome, body
--mono:         'Geist Mono', ui-monospace, monospace      numbers, labels, eyebrows
```

**Numbers:** always `font-variant-numeric: tabular-nums` (or font-feature
`'tnum' 1`). Dollar amounts and health scores use mono. Wallet pubkeys,
mint hashes, intent ids → mono.

**Eyebrows:** mono, 11px, `letter-spacing: 0.16em`, uppercase, muted.

**Display heading:** Fraunces 500, 46–84px, `letter-spacing: -0.02em`,
`line-height: 1`. Single italic word per heading max, in rust.

## Visual grammar

The Sonar geometry — concentric half-arcs, horizon rule, central dot — is
the recurring motif. Use it for:
- Health gauges (half-circle arc with needle on the same geometry)
- Empty states (large faint sonar watermark)
- Loading (a single arc that "sweeps")
- Section dividers (horizon rule with a small dot at the midpoint)
- Background watermarks on hero surfaces (5% opacity, oversized)

Hairlines are 1px `var(--line)`. Hard rules are 1.5px `var(--ink)`.
Borders are square (2px max radius) — never pill-shaped except chips.

## Motion (for the design Claude)

Calm, never showy. Targets:
- Page transitions: 150ms cross-fade.
- Section reveals on scroll: 400ms, ease-out, `opacity 0→1, y 24→0`.
- Number count-up on first scroll-into-view: 600ms, once.
- Sonar arc draw-in (logo, gauges, charts): 700ms `pathLength` 0→1.
- Critical states pulse softly (rust dot, 1.6s ease-in-out, `0.6→1.0` opacity).
- Hover: never bigger than 4px translate or 1.02 scale.

No parallax. One sticky-pin moment max on the landing page.

## Stack assumptions

- Tailwind + CSS variables (variables are authoritative; tw utilities
  reference them via `[--var]` arbitrary values where needed).
- `framer-motion` for component motion, `tailwindcss-animate` for utility
  transitions, `lucide-react` for utility icons (never for the brand mark).
- Charts: lightweight SVG drawn by hand (Recharts is OK if needed; keep
  styling overridden to match the Sonar grammar — no default purple, no
  rounded line caps unless specified).

## Out of scope / explicit nos

- No emoji in UI copy.
- No rainbow or purple gradients. No glassmorphism.
- No drop shadows except the lightest hairline (`0 1px 0 var(--line)`).
- No Inter (replaced by Geist), no JetBrains Mono (replaced by Geist Mono).
- No character illustrations — geometric motifs only.
