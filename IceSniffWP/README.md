# IceSniffWP

Astro-based marketing website for IceSniff.

## Current state

- Built to closely mirror the Dragon app website structure and pacing.
- Rebranded for IceSniff using the repository's existing IceSniff icons and glacier artwork.
- Main product message is centered on:
  - native macOS packet analysis
  - live capture
  - packet detail and analysis views
  - right-side AI rail

## Project structure

- `src/pages/index.astro`:
  main page markup
- `src/styles/global.css`:
  full page styling
- `public/app.js`:
  hero toggles, header state, reveal-on-scroll behavior
- `public/assets/`:
  site assets actually used by the Astro page

## Commands

```bash
cd IceSniffWP
npm install
npm run dev
npm run build
```

## Verified state on 2026-04-08

- `npm install` completed
- `npm run build` completed successfully

## Important note for next session

This folder started as a plain static site and was then converted into Astro.

The old `assets/` folder at the project root is leftover from that first pass and is not needed anymore.
The actual live Astro assets are in `public/assets/`.

## Suggested next cleanup

1. Remove the unused root `assets/` folder.
2. Run the Astro dev server and do a visual browser pass.
3. If we keep expanding the page, split `index.astro` into smaller Astro components.
