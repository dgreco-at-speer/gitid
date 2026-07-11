//! The rmcp server definition and its tools. Each tool is a thin wrapper over
//! the domain layer; results are returned as pretty-printed JSON text so agents
//! get a stable, machine-parseable contract.

use std::collections::BTreeMap;

use anyhow::Result;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData as McpError, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cmd::{Ctx, resolve_dir};
use crate::paths::{GitidPaths, PathStyle, normalize_dir};
use crate::store::mappings::{Mapping, MappingsFile, match_dir};
use crate::store::profiles::{self, Gh, Profile, Signing, SigningFormat, Ssh};
use crate::sync::{SyncReport, sync_all};

/// Start the server on stdio and run until the client disconnects.
pub async fn run(paths: GitidPaths) -> Result<()> {
    use anyhow::Context as _;
    let service = GitidServer::new(paths)
        .serve(rmcp::transport::stdio())
        .await
        .context("failed to start the MCP server")?;
    service.waiting().await.context("MCP server error")?;
    Ok(())
}

#[derive(Clone)]
pub struct GitidServer {
    paths: GitidPaths,
    tool_router: ToolRouter<Self>,
}

// ---- parameter types -------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NameParams {
    /// The profile's slug (the key in profiles.toml).
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirParams {
    /// Absolute directory to inspect. Defaults to the server's working directory.
    #[serde(default)]
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UseParams {
    /// The profile to assign to the directory tree.
    pub profile: String,
    /// Directory to assign. Defaults to the server's working directory.
    #[serde(default)]
    pub dir: Option<String>,
    /// Force case-insensitive matching. Defaults to the platform convention.
    #[serde(default)]
    pub case_insensitive: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddParams {
    /// New profile slug (lowercase letters, digits, '-', '_').
    pub name: String,
    /// git user.name.
    pub git_name: String,
    /// git user.email.
    pub email: String,
    /// Path to the SSH private key (a leading `~` is allowed).
    #[serde(default)]
    pub ssh_key: Option<String>,
    /// Use a key from the running ssh-agent instead of a key file: a SHA256:
    /// fingerprint (prefix ok) or a key comment. Mutually exclusive with ssh_key.
    #[serde(default)]
    pub ssh_agent_key: Option<String>,
    /// Commit-signing method: "ssh", "openpgp", or "none".
    #[serde(default)]
    pub signing: Option<String>,
    /// Signing key: public-key path (ssh), key id (openpgp), or "agent" to sign
    /// with the profile's ssh-agent key.
    #[serde(default)]
    pub signing_key: Option<String>,
    /// Sign commits by default.
    #[serde(default)]
    pub sign_commits: bool,
    /// Provision an isolated GH_CONFIG_DIR for this profile (default: true).
    #[serde(default)]
    pub gh: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditParams {
    /// The profile to modify.
    pub name: String,
    /// New git user.name.
    #[serde(default)]
    pub git_name: Option<String>,
    /// New git user.email.
    #[serde(default)]
    pub email: Option<String>,
    /// New SSH private-key path.
    #[serde(default)]
    pub ssh_key: Option<String>,
    /// Switch to a key from the running ssh-agent: a SHA256: fingerprint
    /// (prefix ok) or a key comment. Mutually exclusive with ssh_key.
    #[serde(default)]
    pub ssh_agent_key: Option<String>,
    /// New signing method: "ssh", "openpgp", or "none".
    #[serde(default)]
    pub signing: Option<String>,
    /// New signing key (required when switching to ssh/openpgp without one set).
    #[serde(default)]
    pub signing_key: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveParams {
    /// The profile to delete.
    pub name: String,
    /// Remove even if directories map to it (and drop those mappings).
    #[serde(default)]
    pub force: bool,
}

// ---- result types ----------------------------------------------------------

#[derive(Serialize)]
struct CurrentResult {
    profile: Option<String>,
    email: Option<String>,
    dir: Option<String>,
}

#[derive(Serialize)]
struct MutationResult<T: Serialize> {
    ok: bool,
    detail: T,
    sync: SyncReport,
}

// ---- tools -----------------------------------------------------------------

#[tool_router]
impl GitidServer {
    pub fn new(paths: GitidPaths) -> Self {
        Self {
            paths,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List all gitid profiles (identities) with their git name, email, and gh setting."
    )]
    async fn gitid_list(&self) -> Result<CallToolResult, McpError> {
        let profiles = profiles::load(&self.paths.profiles_toml()).map_err(internal)?;
        json(&profiles)
    }

    #[tool(description = "Show one gitid profile's full details.")]
    async fn gitid_show(
        &self,
        Parameters(p): Parameters<NameParams>,
    ) -> Result<CallToolResult, McpError> {
        let profiles = profiles::load(&self.paths.profiles_toml()).map_err(internal)?;
        let profile = profiles
            .profiles
            .get(&p.name)
            .ok_or_else(|| invalid(format!("no such profile: {}", p.name)))?;
        json(profile)
    }

    #[tool(description = "List every directory→profile mapping.")]
    async fn gitid_dirs(&self) -> Result<CallToolResult, McpError> {
        let mappings = MappingsFile::load(&self.paths.mappings_toml()).map_err(internal)?;
        json(&mappings)
    }

    #[tool(
        description = "Show which profile is active for a directory (defaults to the working directory)."
    )]
    async fn gitid_current(
        &self,
        Parameters(p): Parameters<DirParams>,
    ) -> Result<CallToolResult, McpError> {
        let abs = resolve_dir(p.dir.as_deref(), &self.paths.home).map_err(internal)?;
        let mappings = MappingsFile::load(&self.paths.mappings_toml()).map_err(internal)?;
        let matched = match_dir(&mappings.mappings, &abs, PathStyle::host());
        let result = match matched {
            None => CurrentResult {
                profile: None,
                email: None,
                dir: None,
            },
            Some(m) => {
                let profiles = profiles::load(&self.paths.profiles_toml()).map_err(internal)?;
                CurrentResult {
                    profile: Some(m.profile.clone()),
                    email: profiles.profiles.get(&m.profile).map(|pr| pr.email.clone()),
                    dir: Some(m.dir.clone()),
                }
            }
        };
        json(&result)
    }

    #[tool(
        description = "Diagnose gitid configuration problems (git version, bootstrap, stale artifacts, mappings, ssh keys, gh, hook)."
    )]
    async fn gitid_doctor(
        &self,
        Parameters(p): Parameters<DirParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.ctx();
        let report = crate::cmd::doctor::collect(&ctx, p.dir.as_deref()).map_err(internal)?;
        json(&DoctorResult {
            failed: report.failed(),
            findings: report.findings,
        })
    }

    #[tool(
        description = "Assign a profile to a directory tree (like `gitid use`). Regenerates all derived config."
    )]
    async fn gitid_use(
        &self,
        Parameters(p): Parameters<UseParams>,
    ) -> Result<CallToolResult, McpError> {
        let profiles = profiles::load(&self.paths.profiles_toml()).map_err(internal)?;
        if !profiles.profiles.contains_key(&p.profile) {
            return Err(invalid(format!("no such profile: {}", p.profile)));
        }

        let abs = resolve_dir(p.dir.as_deref(), &self.paths.home).map_err(internal)?;
        let canonical = std::fs::canonicalize(&abs)
            .map_err(|e| invalid(format!("directory does not exist: {} ({e})", abs.display())))?;

        let style = PathStyle::host();
        let canonical_norm = normalize_dir(&canonical.to_string_lossy(), style);
        let literal_norm = normalize_dir(&abs.to_string_lossy(), style);
        let dir_literal = (literal_norm != canonical_norm).then_some(literal_norm);
        let case_insensitive = p.case_insensitive.unwrap_or_else(|| style.default_icase());

        let mut mappings = MappingsFile::load(&self.paths.mappings_toml()).map_err(internal)?;
        mappings.upsert(Mapping {
            dir: canonical_norm.clone(),
            dir_literal,
            profile: p.profile.clone(),
            case_insensitive,
            env: BTreeMap::new(),
        });
        mappings
            .save(&self.paths.mappings_toml())
            .map_err(internal)?;

        let report = sync_all(&self.paths).map_err(internal)?;
        json(&MutationResult {
            ok: true,
            detail: format!(
                "{} now uses profile {:?}",
                canonical_norm.trim_end_matches('/'),
                p.profile
            ),
            sync: report,
        })
    }

    #[tool(description = "Remove the mapping for a directory (like `gitid forget`).")]
    async fn gitid_forget(
        &self,
        Parameters(p): Parameters<DirParams>,
    ) -> Result<CallToolResult, McpError> {
        let abs = resolve_dir(p.dir.as_deref(), &self.paths.home).map_err(internal)?;
        let dir_norm = match std::fs::canonicalize(&abs) {
            Ok(c) => normalize_dir(&c.to_string_lossy(), PathStyle::host()),
            Err(_) => normalize_dir(&abs.to_string_lossy(), PathStyle::host()),
        };
        let literal_norm = normalize_dir(&abs.to_string_lossy(), PathStyle::host());

        let mut mappings = MappingsFile::load(&self.paths.mappings_toml()).map_err(internal)?;
        let removed = mappings.remove_dir(&dir_norm) || mappings.remove_dir(&literal_norm);
        if !removed {
            return Err(invalid(format!(
                "no mapping for {}",
                dir_norm.trim_end_matches('/')
            )));
        }
        mappings
            .save(&self.paths.mappings_toml())
            .map_err(internal)?;

        let report = sync_all(&self.paths).map_err(internal)?;
        json(&MutationResult {
            ok: true,
            detail: format!("forgot {}", dir_norm.trim_end_matches('/')),
            sync: report,
        })
    }

    #[tool(
        description = "Create a new profile (like `gitid add`). Fully parameterized; no prompting."
    )]
    async fn gitid_add(
        &self,
        Parameters(p): Parameters<AddParams>,
    ) -> Result<CallToolResult, McpError> {
        profiles::validate_name(&p.name).map_err(invalid_from)?;
        let path = self.paths.profiles_toml();
        let existing = profiles::load(&path).map_err(internal)?;
        if existing.profiles.contains_key(&p.name) {
            return Err(invalid(format!(
                "profile {:?} already exists; use gitid_edit",
                p.name
            )));
        }

        let signing = build_signing(p.signing.as_deref(), p.signing_key.clone(), p.sign_commits)
            .map_err(invalid_from)?;
        let ssh = match (&p.ssh_key, &p.ssh_agent_key) {
            (Some(_), Some(_)) => {
                return Err(invalid("ssh_key and ssh_agent_key are mutually exclusive"));
            }
            (Some(key), None) => Some(Ssh::from_path(key.clone())),
            (None, Some(selector)) => {
                crate::agent::validate_selector(selector).map_err(invalid_from)?;
                Some(Ssh::from_agent(selector.clone()))
            }
            (None, None) => None,
        };
        let profile = Profile {
            name: p.git_name.clone(),
            email: p.email.clone(),
            ssh,
            signing,
            gh: p.gh.unwrap_or(true).then_some(Gh { enabled: true }),
            env: BTreeMap::new(),
            extra: BTreeMap::new(),
        };

        let mut doc = profiles::load_doc(&path).map_err(internal)?;
        profiles::upsert_profile(&mut doc, &p.name, &profile).map_err(internal)?;
        profiles::save_doc(&path, &doc).map_err(internal)?;

        let report = sync_all(&self.paths).map_err(internal)?;
        json(&MutationResult {
            ok: true,
            detail: profile,
            sync: report,
        })
    }

    #[tool(
        description = "Modify an existing profile's fields (like `gitid edit`). Only provided fields change."
    )]
    async fn gitid_edit(
        &self,
        Parameters(p): Parameters<EditParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self.paths.profiles_toml();
        let mut file = profiles::load(&path).map_err(internal)?;
        let profile = file
            .profiles
            .get_mut(&p.name)
            .ok_or_else(|| invalid(format!("no such profile: {}", p.name)))?;

        if let Some(v) = &p.git_name {
            profile.name = v.clone();
        }
        if let Some(v) = &p.email {
            profile.email = v.clone();
        }
        if p.ssh_key.is_some() && p.ssh_agent_key.is_some() {
            return Err(invalid("ssh_key and ssh_agent_key are mutually exclusive"));
        }
        if let Some(v) = &p.ssh_key {
            profile.ssh = Some(Ssh::from_path(v.clone()));
        }
        if let Some(v) = &p.ssh_agent_key {
            crate::agent::validate_selector(v).map_err(invalid_from)?;
            profile.ssh = Some(Ssh::from_agent(v.clone()));
        }
        if let Some(kind) = p.signing.as_deref() {
            match kind {
                "none" => profile.signing = None,
                "ssh" | "openpgp" => {
                    let format = if kind == "ssh" {
                        SigningFormat::Ssh
                    } else {
                        SigningFormat::Openpgp
                    };
                    let key = p
                        .signing_key
                        .clone()
                        .or_else(|| profile.signing.as_ref().map(|s| s.key.clone()))
                        .ok_or_else(|| invalid("signing_key is required to set signing"))?;
                    let commits = profile.signing.as_ref().map(|s| s.commits).unwrap_or(false);
                    profile.signing = Some(Signing {
                        format,
                        key,
                        commits,
                        tags: profile.signing.as_ref().and_then(|s| s.tags),
                    });
                }
                other => return Err(invalid(format!("invalid signing method: {other}"))),
            }
        }

        let updated = profile.clone();
        let mut doc = profiles::load_doc(&path).map_err(internal)?;
        profiles::upsert_profile(&mut doc, &p.name, &updated).map_err(internal)?;
        profiles::save_doc(&path, &doc).map_err(internal)?;

        let report = sync_all(&self.paths).map_err(internal)?;
        json(&MutationResult {
            ok: true,
            detail: updated,
            sync: report,
        })
    }

    #[tool(
        description = "Delete a profile (like `gitid remove`). Refuses if directories map to it unless force is set."
    )]
    async fn gitid_remove(
        &self,
        Parameters(p): Parameters<RemoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let profiles_path = self.paths.profiles_toml();
        let existing = profiles::load(&profiles_path).map_err(internal)?;
        if !existing.profiles.contains_key(&p.name) {
            return Err(invalid(format!("no such profile: {}", p.name)));
        }

        let mut mappings = MappingsFile::load(&self.paths.mappings_toml()).map_err(internal)?;
        let mapped: Vec<String> = mappings
            .mappings
            .iter()
            .filter(|m| m.profile == p.name)
            .map(|m| m.dir.clone())
            .collect();
        if !mapped.is_empty() {
            if !p.force {
                return Err(invalid(format!(
                    "{} director(ies) map to {:?}; pass force=true to remove it and its mappings",
                    mapped.len(),
                    p.name
                )));
            }
            mappings.mappings.retain(|m| m.profile != p.name);
            mappings
                .save(&self.paths.mappings_toml())
                .map_err(internal)?;
        }

        let mut doc = profiles::load_doc(&profiles_path).map_err(internal)?;
        profiles::remove_profile(&mut doc, &p.name);
        profiles::save_doc(&profiles_path, &doc).map_err(internal)?;

        let report = sync_all(&self.paths).map_err(internal)?;
        json(&MutationResult {
            ok: true,
            detail: format!(
                "removed profile {:?} (gh auth dir, if any, left in place)",
                p.name
            ),
            sync: report,
        })
    }

    #[tool(
        description = "Regenerate all derived gitid config from the stores (like `gitid sync`)."
    )]
    async fn gitid_sync(&self) -> Result<CallToolResult, McpError> {
        let report = sync_all(&self.paths).map_err(internal)?;
        json(&report)
    }
}

impl GitidServer {
    fn ctx(&self) -> Ctx {
        Ctx {
            paths: self.paths.clone(),
        }
    }
}

#[tool_handler]
impl ServerHandler for GitidServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "gitid manages named git identities (name/email, SSH key, commit signing, gh \
                 auth) and assigns them to directory trees. Use the read tools to inspect \
                 identities and the write tools (gitid_use, gitid_add, …) to change them; every \
                 mutation regenerates the derived git config."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            ..Default::default()
        }
    }
}

// ---- helpers ---------------------------------------------------------------

#[derive(Serialize)]
struct DoctorResult {
    failed: bool,
    findings: Vec<crate::cmd::doctor::Finding>,
}

/// Serialize a value as a pretty JSON text result.
fn json<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(format!("could not serialise result: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn internal(err: anyhow::Error) -> McpError {
    McpError::internal_error(format!("{err:#}"), None)
}

fn invalid(msg: impl Into<String>) -> McpError {
    McpError::invalid_params(msg.into(), None)
}

fn invalid_from(err: anyhow::Error) -> McpError {
    McpError::invalid_params(format!("{err:#}"), None)
}

/// Build a `Signing` from the loose string params shared by add/edit-add paths.
fn build_signing(
    method: Option<&str>,
    key: Option<String>,
    commits: bool,
) -> Result<Option<Signing>> {
    let Some(method) = method else {
        return Ok(None);
    };
    let format = match method {
        "none" => return Ok(None),
        "ssh" => SigningFormat::Ssh,
        "openpgp" => SigningFormat::Openpgp,
        other => anyhow::bail!("invalid signing method: {other} (use ssh, openpgp, or none)"),
    };
    let key = key
        .ok_or_else(|| anyhow::anyhow!("signing_key is required when signing is ssh or openpgp"))?;
    Ok(Some(Signing {
        format,
        key,
        commits,
        tags: None,
    }))
}
