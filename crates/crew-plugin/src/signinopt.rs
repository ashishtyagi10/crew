//! One provider a user can sign in to, as the broker reports it to a host —
//! the data behind the `/model` picker's sign-in rows (and `/logout`'s).
//!
//! `/login` used to print a numbered table and wait for the user to type a
//! name or a number back. A host with a screen can do better: offer the rows
//! as a popup and send `/model <name>` for the one picked. This is the row,
//! states only — nothing here names or carries a credential.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignInOption {
    /// The provider name, as `/model <name>` and `CREW_PROVIDER` spell it.
    pub name: String,
    /// Crew runs this sign-in itself (the device flow) — picking the row
    /// starts it right in the pane. `false`: a vendor CLI owns the sign-in.
    #[serde(default)]
    pub device: bool,
    #[serde(default)]
    pub signed_in: bool,
    /// An API key is present alongside — signing in is still offered, the
    /// grant then serves ahead of the key.
    #[serde(default)]
    pub key_present: bool,
    /// The exact command a vendor CLI's sign-in takes, when one owns it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    /// How to install that CLI, when it is not on this machine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
    /// The command that signs OUT of a CLI-owned sign-in, when the CLI
    /// names one (crew never touches a vendor CLI's store).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logout: Option<String>,
}
