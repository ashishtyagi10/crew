//! The provider auth registry: every way crew can reach a model, as DATA.
//! Each entry declares its name (as `CREW_PROVIDER` spells it), the auth
//! modes it supports, the env var an API key lives in, and — for
//! CLI-delegated providers — which vendor CLI owns the sign-in and how to
//! ask it for its state. Adding a provider is a row here plus at most a
//! thin adapter; routing and planning code never learn provider names.
//!
//! Three rungs by design (see the 2026-08-01 goal doc): `CliDelegated` means
//! the subscription is used INSIDE the vendor's own client (crew drives the
//! CLI, never touches its token store); `OauthDevice` means the provider
//! openly permits third-party device-code OAuth and crew may run the flow
//! itself; `CliMinted` means the vendor's own CLI owns the sign-in AND hands
//! out a short-lived bearer on request (`ant auth print-credentials`), which
//! crew's native provider then presents. A provider with no permitted path
//! stays `ApiKey`-only.

/// How a provider entry can authenticate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AuthMode {
    /// A vendor CLI owns the login; crew probes its signed-in state and
    /// routes work through it (the `/far`-lets-rclone-own-OAuth pattern).
    CliDelegated,
    /// The provider permits third-party device-code OAuth; the entry's
    /// `device` endpoints drive the flow (`auth::device`).
    OauthDevice,
    /// A vendor CLI owns the sign-in and mints a bearer on request; the
    /// entry's `mint` spec drives probe and mint (`auth::mint`).
    CliMinted,
    /// A pasted API key in `key_var`.
    ApiKey,
    /// The deterministic test provider (`CREW_BROKER_MOCK_REPLY`).
    Mock,
}

/// The CLI half of a `CliDelegated` entry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CliSpec {
    /// The binary on `$PATH` — also the roster/adapter name work relays to.
    pub bin: &'static str,
    /// argv (after the binary) of the CLI's own status introspection — the
    /// consent-based probe. Crew NEVER reads the CLI's token store.
    pub status: &'static [&'static str],
    /// The exact command a signed-out user runs to sign in.
    pub login: &'static str,
    /// When set, the status output SAYS signed-in by containing this marker
    /// (case-insensitive) and the exit code is no signal at all — `ant auth
    /// status` exits 0 signed in or out. `None`: the exit code decides.
    pub signed_in_marker: Option<&'static str>,
}

pub(crate) use super::device::{DeviceSpec, QWEN_DEVICE};
pub(crate) use super::mint::MintSpec;

/// One provider the registry knows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProviderAuth {
    pub name: &'static str,
    pub modes: &'static [AuthMode],
    /// The env var an API key is discovered in (`ApiKey` mode).
    pub key_var: Option<&'static str>,
    pub cli: Option<CliSpec>,
    /// The device-flow endpoints (`OauthDevice` mode).
    pub device: Option<DeviceSpec>,
    /// The minting CLI (`CliMinted` mode).
    pub mint: Option<MintSpec>,
}

/// The all-`None` row every literal below fills in from.
const BLANK: ProviderAuth = ProviderAuth {
    name: "",
    modes: &[],
    key_var: None,
    cli: None,
    device: None,
    mint: None,
};

impl ProviderAuth {
    pub(crate) fn delegated(&self) -> bool {
        self.modes.contains(&AuthMode::CliDelegated)
    }
    pub(crate) fn keyed(&self) -> bool {
        self.modes.contains(&AuthMode::ApiKey)
    }
    pub(crate) fn minted(&self) -> bool {
        self.modes.contains(&AuthMode::CliMinted)
    }
}

/// The static half of the table. Keyed entries appear in DISCOVERY order —
/// dashscope before openrouter before anthropic, exactly the order
/// `pick_provider` has always used — and [`entries`] appends the
/// `discover::DIRECT` rows after them, preserving "new providers probe
/// last". Delegated entries lead claude-first, matching the keyless
/// roster's own lead.
static SEED: &[ProviderAuth] = &[
    ProviderAuth {
        name: "claude-code",
        modes: &[AuthMode::CliDelegated],
        cli: Some(CliSpec {
            bin: "claude",
            status: &["auth", "status"],
            login: "claude auth login",
            signed_in_marker: None,
        }),
        ..BLANK
    },
    ProviderAuth {
        name: "codex",
        modes: &[AuthMode::CliDelegated],
        cli: Some(CliSpec {
            bin: "codex",
            status: &["login", "status"],
            login: "codex login",
            signed_in_marker: None,
        }),
        ..BLANK
    },
    ProviderAuth {
        name: "dashscope",
        // Qwen permits third-party device-code OAuth (QWEN_DEVICE above);
        // a pasted key stays equally valid.
        modes: &[AuthMode::ApiKey, AuthMode::OauthDevice],
        key_var: Some("DASHSCOPE_API_KEY"),
        device: Some(QWEN_DEVICE),
        ..BLANK
    },
    ProviderAuth {
        name: "openrouter",
        modes: &[AuthMode::ApiKey],
        key_var: Some("OPENROUTER_API_KEY"),
        ..BLANK
    },
    ProviderAuth {
        name: "anthropic",
        // A key, or the Anthropic CLI's own Console sign-in (`ant auth
        // login`) minting the bearer — Anthropic's sanctioned OAuth for
        // third-party clients; the Claude Code client is NOT.
        modes: &[AuthMode::ApiKey, AuthMode::CliMinted],
        key_var: Some("ANTHROPIC_API_KEY"),
        mint: Some(super::mint::ANT),
        ..BLANK
    },
    ProviderAuth {
        name: "mock",
        modes: &[AuthMode::Mock],
        ..BLANK
    },
];

/// Every provider the registry knows, in discovery order. The
/// `discover::DIRECT` table rows (openai, gemini, deepseek, …) join as
/// plain `ApiKey` entries so the two tables cannot drift: DIRECT stays the
/// one place an OpenAI-wire endpoint is declared, and this stays the one
/// place auth modes are.
pub(crate) fn entries() -> Vec<ProviderAuth> {
    let mut v: Vec<ProviderAuth> = SEED.to_vec();
    for d in crate::broker::discover::DIRECT {
        v.push(ProviderAuth {
            name: d.name,
            modes: &[AuthMode::ApiKey],
            key_var: Some(d.var),
            ..BLANK
        });
    }
    v
}

/// The entry called `name` — matching either the registry name
/// (`claude-code`) or the CLI binary (`claude`), case-insensitively, so a
/// `CREW_PROVIDER` pin can spell it either way.
pub(crate) fn by_name(name: &str) -> Option<ProviderAuth> {
    entries().into_iter().find(|e| {
        e.name.eq_ignore_ascii_case(name) || e.cli.is_some_and(|c| c.bin.eq_ignore_ascii_case(name))
    })
}

/// The CLI-delegated entries, in probe order.
pub(crate) fn delegated() -> Vec<ProviderAuth> {
    entries()
        .into_iter()
        .filter(ProviderAuth::delegated)
        .collect()
}

/// The API-key entries, in discovery order.
pub(crate) fn keyed() -> Vec<ProviderAuth> {
    entries().into_iter().filter(ProviderAuth::keyed).collect()
}

/// The CLI-minted entries, in discovery order.
pub(crate) fn minted() -> Vec<ProviderAuth> {
    entries().into_iter().filter(ProviderAuth::minted).collect()
}

/// The registry name for the provider whose key lives in `var`, if any.
pub(crate) fn name_for_var(var: &str) -> Option<&'static str> {
    entries()
        .into_iter()
        .find(|e| e.key_var == Some(var))
        .map(|e| e.name)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
