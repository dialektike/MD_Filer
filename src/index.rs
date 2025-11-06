use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    pub filename: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteIndex {
    pub mappings: HashMap<Uuid, IndexEntry>,
}

impl NoteIndex {
    pub fn new() -> Self {
        NoteIndex {
            mappings: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("인덱스 파일 읽기 실패: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("인덱스 파일 파싱 실패: {}", e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("JSON 직렬화 실패: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("인덱스 파일 저장 실패: {}", e))
    }

    pub fn add_entry(&mut self, id: Uuid, entry: IndexEntry) {
        self.mappings.insert(id, entry);
    }

    pub fn remove_entry(&mut self, id: &Uuid) -> Option<IndexEntry> {
        self.mappings.remove(id)
    }

    pub fn get_entry(&self, id: &Uuid) -> Option<&IndexEntry> {
        self.mappings.get(id)
    }

    pub fn find_by_filename(&self, filename: &str) -> Option<(Uuid, &IndexEntry)> {
        self.mappings.iter()
            .find(|(_, entry)| entry.filename == filename)
            .map(|(id, entry)| (*id, entry))
    }

    pub fn update_filename(&mut self, id: &Uuid, new_filename: String) {
        if let Some(entry) = self.mappings.get_mut(id) {
            entry.filename = new_filename;
            entry.updated_at = Utc::now();
        }
    }

    // 태그별로 노트 찾기
    pub fn find_by_tag(&self, tag: &str) -> Vec<(Uuid, &IndexEntry)> {
        self.mappings.iter()
            .filter(|(_, entry)| entry.tags.contains(&tag.to_string()))
            .map(|(id, entry)| (*id, entry))
            .collect()
    }

    // 폴더별로 노트 찾기
    pub fn find_by_folder(&self, folder: &str) -> Vec<(Uuid, &IndexEntry)> {
        self.mappings.iter()
            .filter(|(_, entry)| {
                entry.tags.iter().any(|tag| tag.starts_with('@') && tag == folder)
            })
            .map(|(id, entry)| (*id, entry))
            .collect()
    }
}
