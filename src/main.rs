mod app;
mod index;
mod note;
mod shortcuts;

use app::NoteApp;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 노트 디렉토리 설정
    let notes_dir = env::var("NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./notes"));

    println!("🎉 노트앱에 오신 것을 환영합니다!");
    println!("📂 노트 디렉토리: {}", notes_dir.display());

    // 앱 초기화
    let mut app = NoteApp::new(notes_dir.clone())?;

    // 시작 시 목록 표시
    show_notes_list(&app);

    loop {
        println!("\n명령어: [l]ist, [s]how <번호>, [se]arch <검색어>, [t]ags, [r]efresh, [q]uit");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "l" | "list" => {
                show_notes_list(&app);
            }
            "s" | "show" => {
                if parts.len() < 2 {
                    println!("❌ 사용법: show <번호>");
                    continue;
                }
                show_note_detail(&app, parts[1]);
            }
            "se" | "search" => {
                if parts.len() < 2 {
                    println!("❌ 사용법: search <검색어>");
                    continue;
                }
                let query = parts[1..].join(" ");
                search_notes(&app, &query);
            }
            "t" | "tags" => {
                show_tags(&app);
            }
            "r" | "refresh" => {
                println!("🔄 노트 목록 새로고침 중...");
                app = NoteApp::new(notes_dir.clone())?;
                println!("✅ 새로고침 완료!");
                show_notes_list(&app);
            }
            "q" | "quit" => {
                println!("👋 안녕히 가세요!");
                break;
            }
            _ => {
                println!("❌ 알 수 없는 명령어입니다.");
            }
        }
    }

    Ok(())
}

fn show_notes_list(app: &NoteApp) {
    let notes = app.list_notes();

    if notes.is_empty() {
        println!("\n📭 노트가 없습니다.");
        return;
    }

    println!("\n📋 노트 목록 ({} 개)", notes.len());
    println!("{:-<60}", "");

    for (idx, (id, note)) in notes.iter().enumerate() {
        let folder = note.get_folder_tag().unwrap_or("");
        let tags = note.get_regular_tags();
        let tags_str = if tags.is_empty() {
            String::new()
        } else {
            format!("[{}]", tags.join(", "))
        };

        // Shortcuts 개수 표시
        let shortcuts_count = if let Some(shortcuts) = app.shortcuts.get_shortcuts(id) {
            shortcuts.len()
        } else {
            0
        };
        let shortcuts_str = if shortcuts_count > 0 {
            format!(" 🔗{}", shortcuts_count)
        } else {
            String::new()
        };

        println!(
            "{:3}. {} {} {} {}{}",
            idx + 1,
            note.title,
            note.updated_at.format("%Y-%m-%d"),
            folder,
            tags_str,
            shortcuts_str
        );
    }
    println!("{:-<60}", "");
}

fn show_note_detail(app: &NoteApp, number_str: &str) {
    let index = match number_str.parse::<usize>() {
        Ok(n) if n > 0 => n - 1,
        _ => {
            println!("❌ 올바른 번호를 입력하세요.");
            return;
        }
    };

    let notes = app.list_notes();
    if let Some((id, note)) = notes.get(index) {
        println!("\n📝 노트 상세");
        println!("{:-<60}", "");
        println!("제목: {}", note.title);
        println!("파일: {}", note.filename);
        println!("생성: {}", note.created_at.format("%Y-%m-%d %H:%M"));
        println!("수정: {}", note.updated_at.format("%Y-%m-%d %H:%M"));

        if let Some(folder) = note.get_folder_tag() {
            println!("📁 폴더: {}", folder);
        }

        let tags = note.get_regular_tags();
        if !tags.is_empty() {
            println!("🏷️  태그: {}", tags.join(", "));
        }

        // Shortcuts 표시
        if let Some(shortcuts) = app.shortcuts.get_shortcuts(id) {
            if !shortcuts.is_empty() {
                println!("🔗 단축어:");
                for (alias, shortcut) in shortcuts {
                    let target_str = match &shortcut.target {
                        crate::note::LinkTarget::Url { url } => url.clone(),
                        crate::note::LinkTarget::File { path } => path.display().to_string(),
                        crate::note::LinkTarget::Note { id } => app
                            .get_note(id)
                            .map(|n| n.title.clone())
                            .unwrap_or_else(|| format!("(노트 {})", id)),
                    };
                    println!("   {} → {}", alias, target_str);
                }
            }
        }

        println!("{:-<60}", "");
        println!("\n{}", note.content);
    } else {
        println!("❌ 해당 번호의 노트가 없습니다.");
    }
}

fn search_notes(app: &NoteApp, query: &str) {
    let results = app.search_notes(query);

    if results.is_empty() {
        println!("🔍 '{}' 검색 결과가 없습니다.", query);
        return;
    }

    println!("\n🔍 '{}' 검색 결과 ({} 개)", query, results.len());
    println!("{:-<60}", "");

    for (id, note) in results {
        println!("📝 {} - {}", note.title, note.updated_at.format("%Y-%m-%d"));

        // 내용 미리보기 (첫 50자)
        let preview: String = note.content.chars().take(50).collect();
        if !preview.is_empty() {
            println!("   {}", preview.replace('\n', " "));
        }
    }
}

fn show_tags(app: &NoteApp) {
    let folders = app.get_folders();
    let all_tags = app.get_all_tags();
    let regular_tags: Vec<_> = all_tags
        .iter()
        .filter(|tag| !tag.starts_with('@'))
        .collect();

    println!("\n🏷️  태그 목록");
    println!("{:-<60}", "");

    if !folders.is_empty() {
        println!("📁 폴더:");
        for folder in &folders {
            let count = app.get_notes_by_folder(folder).len();
            println!("   {} ({} 개)", folder, count);
        }
    }

    if !regular_tags.is_empty() {
        println!("\n🏷️  일반 태그:");
        for tag in &regular_tags {
            let count = app.index.find_by_tag(tag).len();
            println!("   {} ({} 개)", tag, count);
        }
    }

    if folders.is_empty() && regular_tags.is_empty() {
        println!("태그가 없습니다.");
    }
}
