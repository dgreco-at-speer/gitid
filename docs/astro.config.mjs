// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

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
            "guides/ai-agents",
          ],
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
