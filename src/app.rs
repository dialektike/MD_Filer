use crate::index::{IndexEntry, NoteIndex};
use crate::note::Note;
use crate::shortcuts::ShortcutsRegistry;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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
        let mut index = if index_path.exists() {
            NoteIndex::load(&index_path)?
        } else {
            NoteIndex::new()
        };

        let shortcuts = if shortcuts_path.exists() {
            ShortcutsRegistry::load(&shortcuts_path)?
        } else {
            ShortcutsRegistry::new()
        };

        // 기본 폴더가 watched_folders에 없으면 추가
        let default_folder = notes_dir.to_string_lossy().to_string();
        if index.get_watched_folders().is_empty() {
            index.add_watched_folder(default_folder);
        }

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

        // 모든 watched_folders를 스캔
        for folder_path in self.index.get_watched_folders().clone() {
            let folder = PathBuf::from(&folder_path);
            if !folder.exists() {
                eprintln!("⚠️  폴더가 존재하지 않습니다: {}", folder_path);
                continue;
            }

            let entries = fs::read_dir(&folder)
                .map_err(|e| format!("디렉토리 읽기 실패 {}: {}", folder_path, e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("엔트리 읽기 실패: {}", e))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let file_path = path.to_string_lossy().to_string();

                    // 인덱스에서 UUID 찾기 또는 새로 생성
                    let (id, is_new) = if let Some((id, _)) = self.index.find_by_filename(&filename)
                    {
                        (id, false)
                    } else {
                        (Uuid::new_v4(), true)
                    };

                    let content = fs::read_to_string(&path)
                        .map_err(|e| format!("파일 읽기 실패 {}: {}", filename, e))?;

                    // 인덱스에서 타임스탬프와 태그 가져오기
                    let now = Utc::now();
                    let (tags, created_at, updated_at) =
                        if let Some(entry) = self.index.get_entry(&id) {
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

                            // 인덱스 업데이트 (기존 태그 유지)
                            let entry = IndexEntry {
                                filename: filename.clone(),
                                file_path: file_path.clone(),
                                title: note.title.clone(),
                                created_at: note.created_at,
                                updated_at: note.updated_at,
                                tags: tags,
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
        }

        // 인덱스 저장
        self.save_index()?;
        Ok(())
    }

    // 파일 시스템과 인덱스 동기화
    pub fn sync_with_filesystem(&mut self) -> Result<(), String> {
        // 1. 모든 watched_folders에서 현재 파일 목록 가져오기
        let mut existing_files = std::collections::HashSet::new();

        for folder_path in self.index.get_watched_folders().clone() {
            let folder = PathBuf::from(&folder_path);
            if !folder.exists() {
                continue;
            }

            let entries = fs::read_dir(&folder)
                .map_err(|e| format!("디렉토리 읽기 실패 {}: {}", folder_path, e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("엔트리 읽기 실패: {}", e))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    let file_path = path.to_string_lossy().to_string();
                    existing_files.insert(file_path);
                }
            }
        }

        // 2. 인덱스에서 삭제된 파일 제거
        let mut to_remove = Vec::new();
        for (id, entry) in self.index.mappings.iter() {
            let entry_path = if entry.file_path.is_empty() {
                // 구버전 호환: file_path가 없으면 notes_dir + filename 사용
                self.notes_dir
                    .join(&entry.filename)
                    .to_string_lossy()
                    .to_string()
            } else {
                entry.file_path.clone()
            };

            if !existing_files.contains(&entry_path) {
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
                note.title.to_lowercase().contains(&query_lower)
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

    // 새로운 폴더를 watched_folders에 추가
    pub fn add_watched_folder(&mut self, folder_path: String) -> Result<(), String> {
        let folder = PathBuf::from(&folder_path);

        // 폴더 존재 여부 확인
        if !folder.exists() {
            return Err(format!("폴더가 존재하지 않습니다: {}", folder_path));
        }

        // 이미 추가되어 있는지 확인
        if self.index.get_watched_folders().contains(&folder_path) {
            return Err(format!("이미 추가된 폴더입니다: {}", folder_path));
        }

        // 폴더 추가
        self.index.add_watched_folder(folder_path.clone());

        // 인덱스 저장
        self.save_index()?;

        // 노트 다시 로드
        self.load_notes()?;

        Ok(())
    }

    // watched_folders에서 폴더 제거
    pub fn remove_watched_folder(&mut self, folder_path: &str) -> Result<(), String> {
        if !self.index.remove_watched_folder(folder_path) {
            return Err(format!("폴더를 찾을 수 없습니다: {}", folder_path));
        }

        // 해당 폴더의 노트들을 인덱스에서 제거
        let mut to_remove = Vec::new();
        for (id, entry) in self.index.mappings.iter() {
            if entry.file_path.starts_with(folder_path) {
                to_remove.push(*id);
            }
        }

        for id in to_remove {
            self.index.remove_entry(&id);
        }

        // 인덱스 저장
        self.save_index()?;

        // 노트 다시 로드
        self.load_notes()?;

        Ok(())
    }

    // 관리 중인 폴더 목록 가져오기
    pub fn list_watched_folders(&self) -> &Vec<String> {
        self.index.get_watched_folders()
    }
}
