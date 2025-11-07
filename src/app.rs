use crate::index::{IndexEntry, NoteIndex};
use crate::note::{Note, NoteMeta};
use crate::shortcuts::ShortcutsRegistry;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct NoteApp {
    pub notes: HashMap<Uuid, Note>,
    pub index: NoteIndex,
    pub shortcuts: ShortcutsRegistry,
    pub notes_dir: PathBuf,
}

impl NoteApp {
    pub fn new(notes_dir: PathBuf) -> Result<Self, String> {
        // 디렉토리 생성
        if !notes_dir.exists() {
            fs::create_dir_all(&notes_dir)
                .map_err(|e| format!("노트 디렉토리 생성 실패: {}", e))?;
        }

        let index_path = notes_dir.join(".index.json");
        let shortcuts_path = notes_dir.join(".shortcuts.json");

        // 인덱스와 shortcuts 로드 또는 생성
        let index = if index_path.exists() {
            NoteIndex::load(&index_path)?
        } else {
            NoteIndex::new()
        };

        let shortcuts = if shortcuts_path.exists() {
            ShortcutsRegistry::load(&shortcuts_path)?
        } else {
            ShortcutsRegistry::new()
        };

        let mut app = NoteApp {
            notes: HashMap::new(),
            index,
            shortcuts,
            notes_dir,
        };

        app.load_notes()?;
        Ok(app)
    }

    pub fn load_notes(&mut self) -> Result<(), String> {
        // 먼저 인덱스와 파일 시스템 동기화
        self.sync_with_filesystem()?;

        // 기존 노트 초기화
        self.notes.clear();

        let entries =
            fs::read_dir(&self.notes_dir).map_err(|e| format!("디렉토리 읽기 실패: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("엔트리 읽기 실패: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let filename = entry.file_name().to_string_lossy().to_string();

                // 인덱스에서 UUID 찾기 또는 새로 생성
                let (id, is_new) = if let Some((id, _)) = self.index.find_by_filename(&filename) {
                    (id, false)
                } else {
                    (Uuid::new_v4(), true)
                };

                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("파일 읽기 실패 {}: {}", filename, e))?;

                // 인덱스에서 태그와 타임스탬프 가져오기
                let now = Utc::now();
                let (tags, created_at, updated_at) = if let Some(entry) = self.index.get_entry(&id)
                {
                    (entry.tags.clone(), entry.created_at, now)
                } else {
                    (Vec::new(), now, now)
                };

                match Note::from_markdown(
                    id,
                    filename.clone(),
                    content.clone(),
                    tags.clone(),
                    created_at,
                    updated_at,
                ) {
                    Ok(note) => {
                        // UUID가 파일에 없으면 추가
                        if !Note::has_uuid_in_frontmatter(&content) {
                            if let Err(e) = self.inject_uuid_to_file(&path, &note) {
                                eprintln!("⚠️  UUID 주입 실패 {}: {}", filename, e);
                            } else {
                                println!("✏️  UUID 추가됨: {} ({})", filename, note.id);
                            }
                        }

                        // 인덱스 업데이트 (새 파일이거나 메타데이터 변경 시)
                        let entry = IndexEntry {
                            filename: filename.clone(),
                            title: note.meta.title.clone(),
                            created_at: note.created_at,
                            updated_at: note.updated_at,
                            tags: if is_new { Vec::new() } else { tags },
                        };

                        if is_new {
                            println!("📄 새 노트 발견: {}", filename);
                        }

                        self.index.add_entry(id, entry);
                        self.notes.insert(id, note);
                    }
                    Err(e) => {
                        eprintln!("노트 파싱 실패 {}: {}", filename, e);
                    }
                }
            }
        }

        // 인덱스 저장
        self.save_index()?;
        Ok(())
    }

    // 파일 시스템과 인덱스 동기화
    pub fn sync_with_filesystem(&mut self) -> Result<(), String> {
        // 1. 현재 파일 목록 가져오기
        let mut existing_files = std::collections::HashSet::new();
        let entries =
            fs::read_dir(&self.notes_dir).map_err(|e| format!("디렉토리 읽기 실패: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("엔트리 읽기 실패: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let filename = entry.file_name().to_string_lossy().to_string();
                existing_files.insert(filename);
            }
        }

        // 2. 인덱스에서 삭제된 파일 제거
        let mut to_remove = Vec::new();
        for (id, entry) in self.index.mappings.iter() {
            if !existing_files.contains(&entry.filename) {
                println!("🗑️  삭제된 노트 감지: {}", entry.filename);
                to_remove.push(*id);
            }
        }

        for id in &to_remove {
            self.index.remove_entry(id);
            self.shortcuts.remove_shortcuts(id);
        }

        if !to_remove.is_empty() {
            self.save_index()?;
            self.save_shortcuts()?;
            println!("✅ 인덱스 정리 완료: {}개 항목 제거", to_remove.len());
        }

        Ok(())
    }

    pub fn save_index(&self) -> Result<(), String> {
        let index_path = self.notes_dir.join(".index.json");
        self.index.save(&index_path)
    }

    pub fn save_shortcuts(&self) -> Result<(), String> {
        let shortcuts_path = self.notes_dir.join(".shortcuts.json");
        self.shortcuts.save(&shortcuts_path)
    }

    // 파일에 UUID 주입
    fn inject_uuid_to_file(&self, path: &PathBuf, note: &Note) -> Result<(), String> {
        let markdown = note.to_markdown();
        fs::write(path, markdown).map_err(|e| format!("파일 쓰기 실패: {}", e))
    }

    pub fn list_notes(&self) -> Vec<(&Uuid, &Note)> {
        let mut notes: Vec<_> = self.notes.iter().collect();
        // 최신순으로 정렬
        notes.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
        notes
    }

    pub fn get_note(&self, id: &Uuid) -> Option<&Note> {
        self.notes.get(id)
    }

    pub fn search_notes(&self, query: &str) -> Vec<(&Uuid, &Note)> {
        let query_lower = query.to_lowercase();
        self.notes
            .iter()
            .filter(|(_, note)| {
                note.meta.title.to_lowercase().contains(&query_lower)
                    || note.content.to_lowercase().contains(&query_lower)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn get_notes_by_folder(&self, folder: &str) -> Vec<(&Uuid, &Note)> {
        let folder_tag = if folder.starts_with('@') {
            folder.to_string()
        } else {
            format!("@{}", folder)
        };

        self.notes
            .iter()
            .filter(|(_, note)| note.tags.contains(&folder_tag))
            .collect()
    }

    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags = std::collections::HashSet::new();

        for note in self.notes.values() {
            for tag in &note.tags {
                tags.insert(tag.clone());
            }
        }

        let mut sorted_tags: Vec<_> = tags.into_iter().collect();
        sorted_tags.sort();
        sorted_tags
    }

    pub fn get_folders(&self) -> Vec<String> {
        self.get_all_tags()
            .into_iter()
            .filter(|tag| tag.starts_with('@'))
            .collect()
    }
}
