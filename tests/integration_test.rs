use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// 테스트용 헬퍼 함수들
fn create_test_note(dir: &PathBuf, filename: &str, content: &str) {
    let path = dir.join(filename);
    fs::write(path, content).expect("Failed to write test file");
}

fn create_test_app() -> (TempDir, MD_Filer::app::NoteApp) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let notes_dir = temp_dir.path().to_path_buf();

    // 테스트용 마크다운 파일들 생성
    create_test_note(
        &notes_dir,
        "note1.md",
        r#"---
title: First Note
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-01T00:00:00Z
---

# First Note

This is my first test note about **Rust programming**."#,
    );

    create_test_note(
        &notes_dir,
        "note2.md",
        r#"---
title: Second Note
created_at: 2024-01-02T00:00:00Z
updated_at: 2024-01-02T00:00:00Z
---

# Second Note

This note is about web development."#,
    );

    create_test_note(
        &notes_dir,
        "note3.md",
        r#"# Simple Note

This note has no frontmatter."#,
    );

    let app = MD_Filer::app::NoteApp::new(notes_dir).expect("Failed to create app");
    (temp_dir, app)
}

#[test]
fn test_app_loads_notes() {
    let (_temp_dir, app) = create_test_app();

    let notes = app.list_notes();
    assert_eq!(notes.len(), 3);
}

#[test]
fn test_app_search_notes() {
    let (_temp_dir, app) = create_test_app();

    // "Rust"로 검색
    let results = app.search_notes("Rust");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.meta.title, "First Note");

    // "note"로 검색 (대소문자 무시)
    let results = app.search_notes("note");
    assert!(results.len() >= 2);
}

#[test]
fn test_app_sync_removes_deleted_files() {
    let (temp_dir, mut app) = create_test_app();

    // 초기 노트 개수 확인
    assert_eq!(app.list_notes().len(), 3);

    // 파일 하나 삭제
    let file_to_delete = temp_dir.path().join("note1.md");
    fs::remove_file(file_to_delete).expect("Failed to delete file");

    // 앱 재로드
    app.load_notes().expect("Failed to reload notes");

    // 노트 개수 확인
    assert_eq!(app.list_notes().len(), 2);
}

#[test]
fn test_app_handles_new_files() {
    let (temp_dir, mut app) = create_test_app();

    // 초기 노트 개수
    assert_eq!(app.list_notes().len(), 3);

    // 새 파일 추가
    create_test_note(
        &temp_dir.path().to_path_buf(),
        "note4.md",
        r#"# New Note

This is a newly added note."#,
    );

    // 앱 재로드
    app.load_notes().expect("Failed to reload notes");

    // 노트 개수 확인
    assert_eq!(app.list_notes().len(), 4);
}

#[test]
fn test_index_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let notes_dir = temp_dir.path().to_path_buf();

    // 첫 번째 앱 생성 및 노트 추가
    create_test_note(&notes_dir, "persistent.md", "# Persistent Note");
    {
        let app = MD_Filer::app::NoteApp::new(notes_dir.clone()).expect("Failed to create app");
        assert_eq!(app.list_notes().len(), 1);
    }

    // 새 앱 인스턴스 생성 - 인덱스가 유지되는지 확인
    let app2 = MD_Filer::app::NoteApp::new(notes_dir).expect("Failed to create app");
    assert_eq!(app2.list_notes().len(), 1);

    // 인덱스 파일이 생성되었는지 확인
    let index_path = temp_dir.path().join(".index.json");
    assert!(index_path.exists());
}

#[test]
fn test_get_folders_and_tags() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let notes_dir = temp_dir.path().to_path_buf();

    // 태그가 포함된 노트 생성
    create_test_note(
        &notes_dir,
        "tagged.md",
        r#"---
title: Tagged Note
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-01T00:00:00Z
---

# Tagged Note"#,
    );

    let mut app = MD_Filer::app::NoteApp::new(notes_dir).expect("Failed to create app");

    // 인덱스에 직접 태그 추가 (실제로는 사용자가 추가하는 것)
    if let Some(note) = app.list_notes().first() {
        let id = *note.0;
        if let Some(entry) = app.index.mappings.get_mut(&id) {
            entry.tags = vec![
                "rust".to_string(),
                "@projects".to_string(),
                "tutorial".to_string(),
            ];
        }
        // 노트 재로드
        app.load_notes().expect("Failed to reload");
    }

    let all_tags = app.get_all_tags();
    assert!(all_tags.contains(&"rust".to_string()));
    assert!(all_tags.contains(&"@projects".to_string()));

    let folders = app.get_folders();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0], "@projects");
}
