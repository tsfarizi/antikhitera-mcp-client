//! WASM Component Build Script
//!
//! This script handles:
//! 1. Automatic WIT generation from Rust traits using syn
//! 2. Component compilation for wasm32-wasip1 target
//! 3. Output validation and inspection
//!
//! Usage: `cargo run --release -p build-scripts -- <command>`

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};

/// ANSI color codes for terminal output
mod colors {
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const RED: &str = "\x1b[31m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RESET: &str = "\x1b[0m";
}

use colors::*;

fn main() {
    println!(
        "{}{}=== Building Antikythera WASM Component ==={}\n",
        BOLD, BLUE, RESET
    );

    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match args.as_slice() {
        ["wit"] => generate_wit(),
        ["component"] | ["component-wasm"] => build_component(),
        ["all"] => {
            generate_wit();
            build_component();
        }
        _ => {
            eprintln!("{}Usage:{} component-builder <command>\n", RED, RESET);
            eprintln!("{}Commands:{}", BOLD, RESET);
            eprintln!("  wit          Generate WIT from Rust code");
            eprintln!("  component    Build WASM component");
            eprintln!("  all          Generate WIT and build component");
            exit(1);
        }
    }
}

/// Generate WIT files from Rust source code
fn generate_wit() {
    println!(
        "{}[1/2] Generating WIT from Rust code...{}{}",
        YELLOW, RESET, BLUE
    );

    let sdk_src = project_root().join("antikythera-sdk").join("src");

    let component_rs = {
        let flat_file = sdk_src.join("component.rs");
        if flat_file.exists() {
            flat_file
        } else {
            let module_file = sdk_src.join("component").join("mod.rs");
            if module_file.exists() {
                module_file
            } else {
                eprintln!(
                    "{}✗ component source not found. Checked: {} and {}{}",
                    RED,
                    flat_file.display(),
                    module_file.display(),
                    RESET
                );
                exit(1);
            }
        }
    };

    // Parse and generate WIT
    match wit_from_rust::generate(&component_rs) {
        Ok(wit_content) => {
            // Generate WIT di parent folder (project root)
            let wit_dir = project_root().join("wit");
            fs::create_dir_all(&wit_dir).expect("Failed to create wit directory");

            let wit_file = wit_dir.join("antikythera.wit");
            fs::write(&wit_file, &wit_content).expect("Failed to write WIT file");

            println!("{}✓ WIT generated: {}{}", GREEN, wit_file.display(), RESET);
            println!(
                "\n{}Generated WIT preview:{}\n{}",
                BLUE,
                RESET,
                &wit_content[..wit_content.len().min(500)]
            );
        }
        Err(e) => {
            eprintln!("{}✗ WIT generation failed: {}{}", RED, e, RESET);
            exit(1);
        }
    }

    println!("\n{}WIT generation complete{}\n", GREEN, RESET);
}

/// Build the WASM component
fn build_component() {
    println!(
        "{}[2/2] Building WASM component...{}{}",
        YELLOW, RESET, BLUE
    );

    // Ensure wasm target is installed
    ensure_target_installed("wasm32-wasip1");

    // Build with cargo-component
    if !command_exists("cargo-component") {
        println!(
            "{}cargo-component not found. Installing...{}",
            YELLOW, RESET
        );
        run_command("cargo", &["install", "cargo-component"]);
    }

    let sdk_dir = project_root().join("antikythera-sdk");

    let status = Command::new("cargo")
        .args([
            "component",
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
        ])
        .current_dir(&sdk_dir)
        .status()
        .expect("Failed to run cargo component");

    if !status.success() {
        eprintln!("{}✗ Component build failed{}", RED, RESET);
        exit(1);
    }

    // Find and validate the output file
    let output_dir = project_root()
        .join("target")
        .join("wasm32-wasip1")
        .join("release");

    // Try different naming conventions
    let wasm_files = [
        output_dir.join("antikythera_sdk.wasm"),
        output_dir.join("antikythera-sdk.wasm"),
        output_dir.join("antikythera_sdk_component.wasm"),
    ];

    let wasm_file = wasm_files.iter().find(|f| f.exists());

    match wasm_file {
        Some(file) => {
            println!("{}✓ Component built: {}{}", GREEN, file.display(), RESET);

            // Show component info if wasm-tools is available
            if command_exists("wasm-tools") {
                println!("\n{}Component details:{}", BLUE, RESET);
                let _ = Command::new("wasm-tools")
                    .args(["component", "info", file.to_str().unwrap()])
                    .status();
            }

            println!("\n{}Component build complete!{}\n", GREEN, RESET);
        }
        None => {
            eprintln!(
                "{}✗ Component output not found in {}{}",
                RED,
                output_dir.display(),
                RESET
            );
            eprintln!("{}Expected one of:{}", RED, RESET);
            for f in &wasm_files {
                eprintln!("  - {}", f.display());
            }
            exit(1);
        }
    }
}

// ============================================================================
// WIT Generation from Rust Code
// ============================================================================

mod wit_from_rust {
    use std::fs;
    use std::path::Path;

    pub fn generate(component_rs: &Path) -> Result<String, String> {
        let content =
            fs::read_to_string(component_rs).map_err(|e| format!("Failed to read file: {}", e))?;

        let mut wit = String::new();
        wit.push_str("package antikythera:mcp-framework@1.0.0;\n\n");

        // Parse structs first
        let structs = parse_structs(&content)?;
        wit.push_str(&structs);

        // Parse traits
        let traits = parse_traits(&content)?;
        wit.push_str(&traits);

        // Parse FFI functions from ffi.rs
        let ffi_rs_path = component_rs
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("ffi.rs");

        let ffi_funcs = if ffi_rs_path.exists() {
            let ffi_content = fs::read_to_string(&ffi_rs_path)
                .map_err(|e| format!("Failed to read ffi.rs: {}", e))?;
            parse_ffi_functions(&ffi_content)?
        } else {
            String::new()
        };

        if !ffi_funcs.is_empty() {
            wit.push_str("/// FFI interface for external language bindings\n");
            wit.push_str("interface ffi-server {\n");
            wit.push_str(&ffi_funcs);
            wit.push_str("}\n\n");
        }

        // Add world definition
        wit.push_str("/// Combined world exporting both interfaces\n");
        wit.push_str("world antikythera-mcp {\n");
        wit.push_str("  export prompt-manager;\n");
        wit.push_str("  export mcp-client;\n");
        wit.push_str("  export ffi-server;\n");
        wit.push_str("}\n");

        Ok(wit)
    }

    fn parse_structs(content: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut pos = 0;

        while pos < content.len() {
            // Find "pub struct"
            if let Some(struct_start) = content[pos..].find("pub struct ") {
                let struct_start = pos + struct_start;

                // Extract struct name
                let name_start = struct_start + "pub struct ".len();
                if let Some(brace_pos) = content[name_start..].find('{') {
                    let struct_name = content[name_start..name_start + brace_pos].trim();
                    let brace_start = name_start + brace_pos;

                    // Find matching closing brace
                    if let Some((brace_end, fields_content)) =
                        find_matching_brace(&content[brace_start..])
                    {
                        let fields_content = &fields_content[..fields_content.len() - 1]; // Remove closing brace

                        let wit_name = camel_to_kebab(struct_name);
                        result.push_str(&format!("/// {}\nrecord {} {{\n", struct_name, wit_name));

                        // Parse fields
                        for line in fields_content.lines() {
                            let line = line.trim().trim_end_matches(',');
                            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                                continue;
                            }

                            if let Some((field_name, field_type)) = parse_field_line(line) {
                                let wit_field_name = camel_to_kebab(&field_name);
                                let wit_type = rust_type_to_wit(&field_type);
                                result.push_str(&format!("  {}: {},\n", wit_field_name, wit_type));
                            }
                        }

                        result.push_str("}\n\n");
                        pos = brace_start + brace_end;
                    } else {
                        pos = brace_start + 1;
                    }
                } else {
                    pos = name_start + 1;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    fn parse_traits(content: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut pos = 0;

        while pos < content.len() {
            // Find "pub trait"
            if let Some(trait_start) = content[pos..].find("pub trait ") {
                let trait_start = pos + trait_start;

                // Extract trait name
                let name_start = trait_start + "pub trait ".len();
                if let Some(brace_pos) = content[name_start..].find('{') {
                    let trait_name = content[name_start..name_start + brace_pos].trim();
                    let brace_start = name_start + brace_pos;

                    // Find matching closing brace
                    if let Some((brace_end, trait_content)) =
                        find_matching_brace(&content[brace_start..])
                    {
                        let wit_name = camel_to_kebab(trait_name);
                        result
                            .push_str(&format!("/// {}\ninterface {} {{\n", trait_name, wit_name));

                        // Parse functions in trait
                        let functions =
                            parse_trait_functions(&trait_content[..trait_content.len() - 1]);
                        result.push_str(&functions);

                        result.push_str("}\n\n");
                        pos = brace_start + brace_end;
                    } else {
                        pos = brace_start + 1;
                    }
                } else {
                    pos = name_start + 1;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    fn find_matching_brace(content: &str) -> Option<(usize, String)> {
        if !content.starts_with('{') {
            return None;
        }

        let mut depth = 0;
        let mut in_string = false;
        let mut in_char = false;
        let mut escaped = false;

        for (i, c) in content.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }

            match c {
                '\\' if in_string => escaped = true,
                '"' if !in_char => in_string = !in_string,
                '\'' if !in_string => in_char = !in_char,
                '{' if !in_string && !in_char => depth += 1,
                '}' if !in_string && !in_char => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((i + 1, content[..i + 1].to_string()));
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn parse_trait_functions(trait_content: &str) -> String {
        let mut result = String::new();
        let mut pending_doc = String::new();

        for line in trait_content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("///") {
                // Store doc comment
                pending_doc = trimmed.trim_start_matches("///").trim().to_string();
                continue;
            }

            if trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") {
                if let Some(func_wit) = parse_function_signature(trimmed, &pending_doc) {
                    result.push_str(&func_wit);
                }
                pending_doc.clear();
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with('#')
            {
                // Non-empty, non-comment line - clear pending doc
                if !trimmed.starts_with("fn ") {
                    pending_doc.clear();
                }
            }
        }

        result
    }

    fn parse_function_signature(func_line: &str, doc: &str) -> Option<String> {
        let func_line = func_line
            .trim()
            .trim_end_matches('{')
            .trim_end_matches(';')
            .trim();

        // Extract function name and signature
        let func_line = if let Some(stripped) = func_line.strip_prefix("async ") {
            stripped.trim_start()
        } else {
            func_line
        };

        if !func_line.starts_with("fn ") {
            return None;
        }

        let after_fn = &func_line[3..]; // Remove "fn "

        // Find opening paren
        let paren_open = after_fn.find('(')?;
        let func_name = after_fn[..paren_open].trim();
        let wit_func_name = camel_to_kebab(func_name);

        // Find closing paren
        let after_open = &after_fn[paren_open + 1..];
        let paren_depth = 1;
        let mut paren_close = None;
        let mut depth = paren_depth;

        for (i, c) in after_open.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        paren_close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let paren_close = paren_close?;
        let params_str = &after_open[..paren_close];
        let rest = &after_open[paren_close + 1..].trim();

        // Parse parameters (skip self)
        let params = parse_params(params_str);

        // Build WIT function signature
        let mut wit_func = String::new();

        // Add doc comment
        if !doc.is_empty() {
            wit_func.push_str(&format!("  /// {}\n", doc));
        }

        wit_func.push_str(&format!("  {}", wit_func_name));

        // Add parameters
        if params.is_empty() {
            wit_func.push_str("()");
        } else {
            let params_wit: Vec<String> = params
                .iter()
                .map(|(name, typ)| {
                    let wit_name = camel_to_kebab(name);
                    let wit_type = rust_type_to_wit(typ);
                    format!("{}: {}", wit_name, wit_type)
                })
                .collect();

            wit_func.push_str(&format!("({})", params_wit.join(", ")));
        }

        // Add return type
        if let Some(arrow_pos) = rest.find("->") {
            let ret_type = rest[arrow_pos + 2..].trim();
            if ret_type.contains("Result<") || ret_type.contains("result<") {
                let (ok_type, err_type) = parse_result_type(ret_type);
                let ok_wit = rust_type_to_wit(&ok_type);
                let err_wit = rust_type_to_wit(&err_type);

                // Handle different Result combinations
                match (ok_wit.is_empty(), err_wit.is_empty()) {
                    (false, false) => {
                        wit_func.push_str(&format!(" -> result<{}, {}>", ok_wit, err_wit))
                    }
                    (false, true) => wit_func.push_str(&format!(" -> result<{}>", ok_wit)),
                    (true, false) => wit_func.push_str(&format!(" -> result<_, {}>", err_wit)),
                    (true, true) => wit_func.push_str(" -> result"),
                }
            } else {
                let ret_wit = rust_type_to_wit(ret_type);
                wit_func.push_str(&format!(" -> {}", ret_wit));
            }
        }

        wit_func.push_str(";\n");

        Some(wit_func)
    }

    fn parse_params(params_str: &str) -> Vec<(String, String)> {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut current_param = String::new();

        for c in params_str.chars() {
            match c {
                '<' | '(' => {
                    depth += 1;
                    current_param.push(c);
                }
                '>' | ')' => {
                    depth -= 1;
                    current_param.push(c);
                }
                ',' if depth == 0 => {
                    if let Some((name, typ)) = parse_single_param(&current_param) {
                        result.push((name, typ));
                    }
                    current_param.clear();
                }
                _ => current_param.push(c),
            }
        }

        // Don't forget last param
        if !current_param.trim().is_empty()
            && let Some((name, typ)) = parse_single_param(&current_param)
        {
            result.push((name, typ));
        }

        result
    }

    fn parse_single_param(param: &str) -> Option<(String, String)> {
        let param = param.trim();
        if param.is_empty() || param.contains("self") {
            return None;
        }

        if let Some(colon_pos) = param.find(':') {
            let name = param[..colon_pos].trim().to_string();
            let typ = param[colon_pos + 1..].trim().to_string();
            Some((name, typ))
        } else {
            None
        }
    }

    fn parse_result_type(ret: &str) -> (String, String) {
        // Result<OkType, ErrType>
        let inner = ret
            .trim()
            .trim_start_matches("Result<")
            .trim_start_matches("result<")
            .trim_end_matches('>');

        let mut depth = 0;
        let mut comma_pos = None;

        for (i, c) in inner.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    comma_pos = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if let Some(pos) = comma_pos {
            let ok_type = inner[..pos].trim().to_string();
            let err_type = inner[pos + 1..].trim().to_string();

            // Handle Result<(), String>
            let ok_type = if ok_type == "()" {
                String::new()
            } else {
                ok_type
            };

            (ok_type, err_type)
        } else {
            // Check if it's just Result<(), Error>
            if inner.trim() == "()" {
                (String::new(), String::new())
            } else {
                // Only Ok type, error is unit
                (inner.to_string(), String::new())
            }
        }
    }

    fn rust_type_to_wit(rust_type: &str) -> String {
        let rust_type = rust_type.trim();

        match rust_type {
            "String" | "&str" | "str" => "string".to_string(),
            "bool" => "bool".to_string(),
            "u8" => "u8".to_string(),
            "u16" => "u16".to_string(),
            "u32" => "u32".to_string(),
            "u64" => "u64".to_string(),
            "i8" => "s8".to_string(),
            "i16" => "s16".to_string(),
            "i32" => "s32".to_string(),
            "i64" => "s64".to_string(),
            "f32" => "float32".to_string(),
            "f64" => "float64".to_string(),
            "()" => String::new(), // Unit type
            _ => {
                if rust_type.starts_with("Vec<") {
                    let inner = rust_type
                        .trim_start_matches("Vec<")
                        .trim_end_matches('>')
                        .to_string();
                    format!("list<{}>", rust_type_to_wit(&inner))
                } else if rust_type.starts_with("Option<") {
                    let inner = rust_type
                        .trim_start_matches("Option<")
                        .trim_end_matches('>')
                        .to_string();
                    format!("option<{}>", rust_type_to_wit(&inner))
                } else if rust_type.starts_with('(') && rust_type.ends_with(')') {
                    // Tuple
                    let inner = rust_type.trim_start_matches('(').trim_end_matches(')');
                    let types: Vec<String> = inner
                        .split(',')
                        .map(|t| rust_type_to_wit(t.trim()))
                        .collect();
                    if types.len() == 1 {
                        types[0].clone()
                    } else {
                        format!("tuple<{}>", types.join(", "))
                    }
                } else if rust_type.starts_with("Result<") || rust_type.starts_with("result<") {
                    let (ok, err) = parse_result_type(rust_type);
                    let ok_wit = rust_type_to_wit(&ok);
                    let err_wit = rust_type_to_wit(&err);
                    if err_wit.is_empty() {
                        format!("result<{}>", ok_wit)
                    } else {
                        format!("result<{}, {}>", ok_wit, err_wit)
                    }
                } else if rust_type.starts_with('&') {
                    // Remove reference
                    rust_type_to_wit(rust_type.trim_start_matches('&').trim())
                } else {
                    // Custom type - convert to kebab case
                    camel_to_kebab(rust_type)
                }
            }
        }
    }

    fn camel_to_kebab(input: &str) -> String {
        let mut result = String::new();
        let mut prev_is_upper = false;

        for (i, c) in input.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 && !prev_is_upper {
                    result.push('-');
                }
                result.push(c.to_ascii_lowercase());
                prev_is_upper = true;
            } else if c == '_' {
                result.push('-');
                prev_is_upper = false;
            } else {
                result.push(c);
                prev_is_upper = false;
            }
        }

        result
    }

    fn parse_field_line(line: &str) -> Option<(String, String)> {
        let line = line.trim().trim_end_matches(',');

        // Skip comments and attributes
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            return None;
        }

        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let typ = line[colon_pos + 1..].trim().to_string();

            // Skip if it looks like an attribute or invalid
            if name.starts_with("pub ") {
                let name = name.trim_start_matches("pub ").trim().to_string();
                return Some((name, typ));
            }

            if name.is_empty() || name.starts_with('#') {
                return None;
            }

            Some((name, typ))
        } else {
            None
        }
    }

    /// Parse FFI functions (extern "C" with #[no_mangle])
    fn parse_ffi_functions(content: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut pos = 0;

        while pos < content.len() {
            // Find #[no_mangle]
            if let Some(nomangle_pos) = content[pos..].find("#[no_mangle]") {
                let nomangle_start = pos + nomangle_pos;

                // Find the extern "C" after #[no_mangle]
                if let Some(extern_pos) = content[nomangle_start..].find("extern \"C\"") {
                    let extern_start = nomangle_start + extern_pos;

                    // Find "fn " after extern "C"
                    if let Some(fn_pos) = content[extern_start..].find("fn ") {
                        let fn_start = extern_start + fn_pos;

                        // Get the function signature line (may span multiple lines)
                        let mut sig_lines = String::new();
                        let mut line_start = fn_start;

                        // Collect lines until we find opening brace or semicolon
                        for line in content[line_start..].lines() {
                            sig_lines.push_str(line);
                            sig_lines.push(' ');

                            if line.contains('{') || line.contains("->") {
                                break;
                            }

                            line_start += line.len() + 1; // +1 for newline
                        }

                        // Parse the function
                        if let Some(func_wit) = parse_ffi_function(&sig_lines) {
                            result.push_str(&func_wit);
                            pos = line_start + 1;
                            continue;
                        }
                    }
                }

                pos = nomangle_start + "#[no_mangle]".len();
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse a single FFI function
    fn parse_ffi_function(func_content: &str) -> Option<String> {
        let func_line = func_content.lines().next()?;

        // Extract function name
        if !func_line.contains("fn ") {
            return None;
        }

        let fn_start = func_line.find("fn ")?;
        let after_fn = &func_line[fn_start + 3..];

        // Get function name
        let fn_name_end = after_fn.find('(')?;
        let fn_name = after_fn[..fn_name_end].trim();
        let wit_name = camel_to_kebab(fn_name);

        // Get parameters between ( and )
        let params_start = fn_name_end + 1;
        if let Some(params_end) = after_fn[params_start..].find(')') {
            let params_str = &after_fn[params_start..params_start + params_end];
            let after_paren = &after_fn[params_start + params_end + 1..].trim();

            // Parse parameters
            let params = parse_params(params_str);

            // Parse return type (stop at '{' which is function body)
            let return_type = if let Some(brace_pos) = after_paren.find('{') {
                let before_brace = after_paren[..brace_pos].trim();
                if before_brace.contains("->") {
                    let arrow_pos = before_brace.find("->")?;
                    let ret = before_brace[arrow_pos + 2..].trim();
                    Some(ret.to_string())
                } else {
                    None
                }
            } else if after_paren.contains("->") {
                let arrow_pos = after_paren.find("->")?;
                let ret = after_paren[arrow_pos + 2..].trim();
                Some(ret.to_string())
            } else {
                None
            };

            // Build WIT function signature
            let mut wit_func = String::new();

            // Always include parentheses
            if params.is_empty() {
                wit_func.push_str(&format!("  {}()", wit_name));
            } else {
                let params_wit: Vec<String> = params
                    .iter()
                    .map(|(name, typ)| {
                        let wit_name = camel_to_kebab(name);
                        let wit_type = c_type_to_wit(typ);
                        format!("{}: {}", wit_name, wit_type)
                    })
                    .collect();

                wit_func.push_str(&format!("  {}({})", wit_name, params_wit.join(", ")));
            }

            // Add return type
            if let Some(ret) = return_type {
                let wit_ret = c_type_to_wit(&ret);
                if !wit_ret.is_empty() {
                    wit_func.push_str(&format!(" -> {}", wit_ret));
                }
            }

            wit_func.push_str(";\n");

            Some(wit_func)
        } else {
            None
        }
    }

    /// Convert C types to WIT types
    fn c_type_to_wit(c_type: &str) -> String {
        let c_type = c_type.trim();

        match c_type {
            // Integer types
            "u8" | "uint8_t" => "u8".to_string(),
            "u16" | "uint16_t" => "u16".to_string(),
            "u32" | "uint32_t" | "c_uint" => "u32".to_string(),
            "u64" | "uint64_t" | "c_ulonglong" => "u64".to_string(),
            "i8" | "int8_t" => "s8".to_string(),
            "i16" | "int16_t" => "s16".to_string(),
            "i32" | "int32_t" | "c_int" => "s32".to_string(),
            "i64" | "int64_t" | "c_longlong" => "s64".to_string(),
            "usize" | "size_t" => "u32".to_string(),
            "isize" | "ssize_t" => "s32".to_string(),

            // Pointer types
            "*const c_char" | "*mut c_char" | "const char*" | "char*" => "string".to_string(),
            "*const u8" | "*mut u8" | "*const c_void" | "*mut c_void" => "pointer".to_string(),
            "*const u32" | "*mut u32" => "list<u32>".to_string(),

            // Void
            "()" | "void" => String::new(),

            // Bool
            "bool" | "c_bool" => "bool".to_string(),

            // Float
            "f32" | "c_float" => "float32".to_string(),
            "f64" | "c_double" => "float64".to_string(),

            _ => {
                // Handle complex types
                if c_type.contains('*') {
                    // It's a pointer - determine what it points to
                    if c_type.contains("c_char") {
                        "string".to_string()
                    } else {
                        "pointer".to_string()
                    }
                } else {
                    // Unknown type - convert to kebab case
                    camel_to_kebab(c_type)
                }
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Run a command and print output
fn run_command(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd).args(args).status().unwrap_or_else(|e| {
        eprintln!("{}Failed to execute {}: {}{}", RED, cmd, e, RESET);
        exit(1);
    });

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}

/// Ensure a Rust target is installed
fn ensure_target_installed(target: &str) {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to run rustup");

    let installed = String::from_utf8_lossy(&output.stdout);

    if !installed.lines().any(|line| line.trim() == target) {
        println!("{}Installing target: {}{}", YELLOW, target, RESET);
        run_command("rustup", &["target", "add", target]);
    }
}

/// Get the project root directory
fn project_root() -> PathBuf {
    let mut current = env::current_exe()
        .expect("Failed to get current exe path")
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();

    while current.parent().is_some() {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

            if content.contains("[workspace]") {
                return current;
            }
        }
        current = current.parent().unwrap().to_path_buf();
    }

    env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("Failed to get current directory"))
}
