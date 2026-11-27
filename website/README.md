# Coven Threads documentation website

A static documentation site in the visual language of Coven Docs: near-black surfaces, lavender accents, Inter body text, EB Garamond display type, and JetBrains Mono code. The bundled Latin fonts are copied unchanged from `OpenCoven/coven-docs/app/fonts`; their complete SIL Open Font License notices are retained in `assets/fonts/`.

## Build and validate

```sh
cd website
npm ci
npm run build
npm run check
npm run preview
```

The preview listens on `127.0.0.1:18474` and supports the same clean URLs as Vercel. Stop it with Ctrl-C.

The build reads the selected canonical repository Markdown listed in `pages.json`. It does not duplicate or amend the authority contracts. Internal selected-page links stay on the site; links to other documents, specifications, code, and diagrams point to the exact GitHub source revision. `dist/provenance.json` records that revision. Historical status dates and acceptance limits remain in the source text.

The static output has no server functions, telemetry, provider keys, or remote font requests. Search runs locally against a generated index. Readers can switch themes, copy code, follow heading links, and use the mobile documentation menu. The page source links identify the contract owner.

## Vercel

Set the project's Root Directory to `website`, Framework Preset to Other, Build Command to `npm run build && npm run check`, and Output Directory to `dist`. The included `vercel.json` sets the same build and output contract, clean URLs, and response headers.

For a checkout deployment, invoke Vercel from the repository root with an existing project configured to use `website`, so its build retains access to the parent Markdown. Do not deploy the `website/` directory alone: canonical content lives in `../docs/`. Alternatively deploy the already-built `dist/` directory as static files with no build command.

```sh
npm ci --prefix website
npm run build --prefix website
npm run check --prefix website
```

Verify the live homepage, a nested documentation route, `search.json`, and `provenance.json` after deployment. A source build is not a deployment receipt. Rollback through the Vercel deployment dashboard to the preceding successful deployment; the site has no data migrations or daemon effects.

The generated `dist/vercel.json` carries clean URLs and headers for a prebuilt static deployment. The root source is not uploaded when deploying only `dist/`.
