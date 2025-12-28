//! Test262 conformance test runner for tsrun
//!
//! Usage:
//!   test262-runner [OPTIONS] [PATTERN]
//!
//! Options:
//!   --test262-dir <PATH>  Path to test262 directory (default: ./test262)
//!   --filter <PATTERN>    Filter tests by path pattern
//!   --features <LIST>     Only run tests requiring these features (comma-separated)
//!   --skip-features <LIST> Skip tests requiring these features (comma-separated)
//!   --verbose             Show detailed output for each test
//!   --stop-on-fail        Stop on first failure
//!   --list                List matching tests without running
//!   --strict-only         Only run strict mode variants
//!   --non-strict-only     Only run non-strict mode variants
//!
//! Examples:
//!   test262-runner test/language/expressions/addition
//!   test262-runner --filter "array" test/built-ins/Array
//!   test262-runner --skip-features "BigInt,WeakRef" test/language

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tsrun::{JsError, Runtime, RuntimeResult};

/// Test metadata parsed from YAML frontmatter
#[derive(Debug, Default)]
struct TestMetadata {
    description: String,
    info: Option<String>,
    features: Vec<String>,
    includes: Vec<String>,
    flags: HashSet<String>,
    negative: Option<NegativeExpectation>,
    locale: Vec<String>,
}

#[derive(Debug)]
struct NegativeExpectation {
    phase: String, // "parse", "resolution", "runtime"
    error_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TestResult {
    Pass,
    Fail,
    Skip,
    Timeout,
}

#[derive(Debug)]
struct TestOutcome {
    result: TestResult,
    mode: &'static str, // "strict" or "non-strict"
    error: Option<String>,
    #[allow(dead_code)]
    duration: Duration,
}

struct TestRunner {
    test262_dir: PathBuf,
    harness_cache: HashMap<String, String>,
    verbose: bool,
    stop_on_fail: bool,
    strict_only: bool,
    non_strict_only: bool,
    skip_features: HashSet<String>,
    required_features: HashSet<String>,
}

impl TestRunner {
    fn new(test262_dir: PathBuf) -> Self {
        Self {
            test262_dir,
            harness_cache: HashMap::new(),
            verbose: false,
            stop_on_fail: false,
            strict_only: false,
            non_strict_only: false,
            skip_features: HashSet::new(),
            required_features: HashSet::new(),
        }
    }

    /// Load harness file content, caching results
    fn load_harness(&mut self, name: &str) -> Result<String, String> {
        if let Some(content) = self.harness_cache.get(name) {
            return Ok(content.clone());
        }

        let path = self.test262_dir.join("harness").join(name);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to load harness/{}: {}", name, e))?;

        self.harness_cache.insert(name.to_string(), content.clone());
        Ok(content)
    }

    /// Parse YAML frontmatter from test file
    fn parse_metadata(&self, source: &str) -> TestMetadata {
        let mut meta = TestMetadata::default();

        // Find YAML block
        let start = match source.find("/*---") {
            Some(i) => i + 5,
            None => return meta,
        };
        let rest = match source.get(start..) {
            Some(s) => s,
            None => return meta,
        };
        let end = match rest.find("---*/") {
            Some(i) => start + i,
            None => return meta,
        };

        let yaml = match source.get(start..end) {
            Some(s) => s,
            None => return meta,
        };

        // Simple YAML parsing (not full spec, but handles test262 format)
        let mut current_key = String::new();
        let mut in_multiline = false;
        let mut multiline_value = String::new();

        for line in yaml.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                if in_multiline {
                    multiline_value.push('\n');
                }
                continue;
            }

            // Check for multiline continuation
            if in_multiline {
                if line.starts_with("  ") || line.starts_with('\t') {
                    multiline_value.push_str(trimmed);
                    multiline_value.push('\n');
                    continue;
                } else {
                    // End of multiline
                    match current_key.as_str() {
                        "info" => meta.info = Some(multiline_value.trim().to_string()),
                        "description" => meta.description = multiline_value.trim().to_string(),
                        _ => {}
                    }
                    in_multiline = false;
                    multiline_value.clear();
                }
            }

            // Parse key: value
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed.get(..colon_pos).unwrap_or("").trim();
                let value = trimmed.get(colon_pos + 1..).unwrap_or("").trim();

                current_key = key.to_string();

                // Check for multiline indicator
                if value == "|" || value == ">" {
                    in_multiline = true;
                    continue;
                }

                match key {
                    "description" => meta.description = value.to_string(),
                    "features" => {
                        meta.features = parse_yaml_array(value);
                    }
                    "includes" => {
                        meta.includes = parse_yaml_array(value);
                    }
                    "flags" => {
                        for flag in parse_yaml_array(value) {
                            meta.flags.insert(flag);
                        }
                    }
                    "locale" => {
                        meta.locale = parse_yaml_array(value);
                    }
                    "phase" => {
                        if let Some(ref mut neg) = meta.negative {
                            neg.phase = value.to_string();
                        }
                    }
                    "type" => {
                        if let Some(ref mut neg) = meta.negative {
                            neg.error_type = value.to_string();
                        }
                    }
                    "negative" => {
                        meta.negative = Some(NegativeExpectation {
                            phase: String::new(),
                            error_type: String::new(),
                        });
                    }
                    _ => {}
                }
            } else if let Some(rest) = trimmed.strip_prefix("- ") {
                // Array item
                let item = rest.trim().to_string();
                match current_key.as_str() {
                    "features" => meta.features.push(item),
                    "includes" => meta.includes.push(item),
                    "flags" => {
                        meta.flags.insert(item);
                    }
                    "locale" => meta.locale.push(item),
                    _ => {}
                }
            }
        }

        meta
    }

    /// Check if test should be skipped based on features
    fn should_skip(&self, meta: &TestMetadata) -> Option<String> {
        // Check for unsupported features
        for feature in &meta.features {
            if self.skip_features.contains(feature) {
                return Some(format!("skipped feature: {}", feature));
            }
        }

        // Check required features filter
        if !self.required_features.is_empty() {
            let has_required = meta
                .features
                .iter()
                .any(|f| self.required_features.contains(f));
            if !has_required && !meta.features.is_empty() {
                return Some("does not match required features".to_string());
            }
        }

        // Skip module tests for now (need module support)
        if meta.flags.contains("module") {
            return Some("module tests not yet supported".to_string());
        }

        // Skip async tests for now
        if meta.flags.contains("async") {
            return Some("async tests not yet supported".to_string());
        }

        // Skip tests requiring agents/atomics
        if meta.flags.contains("CanBlockIsTrue") || meta.flags.contains("CanBlockIsFalse") {
            return Some("atomics/agents not supported".to_string());
        }

        None
    }

    /// Build the preamble code (harness files)
    fn build_preamble(&mut self, meta: &TestMetadata) -> Result<String, String> {
        let mut preamble = String::new();

        // Skip harness for raw tests
        if meta.flags.contains("raw") {
            return Ok(preamble);
        }

        // Always include sta.js and assert.js (unless raw)
        preamble.push_str(&self.load_harness("sta.js")?);
        preamble.push('\n');
        preamble.push_str(&self.load_harness("assert.js")?);
        preamble.push('\n');

        // Include additional harness files
        for include in &meta.includes {
            preamble.push_str(&self.load_harness(include)?);
            preamble.push('\n');
        }

        Ok(preamble)
    }

    /// Run a single test in a specific mode
    fn run_test_mode(
        &mut self,
        _test_path: &Path,
        source: &str,
        meta: &TestMetadata,
        strict: bool,
    ) -> TestOutcome {
        let mode = if strict { "strict" } else { "non-strict" };
        let start = Instant::now();

        // Build full source
        let preamble = match self.build_preamble(meta) {
            Ok(p) => p,
            Err(e) => {
                return TestOutcome {
                    result: TestResult::Fail,
                    mode,
                    error: Some(format!("Failed to load harness: {}", e)),
                    duration: start.elapsed(),
                }
            }
        };

        // Add strict mode directive if needed
        let test_source = if strict && !meta.flags.contains("raw") {
            format!("\"use strict\";\n{}", source)
        } else {
            source.to_string()
        };

        let full_source = format!("{}\n{}", preamble, test_source);

        // Create runtime
        let mut runtime = Runtime::new();
        runtime.set_gc_threshold(100);
        runtime.set_timeout_ms(10_000); // 10 second timeout per test

        // Execute test
        let result = runtime.eval(&full_source);
        let duration = start.elapsed();

        // Check result against expectations
        match (&meta.negative, result) {
            // Expected to throw, and it did
            (Some(neg), Err(ref err)) => {
                let error_matches = check_error_type(err, &neg.error_type);
                let phase_matches = match neg.phase.as_str() {
                    "parse" => matches!(err, JsError::SyntaxError { .. }),
                    "runtime" => !matches!(err, JsError::SyntaxError { .. }),
                    "resolution" => true, // Module resolution - we'd need module support
                    _ => true,
                };

                if error_matches && phase_matches {
                    TestOutcome {
                        result: TestResult::Pass,
                        mode,
                        error: None,
                        duration,
                    }
                } else {
                    TestOutcome {
                        result: TestResult::Fail,
                        mode,
                        error: Some(format!(
                            "Expected {} in {} phase, got: {}",
                            neg.error_type,
                            neg.phase,
                            format_error(err)
                        )),
                        duration,
                    }
                }
            }

            // Expected to throw, but didn't
            (Some(neg), Ok(_)) => TestOutcome {
                result: TestResult::Fail,
                mode,
                error: Some(format!(
                    "Expected {} to be thrown, but test completed successfully",
                    neg.error_type
                )),
                duration,
            },

            // Expected to pass, and it did
            (None, Ok(RuntimeResult::Complete(_))) => TestOutcome {
                result: TestResult::Pass,
                mode,
                error: None,
                duration,
            },

            // Expected to pass, but needs imports (module test that slipped through)
            (None, Ok(RuntimeResult::NeedImports(_))) => TestOutcome {
                result: TestResult::Skip,
                mode,
                error: Some("Test requires module imports".to_string()),
                duration,
            },

            // Expected to pass, but suspended (async test that slipped through)
            (None, Ok(RuntimeResult::Suspended { .. })) => TestOutcome {
                result: TestResult::Skip,
                mode,
                error: Some("Test requires async support".to_string()),
                duration,
            },

            // Expected to pass, but threw
            (None, Err(ref err)) => {
                // Check for timeout
                let error_str = format_error(err);
                let result = if error_str.contains("timeout") || error_str.contains("Timeout") {
                    TestResult::Timeout
                } else {
                    TestResult::Fail
                };

                TestOutcome {
                    result,
                    mode,
                    error: Some(error_str),
                    duration,
                }
            }
        }
    }

    /// Run a single test file
    fn run_test(&mut self, test_path: &Path) -> Vec<TestOutcome> {
        let mut outcomes = Vec::new();

        // Read test file
        let source = match fs::read_to_string(test_path) {
            Ok(s) => s,
            Err(e) => {
                return vec![TestOutcome {
                    result: TestResult::Fail,
                    mode: "n/a",
                    error: Some(format!("Failed to read test: {}", e)),
                    duration: Duration::ZERO,
                }];
            }
        };

        // Parse metadata
        let meta = self.parse_metadata(&source);

        // Check if should skip
        if let Some(reason) = self.should_skip(&meta) {
            return vec![TestOutcome {
                result: TestResult::Skip,
                mode: "n/a",
                error: Some(reason),
                duration: Duration::ZERO,
            }];
        }

        // Determine which modes to run
        let run_strict = !meta.flags.contains("noStrict")
            && !meta.flags.contains("raw")
            && !self.non_strict_only;
        let run_non_strict = !meta.flags.contains("onlyStrict")
            && !meta.flags.contains("module")
            && !self.strict_only;

        // Run in applicable modes
        if run_non_strict {
            outcomes.push(self.run_test_mode(test_path, &source, &meta, false));
        }
        if run_strict {
            outcomes.push(self.run_test_mode(test_path, &source, &meta, true));
        }

        outcomes
    }
}

/// Parse YAML array format: [a, b, c] or - items
fn parse_yaml_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if !trimmed.is_empty() {
        vec![trimmed.to_string()]
    } else {
        Vec::new()
    }
}

/// Check if error matches expected type
fn check_error_type(err: &JsError, expected: &str) -> bool {
    match err {
        JsError::SyntaxError { .. } => expected == "SyntaxError",
        JsError::TypeError { .. } => expected == "TypeError",
        JsError::ReferenceError { .. } => expected == "ReferenceError",
        JsError::RangeError { .. } => expected == "RangeError",
        JsError::RuntimeError { kind, message, .. } => {
            expected == "Error" || kind == expected || message.contains(expected)
        }
        JsError::ModuleError { .. } => expected == "SyntaxError" || expected == "Error",
        JsError::Internal(_) => false,
        JsError::Thrown => expected == "Error",
        JsError::ThrownValue { .. } => expected == "Error",
        JsError::GeneratorYield { .. } => false,
        JsError::Timeout { .. } => false,
        JsError::OptionalChainShortCircuit => false,
    }
}

/// Format error for display
fn format_error(err: &JsError) -> String {
    match err {
        JsError::SyntaxError { message, location } => {
            format!("SyntaxError: {} at {}", message, location)
        }
        JsError::TypeError { message, location } => {
            if let Some(loc) = location {
                format!("TypeError: {} at {}", message, loc)
            } else {
                format!("TypeError: {}", message)
            }
        }
        JsError::ReferenceError { name } => format!("ReferenceError: {} is not defined", name),
        JsError::RangeError { message } => format!("RangeError: {}", message),
        JsError::RuntimeError { kind, message, .. } => format!("{}: {}", kind, message),
        JsError::ModuleError { message } => format!("ModuleError: {}", message),
        JsError::Internal(msg) => format!("InternalError: {}", msg),
        JsError::Thrown => "Error: (thrown)".to_string(),
        JsError::ThrownValue { guarded } => format!("Error: {:?}", guarded.value),
        JsError::GeneratorYield { guarded } => format!("GeneratorYield: {:?}", guarded.value),
        JsError::Timeout {
            timeout_ms,
            elapsed_ms,
        } => {
            format!(
                "Timeout: exceeded {}ms limit (ran {}ms)",
                timeout_ms, elapsed_ms
            )
        }
        JsError::OptionalChainShortCircuit => {
            "OptionalChainShortCircuit (internal error - should not reach here)".to_string()
        }
    }
}

/// Collect test files from a directory
fn collect_tests(dir: &Path, pattern: Option<&str>) -> Vec<PathBuf> {
    let mut tests = Vec::new();

    if dir.is_file() {
        if dir.extension().is_some_and(|e| e == "js") {
            let path_str = dir.to_string_lossy();
            // Skip fixture files
            if !path_str.contains("_FIXTURE") {
                if let Some(pat) = pattern {
                    if path_str.contains(pat) {
                        tests.push(dir.to_path_buf());
                    }
                } else {
                    tests.push(dir.to_path_buf());
                }
            }
        }
        return tests;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                tests.extend(collect_tests(&path, pattern));
            } else if path.extension().is_some_and(|e| e == "js") {
                let path_str = path.to_string_lossy();
                // Skip fixture files
                if !path_str.contains("_FIXTURE") {
                    if let Some(pat) = pattern {
                        if path_str.contains(pat) {
                            tests.push(path);
                        }
                    } else {
                        tests.push(path);
                    }
                }
            }
        }
    }

    tests.sort();
    tests
}

fn print_usage(program: &str) {
    eprintln!(
        "Usage: {} [OPTIONS] <TEST_PATH>

Test262 conformance test runner for tsrun

Arguments:
  <TEST_PATH>  Path to test file or directory (relative to test262/test/)

Options:
  --test262-dir <PATH>     Path to test262 directory (default: ./test262)
  --filter <PATTERN>       Filter tests by path pattern
  --features <LIST>        Only run tests requiring these features (comma-separated)
  --skip-features <LIST>   Skip tests requiring these features (comma-separated)
  --verbose                Show detailed output for each test
  --stop-on-fail           Stop on first failure
  --list                   List matching tests without running
  --strict-only            Only run strict mode variants
  --non-strict-only        Only run non-strict mode variants
  --help                   Show this help message

Examples:
  {} language/expressions/addition
  {} --filter array built-ins/Array
  {} --skip-features BigInt,WeakRef language
  {} --verbose --stop-on-fail language/statements",
        program, program, program, program, program
    );
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map_or("test262-runner", |s| s.as_str());

    // Parse arguments
    let mut test262_dir = PathBuf::from("test262");
    let mut filter: Option<String> = None;
    let mut skip_features: HashSet<String> = HashSet::new();
    let mut required_features: HashSet<String> = HashSet::new();
    let mut verbose = false;
    let mut stop_on_fail = false;
    let mut list_only = false;
    let mut strict_only = false;
    let mut non_strict_only = false;
    let mut test_path: Option<String> = None;

    // Default skip features (things we don't support yet)
    let default_skip = [
        "BigInt",
        "WeakRef",
        "WeakMap",
        "WeakSet",
        "FinalizationRegistry",
        "Atomics",
        "SharedArrayBuffer",
        "Temporal",
        "decorators",
        "import-assertions",
        "import-attributes",
        "json-modules",
        "regexp-lookbehind",
        "regexp-named-groups",
        "regexp-unicode-property-escapes",
        "tail-call-optimization",
        "top-level-await",
        "ShadowRealm",
        "resizable-arraybuffer",
        "arraybuffer-transfer",
        "Array.fromAsync",
        "iterator-helpers",
        "set-methods",
        "promise-with-resolvers",
        "Intl",
        "TypedArray",
        "ArrayBuffer",
        "DataView",
    ];
    for f in default_skip {
        skip_features.insert(f.to_string());
    }

    let mut i = 1;
    while i < args.len() {
        let arg = match args.get(i) {
            Some(a) => a,
            None => break,
        };
        match arg.as_str() {
            "--help" | "-h" => {
                print_usage(program);
                return Ok(());
            }
            "--test262-dir" => {
                i += 1;
                test262_dir = PathBuf::from(args.get(i).ok_or("Missing value for --test262-dir")?);
            }
            "--filter" => {
                i += 1;
                filter = Some(args.get(i).ok_or("Missing value for --filter")?.clone());
            }
            "--features" => {
                i += 1;
                let features_str = args.get(i).ok_or("Missing value for --features")?;
                for f in features_str.split(',') {
                    required_features.insert(f.trim().to_string());
                }
            }
            "--skip-features" => {
                i += 1;
                let features_str = args.get(i).ok_or("Missing value for --skip-features")?;
                for f in features_str.split(',') {
                    skip_features.insert(f.trim().to_string());
                }
            }
            "--verbose" | "-v" => verbose = true,
            "--stop-on-fail" => stop_on_fail = true,
            "--list" => list_only = true,
            "--strict-only" => strict_only = true,
            "--non-strict-only" => non_strict_only = true,
            _ if !arg.starts_with('-') => {
                test_path = Some(arg.clone());
            }
            _ => {
                return Err(format!("Unknown option: {}", arg).into());
            }
        }
        i += 1;
    }

    // Require test path
    let test_path = test_path.ok_or("Missing test path argument. Use --help for usage.")?;

    // Resolve test path
    let full_test_path = if Path::new(&test_path).is_absolute() {
        PathBuf::from(&test_path)
    } else {
        test262_dir.join("test").join(&test_path)
    };

    if !full_test_path.exists() {
        return Err(format!("Test path not found: {}", full_test_path.display()).into());
    }

    // Collect tests
    let tests = collect_tests(&full_test_path, filter.as_deref());

    if tests.is_empty() {
        println!("No tests found matching criteria");
        return Ok(());
    }

    println!("Found {} test files", tests.len());

    if list_only {
        for test in &tests {
            println!("{}", test.display());
        }
        return Ok(());
    }

    // Create runner
    let mut runner = TestRunner::new(test262_dir);
    runner.verbose = verbose;
    runner.stop_on_fail = stop_on_fail;
    runner.strict_only = strict_only;
    runner.non_strict_only = non_strict_only;
    runner.skip_features = skip_features;
    runner.required_features = required_features;

    // Run tests
    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut skip_count = 0;
    let mut timeout_count = 0;
    let total_start = Instant::now();

    for (idx, test_path) in tests.iter().enumerate() {
        let relative_path = test_path
            .strip_prefix(&runner.test262_dir)
            .unwrap_or(test_path);

        if verbose {
            print!(
                "[{}/{}] {} ... ",
                idx + 1,
                tests.len(),
                relative_path.display()
            );
            io::stdout().flush().ok();
        }

        let outcomes = runner.run_test(test_path);

        let mut test_passed = true;
        for outcome in &outcomes {
            match outcome.result {
                TestResult::Pass => pass_count += 1,
                TestResult::Fail => {
                    fail_count += 1;
                    test_passed = false;
                }
                TestResult::Skip => skip_count += 1,
                TestResult::Timeout => {
                    timeout_count += 1;
                    test_passed = false;
                }
            }
        }

        if verbose {
            let status = if outcomes.iter().all(|o| o.result == TestResult::Skip) {
                "SKIP"
            } else if test_passed {
                "PASS"
            } else {
                "FAIL"
            };
            println!("{}", status);

            // Show details for failures
            if !test_passed {
                for outcome in &outcomes {
                    if outcome.result == TestResult::Fail || outcome.result == TestResult::Timeout {
                        if let Some(ref err) = outcome.error {
                            println!("  [{}] {}", outcome.mode, err);
                        }
                    }
                }
            }
        } else {
            // Progress indicator
            let c = if outcomes.iter().all(|o| o.result == TestResult::Skip) {
                'S'
            } else if test_passed {
                '.'
            } else {
                'F'
            };
            print!("{}", c);
            if (idx + 1) % 80 == 0 {
                println!(" [{}/{}]", idx + 1, tests.len());
            }
            io::stdout().flush().ok();
        }

        if stop_on_fail && !test_passed {
            println!("\n\nStopping on first failure.");
            println!("Failed test: {}", relative_path.display());
            for outcome in &outcomes {
                if let Some(ref err) = outcome.error {
                    println!("  [{}] {}", outcome.mode, err);
                }
            }
            break;
        }
    }

    let total_duration = total_start.elapsed();

    // Print summary
    println!("\n");
    println!("═══════════════════════════════════════════════════════════════");
    println!("                        TEST RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Passed:   {:>6}", pass_count);
    println!("  Failed:   {:>6}", fail_count);
    println!("  Skipped:  {:>6}", skip_count);
    println!("  Timeout:  {:>6}", timeout_count);
    println!("───────────────────────────────────────────────────────────────");
    println!(
        "  Total:    {:>6}",
        pass_count + fail_count + skip_count + timeout_count
    );
    println!(
        "  Pass rate: {:>5.1}% (excluding skipped)",
        if pass_count + fail_count + timeout_count > 0 {
            100.0 * pass_count as f64 / (pass_count + fail_count + timeout_count) as f64
        } else {
            0.0
        }
    );
    println!("  Duration: {:>5.2}s", total_duration.as_secs_f64());
    println!("═══════════════════════════════════════════════════════════════");

    if fail_count > 0 || timeout_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
