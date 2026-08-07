#![cfg(feature = "simdutf")]
// Tests for the simdutf feature-gated bindings.
//
// These tests require `V8_FROM_SOURCE=1 cargo test --features simdutf`.

use v8::simdutf;

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn validate_utf8() {
  assert!(simdutf::validate_utf8(b"hello world"));
  assert!(simdutf::validate_utf8("café".as_bytes()));
  assert!(simdutf::validate_utf8("日本語".as_bytes()));
  assert!(simdutf::validate_utf8(b""));
  // Invalid: continuation byte without start
  assert!(!simdutf::validate_utf8(&[0x80]));
  // Invalid: overlong 2-byte
  assert!(!simdutf::validate_utf8(&[0xC0, 0x80]));
}

#[test]
fn validate_utf8_with_errors() {
  let r = simdutf::validate_utf8_with_errors(b"hello");
  assert!(r.is_ok());

  let r = simdutf::validate_utf8_with_errors(&[b'a', 0x80, b'b']);
  assert!(!r.is_ok());
  assert_eq!(r.count, 1); // error at byte index 1
}

#[test]
fn validate_ascii() {
  assert!(simdutf::validate_ascii(b"hello world 123"));
  assert!(!simdutf::validate_ascii(&[0x80]));
  assert!(!simdutf::validate_ascii("café".as_bytes()));
  assert!(simdutf::validate_ascii(b""));
}

#[test]
fn validate_utf16le() {
  // "hello" in UTF-16LE
  let data: Vec<u16> = "hello".encode_utf16().collect();
  assert!(simdutf::validate_utf16le(&data));
  // Unpaired high surrogate
  assert!(!simdutf::validate_utf16le(&[0xD800]));
  assert!(simdutf::validate_utf16le(&[]));
}

#[test]
fn validate_utf32() {
  let data: Vec<u32> = "hello 日本語".chars().map(|c| c as u32).collect();
  assert!(simdutf::validate_utf32(&data));
  // Invalid: surrogate
  assert!(!simdutf::validate_utf32(&[0xD800]));
  // Invalid: out of range
  assert!(!simdutf::validate_utf32(&[0x110000]));
}

// ---------------------------------------------------------------------------
// Conversion: UTF-8 <-> UTF-16LE
// ---------------------------------------------------------------------------

#[test]
fn utf8_to_utf16le_roundtrip() {
  let input = "hello café 日本語";
  let utf8 = input.as_bytes();
  let expected_utf16: Vec<u16> = input.encode_utf16().collect();

  let utf16_len = simdutf::utf16_length_from_utf8(utf8);
  assert_eq!(utf16_len, expected_utf16.len());

  let mut output = vec![0u16; utf16_len];
  let written = unsafe { simdutf::convert_utf8_to_utf16le(utf8, &mut output) };
  assert_eq!(written, expected_utf16.len());
  assert_eq!(output, expected_utf16);

  // Round-trip back to UTF-8
  let utf8_len = simdutf::utf8_length_from_utf16le(&output);
  assert_eq!(utf8_len, utf8.len());

  let mut utf8_out = vec![0u8; utf8_len];
  let written =
    unsafe { simdutf::convert_utf16le_to_utf8(&output, &mut utf8_out) };
  assert_eq!(written, utf8.len());
  assert_eq!(&utf8_out, utf8);
}

// ---------------------------------------------------------------------------
// Conversion: Latin-1 <-> UTF-8
// ---------------------------------------------------------------------------

#[test]
fn latin1_to_utf8_roundtrip() {
  // Latin-1 bytes for "café" (c=0x63, a=0x61, f=0x66, é=0xE9)
  let latin1 = &[0x63u8, 0x61, 0x66, 0xE9];
  let expected_utf8 = "café";

  let utf8_len = simdutf::utf8_length_from_latin1(latin1);
  // é expands to 2 bytes in UTF-8, so length >= 4
  assert!(utf8_len >= latin1.len());

  let mut output = vec![0u8; utf8_len];
  let written = unsafe { simdutf::convert_latin1_to_utf8(latin1, &mut output) };
  assert_eq!(&output[..written], expected_utf8.as_bytes());

  // Round-trip back to Latin-1
  let latin1_len = simdutf::latin1_length_from_utf8(&output[..written]);
  assert_eq!(latin1_len, latin1.len());

  let mut latin1_out = vec![0u8; latin1_len];
  let written2 = unsafe {
    simdutf::convert_utf8_to_latin1(&output[..written], &mut latin1_out)
  };
  assert_eq!(written2, latin1.len());
  assert_eq!(&latin1_out, latin1);
}

// ---------------------------------------------------------------------------
// Conversion: UTF-8 <-> UTF-32
// ---------------------------------------------------------------------------

#[test]
fn utf8_to_utf32_roundtrip() {
  let input = "hello 🌍";
  let utf8 = input.as_bytes();
  let expected: Vec<u32> = input.chars().map(|c| c as u32).collect();

  let len = simdutf::utf32_length_from_utf8(utf8);
  assert_eq!(len, expected.len());

  let mut output = vec![0u32; len];
  let written = unsafe { simdutf::convert_utf8_to_utf32(utf8, &mut output) };
  assert_eq!(written, expected.len());
  assert_eq!(output, expected);

  // Round-trip
  let utf8_len = simdutf::utf8_length_from_utf32(&output);
  let mut utf8_out = vec![0u8; utf8_len];
  let written =
    unsafe { simdutf::convert_utf32_to_utf8(&output, &mut utf8_out) };
  assert_eq!(&utf8_out[..written], utf8);
}

// ---------------------------------------------------------------------------
// Counting
// ---------------------------------------------------------------------------

#[test]
fn count_utf8_codepoints() {
  assert_eq!(simdutf::count_utf8("hello".as_bytes()), 5);
  assert_eq!(simdutf::count_utf8("café".as_bytes()), 4);
  assert_eq!(simdutf::count_utf8("日本語".as_bytes()), 3);
  assert_eq!(simdutf::count_utf8("🌍".as_bytes()), 1);
}

#[test]
fn count_utf16le_codepoints() {
  let data: Vec<u16> = "hello 🌍".encode_utf16().collect();
  // "hello " = 6 codepoints, 🌍 = 1 codepoint (but 2 UTF-16 code units)
  assert_eq!(simdutf::count_utf16le(&data), 7);
}

// ---------------------------------------------------------------------------
// Length calculation
// ---------------------------------------------------------------------------

#[test]
fn length_calculations() {
  let utf8 = "hello café 日本語".as_bytes();
  let utf16: Vec<u16> = "hello café 日本語".encode_utf16().collect();

  assert_eq!(simdutf::utf16_length_from_utf8(utf8), utf16.len());
  assert_eq!(simdutf::utf8_length_from_utf16le(&utf16), utf8.len());
}

// ---------------------------------------------------------------------------
// Encoding detection
// ---------------------------------------------------------------------------

#[test]
fn detect_encodings_ascii() {
  let data = b"hello world";
  let mask = simdutf::detect_encodings(data);
  // Pure ASCII is valid in all encodings
  assert_ne!(mask & simdutf::encoding::UTF8, 0);
}

// ---------------------------------------------------------------------------
// Base64
// ---------------------------------------------------------------------------

#[test]
fn base64_roundtrip() {
  let input = b"Hello, World!";

  let b64_len = simdutf::base64_length_from_binary(
    input.len(),
    simdutf::Base64Options::Default,
  );
  let mut b64 = vec![0u8; b64_len];
  let written = unsafe {
    simdutf::binary_to_base64(input, &mut b64, simdutf::Base64Options::Default)
  };
  let b64 = &b64[..written];
  assert_eq!(b64, b"SGVsbG8sIFdvcmxkIQ==");

  let max_bin_len = simdutf::maximal_binary_length_from_base64(b64);
  let mut decoded = vec![0u8; max_bin_len];
  let result = unsafe {
    simdutf::base64_to_binary(
      b64,
      &mut decoded,
      simdutf::Base64Options::Default,
      simdutf::LastChunkHandling::Loose,
    )
  };
  assert!(result.is_ok());
  assert_eq!(&decoded[..result.count], input);
}

#[test]
fn base64_url_safe() {
  let input = b"\xfb\xff\xfe"; // bytes that produce +/= in standard base64

  let b64_len = simdutf::base64_length_from_binary(
    input.len(),
    simdutf::Base64Options::Url,
  );
  let mut b64 = vec![0u8; b64_len];
  let written = unsafe {
    simdutf::binary_to_base64(input, &mut b64, simdutf::Base64Options::Url)
  };
  let b64 = &b64[..written];
  // URL-safe base64 should not contain + or /
  assert!(!b64.contains(&b'+'));
  assert!(!b64.contains(&b'/'));

  let max_bin_len = simdutf::maximal_binary_length_from_base64(b64);
  let mut decoded = vec![0u8; max_bin_len];
  let result = unsafe {
    simdutf::base64_to_binary(
      b64,
      &mut decoded,
      simdutf::Base64Options::Url,
      simdutf::LastChunkHandling::Loose,
    )
  };
  assert!(result.is_ok());
  assert_eq!(&decoded[..result.count], input);
}
