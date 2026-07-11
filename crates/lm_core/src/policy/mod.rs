//! Local-first safety policy and block list models.

use crate::UserId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSafetyPolicy {
    pub stranger_messages: StrangerMessagePolicy,
    pub allow_stranger_attachments: bool,
    pub allow_stranger_group_invites: bool,
    pub auto_download_media: bool,
    pub warn_external_links: bool,
    pub warn_executable_files: bool,
    pub enable_text_filter: bool,
    pub text_filter_level: FilterLevel,
}

impl Default for LocalSafetyPolicy {
    fn default() -> Self {
        Self {
            stranger_messages: StrangerMessagePolicy::FriendRequestOnly,
            allow_stranger_attachments: false,
            allow_stranger_group_invites: false,
            auto_download_media: false,
            warn_external_links: true,
            warn_executable_files: true,
            enable_text_filter: true,
            text_filter_level: FilterLevel::Standard,
        }
    }
}

impl LocalSafetyPolicy {
    /// Evaluate local-only safety hints for text. This is intentionally not a
    /// community moderation system: it only helps a user's own device decide
    /// whether to warn, blur/hide, or drop content before display/storage.
    pub fn evaluate_text(&self, text: &str) -> FilterAction {
        if !self.enable_text_filter || self.text_filter_level == FilterLevel::Off {
            return FilterAction::Allow;
        }
        let lower = text.to_lowercase();
        let mut action = FilterAction::Allow;
        if self.warn_external_links && (lower.contains("http://") || lower.contains("https://")) {
            action = action.max(FilterAction::Warn);
        }
        if self.warn_executable_files && looks_like_executable_name(&lower) {
            action = action.max(match self.text_filter_level {
                FilterLevel::Off => FilterAction::Allow,
                FilterLevel::Relaxed => FilterAction::Warn,
                FilterLevel::Standard => FilterAction::Blur,
                FilterLevel::Strict => FilterAction::Hide,
            });
        }
        action
    }
}

fn looks_like_executable_name(lower: &str) -> bool {
    [
        ".exe", ".msi", ".bat", ".cmd", ".scr", ".apk", ".ipa", ".dmg", ".pkg", ".sh",
    ]
    .iter()
    .any(|suffix| lower.contains(suffix))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrangerMessagePolicy {
    BlockAll,
    FriendRequestOnly,
    AllowTextOnly,
    AllowAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterLevel {
    Off,
    Relaxed,
    Standard,
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FilterAction {
    Allow,
    Warn,
    Blur,
    Hide,
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntry {
    pub user_id: UserId,
    pub reason: Option<String>,
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_warns_links_and_blurs_executables() {
        let policy = LocalSafetyPolicy::default();
        assert_eq!(policy.evaluate_text("hello"), FilterAction::Allow);
        assert_eq!(
            policy.evaluate_text("see https://example.test"),
            FilterAction::Warn
        );
        assert_eq!(policy.evaluate_text("run update.exe"), FilterAction::Blur);
    }

    #[test]
    fn strict_policy_hides_executables() {
        let policy = LocalSafetyPolicy {
            text_filter_level: FilterLevel::Strict,
            ..Default::default()
        };
        assert_eq!(policy.evaluate_text("install app.apk"), FilterAction::Hide);
    }

    #[test]
    fn disabled_policy_allows() {
        let policy = LocalSafetyPolicy {
            enable_text_filter: false,
            ..Default::default()
        };
        assert_eq!(
            policy.evaluate_text("https://x update.exe"),
            FilterAction::Allow
        );
    }
}
