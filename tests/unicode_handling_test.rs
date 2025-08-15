// Unicode handling comprehensive test
// Tests for Issue #106: Server fails to properly handle certain Unicode content

use anyhow::Result;
use kotadb::builders::DocumentBuilder;
use kotadb::types::{ValidatedPath, ValidatedTitle};

#[tokio::test]
async fn test_comprehensive_unicode_support() -> Result<()> {
    // Test various Unicode characters in paths
    let unicode_paths = [
        "/documents/русский.md", // Cyrillic
        "/documents/中文.md",    // Chinese
        "/documents/العربية.md", // Arabic
        "/documents/🚀📝.md",    // Emojis
        "/documents/café.md",    // Accented characters
        "/documents/naïve.md",   // Diacritics
        "/documents/ñoño.md",    // Spanish
        "/documents/müller.md",  // German
        "/documents/Москва.md",  // More Cyrillic
        "/documents/東京.md",    // Japanese Kanji
        "/documents/한국.md",    // Korean
    ];

    println!("Testing Unicode path support...");
    for path_str in &unicode_paths {
        match ValidatedPath::new(path_str) {
            Ok(_) => println!("✅ Path accepted: {}", path_str),
            Err(e) => println!("❌ Path rejected: {} - Error: {}", path_str, e),
        }
    }

    // Test Unicode characters in titles
    let unicode_titles = [
        "Русский документ",        // Cyrillic
        "中文文档",                // Chinese
        "وثيقة عربية",             // Arabic
        "Document with 🚀 emojis", // Emojis
        "Café Menu",               // Accented
        "Naïve Approach",          // Diacritics
        "España Guide",            // Spanish
        "Müller's Notes",          // German
        "Москва Travel Guide",     // Mixed Cyrillic/English
        "東京 Guide",              // Japanese/English
        "한국 문서",               // Korean
    ];

    println!("\nTesting Unicode title support...");
    for title_str in &unicode_titles {
        match ValidatedTitle::new(*title_str) {
            Ok(_) => println!("✅ Title accepted: {}", title_str),
            Err(e) => println!("❌ Title rejected: {} - Error: {}", title_str, e),
        }
    }

    // Test Unicode content in documents
    let unicode_content_samples = [
        "Hello, 世界! 🌍",             // Mixed English/Chinese/Emoji
        "Привет мир! Как дела?",       // Cyrillic
        "مرحبا بالعالم",               // Arabic
        "¡Hola mundo! ¿Cómo estás?",   // Spanish with punctuation
        "Bonjour le monde! 🇫🇷",        // French with flag emoji
        "Hej världen! 🇸🇪",             // Swedish with flag emoji
        "𝕳𝖊𝖑𝖑𝖔 𝖂𝖔𝖗𝖑𝖉!",                // Mathematical bold
        "🚀🌟✨💫🎉🎊🎈🎁",            // Multiple emojis
        "\u{1F600}\u{1F601}\u{1F602}", // Unicode escape sequences
    ];

    println!("\nTesting Unicode content in documents...");
    for (i, content_str) in unicode_content_samples.iter().enumerate() {
        let path = format!("test/unicode_{}.md", i);
        let title = format!("Unicode Test {}", i);

        let result = DocumentBuilder::new()
            .path(&path)?
            .title(&title)?
            .content(content_str.as_bytes())
            .build();

        match result {
            Ok(doc) => {
                println!(
                    "✅ Document created with Unicode content: {} chars",
                    content_str.len()
                );

                // Verify content round-trip
                let retrieved_content = String::from_utf8_lossy(&doc.content);
                if retrieved_content == *content_str {
                    println!("  ✅ Content round-trip successful");
                } else {
                    println!("  ❌ Content round-trip failed!");
                    println!("    Expected: {}", content_str);
                    println!("    Got: {}", retrieved_content);
                }
            }
            Err(e) => println!("❌ Document creation failed: {}", e),
        }
    }

    // Test edge cases with Unicode normalization
    println!("\nTesting Unicode normalization edge cases...");

    // These should be equivalent but might be represented differently
    let normalization_tests = [
        ("café", "cafe\u{0301}"),   // Precomposed vs combining characters
        ("naïve", "nai\u{0308}ve"), // Precomposed vs combining diaeresis
    ];

    for (precomposed, combining) in normalization_tests {
        println!(
            "Testing normalization: '{}' vs '{}'",
            precomposed, combining
        );

        let path1_result = ValidatedPath::new(format!("test/{}.md", precomposed));
        let path2_result = ValidatedPath::new(format!("test/{}.md", combining));

        match (path1_result, path2_result) {
            (Ok(_), Ok(_)) => println!("  ✅ Both forms accepted"),
            (Err(e), _) => println!("  ❌ Precomposed failed: {}", e),
            (_, Err(e)) => println!("  ❌ Combining failed: {}", e),
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_unicode_edge_cases() -> Result<()> {
    // Test various Unicode edge cases that might cause issues

    // Test very long Unicode strings
    let long_unicode = "🚀".repeat(100);
    match ValidatedTitle::new(&long_unicode) {
        Ok(_) => println!("✅ Long Unicode title accepted"),
        Err(e) => println!("❌ Long Unicode title rejected: {}", e),
    }

    // Test Unicode with control characters (should be handled gracefully)
    let with_controls = "Text\u{200B}with\u{FEFF}controls"; // Zero-width space and BOM
    match ValidatedTitle::new(with_controls) {
        Ok(_) => println!("✅ Unicode with controls accepted"),
        Err(e) => println!("❌ Unicode with controls rejected: {}", e),
    }

    // Test bidirectional text (Arabic + English)
    let bidi_text = "Hello مرحبا World";
    match ValidatedTitle::new(bidi_text) {
        Ok(_) => println!("✅ Bidirectional text accepted"),
        Err(e) => println!("❌ Bidirectional text rejected: {}", e),
    }

    // Test Unicode in different positions of path
    let unicode_paths = [
        "/🚀/document.md",      // Unicode directory
        "/documents/🚀.md",     // Unicode filename
        "/🚀/📝/document.md",   // Multiple Unicode components
        "/русский/документ.md", // Unicode directory and filename
    ];

    for path in unicode_paths {
        match ValidatedPath::new(path) {
            Ok(_) => println!("✅ Complex Unicode path accepted: {}", path),
            Err(e) => println!("❌ Complex Unicode path rejected: {} - {}", path, e),
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_unicode_content_processing() -> Result<()> {
    // Test that Unicode content is properly handled throughout the system

    let unicode_content = r#"
    # Unicode Test Document 🚀

    This document contains various Unicode characters:
    
    - **Emojis**: 🎉 🎊 🎈 🎁 🌟 ✨ 💫
    - **Cyrillic**: Привет мир! Как дела?
    - **Chinese**: 你好世界！你好吗？
    - **Arabic**: مرحبا بالعالم! كيف حالك؟
    - **Mathematical**: 𝕳𝖊𝖑𝖑𝖔 𝖂𝖔𝖗𝖑𝖉
    - **Special chars**: ©️ ™️ ®️ ℠ ℗
    
    Mixed scripts: Hello مرحبا 世界 Мир 🌍
    "#;

    let doc = DocumentBuilder::new()
        .path("test/comprehensive_unicode.md")?
        .title("Comprehensive Unicode Test 🧪")?
        .content(unicode_content.as_bytes())
        .build()?;

    // Verify all Unicode content is preserved
    let retrieved_content = String::from_utf8_lossy(&doc.content);
    assert_eq!(
        retrieved_content, unicode_content,
        "Unicode content should be preserved exactly"
    );

    // Verify title handling
    assert_eq!(doc.title.as_str(), "Comprehensive Unicode Test 🧪");

    // Verify path handling
    assert_eq!(doc.path.as_str(), "test/comprehensive_unicode.md");

    println!("✅ All Unicode content properly preserved and handled");

    Ok(())
}
