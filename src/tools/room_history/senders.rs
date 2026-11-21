use std::collections::{HashMap, HashSet};

use mxlink::matrix_sdk::ruma::{OwnedUserId, UserId};

/// Candidates listed when a name is ambiguous.
const MAX_CANDIDATES: usize = 10;

/// Resolves sender names (Matrix user IDs, display names or the part of user IDs before the server name)
/// to user IDs, using the room's members.
///
/// Names matching several members (or none) give an error message for the model, listing the candidates,
/// so that it can ask the user or try again, instead of the bot guessing.
pub fn resolve(
    names: &[String],
    members: &HashMap<OwnedUserId, Option<String>>,
) -> Result<HashSet<OwnedUserId>, String> {
    let mut resolved = HashSet::new();
    let mut problems = vec![];

    for name in names {
        // User IDs are taken as they are, also for people who are no longer members.
        if name.starts_with('@') {
            match UserId::parse(name.as_str()) {
                Ok(user_id) => {
                    resolved.insert(user_id);
                }
                Err(_) => problems.push(format!("`{name}` is not a valid Matrix user ID.")),
            }
            continue;
        }

        let mut candidates =
            matching_members(members, |candidate| candidate == name.to_lowercase());

        if candidates.is_empty() {
            candidates = matching_members(members, |candidate| {
                candidate.contains(&name.to_lowercase())
            });
        }

        match candidates.as_slice() {
            [] => problems.push(format!("No room member matches `{name}`.")),
            [user_id] => {
                resolved.insert(user_id.clone());
            }
            _ => problems.push(format!(
                "`{name}` matches several room members: {}. Ask the user which one they mean, or call again with user IDs.",
                describe_candidates(&candidates, members)
            )),
        }
    }

    if problems.is_empty() {
        Ok(resolved)
    } else {
        Err(problems.join("\n"))
    }
}

/// Returns the members (sorted) whose display name or user ID localpart (lowercased) matches.
fn matching_members(
    members: &HashMap<OwnedUserId, Option<String>>,
    matches: impl Fn(&str) -> bool,
) -> Vec<OwnedUserId> {
    let mut candidates: Vec<OwnedUserId> = members
        .iter()
        .filter(|(user_id, display_name)| {
            matches(&user_id.localpart().to_lowercase())
                || display_name
                    .as_deref()
                    .is_some_and(|display_name| matches(&display_name.to_lowercase()))
        })
        .map(|(user_id, _)| user_id.clone())
        .collect();

    candidates.sort();
    candidates
}

fn describe_candidates(
    candidates: &[OwnedUserId],
    members: &HashMap<OwnedUserId, Option<String>>,
) -> String {
    let mut described: Vec<String> = candidates
        .iter()
        .take(MAX_CANDIDATES)
        .map(|user_id| match members.get(user_id).cloned().flatten() {
            Some(display_name) => format!("{user_id} ({})", super::format::one_line(&display_name)),
            None => user_id.to_string(),
        })
        .collect();

    if candidates.len() > MAX_CANDIDATES {
        described.push(format!("and {} more", candidates.len() - MAX_CANDIDATES));
    }

    described.join(", ")
}
