//! V3 federation addressing: the location half of an ask address (`pane@LOC`).
//! `LOC` is either a local instance id or a `crew://host[:port]/instance` URL
//! naming another host. This is pure parse/resolve logic — the socket dial and
//! the (future) relay transport live elsewhere. Widening the address is all v3
//! needs from the resolver; the engine is untouched
//! (docs/vision/sentinel-network.md).

/// Default port for the cross-host relay (opt-in; a host only listens once its
/// operator turns federation on and shares an invite).
pub(crate) const DEFAULT_RELAY_PORT: u16 = 7733;

/// Where a federated ask should go.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Location {
    /// An instance on this host (its Unix socket).
    Local(String),
    /// An instance on another host, reached via the relay.
    Remote {
        host: String,
        port: u16,
        instance: String,
    },
}

/// Parse the location half (the part after `pane@`). `crew://host[:port]/inst`
/// → [`Location::Remote`]; any other non-empty string → a local instance id.
/// `None` for an empty or malformed location.
pub(crate) fn parse_location(loc: &str) -> Option<Location> {
    if let Some(rest) = loc.strip_prefix("crew://") {
        let (authority, instance) = rest.split_once('/')?;
        if instance.is_empty() {
            return None;
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) => (h, p.parse().ok()?),
            None => (authority, DEFAULT_RELAY_PORT),
        };
        if host.is_empty() {
            return None;
        }
        Some(Location::Remote {
            host: host.to_string(),
            port,
            instance: instance.to_string(),
        })
    } else if loc.is_empty() {
        None
    } else {
        Some(Location::Local(loc.to_string()))
    }
}

/// Resolve an ask address into `(pane, local-instance)`. `Err(message)` for a
/// `crew://` remote until the relay transport lands — an honest "not yet"
/// rather than a silent local fallback. A bare address or a local instance id
/// resolves to `Ok`.
pub(crate) fn resolve_target(to: &str) -> Result<(&str, Option<String>), String> {
    let (pane, loc) = crate::askroute::split_instance(to);
    match loc.map(parse_location) {
        Some(Some(Location::Remote {
            host,
            port,
            instance,
        })) => Err(format!(
            "cross-host relay not yet available (crew://{host}:{port}/{instance})"
        )),
        Some(Some(Location::Local(id))) => Ok((pane, Some(id))),
        _ => Ok((pane, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_ids_and_crew_urls() {
        assert_eq!(
            parse_location("alpha"),
            Some(Location::Local("alpha".into()))
        );
        assert_eq!(parse_location(""), None);
        assert_eq!(
            parse_location("crew://host.example/main"),
            Some(Location::Remote {
                host: "host.example".into(),
                port: DEFAULT_RELAY_PORT,
                instance: "main".into(),
            })
        );
        assert_eq!(
            parse_location("crew://10.0.0.4:9000/build"),
            Some(Location::Remote {
                host: "10.0.0.4".into(),
                port: 9000,
                instance: "build".into(),
            })
        );
        // Malformed crew:// URLs are rejected, not silently downgraded.
        assert_eq!(parse_location("crew://host"), None); // no instance
        assert_eq!(parse_location("crew:///main"), None); // no host
        assert_eq!(parse_location("crew://host:notaport/main"), None);
    }

    #[test]
    fn resolve_target_splits_pane_and_reports_remote_as_pending() {
        assert_eq!(resolve_target("schema"), Ok(("schema", None)));
        assert_eq!(
            resolve_target("schema@alpha"),
            Ok(("schema", Some("alpha".into())))
        );
        let err = resolve_target("schema@crew://host/main").unwrap_err();
        assert!(err.contains("not yet available") && err.contains("crew://host"));
    }
}
