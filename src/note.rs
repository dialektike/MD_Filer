use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>, // 옵션으로 변경
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>, // 파일에 저장된 UUID (옵션)
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub filename: String,
    pub meta: NoteMeta,
    pub title: String,             // 실제 title (항상 존재)
    pub created_at: DateTime<Utc>, // 인덱스에서 관리
    pub updated_at: DateTime<Utc>, // 인덱스에서 관리
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
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        if let Some((frontmatter, body)) = Self::split_frontmatter(&content) {
            let mut meta: NoteMeta =
                serde_yaml::from_str(&frontmatter).map_err(|e| format!("YAML 파싱 오류: {}", e))?;

            // 파일에 UUID가 있으면 사용, 없으면 매개변수의 UUID 사용
            let actual_id = meta.id.unwrap_or(id);

            // title이 없으면 자동 생성
            let title = meta
                .title
                .clone()
                .or_else(|| Self::extract_title_from_content(&body))
                .unwrap_or_else(|| filename.trim_end_matches(".md").to_string());

            // meta에 UUID와 title 설정 (파일에 저장할 준비)
            meta.id = Some(actual_id);
            meta.title = Some(title.clone());

            Ok(Note {
                id: actual_id,
                filename,
                meta,
                title,
                created_at,
                updated_at,
                content: body,
                tags,
            })
        } else {
            // frontmatter가 없는 경우 - 기본 메타데이터 생성
            let title = Self::extract_title_from_content(&content)
                .unwrap_or_else(|| filename.trim_end_matches(".md").to_string());

            Ok(Note {
                id,
                filename: filename.clone(),
                meta: NoteMeta {
                    title: Some(title.clone()),
                    id: Some(id), // UUID 포함
                },
                title,
                created_at,
                updated_at,
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

    // 컨텐츠에서 태그 추출 (#tag 또는 @folder 형식)
    #[allow(dead_code)]
    pub fn extract_tags_from_content(content: &str) -> Vec<String> {
        use std::collections::HashSet;
        let mut tags = HashSet::new();

        // 정규식 대신 단순 파싱 사용
        for line in content.lines() {
            let words = line.split_whitespace();
            for word in words {
                // @folder 형식 태그
                if word.starts_with('@') && word.len() > 1 {
                    let tag = word
                        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
                    if tag.len() > 1 {
                        tags.insert(tag.to_string());
                    }
                }
                // #tag 형식 태그
                else if word.starts_with('#') && word.len() > 1 {
                    let tag = word[1..]
                        .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
                    if !tag.is_empty() {
                        tags.insert(tag.to_string());
                    }
                }
            }
        }

        tags.into_iter().collect()
    }

    // 폴더 태그 가져오기 (@로 시작하는 태그)
    pub fn get_folder_tag(&self) -> Option<&str> {
        self.tags
            .iter()
            .find(|tag| tag.starts_with('@'))
            .map(|s| s.as_str())
    }

    // 폴더 이름 가져오기 (@ 제거)
    pub fn get_folder_name(&self) -> Option<String> {
        self.get_folder_tag()
            .map(|tag| tag.trim_start_matches('@').to_string())
    }

    // 폴더 표시용 문자열 (이모지 포함)
    pub fn get_folder_display(&self) -> String {
        self.get_folder_name()
            .map(|name| format!("📁{}", name))
            .unwrap_or_default()
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
        format!("---\n{}---\n{}", frontmatter, self.content)
    }

    // frontmatter가 있는지 확인
    #[allow(dead_code)]
    pub fn has_frontmatter(content: &str) -> bool {
        content.starts_with("---\n")
    }

    // frontmatter에 UUID가 있는지 확인
    pub fn has_uuid_in_frontmatter(content: &str) -> bool {
        if let Some((frontmatter, _)) = Self::split_frontmatter(content) {
            if let Ok(meta) = serde_yaml::from_str::<NoteMeta>(&frontmatter) {
                return meta.id.is_some();
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_with_frontmatter() {
        let content = r#"---
title: Test Note
---

# Test Content

This is a test note."#;

        let id = Uuid::new_v4();
        let tags = vec!["test".to_string(), "@folder".to_string()];
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "test.md".to_string(),
            content.to_string(),
            tags.clone(),
            now,
            now,
        )
        .unwrap();

        assert_eq!(note.title, "Test Note");
        assert_eq!(note.filename, "test.md");
        assert_eq!(note.tags, tags);
        assert!(note.content.contains("# Test Content"));
    }

    #[test]
    fn test_parse_note_without_frontmatter() {
        let content = r#"# My Title

This is content without frontmatter."#;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "test.md".to_string(),
            content.to_string(),
            vec![],
            now,
            now,
        )
        .unwrap();

        assert_eq!(note.title, "My Title");
        assert_eq!(note.filename, "test.md");
    }

    #[test]
    fn test_parse_note_without_frontmatter_or_title() {
        let content = "Just some content.";

        let id = Uuid::new_v4();
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "myfile.md".to_string(),
            content.to_string(),
            vec![],
            now,
            now,
        )
        .unwrap();

        assert_eq!(note.title, "myfile");
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
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "test.md".to_string(),
            "# Test".to_string(),
            tags,
            now,
            now,
        )
        .unwrap();

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
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "test.md".to_string(),
            "# Test".to_string(),
            tags,
            now,
            now,
        )
        .unwrap();

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
---

# Test Content"#;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let note = Note::from_markdown(
            id,
            "test.md".to_string(),
            content.to_string(),
            vec![],
            now,
            now,
        )
        .unwrap();
        let markdown = note.to_markdown();

        assert!(markdown.starts_with("---\n"));
        assert!(markdown.contains("title: Test Note"));
        assert!(markdown.contains("# Test Content"));
    }
}
