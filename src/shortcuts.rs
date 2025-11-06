use crate::note::{LinkTarget, Shortcut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShortcutsRegistry {
    // note_id -> shortcuts mapping
    pub shortcuts: HashMap<Uuid, HashMap<String, Shortcut>>,
}

impl ShortcutsRegistry {
    pub fn new() -> Self {
        ShortcutsRegistry {
            shortcuts: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Shortcuts 파일 읽기 실패: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Shortcuts 파일 파싱 실패: {}", e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("JSON 직렬화 실패: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Shortcuts 파일 저장 실패: {}", e))
    }

    pub fn get_shortcuts(&self, note_id: &Uuid) -> Option<&HashMap<String, Shortcut>> {
        self.shortcuts.get(note_id)
    }

    pub fn add_shortcut(&mut self, note_id: Uuid, alias: String, shortcut: Shortcut) {
        self.shortcuts
            .entry(note_id)
            .or_insert_with(HashMap::new)
            .insert(alias, shortcut);
    }

    pub fn remove_shortcuts(&mut self, note_id: &Uuid) -> Option<HashMap<String, Shortcut>> {
        self.shortcuts.remove(note_id)
    }

    // 특정 노트를 참조하는 모든 shortcuts 찾기
    pub fn find_references_to_note(&self, target_id: &Uuid) -> Vec<(Uuid, String, &Shortcut)> {
        let mut references = Vec::new();
        
        for (note_id, shortcuts_map) in &self.shortcuts {
            for (alias, shortcut) in shortcuts_map {
                if let LinkTarget::Note { id } = &shortcut.target {
                    if id == target_id {
                        references.push((*note_id, alias.clone(), shortcut));
                    }
                }
            }
        }
        
        references
    }
}
