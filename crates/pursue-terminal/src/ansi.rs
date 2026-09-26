//! ANSI escape sequence utilities for terminal output display.
//!
//! # Core Architectural Invariant
//!
//! - **RAW OUTPUT = SOURCE OF TRUTH**: When command output is captured into
//!   the evidence repository, raw unaltered bytes are hashed and preserved.
//!   Stripping ANSI escape codes or altering bytes before hashing would invalidate
//!   the evidentiary integrity of the artifact.
//! - **DISPLAY OUTPUT = DERIVED REPRESENTATION**: The helper [`strip_ansi`] is
//!   provided solely as a view/display utility for presentation, logging, or terminal
//!   UI preview without risking terminal injection attacks. It must never overwrite
//!   or replace the raw evidentiary bytes.

/// Strips ANSI escape sequences (CSI, OSC, and standard 2-byte escape sequences)
/// from the byte slice and returns a sanitized UTF-8 string.
///
/// Invalid UTF-8 bytes are converted losslessly using the Unicode replacement character (`\u{FFFD}`).
pub fn strip_ansi(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            // Escape character encountered
            i += 1;
            if i >= bytes.len() {
                break;
            }

            match bytes[i] {
                // Control Sequence Introducer (CSI): ESC [ ... <final_byte 0x40-0x7e>
                b'[' => {
                    i += 1;
                    while i < bytes.len() {
                        let b = bytes[i];
                        i += 1;
                        if (0x40..=0x7e).contains(&b) {
                            break;
                        }
                    }
                }
                // Operating System Command (OSC): ESC ] ... (ST / BEL)
                b']' => {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            // BEL terminator
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            // String Terminator (ST): ESC \
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                // Standard 2-character escape sequences: ESC followed by byte in 0x40..=0x5F
                0x40..=0x5f => {
                    i += 1;
                }
                // Unknown escape sequence: skip this character
                _ => {
                    i += 1;
                }
            }
        } else {
            // Normal byte: scan contiguous non-escape slice
            let start = i;
            while i < bytes.len() && bytes[i] != 0x1b {
                i += 1;
            }
            output.push_str(&String::from_utf8_lossy(&bytes[start..i]));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;

    #[test]
    fn plain_text_passes_through_unchanged() {
        let input = b"Hello, world! 123";
        assert_eq!(strip_ansi(input), "Hello, world! 123");
    }

    #[test]
    fn strips_sgr_color_codes() {
        let input = b"\x1b[31;1mRed Bold Text\x1b[0m normal";
        assert_eq!(strip_ansi(input), "Red Bold Text normal");
    }

    #[test]
    fn strips_cursor_movement_and_erase_codes() {
        let input = b"\x1b[2J\x1b[HCleared screen\x1b[1K";
        assert_eq!(strip_ansi(input), "Cleared screen");
    }

    #[test]
    fn strips_osc_window_titles() {
        let input_bel = b"\x1b]0;My Title\x07Visible text";
        assert_eq!(strip_ansi(input_bel), "Visible text");

        let input_st = b"\x1b]0;Another Title\x1b\\Visible text";
        assert_eq!(strip_ansi(input_st), "Visible text");
    }

    #[test]
    fn handles_incomplete_escape_at_eof() {
        let input = b"Trailing escape \x1b";
        assert_eq!(strip_ansi(input), "Trailing escape ");

        let input_csi = b"Trailing CSI \x1b[";
        assert_eq!(strip_ansi(input_csi), "Trailing CSI ");
    }

    #[test]
    fn handles_non_utf8_losslessly() {
        let input = b"\x1b[32mValid\x1b[0m \xff \xfe";
        let stripped = strip_ansi(input);
        assert!(stripped.starts_with("Valid "));
        assert!(stripped.contains('\u{FFFD}'));
    }

    #[test]
    fn raw_input_remains_unmodified_invariant() {
        // Raw bytes are preserved for evidence; strip_ansi produces a separate view
        let raw_bytes = b"\x1b[33mEvidence output\x1b[0m\n";
        let display_view = strip_ansi(raw_bytes);

        assert_eq!(display_view, "Evidence output\n");
        assert_eq!(raw_bytes.len(), 25); // Raw byte length untouched
        assert_ne!(raw_bytes, display_view.as_bytes());
    }
}
