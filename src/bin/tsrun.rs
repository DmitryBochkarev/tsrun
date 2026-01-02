//! CLI tool for running TypeScript files using tsrun
//!
//! Usage: tsrun [options] <entry-point.ts>
//!
//! Options:
//!   --timeout <ms>     Maximum execution time in milliseconds (default: unlimited)
//!   --max-depth <n>    Maximum call stack depth (default: unlimited)
//!
//! Supports static imports - modules are resolved relative to the importing file.
//! Nested imports are supported.

use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tsrun::{Interpreter, JsValue, ModulePath, StepResult};

/// Minimal package.json representation for module resolution
#[derive(serde::Deserialize)]
struct PackageJson {
    /// ESM entry point (preferred)
    module: Option<String>,
    /// CommonJS entry point
    main: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Format an error with file context and import chain
fn format_error(
    error: &str,
    file: &str,
    import_chain: &[(String, String)], // (importer, specifier)
) -> String {
    let mut msg = format!("{}\n\n  File: {}", error, file);

    if !import_chain.is_empty() {
        msg.push_str("\n\n  Import chain:");
        for (i, (importer, specifier)) in import_chain.iter().enumerate() {
            msg.push_str(&format!(
                "\n    {}. {} imported '{}'",
                i + 1,
                importer,
                specifier
            ));
        }
    }

    msg
}

/// CLI configuration
struct Config {
    entry_path: PathBuf,
    timeout_ms: Option<u64>,
    max_depth: Option<usize>,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = env::args().collect();
    let program_name = args.first().map_or("tsrun", |s| s.as_str());

    let mut timeout_ms: Option<u64> = None;
    let mut max_depth: Option<usize> = None;
    let mut entry_arg: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        let Some(arg) = args.get(i) else {
            break;
        };
        if arg == "--timeout" {
            i += 1;
            timeout_ms = Some(
                args.get(i)
                    .ok_or_else(|| "--timeout requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--timeout must be a positive integer".to_string())?,
            );
        } else if arg == "--max-depth" {
            i += 1;
            max_depth = Some(
                args.get(i)
                    .ok_or_else(|| "--max-depth requires a value".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "--max-depth must be a positive integer".to_string())?,
            );
        } else if arg.starts_with('-') {
            return Err(format!("Unknown option: {}", arg));
        } else {
            entry_arg = Some(arg);
        }
        i += 1;
    }

    let entry_arg = entry_arg.ok_or_else(|| {
        format!(
            "Usage: {} [--timeout <ms>] [--max-depth <n>] <entry-point.ts>",
            program_name
        )
    })?;

    let entry_path = PathBuf::from(entry_arg);
    // Make entry_path absolute for consistent path resolution
    let entry_path = if entry_path.is_absolute() {
        entry_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(entry_path)
    };

    Ok(Config {
        entry_path,
        timeout_ms,
        max_depth,
    })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_args().map_err(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    })?;

    let entry_path = config.entry_path;
    let entry_dir = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let source = fs::read_to_string(&entry_path)
        .map_err(|e| format!("Cannot read {}: {}", entry_path.display(), e))?;

    let mut interp = Interpreter::new();
    // Allow overriding GC threshold via environment variable for stress testing
    if let Ok(threshold_str) = std::env::var("GC_THRESHOLD") {
        if let Ok(threshold) = threshold_str.parse::<usize>() {
            interp.set_gc_threshold(threshold);
        }
    } else {
        interp.set_gc_threshold(100);
    }

    // Track provided modules by resolved path to avoid reloading
    let mut provided: FxHashMap<ModulePath, PathBuf> = FxHashMap::default();

    // Track import chain for error reporting: maps resolved_path -> (importer, specifier)
    let mut import_chain: FxHashMap<String, (String, String)> = FxHashMap::default();

    // Start evaluation - may return NeedImports
    let entry_file = entry_path.display().to_string();

    // Helper to build import chain from a file back to entry
    let build_chain =
        |file: &str, chain_map: &FxHashMap<String, (String, String)>| -> Vec<(String, String)> {
            let mut chain = Vec::new();
            let mut current = file.to_string();
            while let Some((importer, specifier)) = chain_map.get(&current) {
                chain.push((importer.clone(), specifier.clone()));
                current = importer.clone();
            }
            chain.reverse();
            chain
        };

    // Helper to load and provide a module
    let load_and_provide_module = |interp: &mut Interpreter,
                                   req: &tsrun::ImportRequest,
                                   provided: &mut FxHashMap<ModulePath, PathBuf>,
                                   import_chain: &mut FxHashMap<String, (String, String)>|
     -> Result<(), Box<dyn std::error::Error>> {
        // Use resolved_path for deduplication
        if provided.contains_key(&req.resolved_path) {
            return Ok(());
        }

        // Track the import chain
        let importer = req
            .importer
            .as_ref()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| entry_file.clone());

        // Determine the canonical filesystem path
        let canonical_path = if ModulePath::is_bare(&req.specifier) {
            // Bare specifier (e.g., "lodash", "@scope/pkg") - resolve from node_modules
            let start_dir = req
                .importer
                .as_ref()
                .and_then(|p| PathBuf::from(p.as_str()).parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| entry_dir.clone());

            resolve_node_module(&req.specifier, &start_dir).map_err(|e| {
                let chain = build_chain(&importer, import_chain);
                format_error(&e.to_string(), &importer, &chain)
            })?
        } else {
            // Relative/absolute path - resolve with file extension
            let base_path = PathBuf::from(req.resolved_path.as_str());
            resolve_file_with_extensions(&base_path).map_err(|e| {
                let chain = build_chain(&importer, import_chain);
                format_error(&e.to_string(), &importer, &chain)
            })?
        };

        let canonical_str = canonical_path.to_string_lossy().to_string();

        // Record this import for chain tracking
        import_chain.insert(
            canonical_str.clone(),
            (importer.clone(), req.specifier.clone()),
        );

        // Load module from filesystem
        let (module_source, module_dir) = load_module(&canonical_str).map_err(|e| {
            let chain = build_chain(&canonical_str, import_chain);
            format_error(&e.to_string(), &canonical_str, &chain)
        })?;

        // Provide to interpreter using the resolved_path it expects
        interp
            .provide_module(req.resolved_path.clone(), &module_source)
            .map_err(|e| {
                let chain = build_chain(&canonical_str, import_chain);
                format_error(&e.to_string(), &canonical_str, &chain)
            })?;
        provided.insert(req.resolved_path.clone(), module_dir);
        Ok(())
    };

    // Choose execution strategy based on whether limits are configured
    let has_limits = config.timeout_ms.is_some() || config.max_depth.is_some();

    if has_limits {
        // Step-based execution with limit checking
        let initial_result = interp
            .prepare(&source, Some(ModulePath::new(entry_file.as_str())))
            .map_err(|e| format_error(&e.to_string(), &entry_file, &[]))?;

        let start_time = Instant::now();
        let mut step_result = initial_result;

        loop {
            // Check limits before each step
            if let Some(timeout) = config.timeout_ms {
                let elapsed = start_time.elapsed().as_millis() as u64;
                if elapsed >= timeout {
                    return Err(format!("Execution timed out after {}ms", timeout).into());
                }
            }
            if let Some(max_depth) = config.max_depth {
                let depth = interp.call_depth();
                if depth > max_depth {
                    return Err(
                        format!("Maximum call depth exceeded: {} > {}", depth, max_depth).into(),
                    );
                }
            }

            match step_result {
                StepResult::Continue => {
                    step_result = interp.step().map_err(|e| format!("{}", e))?;
                }
                StepResult::Complete(runtime_value) => {
                    print_value(runtime_value.value());
                    return Ok(());
                }
                StepResult::Done => {
                    return Ok(());
                }
                StepResult::NeedImports(import_requests) => {
                    for req in &import_requests {
                        load_and_provide_module(
                            &mut interp,
                            req,
                            &mut provided,
                            &mut import_chain,
                        )?;
                    }
                    step_result = interp.step().map_err(|e| format!("{}", e))?;
                }
                StepResult::Suspended { pending, .. } => {
                    return Err(format!(
                        "Async operations not supported in CLI (pending orders: {})",
                        pending.len()
                    )
                    .into());
                }
            }
        }
    } else {
        // Fast path: no limits, step-based execution
        interp
            .prepare(&source, Some(ModulePath::new(entry_file.as_str())))
            .map_err(|e| format_error(&e.to_string(), &entry_file, &[]))?;

        loop {
            match interp.step().map_err(|e| format!("{}", e))? {
                StepResult::Continue => continue,
                StepResult::Complete(runtime_value) => {
                    print_value(runtime_value.value());
                    return Ok(());
                }
                StepResult::Done => {
                    return Ok(());
                }
                StepResult::NeedImports(import_requests) => {
                    for req in &import_requests {
                        load_and_provide_module(
                            &mut interp,
                            req,
                            &mut provided,
                            &mut import_chain,
                        )?;
                    }
                }
                StepResult::Suspended { pending, .. } => {
                    return Err(format!(
                        "Async operations not supported in CLI (pending orders: {})",
                        pending.len()
                    )
                    .into());
                }
            }
        }
    }
}

/// Parse a bare specifier into package name and optional subpath.
///
/// Examples:
/// - "lodash" -> ("lodash", None)
/// - "lodash/fp" -> ("lodash", Some("fp"))
/// - "@scope/pkg" -> ("@scope/pkg", None)
/// - "@scope/pkg/utils" -> ("@scope/pkg", Some("utils"))
fn parse_bare_specifier(
    specifier: &str,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    if specifier.starts_with('@') {
        // Scoped package: @scope/name or @scope/name/subpath
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();

        let scope = parts
            .first()
            .ok_or_else(|| format!("Invalid scoped package specifier: {}", specifier))?;
        let name = parts
            .get(1)
            .ok_or_else(|| format!("Invalid scoped package specifier: {}", specifier))?;

        let package_name = format!("{}/{}", scope, name);
        let subpath = parts.get(2).map(|s| s.to_string());

        Ok((package_name, subpath))
    } else {
        // Regular package: name or name/subpath
        let parts: Vec<&str> = specifier.splitn(2, '/').collect();
        let package_name = parts
            .first()
            .ok_or_else(|| format!("Invalid package specifier: {}", specifier))?
            .to_string();
        let subpath = parts.get(1).map(|s| s.to_string());

        Ok((package_name, subpath))
    }
}

/// Resolve a file path, trying TypeScript extensions first, then JavaScript.
///
/// Tries in order:
/// 1. Exact path (if has extension and exists)
/// 2. path.ts
/// 3. path.js
/// 4. path/index.ts
/// 5. path/index.js
fn resolve_file_with_extensions(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // If path already has an extension and exists, use it
    if path.extension().is_some() && path.exists() {
        return Ok(path.to_path_buf());
    }

    // Try TypeScript extensions first (project preference)
    let extensions = ["ts", "js"];

    for ext in &extensions {
        let with_ext = path.with_extension(ext);
        if with_ext.exists() {
            return Ok(with_ext);
        }
    }

    // Try as directory with index file
    let index_files = ["index.ts", "index.js"];
    for index in &index_files {
        let index_path = path.join(index);
        if index_path.exists() {
            return Ok(index_path);
        }
    }

    Err(format!("Cannot resolve module path: {}", path.display()).into())
}

/// Resolve the entry point for a package directory.
///
/// # Arguments
/// * `package_dir` - Path to the package directory (e.g., node_modules/lodash)
/// * `subpath` - Optional subpath within the package (e.g., "fp" for "lodash/fp")
fn resolve_package_entry(
    package_dir: &Path,
    subpath: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(sub) = subpath {
        // Subpath import: resolve the subpath with extension/directory handling
        let target = package_dir.join(sub);
        return resolve_file_with_extensions(&target);
    }

    // Root import: read package.json for entry point
    let package_json_path = package_dir.join("package.json");

    if package_json_path.exists() {
        let content = fs::read_to_string(&package_json_path)?;
        let pkg: PackageJson = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", package_json_path.display(), e))?;

        // Try entry points in order: module (ESM) > main (CJS) > index.js
        let entry = pkg
            .module
            .or(pkg.main)
            .unwrap_or_else(|| "index.js".to_string());

        let entry_path = package_dir.join(&entry);
        return resolve_file_with_extensions(&entry_path);
    }

    // No package.json - try index files
    resolve_file_with_extensions(&package_dir.join("index"))
}

/// Resolve a bare specifier (e.g., "lodash", "@scope/pkg") to an absolute path
/// following Node.js resolution algorithm.
///
/// # Arguments
/// * `specifier` - The bare specifier (e.g., "lodash", "@scope/pkg", "lodash/fp")
/// * `start_dir` - Directory to start searching from (importer's directory or cwd)
///
/// # Returns
/// * `Ok(PathBuf)` - Resolved absolute path to the module entry point
/// * `Err` - Module not found with details about searched paths
fn resolve_node_module(
    specifier: &str,
    start_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. Parse the specifier to extract package name and subpath
    let (package_name, subpath) = parse_bare_specifier(specifier)?;

    // 2. Walk up directory tree looking for node_modules/<package>
    let mut current_dir = start_dir.to_path_buf();
    let mut searched_dirs = Vec::new();

    loop {
        let node_modules = current_dir.join("node_modules");
        let package_dir = node_modules.join(&package_name);

        if package_dir.is_dir() {
            // Found package directory - resolve the entry point
            return resolve_package_entry(&package_dir, subpath.as_deref());
        }

        searched_dirs.push(node_modules);

        // Move to parent directory
        if let Some(parent) = current_dir.parent() {
            if parent == current_dir {
                // Reached filesystem root
                break;
            }
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Module not found - provide helpful error
    Err(format!(
        "Cannot find module '{}'\nSearched in:\n{}",
        specifier,
        searched_dirs
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    )
    .into())
}

/// Load a module from a canonical path
fn load_module(canonical_path: &str) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let path = PathBuf::from(canonical_path);

    let source = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to load module '{}': {}", canonical_path, e))?;

    let module_dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    Ok((source, module_dir))
}

fn print_value(value: &JsValue) {
    match value {
        JsValue::Undefined => println!("undefined"),
        JsValue::Null => println!("null"),
        JsValue::Boolean(b) => println!("{}", b),
        JsValue::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                println!("{}", *n as i64);
            } else {
                println!("{}", n);
            }
        }
        JsValue::String(s) => println!("{}", s),
        JsValue::Object(obj) => {
            // Check if it's a function
            if obj.borrow().is_callable() {
                println!("[Function]");
                return;
            }
            // Try to convert to JSON for pretty printing
            if let Ok(json) = value_to_json(value) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", value))
                );
            } else {
                println!("{:?}", value);
            }
        }
        JsValue::Symbol(_) => println!("[Symbol]"),
    }
}

fn value_to_json(value: &JsValue) -> Result<serde_json::Value, &'static str> {
    use tsrun::Gc;
    use tsrun::value::JsObject;

    fn to_json_inner(
        value: &JsValue,
        visited: &mut FxHashSet<Gc<JsObject>>,
    ) -> Result<serde_json::Value, &'static str> {
        match value {
            JsValue::Undefined => Ok(serde_json::Value::Null),
            JsValue::Null => Ok(serde_json::Value::Null),
            JsValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            JsValue::Number(n) => {
                if n.is_nan() || n.is_infinite() {
                    Ok(serde_json::Value::Null)
                } else {
                    Ok(serde_json::json!(*n))
                }
            }
            JsValue::String(s) => Ok(serde_json::Value::String(s.to_string())),
            JsValue::Object(obj) => {
                // Check for circular references
                if visited.contains(obj) {
                    return Err("Circular reference detected");
                }
                visited.insert(obj.clone());

                let borrowed = obj.borrow();

                // Functions can't be serialized to JSON
                if borrowed.is_callable() {
                    visited.remove(obj);
                    return Err("Cannot convert function to JSON");
                }

                // Check if it's an array
                if let Some(elements) = borrowed.array_elements() {
                    let mut arr = Vec::with_capacity(elements.len());
                    for elem in elements {
                        arr.push(to_json_inner(elem, visited)?);
                    }
                    visited.remove(obj);
                    return Ok(serde_json::Value::Array(arr));
                }

                // Regular object
                let mut map = serde_json::Map::new();
                for (key, prop) in borrowed.properties.iter() {
                    if let tsrun::value::PropertyKey::String(s) = key {
                        // Skip non-serializable values (functions, symbols)
                        if let Ok(json_val) = to_json_inner(&prop.value, visited) {
                            map.insert(s.to_string(), json_val);
                        }
                    }
                }
                visited.remove(obj);
                Ok(serde_json::Value::Object(map))
            }
            JsValue::Symbol(_) => Err("Cannot convert symbol to JSON"),
        }
    }

    let mut visited = FxHashSet::default();
    to_json_inner(value, &mut visited)
}
