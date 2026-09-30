# AgentHub — Spatial Capability Console

## 1. Experience principles

AgentHub should feel like a calm, native control surface for a high-impact local system. The interface follows five rules:

1. **Agency first** — current state, destructive scope, and the next action are visible before the user commits.
2. **One spatial model** — Canonical is the source; Cursor, Codex, and Claude Code are destinations. Every screen preserves that direction.
3. **Material, not decoration** — translucency distinguishes navigation, working surfaces, and modal layers. It is never used over dense text.
4. **Immediate response** — controls acknowledge pointer-down instantly; state changes use short, interruptible transitions.
5. **Quiet confidence** — generous space, strong type hierarchy, restrained color, and no dashboard noise.

## 2. Foundations

Runtime tokens in `src/styles/tokens.css` are canonical.

- Typography uses the operating-system UI stack (`SF Pro` where available) with optical sizing and a system monospace stack for paths and digests.
- The canvas is a cool neutral field with subtle ambient light. Content uses translucent white materials in light mode and graphite materials in dark mode.
- Blue is the single interaction accent. Green, amber, and red are reserved for semantic state.
- Geometry uses 10px controls, 18–28px surfaces, and fully rounded compact status indicators.
- Layering uses borders plus soft multi-stage shadows. Blur is applied only to floating navigation, sticky actions, dialogs, and large hero materials.

## 3. Layout and signature

Desktop uses a floating glass navigation rail whose width scales from 248px to 292px with the desktop viewport, and a centered, fluid content stage capped at 1680px. The rail's mark, type, spacing, and controls scale with the same wide-screen breakpoint so it remains proportionate when the window is maximized. High-information workspaces such as Target Sync use the full stage width; reading-heavy sections retain a narrower measure inside that stage. At narrow widths the navigation becomes a horizontal dock and content stacks.

The signature component is the **Canonical Flow**: a luminous source node, four capability streams, and three destination cards. It communicates inventory and target readiness without pretending to be an interactive diagram.

Initialization uses four quiet capability groups rather than one undifferentiated scrolling list. Group headers carry icon, name, selected/available count and group-level selection; rows allocate the flexible column to the full resource name, with source and path as secondary metadata. The sync Plan uses the same capability vocabulary and places named create/replace/delete chips above the lower-level file-path disclosure.

Host Resources uses the same four capability groups, with target tabs as the primary axis. Small semantic source pills use blue for a Canonical match, neutral graphite for host-only content, and amber for a protected host constraint. Destructive selection never relies on color alone: every row includes text, icon and control state, while protected rows retain their explanation in place.

Long synchronization pages keep the document as the primary scroll surface. Compact inner lists return boundary gestures to the page; no capability picker may trap upward navigation. A destructive empty-domain projection is represented as a named state such as `Clear Shared Agents Skills`, never inferred from an empty selection alone.

The AgentHub brand mark is a restrained graphite rounded square containing one continuous electric-blue orchestration symbol: a central Canonical core with three projection channels, whose negative space suggests an abstract “A”. It stays flat and legible at 16–32px; glow, circular arrows, robot-face geometry, decorative nodes and literal lettering are prohibited. The same raster source is used for the in-app brand, window icon, and generated package icon set. Capability stream lengths are data-bound to their displayed counts: the largest non-zero inventory occupies the full track, zero has no fill, and other domains scale proportionally.

## 4. Components

Shared owners are Button, Dialog, Toast, SearchField, StatusBadge, PageHeader, material cards, empty states, capability rows, target cards, and Canonical Flow.

- Controls have a minimum 44px target.
- Press feedback begins on pointer-down with a subtle scale/material shift.
- Destructive actions always use an in-app confirmation dialog.
- Non-interactive cards do not imitate buttons.
- Dense paths and diffs use opaque inset wells to preserve readability.

## 5. Motion and accessibility

- Hover/focus transitions: 140–180ms.
- Page and material entry: 280–420ms with a critically damped ease (`cubic-bezier(.22, 1, .36, 1)`).
- Exit/close motion is visually symmetric and fast.
- Motion never blocks input and no ornamental animation loops indefinitely.
- `prefers-reduced-motion` removes transform and blur animation.
- `prefers-reduced-transparency` replaces glass with opaque materials.
- `prefers-contrast: more` strengthens separators and focus rings.
- WCAG 2.2 AA contrast, visible keyboard focus, logical DOM order, a skip link, semantic landmarks, and live regions are required.

## 6. Responsive behavior

- ≥ 1100px: navigation rail plus multi-column content.
- ≥ 1600px: the content stage remains centered and Target Sync expands its capability summary and file-detail regions instead of leaving a large one-sided void.
- 720–1099px: compact rail/top dock, two-column cards where useful.
- < 720px: single-column content, horizontally scrollable navigation, full-width actions, and dialogs inset by 12px.
