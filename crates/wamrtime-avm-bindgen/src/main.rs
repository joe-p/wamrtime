use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <rust_file> <go_file>", args[0]);
        std::process::exit(1);
    }

    let rust_input = fs::read_to_string(&args[1]).expect("Failed to read Rust input file");
    let go_content = fs::read_to_string(&args[2]).expect("Failed to read Go file");

    let functions = parse_rust_functions(&rust_input);
    let generated = generate_cgo_code(&functions);

    let new_go_content = replace_bindgen_section(&go_content, &generated);

    fs::write(&args[2], new_go_content).expect("Failed to write Go file");
}

fn replace_bindgen_section(go_content: &str, generated_code: &str) -> String {
    let start_marker = "// WAMR_BINDGEN SECTION_START";
    let end_marker = "// WAMR_BINDGEN SECTION_END";

    if let Some(start_pos) = go_content.find(start_marker) {
        if let Some(end_pos) = go_content.find(end_marker) {
            let before = &go_content[..start_pos];
            let after = &go_content[end_pos + end_marker.len()..];

            return format!(
                "{}{}\n{}\n{}{}",
                before, start_marker, generated_code, end_marker, after
            );
        }
    }

    go_content.to_string()
}

#[derive(Debug)]
struct Function {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
}

fn parse_rust_functions(input: &str) -> Vec<Function> {
    let mut functions = Vec::new();

    if let Some(start) = input.find("avm_host_functions!") {
        let after_macro = &input[start..];
        if let Some(brace_start) = after_macro.find('{') {
            if let Some(brace_end) = after_macro.find('}') {
                let content = &after_macro[brace_start + 1..brace_end];

                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with("//") {
                        continue;
                    }

                    if let Some(func) = parse_function_line(line) {
                        functions.push(func);
                    }
                }
            }
        }
    }

    functions
}

fn parse_function_line(line: &str) -> Option<Function> {
    let line = line.trim();
    if !line.contains('(') {
        return None;
    }

    let line = line.trim_end_matches(';').trim();

    let (signature, return_type) = if let Some(pos) = line.rfind("->") {
        let sig = line[..pos].trim();
        let ret = line[pos + 2..].trim().to_string();
        (sig, Some(ret))
    } else {
        (line, None)
    };

    let paren_start = signature.find('(')?;
    let paren_end = signature.rfind(')')?;

    let name = signature[..paren_start].trim().to_string();
    let params_str = &signature[paren_start + 1..paren_end];

    let mut params = Vec::new();
    if !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }

            let parts: Vec<&str> = param.splitn(2, ':').collect();
            if parts.len() == 2 {
                let param_name = parts[0].trim().to_string();
                let param_type = parts[1].trim().to_string();
                params.push((param_name, param_type));
            }
        }
    }

    Some(Function {
        name,
        params,
        return_type,
    })
}

fn rust_type_to_c(rust_type: &str) -> String {
    match rust_type {
        "u8" => "uint8_t".to_string(),
        "u16" => "uint16_t".to_string(),
        // Use CGo-compatible types: unsigned int matches GoUint32, unsigned long long matches GoUint64
        "u32" => "unsigned int".to_string(),
        "u64" => "unsigned long long".to_string(),
        "i8" => "int8_t".to_string(),
        "i16" => "int16_t".to_string(),
        // Use CGo-compatible types: int matches GoInt32
        "i32" => "int".to_string(),
        "i64" => "int64_t".to_string(),
        "*const u8" => "const uint8_t*".to_string(),
        "*mut u8" => "uint8_t*".to_string(),
        _ if rust_type.starts_with("*const") => {
            let inner = rust_type.strip_prefix("*const").unwrap().trim();
            format!("const {}*", rust_type_to_c(inner))
        }
        _ if rust_type.starts_with("*mut") => {
            let inner = rust_type.strip_prefix("*mut").unwrap().trim();
            format!("{}*", rust_type_to_c(inner))
        }
        _ => rust_type.to_string(),
    }
}

fn to_camel_case(snake: &str) -> String {
    snake
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

fn generate_cgo_code(functions: &[Function]) -> String {
    let mut output = String::new();

    for func in functions {
        let camel_name = to_camel_case(&func.name);
        let go_func_name = format!("go{}", camel_name);

        let c_return_type = func
            .return_type
            .as_ref()
            .map(|t| rust_type_to_c(t))
            .unwrap_or_else(|| "void".to_string());

        let mut c_params = vec!["void* exec_env".to_string(), "void* ctx".to_string()];
        let mut go_params = vec!["void* exec_env".to_string(), "void* ctx".to_string()];

        for (name, rust_type) in &func.params {
            let c_type = rust_type_to_c(rust_type);
            c_params.push(format!("{} {}", c_type, name));

            let go_c_type = if c_type.starts_with("const ") {
                c_type.strip_prefix("const ").unwrap().to_string()
            } else {
                c_type
            };
            go_params.push(format!("{} {}", go_c_type, name));
        }

        output.push_str(&format!(
            "typedef {} (*{}Fn)({});\n\n",
            c_return_type,
            camel_name,
            c_params.join(", ")
        ));

        output.push_str(&format!(
            "extern {} {}({});\n\n",
            c_return_type,
            go_func_name,
            go_params.join(", ")
        ));

        output.push_str(&format!(
            "static inline {}Fn getGo{}() {{\n",
            camel_name, camel_name
        ));
        output.push_str(&format!("\treturn ({}Fn){};\n", camel_name, go_func_name));
        output.push_str("}\n\n");
    }

    let mut init_params = vec![];
    for func in functions {
        let camel_name = to_camel_case(&func.name);
        init_params.push(format!("{}Fn {}_impl", camel_name, func.name));
    }

    output.push_str(&format!("void avm_init({});", init_params.join(", ")));

    output
}
