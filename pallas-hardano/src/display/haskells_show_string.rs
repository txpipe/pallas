/// Haskell’s `Data.Text` (and `Data.String`) have "slightly" non-standard way
/// of displaying non-printable Unicode characters in their `Show` instances.
/// Let’s make sure we replicate that correctly in Rust.
pub(crate) fn haskell_show_string(s: &str) -> String {
    fn is_oct_digit(c: char) -> bool {
        ('0'..='7').contains(&c)
    }

    let mut result = String::new();
    result.push('"');

    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\x07' => result.push_str("\\a"),                // Bell
            '\x08' => result.push_str("\\b"),                // Backspace
            '\x0C' => result.push_str("\\f"),                // Form Feed
            '\x0A' => result.push_str("\\n"),                // Line Feed
            '\x0D' => result.push_str("\\r"),                // Carriage Return
            '\x09' => result.push_str("\\t"),                // Horizontal Tab
            '\x0B' => result.push_str("\\v"),                // Vertical Tab
            c if (' '..='~').contains(&c) => result.push(c), // Printable ASCII
            c => {
                let abbreviation = match c {
                    '\x00' => "NUL",
                    '\x01' => "SOH",
                    '\x02' => "STX",
                    '\x03' => "ETX",
                    '\x04' => "EOT",
                    '\x05' => "ENQ",
                    '\x06' => "ACK",
                    '\x0E' => "SO",
                    '\x0F' => "SI",
                    '\x10' => "DLE",
                    '\x11' => "DC1",
                    '\x12' => "DC2",
                    '\x13' => "DC3",
                    '\x14' => "DC4",
                    '\x15' => "NAK",
                    '\x16' => "SYN",
                    '\x17' => "ETB",
                    '\x18' => "CAN",
                    '\x19' => "EM",
                    '\x1A' => "SUB",
                    '\x1B' => "ESC",
                    '\x1C' => "FS",
                    '\x1D' => "GS",
                    '\x1E' => "RS",
                    '\x1F' => "US",
                    '\x7F' => "DEL",
                    _ => "",
                };
                if !abbreviation.is_empty() {
                    result.push('\\');
                    result.push_str(abbreviation);

                    // Insert \& if next character would be confusing
                    if let Some(&next_c) = chars.peek() {
                        if abbreviation == "SO" && next_c == 'H' {
                            result.push_str("\\&");
                        }
                    }
                } else if c <= '\x7F' {
                    // Control characters without abbreviation (use octal escape)
                    let code = c as u32 % 256;
                    let escape_seq = format!("{code:o}");
                    result.push('\\');
                    result.push_str(&escape_seq);

                    // Insert \& if next character is an octal digit
                    if let Some(&next_c) = chars.peek() {
                        if is_oct_digit(next_c) {
                            result.push_str("\\&");
                        }
                    }
                } else {
                    // Characters with code > 127 (use decimal escape)
                    let code = c as u32;
                    result.push('\\');
                    result.push_str(&code.to_string());

                    // Insert \& if next character is a decimal digit
                    if let Some(&next_c) = chars.peek() {
                        if next_c.is_ascii_digit() {
                            result.push_str("\\&");
                        }
                    }
                }
            }
        }
    }

    result.push('"');
    result
}
