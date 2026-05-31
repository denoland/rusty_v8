fn main() {
  if cfg!(debug_assertions) || std::env::var("CI").is_ok() {
    return;
  }

  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let scope, isolate);
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);

  println!(
    "simdutf feature: {}",
    if cfg!(feature = "simdutf") {
      "ENABLED"
    } else {
      "DISABLED"
    }
  );
  println!();

  // --- Build test strings of various sizes ---
  let sizes: &[usize] = &[16, 64, 256, 1024, 4096, 16384];

  for &size in sizes {
    println!("=== String length: {size} ===");

    // 1) Pure ASCII one-byte string
    {
      let ascii_data: String =
        (0..size).map(|i| (b'A' + (i % 26) as u8) as char).collect();
      let v8_str = v8::String::new(scope, &ascii_data).unwrap();
      bench_to_rust_string_lossy(scope, v8_str, "ascii_to_rust_string_lossy");
      bench_write_utf8_into(scope, v8_str, "ascii_write_utf8_into");
      bench_to_rust_cow_lossy(scope, v8_str, "ascii_to_rust_cow_lossy");
    }

    // 2) Latin-1 string (non-ASCII one-byte)
    //    Create via new_from_one_byte with bytes in 0x80..0xFF range
    {
      let latin1_data: Vec<u8> =
        (0..size).map(|i| 0xC0 + (i % 64) as u8).collect();
      let v8_str = v8::String::new_from_one_byte(
        scope,
        &latin1_data,
        v8::NewStringType::Normal,
      )
      .unwrap();
      bench_to_rust_string_lossy(scope, v8_str, "latin1_to_rust_string_lossy");
      bench_write_utf8_into(scope, v8_str, "latin1_write_utf8_into");
      bench_to_rust_cow_lossy(scope, v8_str, "latin1_to_rust_cow_lossy");
    }

    // 3) Two-byte string (UTF-16 with characters outside Latin-1)
    //    Use codepoints like U+0400..U+04FF (Cyrillic) to force two-byte representation
    {
      let utf16_data: Vec<u16> =
        (0..size).map(|i| 0x0400 + (i % 256) as u16).collect();
      let v8_str = v8::String::new_from_two_byte(
        scope,
        &utf16_data,
        v8::NewStringType::Normal,
      )
      .unwrap();
      bench_to_rust_string_lossy(scope, v8_str, "twobyte_to_rust_string_lossy");
      bench_write_utf8_into(scope, v8_str, "twobyte_write_utf8_into");
      bench_to_rust_cow_lossy(scope, v8_str, "twobyte_to_rust_cow_lossy");
    }

    println!();
  }
}

fn bench_to_rust_string_lossy(
  scope: &mut v8::PinScope<'_, '_>,
  s: v8::Local<'_, v8::String>,
  label: &str,
) {
  let iterations = iterations_for_length(s.length());
  let start = std::time::Instant::now();
  for _ in 0..iterations {
    let _ = std::hint::black_box(s.to_rust_string_lossy(scope));
  }
  let elapsed = start.elapsed();
  print_result(label, elapsed, iterations, s.length());
}

fn bench_write_utf8_into(
  scope: &mut v8::PinScope<'_, '_>,
  s: v8::Local<'_, v8::String>,
  label: &str,
) {
  let iterations = iterations_for_length(s.length());
  let mut buf = String::new();
  let start = std::time::Instant::now();
  for _ in 0..iterations {
    s.write_utf8_into(scope, &mut buf);
    std::hint::black_box(&buf);
  }
  let elapsed = start.elapsed();
  print_result(label, elapsed, iterations, s.length());
}

fn bench_to_rust_cow_lossy(
  scope: &mut v8::PinScope<'_, '_>,
  s: v8::Local<'_, v8::String>,
  label: &str,
) {
  let iterations = iterations_for_length(s.length());
  let start = std::time::Instant::now();
  for _ in 0..iterations {
    let mut buffer = [std::mem::MaybeUninit::uninit(); 2048];
    let _ = std::hint::black_box(s.to_rust_cow_lossy(scope, &mut buffer));
  }
  let elapsed = start.elapsed();
  print_result(label, elapsed, iterations, s.length());
}

fn iterations_for_length(len: usize) -> u64 {
  // More iterations for shorter strings to get stable timings
  match len {
    0..=64 => 5_000_000,
    65..=512 => 2_000_000,
    513..=4096 => 500_000,
    _ => 100_000,
  }
}

fn print_result(
  label: &str,
  elapsed: std::time::Duration,
  iterations: u64,
  length: usize,
) {
  let total_ns = elapsed.as_nanos() as f64;
  let ns_per_iter = total_ns / iterations as f64;
  let throughput_mb =
    (length as f64 * iterations as f64) / (elapsed.as_secs_f64() * 1e6);
  println!("  {ns_per_iter:8.1} ns/iter  {throughput_mb:8.1} MB/s  {label}",);
}
