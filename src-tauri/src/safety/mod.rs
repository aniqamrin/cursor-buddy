use serde::{Deserialize, Serialize};

/// What Cursor Buddy is allowed to do on the user's computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Observe,
    Guide,
    Assist,
    Autopilot,
}

impl PermissionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionLevel::Observe => "observe",
            PermissionLevel::Guide => "guide",
            PermissionLevel::Assist => "assist",
            PermissionLevel::Autopilot => "autopilot",
        }
    }

    pub fn parse(s: &str) -> Option<PermissionLevel> {
        match s.to_lowercase().as_str() {
            "observe" => Some(PermissionLevel::Observe),
            "guide" => Some(PermissionLevel::Guide),
            "assist" => Some(PermissionLevel::Assist),
            "autopilot" => Some(PermissionLevel::Autopilot),
            _ => None,
        }
    }

    /// Whether this level permits the AI to control mouse/keyboard itself.
    pub fn allows_control(&self) -> bool {
        matches!(self, PermissionLevel::Assist | PermissionLevel::Autopilot)
    }
}

/// A single computer action requested by the agent (Phase 5 wires executors).
#[derive(Clone, Debug)]
pub struct PlannedAction {
    pub tool: String,
    pub argument: String,
}

/// Outcome of a safety review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    RequireConfirmation { reason: &'static str },
    Deny { reason: &'static str },
}

impl Decision {
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Decision::RequireConfirmation { .. })
    }
}

/// Keywords that always escalate to explicit confirmation regardless of
/// permission level. Verb stems ("delet", "purchas") match any conjugation
/// so reworded requests cannot slip past. The Safety Layer is checked
/// *before* any executor runs.
const SENSITIVE_PATTERNS: &[&str] = &[
    "send message", "send email", "send mail",
    "sending message", "sending email", "sending mail",
    "delet", "purchas", "buy ", "checkout",
    "change password", "reset password",
    "install", "uninstall", "run executable", "format disk", "transfer money",
    "payment", "paying", "security setting", "sign out", "log out everywhere",
];

pub struct SafetyLayer;

impl SafetyLayer {
    /// Review a planned action against the current permission level.
    ///
    /// Phase 1 has no executors yet; this is the gate every future
    /// mouse/keyboard/app action must pass through. Keeping it early means
    /// automation can never bypass it later.
    pub fn evaluate(action: &PlannedAction, level: PermissionLevel) -> Decision {
        if !level.allows_control() {
            return Decision::Deny {
                reason: "current permission level does not allow computer control",
            };
        }

        let haystack = format!("{} {}", action.tool, action.argument).to_lowercase();
        for pattern in SENSITIVE_PATTERNS {
            if haystack.contains(pattern) {
                // Even Autopilot confirms sensitive actions (spec: never silent).
                return Decision::RequireConfirmation {
                    reason: "potentially sensitive action",
                };
            }
        }

        match level {
            PermissionLevel::Assist => Decision::RequireConfirmation {
                reason: "ASSIST level confirms each individual action",
            },
            PermissionLevel::Autopilot => Decision::Allow,
            _ => unreachable!("control levels checked above"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_and_guide_deny_control() {
        let a = PlannedAction { tool: "click".into(), argument: "(10,10)".into() };
        assert_eq!(
            SafetyLayer::evaluate(&a, PermissionLevel::Observe),
            Decision::Deny { reason: "current permission level does not allow computer control" }
        );
    }

    #[test]
    fn assist_requires_confirmation_for_normal_actions() {
        let a = PlannedAction { tool: "click".into(), argument: "(10,10)".into() };
        assert!(SafetyLayer::evaluate(&a, PermissionLevel::Assist).requires_confirmation());
    }

    #[test]
    fn autopilot_allows_normal_but_confirms_sensitive() {
        let normal = PlannedAction { tool: "press_key".into(), argument: "Enter".into() };
        assert_eq!(SafetyLayer::evaluate(&normal, PermissionLevel::Autopilot), Decision::Allow);

        let sensitive = PlannedAction {
            tool: "type_text".into(),
            argument: "sending email to boss".into(),
        };
        assert!(SafetyLayer::evaluate(&sensitive, PermissionLevel::Autopilot).requires_confirmation());
    }
}
