// @ts-check
import { readFileSync } from "node:fs";
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";

// The crate version is the single source of truth for the version shown in the
// docs. Read it from Cargo.toml at build time and substitute the literal token
// `%GITID_VERSION%` wherever it appears in content, so no version number is
// hand-maintained here. The `^version = "…"` anchor matches the [package]
// version, not the un-anchored `version =` entries under [dependencies].
const gitidVersion = readFileSync(
  new URL("../Cargo.toml", import.meta.url),
  "utf8",
).match(/^version = "(.+?)"/m)?.[1];
if (!gitidVersion) throw new Error("could not read version from ../Cargo.toml");

// Tiny remark plugin: replace `%GITID_VERSION%` with the crate version in text
// and code nodes. Works uniformly for .md and .mdx (the `%…%` token is never
// parsed as an MDX `{expr}`), so no per-file conversion is needed.
function remarkGitidVersion() {
  return (tree) => {
    const visit = (node) => {
      if (
        (node.type === "text" ||
          node.type === "code" ||
          node.type === "inlineCode") &&
        typeof node.value === "string"
      ) {
        node.value = node.value.replaceAll("%GITID_VERSION%", gitidVersion);
      }
      if (Array.isArray(node.children)) node.children.forEach(visit);
    };
    visit(tree);
  };
}

// The deployment target is not decided yet. When a host is chosen, set
// DOCS_SITE / DOCS_BASE in the deploy environment, e.g.
//   DOCS_SITE=https://example.github.io DOCS_BASE=/gitid
export default defineConfig({
  site: process.env.DOCS_SITE,
  base: process.env.DOCS_BASE,
  markdown: { remarkPlugins: [remarkGitidVersion] },
  integrations: [
    starlight({
      title: "gitid",
      description:
        "Switch between git identities — name/email, SSH key, commit signing, GitHub CLI auth — automatically, per directory tree.",
      plugins: [
        // Generate /llms.txt, /llms-full.txt and /llms-small.txt for LLM
        // consumers. These links need an absolute site URL, so only register
        // the plugin when the deploy host is configured (DOCS_SITE) — matching
        // the site/base env-var pattern above. Without DOCS_SITE (CI/local PR
        // builds) the plugins array is empty and behaviour is unchanged.
        ...(process.env.DOCS_SITE ? [starlightLlmsTxt()] : []),
      ],
      components: {
        // Override the splash hero to render the animated bian lian octocat
        // mascot when a page declares no `hero.image`.
        Hero: "./src/components/Hero.astro",
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/dgreco-at-speer/gitid",
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            "getting-started/how-it-works",
            "getting-started/installation",
            "getting-started/quickstart",
          ],
        },
        {
          label: "Guides",
          items: [
            "guides/profiles",
            "guides/directory-mappings",
            "guides/shell-integration",
            "guides/ssh-keys",
            "guides/commit-signing",
            "guides/github-cli",
            "guides/prompt-integration",
            "guides/self-update",
          ],
        },
        {
          label: "AI",
          items: ["ai/overview", "ai/mcp-server", "ai/agent-skills"],
        },
        {
          label: "Reference",
          items: [
            {
              label: "Commands",
              collapsed: true,
              items: [{ autogenerate: { directory: "reference/commands" } }],
            },
            "reference/profiles-toml",
            "reference/files",
            "reference/environment-variables",
            "reference/exit-codes",
          ],
        },
        {
          label: "Troubleshooting",
          items: [
            "troubleshooting/common-issues",
            "troubleshooting/how-resolution-works",
          ],
        },
      ],
    }),
  ],
});
