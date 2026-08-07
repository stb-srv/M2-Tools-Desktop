/// <reference types="vite/client" />

// Vite's own client types cover known asset extensions but not arbitrary
// `?raw` imports of a `.md` file - used by the Quest-Wiki to bundle the
// copied documentation pages as plain strings at build time.
declare module "*.md?raw" {
  const content: string;
  export default content;
}
