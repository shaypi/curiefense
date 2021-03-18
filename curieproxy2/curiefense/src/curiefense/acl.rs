use crate::curiefense::config::raw::ACLProfile;
use crate::curiefense::interface::Tags;

use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize)]
pub struct ACLDecision {
    pub allowed: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub enum ACLResult {
    /// bypass found
    Bypass(ACLDecision),
    /// bots, human results
    Match(BotHuman),
}

#[derive(Debug, Serialize)]
pub struct BotHuman {
    pub bot: Option<ACLDecision>,
    pub human: Option<ACLDecision>,
}

pub fn check_acl(tags: &Tags, acl: &ACLProfile) -> ACLResult {
    let subcheck = |checks: &HashSet<String>, allowed: bool| {
        let tags: Vec<String> = checks.intersection(tags.as_hash_ref()).cloned().collect();
        if tags.is_empty() {
            None
        } else {
            Some(ACLDecision { allowed, tags })
        }
    };
    subcheck(&acl.force_deny, false)
        .map(ACLResult::Bypass)
        .or_else(|| subcheck(&acl.bypass, true).map(ACLResult::Bypass))
        .unwrap_or_else(|| {
            let botresult =
                subcheck(&acl.allow_bot, true).or_else(|| subcheck(&acl.deny_bot, false));
            let humanresult = subcheck(&acl.allow, true).or_else(|| subcheck(&acl.deny, false));

            ACLResult::Match(BotHuman {
                bot: botresult,
                human: humanresult,
            })
        })
}
