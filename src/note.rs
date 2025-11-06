use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LinkTarget {
    File { path: PathBuf },
    Url { url: String },
    Note { id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub alias: String,
    pub target: LinkTarget,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub filename: String,
    pub meta: NoteMeta,
    pub content: String,
    pub tags: Vec<String>, // 인덱스에서 로드된 태그
}

impl Note {
    // 마크다운 파일 파싱
    pub fn from_markdown(
        id: Uuid,
        filename: String,
        content: String,
        tags: Vec<String>,
    ) -> Result<Self, String> {
        if let Some((frontmatter, body)) = Self::split_frontmatter(&content) {
            let meta: NoteMeta =
                serde_yaml::from_str(&frontmatter).map_err(|e| format!("YAML 파싱 오류: {}", e))?;

            Ok(Note {
                id,
                filename,
                meta,
                content: body,
                tags,
            })
        } else {
            // frontmatter가 없는 경우 - 기본 메타데이터 생성
            let title = Self::extract_title_from_content(&content)
                .unwrap_or_else(|| filename.trim_end_matches(".md").to_string());

            let now = Utc::now();
            Ok(Note {
                id,
                filename: filename.clone(),
                meta: NoteMeta {
                    title,
                    created_at: now,
                    updated_at: now,
                },
                content,
                tags,
            })
        }
    }

    fn split_frontmatter(content: &str) -> Option<(String, String)> {
        if content.starts_with("---\n") {
            let parts: Vec<&str> = content.splitn(3, "---\n").collect();
            if parts.len() == 3 {
                return Some((parts[1].to_string(), parts[2].to_string()));
            }
        }
        None
    }

    fn extract_title_from_content(content: &str) -> Option<String> {
        for line in content.lines() {
            if line.starts_with("# ") {
                return Some(line.trim_start_matches("# ").trim().to_string());
            }
        }
        None
    }

    // 폴더 태그 가져오기 (@로 시작하는 태그)
    pub fn get_folder_tag(&self) -> Option<&str> {
        self.tags
            .iter()
            .find(|tag| tag.starts_with('@'))
            .map(|s| s.as_str())
    }

    // 일반 태그들 가져오기 (@로 시작하지 않는 태그)
    pub fn get_regular_tags(&self) -> Vec<&str> {
        self.tags
            .iter()
            .filter(|tag| !tag.starts_with('@'))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn to_markdown(&self) -> String {
        let frontmatter = serde_yaml::to_string(&self.meta).unwrap_or_default();
        format!("---\n{}---\n\n{}", frontmatter, self.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_with_frontmatter() {
        let content = r#"---
title: Test Note
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-02T00:00:00Z
---

# Test Content

This is a test note."#;

        let id = Uuid::new_v4();
        let tags = vec!["test".to_string(), "@folder".to_string()];
        let note =
            Note::from_markdown(id, "test.md".to_string(), content.to_string(), tags.clone())
                .unwrap();

        assert_eq!(note.meta.title, "Test Note");
        assert_eq!(note.filename, "test.md");
        assert_eq!(note.tags, tags);
        assert!(note.content.contains("# Test Content"));
    }

    #[test]
    fn test_parse_note_without_frontmatter() {
        let content = r#"# My Title

This is content without frontmatter."#;

        let id = Uuid::new_v4();
        let note =
            Note::from_markdown(id, "test.md".to_string(), content.to_string(), vec![]).unwrap();

        assert_eq!(note.meta.title, "My Title");
        assert_eq!(note.filename, "test.md");
    }

    #[test]
    fn test_parse_note_without_frontmatter_or_title() {
        let content = "Just some content.";

        let id = Uuid::new_v4();
        let note =
            Note::from_markdown(id, "myfile.md".to_string(), content.to_string(), vec![]).unwrap();

        assert_eq!(note.meta.title, "myfile");
        assert_eq!(note.filename, "myfile.md");
    }

    #[test]
    fn test_get_folder_tag() {
        let id = Uuid::new_v4();
        let tags = vec![
            "tag1".to_string(),
            "@projects".to_string(),
            "tag2".to_string(),
        ];
        let note =
            Note::from_markdown(id, "test.md".to_string(), "# Test".to_string(), tags).unwrap();

        assert_eq!(note.get_folder_tag(), Some("@projects"));
    }

    #[test]
    fn test_get_regular_tags() {
        let id = Uuid::new_v4();
        let tags = vec![
            "rust".to_string(),
            "@work".to_string(),
            "programming".to_string(),
        ];
        let note =
            Note::from_markdown(id, "test.md".to_string(), "# Test".to_string(), tags).unwrap();

        let regular_tags = note.get_regular_tags();
        assert_eq!(regular_tags.len(), 2);
        assert!(regular_tags.contains(&"rust"));
        assert!(regular_tags.contains(&"programming"));
        assert!(!regular_tags.contains(&"@work"));
    }

    #[test]
    fn test_to_markdown() {
        let content = r#"---
title: Test Note
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-02T00:00:00Z
---

# Test Content"#;

        let id = Uuid::new_v4();
        let note =
            Note::from_markdown(id, "test.md".to_string(), content.to_string(), vec![]).unwrap();
        let markdown = note.to_markdown();

        assert!(markdown.starts_with("---\n"));
        assert!(markdown.contains("title: Test Note"));
        assert!(markdown.contains("# Test Content"));
    }
}
