use std::num::NonZeroU16;

use agmux_tui::{encode_key_to_pty_bytes, screen_to_ratatui_lines, ui::active_viewport_inner_size};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier},
};

#[test]
fn test_screen_to_ratatui_lines_256_and_truecolor_and_modifiers() {
    let rows = NonZeroU16::new(10).unwrap();
    let cols = NonZeroU16::new(80).unwrap();
    let mut parser = vt100::Parser::new(rows, cols, 0);

    // Process ANSI sequences:
    // 1. 256-color foreground (idx 42)
    // 2. 24-bit Truecolor foreground (rgb 250, 120, 30) & Truecolor background (rgb 10, 20, 30)
    // 3. Bold, Italic, Underline, Reversed
    let input = b"\x1b[38;5;42mColor42 \x1b[38;2;250;120;30;48;2;10;20;30mTrueRGB \x1b[1;3;4;7mStyled\x1b[0m End";
    parser.process(input);

    let lines = screen_to_ratatui_lines(parser.screen());
    assert!(!lines.is_empty(), "Lines must not be empty");

    let first_line = &lines[0];
    assert!(
        first_line.spans.len() >= 4,
        "Expected multiple styled spans, got {:?}",
        first_line.spans
    );

    // Span 1: "Color42 " with fg Indexed(42)
    let span1 = &first_line.spans[0];
    assert_eq!(span1.content, "Color42 ");
    assert_eq!(span1.style.fg, Some(Color::Indexed(42)));

    // Span 2: "TrueRGB " with fg Rgb(250, 120, 30) and bg Rgb(10, 20, 30)
    let span2 = &first_line.spans[1];
    assert_eq!(span2.content, "TrueRGB ");
    assert_eq!(span2.style.fg, Some(Color::Rgb(250, 120, 30)));
    assert_eq!(span2.style.bg, Some(Color::Rgb(10, 20, 30)));

    // Span 3: "Styled" with bold, italic, underline, reversed
    let span3 = &first_line.spans[2];
    assert_eq!(span3.content, "Styled");
    assert!(span3.style.add_modifier.contains(Modifier::BOLD));
    assert!(span3.style.add_modifier.contains(Modifier::ITALIC));
    assert!(span3.style.add_modifier.contains(Modifier::UNDERLINED));
    assert!(span3.style.add_modifier.contains(Modifier::REVERSED));

    // Span 4: " End" with reset style
    let span4 = &first_line.spans[3];
    assert_eq!(span4.content, " End");
    assert_eq!(span4.style.fg, None);
    assert_eq!(span4.style.bg, None);
}

#[test]
fn test_screen_to_ratatui_lines_coalescing_and_wide_unicode() {
    let rows = NonZeroU16::new(5).unwrap();
    let cols = NonZeroU16::new(40).unwrap();
    let mut parser = vt100::Parser::new(rows, cols, 0);

    // Wide character: crab emoji 🦀
    parser.process("🦀 Rust Power\r\nLine 2".as_bytes());

    let lines = screen_to_ratatui_lines(parser.screen());
    assert!(lines.len() >= 2);

    let line0_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(line0_text, "🦀 Rust Power");

    let line1_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(line1_text, "Line 2");
}

#[test]
fn test_encode_key_to_pty_bytes_full_navigation_and_alt() {
    // Normal Tab
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Tab, KeyModifiers::NONE),
        Some(b"\t".to_vec())
    );

    // Shift + Tab (BackTab)
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::BackTab, KeyModifiers::NONE),
        Some(b"\x1b[Z".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Tab, KeyModifiers::SHIFT),
        Some(b"\x1b[Z".to_vec())
    );

    // Alt + f (Word Forward)
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Char('f'), KeyModifiers::ALT),
        Some(vec![0x1b, b'f'])
    );

    // Alt + b (Word Backward)
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Char('b'), KeyModifiers::ALT),
        Some(vec![0x1b, b'b'])
    );

    // Alt + Backspace (Backward Kill Word)
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Backspace, KeyModifiers::ALT),
        Some(vec![0x1b, 0x7f])
    );

    // Ctrl + Left / Right
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Left, KeyModifiers::CONTROL),
        Some(b"\x1b[1;5D".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Right, KeyModifiers::CONTROL),
        Some(b"\x1b[1;5C".to_vec())
    );

    // PageUp / PageDown / Insert / Delete
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::PageUp, KeyModifiers::NONE),
        Some(b"\x1b[5~".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::PageDown, KeyModifiers::NONE),
        Some(b"\x1b[6~".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Insert, KeyModifiers::NONE),
        Some(b"\x1b[2~".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::Delete, KeyModifiers::NONE),
        Some(b"\x1b[3~".to_vec())
    );

    // F1 - F4
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::F(1), KeyModifiers::NONE),
        Some(b"\x1bOP".to_vec())
    );
    assert_eq!(
        encode_key_to_pty_bytes(KeyCode::F(4), KeyModifiers::NONE),
        Some(b"\x1bOS".to_vec())
    );
}

#[test]
fn test_active_viewport_inner_size_calculation() {
    let area = Rect::new(0, 0, 100, 40);
    // Unsplit view: status bar takes 1 row (rows[0] = 39), viewport takes 65% width (65 cols)
    // Inner size subtracts 2 for borders: 65 - 2 = 63 cols, 39 - 2 = 37 rows
    let (cols, rows) = active_viewport_inner_size(area, false);
    assert_eq!(cols, 63);
    assert_eq!(rows, 37);

    // Split view: 50% of 65 = 32 width; 32 - 2 = 30 cols
    let (split_cols, split_rows) = active_viewport_inner_size(area, true);
    assert_eq!(split_cols, 30);
    assert_eq!(split_rows, 37);
}
