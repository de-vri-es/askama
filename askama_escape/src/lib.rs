use std::fmt::{self, Display, Formatter};
use std::io::{self, prelude::*};
use std::str;

pub struct MarkupDisplay<T, O>
where
    T: Display,
    O: OutputFormat,
{
    value: DisplayValue<T>,
    out: O,
}

impl<T, O> MarkupDisplay<T, O>
where
    T: Display,
    O: OutputFormat,
{
    pub fn new_unsafe(value: T, out: O) -> Self {
        Self {
            value: DisplayValue::Unsafe(value),
            out,
        }
    }

    pub fn new_safe(value: T, out: O) -> Self {
        Self {
            value: DisplayValue::Safe(value),
            out,
        }
    }

    pub fn mark_safe(mut self) -> MarkupDisplay<T, O> {
        self.value = match self.value {
            DisplayValue::Unsafe(t) => DisplayValue::Safe(t),
            _ => self.value,
        };
        self
    }
}

impl<T, O> Display for MarkupDisplay<T, O>
where
    T: Display,
    O: OutputFormat,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.value {
            DisplayValue::Unsafe(ref t) => write!(
                EscapeWriter {
                    fmt,
                    out: &self.out
                },
                "{}",
                t
            )
            .map_err(|_| fmt::Error),
            DisplayValue::Safe(ref t) => t.fmt(fmt),
        }
    }
}

pub struct EscapeWriter<'a, 'b: 'a, O> {
    fmt: &'a mut fmt::Formatter<'b>,
    out: &'a O,
}

impl<O> io::Write for EscapeWriter<'_, '_, O>
where
    O: OutputFormat,
{
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.out
            .write_escaped_bytes(self.fmt, bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn escape<O>(s: &str, out: O) -> Escaped<'_, O>
where
    O: OutputFormat,
{
    Escaped {
        bytes: s.as_bytes(),
        out,
    }
}

pub struct Escaped<'a, O>
where
    O: OutputFormat,
{
    bytes: &'a [u8],
    out: O,
}

impl<'a, O> ::std::fmt::Display for Escaped<'a, O>
where
    O: OutputFormat,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.out.write_escaped_bytes(fmt, self.bytes)
    }
}

pub struct Html;

macro_rules! escaping_body {
    ($start:ident, $i:ident, $fmt:ident, $bytes:ident, $quote:expr) => {{
        if $start < $i {
            $fmt.write_str(unsafe { str::from_utf8_unchecked(&$bytes[$start..$i]) })?;
        }
        $fmt.write_str($quote)?;
        $start = $i + 1;
    }};
}

impl OutputFormat for Html {
    fn write_escaped_bytes(&self, fmt: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
        let mut start = 0;
        for (i, b) in bytes.iter().enumerate() {
            if b.wrapping_sub(b'"') <= FLAG {
                match *b {
                    b'<' => escaping_body!(start, i, fmt, bytes, "&lt;"),
                    b'>' => escaping_body!(start, i, fmt, bytes, "&gt;"),
                    b'&' => escaping_body!(start, i, fmt, bytes, "&amp;"),
                    b'"' => escaping_body!(start, i, fmt, bytes, "&quot;"),
                    b'\'' => escaping_body!(start, i, fmt, bytes, "&#x27;"),
                    b'/' => escaping_body!(start, i, fmt, bytes, "&#x2f;"),
                    _ => (),
                }
            }
        }
        fmt.write_str(unsafe { str::from_utf8_unchecked(&bytes[start..]) })
    }
}

pub struct Text;

impl OutputFormat for Text {
    fn write_escaped_bytes(&self, fmt: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
        fmt.write_str(unsafe { str::from_utf8_unchecked(bytes) })
    }
}

#[derive(Debug, PartialEq)]
enum DisplayValue<T>
where
    T: Display,
{
    Safe(T),
    Unsafe(T),
}

pub trait OutputFormat {
    fn write_escaped_bytes(&self, fmt: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result;
}

const FLAG: u8 = b'>' - b'"';

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_escape() {
        assert_eq!(escape("", Html).to_string(), "");
        assert_eq!(escape("<&>", Html).to_string(), "&lt;&amp;&gt;");
        assert_eq!(escape("bla&", Html).to_string(), "bla&amp;");
        assert_eq!(escape("<foo", Html).to_string(), "&lt;foo");
        assert_eq!(escape("bla&h", Html).to_string(), "bla&amp;h");
    }
}
