// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";

// The deployment target is not decided yet. When a host is chosen, set
// DOCS_SITE / DOCS_BASE in the deploy environment, e.g.
//   DOCS_SITE=https://example.github.io DOCS_BASE=/gitid
export default defineConfig({
  site: process.env.DOCS_SITE,
  base: process.env.DOCS_BASE,
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
