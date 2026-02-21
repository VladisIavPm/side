mod lexer;
mod parser;
mod ast;
mod interpreter;
mod value;
mod error;
mod packer;
mod project;
mod logger;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use logos::Logos;
use parser::Parser;
use interpreter::Interpreter;
use project::SpackConfig;

const STATE_FILE: &str = ".spack_current";

fn save_current_config(path: &Path) -> Result<(), String> {
    fs::write(STATE_FILE, path.to_string_lossy().as_bytes())
        .map_err(|e| format!("Failed to save state: {}", e))
}

fn load_current_config() -> Option<PathBuf> {
    fs::read_to_string(STATE_FILE).ok().map(PathBuf::from)
}

fn build_from_config(config_path: &Path) -> Result<(), String> {
    let config = SpackConfig::from_file(config_path)?;
    let main_file = config_path.parent().unwrap_or(Path::new(".")).join(&config.main);
    let main_file = main_file.to_str().ok_or("Invalid main file path")?;
    let output = config_path.parent().unwrap_or(Path::new(".")).join(config.output_name());
    let output = output.to_str().ok_or("Invalid output path")?;
    
    packer::build(main_file, output)?;
    println!("Built project '{}' v{}", config.name, config.version);
    Ok(())
}

fn run_script(source: &str, interpreter: &mut Interpreter) -> anyhow::Result<()> {
    let lex = lexer::Token::lexer(source);
    let tokens: Vec<_> = lex.map(|r| r.unwrap_or(lexer::Token::Error)).collect();
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    interpreter.run(program)?;
    Ok(())
}

fn run_file_with_interpreter(filename: &str, interpreter: &mut Interpreter) -> anyhow::Result<()> {
    let source = fs::read_to_string(filename)?;
    logger::info(&format!("Executing file: {}", filename));
    run_script(&source, interpreter)
}

fn repl_loop_with_interpreter(interpreter: &mut Interpreter) -> anyhow::Result<()> {
    println!("Side Language Interpreter v1.2 with Spack");
    println!("Commands:");
    println!("  spack run <file>              - execute script");
    println!("  spack build [file]            - build executable from config");
    println!("  spack connect <file>          - connect project config");
    println!("  spack link run <file>         - load module (globals persist)");
    println!("  spack link build <file>       - build module as standalone exe");
    println!("  spack link list               - show loaded modules");
    println!("  spack link clear              - clear module cache");
    println!("  exit                          - exit REPL");
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "exit" {
            break;
        }
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "spack" if parts.len() >= 2 => match parts[1] {
                "run" if parts.len() >= 3 => {
                    if let Err(e) = run_file_with_interpreter(parts[2], interpreter) {
                        eprintln!("Error: {}", e);
                        logger::error(&format!("Run error: {}", e));
                    }
                }
                "build" => {
                    if parts.len() >= 3 {
                        if let Err(e) = build_from_config(Path::new(parts[2])) {
                            eprintln!("Build error: {}", e);
                            logger::error(&format!("Build error: {}", e));
                        }
                    } else {
                        if let Some(config_path) = load_current_config() {
                            if config_path.exists() {
                                if let Err(e) = build_from_config(&config_path) {
                                    eprintln!("Build error: {}", e);
                                    logger::error(&format!("Build error: {}", e));
                                }
                            } else {
                                eprintln!("No config found. Run 'spack connect <file>' first.");
                            }
                        } else {
                            eprintln!("No project connected. Run 'spack connect <file>' first.");
                        }
                    }
                }
                "connect" if parts.len() >= 3 => {
                    let config_file = parts[2];
                    let path = Path::new(config_file);
                    if !path.exists() {
                        eprintln!("Config file not found: {}", config_file);
                    } else if let Err(e) = save_current_config(path) {
                        eprintln!("Failed to connect: {}", e);
                    } else {
                        println!("Connected to project: {}", config_file);
                        logger::info(&format!("Connected to project: {}", config_file));
                    }
                }
                "link" if parts.len() >= 3 => match parts[2] {
                    "run" if parts.len() >= 4 => {
                        let filename = parts[3];
                        match interpreter.load_module(filename) {
                            Ok(()) => {
                                println!("Module loaded: {}", filename);
                                logger::info(&format!("Module loaded: {}", filename));
                            }
                            Err(e) => {
                                eprintln!("Module error: {}", e);
                                logger::error(&format!("Module error: {}", e));
                            }
                        }
                    }
                    "build" if parts.len() >= 4 => {
                        let filename = parts[3];
                        let output = if parts.len() >= 5 {
                            parts[4].to_string()
                        } else {
                            Path::new(filename).with_extension("exe").to_string_lossy().to_string()
                        };
                        match packer::build(filename, &output) {
                            Ok(()) => {
                                println!("Built module: {}", output);
                                logger::info(&format!("Built module: {}", output));
                            }
                            Err(e) => {
                                eprintln!("Build error: {}", e);
                                logger::error(&format!("Build error: {}", e));
                            }
                        }
                    }
                    "list" => {
                        let modules = interpreter.list_modules();
                        if modules.is_empty() {
                            println!("No modules loaded.");
                        } else {
                            println!("Loaded modules:");
                            for m in modules {
                                println!("  {}", m);
                            }
                        }
                    }
                    "clear" => {
                        interpreter.clear_modules();
                        println!("Module cache cleared.");
                        logger::info("Module cache cleared");
                    }
                    _ => println!("Usage: spack link run <file> | spack link build <file> [output] | spack link list | spack link clear"),
                },
                _ => println!("Unknown spack command. Try 'spack run', 'spack build', 'spack connect', or 'spack link'."),
            },
            _ => println!("Unknown command. Type 'exit' to quit."),
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Инициализация логов
    logger::init().ok();
    logger::info("Side interpreter started");

    // Если встроенный скрипт – выполняем и ждём Enter
    if packer::has_embedded() {
        let mut interpreter = Interpreter::new();
        if let Some(source) = packer::extract_embedded() {
            run_script(&source, &mut interpreter)?;
            println!("\nExecution finished. Press Enter to exit...");
            logger::info("Embedded script finished");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            return Ok(());
        } else {
            eprintln!("Error: embedded script corrupted");
            logger::error("Embedded script corrupted");
            std::process::exit(1);
        }
    }

    let args: Vec<String> = env::args().collect();
    let mut interpreter = Interpreter::new();

    if args.len() >= 2 {
        match args[1].as_str() {
            "spack" if args.len() >= 3 => match args[2].as_str() {
                "run" if args.len() >= 4 => {
                    let filename = &args[3];
                    return run_file_with_interpreter(filename, &mut interpreter);
                }
                "build" => {
                    if args.len() >= 4 {
                        let config_file = &args[3];
                        if let Err(e) = build_from_config(Path::new(config_file)) {
                            eprintln!("Build error: {}", e);
                            logger::error(&format!("Build error: {}", e));
                            std::process::exit(1);
                        }
                        return Ok(());
                    } else {
                        if let Some(config_path) = load_current_config() {
                            if config_path.exists() {
                                if let Err(e) = build_from_config(&config_path) {
                                    eprintln!("Build error: {}", e);
                                    logger::error(&format!("Build error: {}", e));
                                    std::process::exit(1);
                                }
                                return Ok(());
                            } else {
                                eprintln!("No config found. Run 'spack connect <file>' first.");
                                std::process::exit(1);
                            }
                        } else {
                            eprintln!("No project connected. Run 'spack connect <file>' first.");
                            std::process::exit(1);
                        }
                    }
                }
                "connect" if args.len() >= 4 => {
                    let config_file = &args[3];
                    let path = Path::new(config_file);
                    if !path.exists() {
                        eprintln!("Config file not found: {}", config_file);
                        std::process::exit(1);
                    }
                    if let Err(e) = save_current_config(path) {
                        eprintln!("Failed to connect: {}", e);
                        std::process::exit(1);
                    }
                    println!("Connected to project: {}", config_file);
                    logger::info(&format!("Connected to project: {}", config_file));
                    return Ok(());
                }
                "link" if args.len() >= 4 => match args[3].as_str() {
                    "run" if args.len() >= 5 => {
                        let filename = &args[4];
                        interpreter.load_module(filename)
                            .map_err(|e| anyhow::anyhow!("{}", e))?;
                        println!("Module loaded: {}", filename);
                        logger::info(&format!("Module loaded: {}", filename));
                        return Ok(());
                    }
                    "build" if args.len() >= 5 => {
                        let filename = &args[4];
                        let output = if args.len() >= 6 {
                            args[5].clone()
                        } else {
                            Path::new(filename).with_extension("exe").to_string_lossy().to_string()
                        };
                        match packer::build(filename, &output) {
                            Ok(()) => {
                                println!("Built module: {}", output);
                                logger::info(&format!("Built module: {}", output));
                                return Ok(());
                            }
                            Err(e) => {
                                eprintln!("Build error: {}", e);
                                logger::error(&format!("Build error: {}", e));
                                std::process::exit(1);
                            }
                        }
                    }
                    "list" => {
                        let modules = interpreter.list_modules();
                        if modules.is_empty() {
                            println!("No modules loaded.");
                        } else {
                            println!("Loaded modules:");
                            for m in modules {
                                println!("  {}", m);
                            }
                        }
                        return Ok(());
                    }
                    "clear" => {
                        interpreter.clear_modules();
                        println!("Module cache cleared.");
                        logger::info("Module cache cleared");
                        return Ok(());
                    }
                    _ => {
                        eprintln!("Usage: spack link run <file> | spack link build <file> [output] | spack link list | spack link clear");
                        std::process::exit(1);
                    }
                },
                _ => {
                    eprintln!("Usage: spack run <file> | spack build [file] | spack connect <file> | spack link ...");
                    std::process::exit(1);
                }
            },
            _ => {
                let filename = &args[1];
                return run_file_with_interpreter(filename, &mut interpreter);
            }
        }
    } else {
        repl_loop_with_interpreter(&mut interpreter)?;
    }
    
    logger::info("Side interpreter finished");
    Ok(())
}