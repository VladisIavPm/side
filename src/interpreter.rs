use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use std::thread::sleep;
use std::time::Duration;
use std::io::{self, Write, Read};
use std::fs;
use std::path::{Path, PathBuf};
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_void};

use logos::Logos;

use crate::ast::*;
use crate::value::Value;
use crate::error::RuntimeError;
use crate::lexer;
use crate::parser::Parser;
use crate::logger;

// Структуры для взаимодействия с нативными модулями
#[repr(C)]
struct NativeFunction {
    name: *const std::os::raw::c_char,
    args: u32,
    func: *const c_void,
}

#[repr(C)]
struct NativeModuleInfo {
    name: *const std::os::raw::c_char,
    functions: *const NativeFunction,
    count: u32,
}

// Информация о загруженной функции
struct NativeFunctionInfo {
    arg_count: u32,
    func_ptr: *const c_void,
}

// Загруженный нативный модуль
struct NativeModule {
    _lib: Library,
    functions: HashMap<String, NativeFunctionInfo>,
}

pub struct Interpreter {
    globals: Rc<RefCell<HashMap<String, Variable>>>,
    functions: HashMap<String, Proc>,
    forms: HashMap<String, Form>,
    loaded_modules: HashSet<PathBuf>,
    native_modules: HashMap<String, NativeModule>,
    current_line: usize,
}

struct Variable {
    value: Value,
    mutable: bool,
}

#[derive(Clone)]
struct Proc {
    params: Vec<String>,
    body: Vec<Stmt>,
}

struct Form {
    fields: HashMap<String, FieldInfo>,
}

struct FieldInfo {
    mutable: bool,
    default: Option<Value>,
}

struct Environment {
    locals: HashMap<String, Variable>,
    globals: Rc<RefCell<HashMap<String, Variable>>>,
}

impl Environment {
    fn new(globals: Rc<RefCell<HashMap<String, Variable>>>) -> Self {
        Environment {
            locals: HashMap::new(),
            globals,
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(var) = self.locals.get(name) {
            Some(var.value.clone())
        } else if let Ok(globals) = self.globals.try_borrow() {
            globals.get(name).map(|v| v.value.clone())
        } else {
            None
        }
    }

    fn set(&mut self, name: String, value: Value, mutable: bool) -> Result<(), RuntimeError> {
        if let Some(var) = self.locals.get_mut(&name) {
            if var.mutable {
                var.value = value;
                Ok(())
            } else {
                Err(RuntimeError::TypeError(format!("Cannot modify constant '{}'", name)))
            }
        } else {
            let mut globals = self.globals.borrow_mut();
            if let Some(var) = globals.get_mut(&name) {
                if var.mutable {
                    var.value = value;
                    Ok(())
                } else {
                    Err(RuntimeError::TypeError(format!("Cannot modify constant '{}'", name)))
                }
            } else {
                self.locals.insert(name, Variable { value, mutable });
                Ok(())
            }
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            globals: Rc::new(RefCell::new(HashMap::new())),
            functions: HashMap::new(),
            forms: HashMap::new(),
            loaded_modules: HashSet::new(),
            native_modules: HashMap::new(),
            current_line: 0,
        }
    }

    pub fn run(&mut self, program: Program) -> Result<(), RuntimeError> {
        logger::info("Starting program execution");
        let mut env = Environment::new(self.globals.clone());
        self.run_program_in_env(program, &mut env)
    }

    fn run_program_in_env(&mut self, program: Program, env: &mut Environment) -> Result<(), RuntimeError> {
        for item in program.items {
            match item {
                Item::Decl(decl) => self.declare(decl, env)?,
                Item::Stmt(stmt) => self.execute_stmt(&stmt, env)?,
            }
        }
        Ok(())
    }

    // ==================== NATIVE MODULES ====================

    fn load_native_module(&mut self, name: &str) -> Result<(), RuntimeError> {
        logger::info(&format!("Loading native module: {}", name));

        #[cfg(target_os = "windows")]
        let lib_path = format!("modules/{}.dll", name);
        #[cfg(target_os = "linux")]
        let lib_path = format!("modules/lib{}.so", name);
        #[cfg(target_os = "macos")]
        let lib_path = format!("modules/lib{}.dylib", name);

        let lib = unsafe {
            Library::new(&lib_path)
                .map_err(|e| RuntimeError::TypeError(format!("Cannot load module '{}': {}", name, e)))?
        };

        unsafe {
            let info_sym: Symbol<*const NativeModuleInfo> = lib.get(b"side_module_info\0")
                .map_err(|e| RuntimeError::TypeError(format!("Module '{}' has no info symbol: {}", name, e)))?;
            let info = &**info_sym;

            let module_name = CStr::from_ptr(info.name).to_string_lossy().into_owned();

            let funcs_slice = std::slice::from_raw_parts(info.functions, info.count as usize);
            let mut functions = HashMap::new();

            for f in funcs_slice {
                let func_name = CStr::from_ptr(f.name).to_string_lossy().into_owned();
                functions.insert(func_name, NativeFunctionInfo {
                    arg_count: f.args,
                    func_ptr: f.func,
                });
            }

            self.native_modules.insert(module_name, NativeModule {
                _lib: lib,
                functions,
            });
        }

        logger::info(&format!("Native module '{}' loaded", name));
        Ok(())
    }

    // ==================== PUBLIC METHODS ====================

    pub fn load_module(&mut self, path: &str) -> Result<(), RuntimeError> {
        let current_dir = std::env::current_dir()
            .map_err(|e| RuntimeError::TypeError(format!("Cannot get current dir: {}", e)))?;
        let full_path = current_dir.join(path);
        let canonical = fs::canonicalize(&full_path)
            .map_err(|e| RuntimeError::TypeError(format!("Cannot resolve path {}: {}", path, e)))?;

        if self.loaded_modules.contains(&canonical) {
            return Ok(());
        }

        let source = fs::read_to_string(&canonical)
            .map_err(|e| RuntimeError::TypeError(format!("Cannot read module {}: {}", path, e)))?;
        let lex = lexer::Token::lexer(&source);
        let tokens: Vec<_> = lex.map(|r| r.unwrap_or(lexer::Token::Error)).collect();
        let mut parser = Parser::new(tokens);
        let module_program = parser.parse()
            .map_err(|e| RuntimeError::TypeError(format!("Parse error in module {}: {}", path, e)))?;

        self.loaded_modules.insert(canonical);
        let mut module_env = Environment::new(self.globals.clone());
        self.run_program_in_env(module_program, &mut module_env)?;

        Ok(())
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.loaded_modules.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    pub fn clear_modules(&mut self) {
        self.loaded_modules.clear();
    }

    // ==================== DECLARATIONS ====================

    fn declare(&mut self, decl: Decl, env: &mut Environment) -> Result<(), RuntimeError> {
        match decl {
            Decl::Link { path, alias: _alias } => {
                logger::info(&format!("Loading module: {}", path));
                let current_dir = std::env::current_dir()
                    .map_err(|e| RuntimeError::TypeError(format!("Cannot get current dir: {}", e)))?;
                let full_path = current_dir.join(&path);
                let canonical = fs::canonicalize(&full_path)
                    .map_err(|e| RuntimeError::TypeError(format!("Cannot resolve path {}: {}", path, e)))?;

                if self.loaded_modules.contains(&canonical) {
                    return Ok(());
                }

                let source = fs::read_to_string(&canonical)
                    .map_err(|e| RuntimeError::TypeError(format!("Cannot read module {}: {}", path, e)))?;
                let lex = lexer::Token::lexer(&source);
                let tokens: Vec<_> = lex.map(|r| r.unwrap_or(lexer::Token::Error)).collect();
                let mut parser = Parser::new(tokens);
                let module_program = parser.parse()
                    .map_err(|e| RuntimeError::TypeError(format!("Parse error in module {}: {}", path, e)))?;

                self.loaded_modules.insert(canonical);
                self.run_program_in_env(module_program, env)?;

                Ok(())
            }
            Decl::Proc { name, params, body } => {
                self.functions.insert(name, Proc { params, body });
                Ok(())
            }
            Decl::Form { name, fields } => {
                let mut fields_map = HashMap::new();
                for field in fields {
                    let default = match field.initial {
                        Some(expr) => {
                            let mut dummy_env = Environment::new(self.globals.clone());
                            Some(self.eval_expr(&expr, &mut dummy_env)?)
                        }
                        None => None,
                    };
                    fields_map.insert(field.name, FieldInfo { mutable: field.mutable, default });
                }
                self.forms.insert(name, Form { fields: fields_map });
                Ok(())
            }
        }
    }

    // ==================== STATEMENTS ====================

    fn execute_stmt(&mut self, stmt: &Stmt, env: &mut Environment) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Set { name, value, mutable } => {
                let val = self.eval_expr(value, env)?;
                env.set(name.clone(), val, *mutable)?;
                Ok(())
            }
            Stmt::Assign { target, value } => {
                let val = self.eval_expr(value, env)?;
                self.assign(target, val, env)?;
                Ok(())
            }
            Stmt::Log(expr) => {
                let val = self.eval_expr(expr, env)?;
                println!("{}", val);
                Ok(())
            }
            Stmt::If { condition, then_block, else_block } => {
                let cond = self.eval_expr(condition, env)?;
                if self.is_truthy(&cond) {
                    self.execute_block(then_block, env)
                } else if let Some(else_block) = else_block {
                    self.execute_block(else_block, env)
                } else {
                    Ok(())
                }
            }
            Stmt::Loop { condition, body } => {
                loop {
                    let cond = self.eval_expr(condition, env)?;
                    if !self.is_truthy(&cond) {
                        break;
                    }
                    match self.execute_block(body, env) {
                        Ok(()) => continue,
                        Err(RuntimeError::Break) => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
            Stmt::Break => Err(RuntimeError::Break),
            Stmt::Wait(expr) => {
                let secs = self.eval_expr(expr, env)?;
                match secs {
                    Value::Whole(n) => sleep(Duration::from_secs(n as u64)),
                    Value::Fraction(f) => sleep(Duration::from_secs_f64(f)),
                    _ => return Err(RuntimeError::TypeError("wait expects number".to_string())),
                }
                Ok(())
            }
            Stmt::Return(expr) => {
                let val = match expr {
                    Some(e) => self.eval_expr(e, env)?,
                    None => Value::None,
                };
                Err(RuntimeError::Return(val))
            }
            Stmt::Trap { try_block, catch_block } => {
                match self.execute_block(try_block, env) {
                    Ok(()) => Ok(()),
                    Err(_) => self.execute_block(catch_block, env),
                }
            }
            Stmt::ExprStmt(expr) => {
                self.eval_expr(expr, env)?;
                Ok(())
            }
        }
    }

    fn execute_block(&mut self, stmts: &[Stmt], env: &mut Environment) -> Result<(), RuntimeError> {
        for stmt in stmts {
            self.execute_stmt(stmt, env)?;
        }
        Ok(())
    }

    // ==================== EXPRESSIONS ====================

    fn eval_expr(&mut self, expr: &Expr, env: &mut Environment) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(val) => Ok(val.clone()),
            Expr::List(elems) => {
                let mut values = Vec::new();
                for elem in elems {
                    values.push(self.eval_expr(elem, env)?);
                }
                Ok(Value::List(values))
            }
            Expr::Variable(name) => {
                env.get(name).ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
            }
            Expr::Binary { left, op, right } => {
                let left_val = self.eval_expr(left, env)?;
                let right_val = self.eval_expr(right, env)?;
                self.eval_binary(op, left_val, right_val)
            }
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr, env)?;
                self.eval_unary(op, val)
            }
            Expr::Call { name, args } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expr(arg, env)?);
                }
                self.call_function(name, arg_vals, env)
            }
            Expr::Field { object, field } => {
                let obj = self.eval_expr(object, env)?;
                self.get_field(obj, field)
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object, env)?;
                let idx = self.eval_expr(index, env)?;
                self.get_index(obj, idx)
            }
            Expr::New { name } => {
                self.create_object(name)
            }
            Expr::Entry { prompt } => {
                let prompt_str = if let Some(p) = prompt {
                    let val = self.eval_expr(p, env)?;
                    match val {
                        Value::String(s) => s,
                        _ => return Err(RuntimeError::TypeError("entry prompt must be string".to_string())),
                    }
                } else {
                    String::new()
                };
                print!("{}", prompt_str);
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                Ok(Value::String(input.trim().to_string()))
            }
        }
    }

    fn eval_binary(&self, op: &BinOp, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match op {
            BinOp::Add => left.add(right),
            BinOp::Sub => left.sub(right),
            BinOp::Mul => left.mul(right),
            BinOp::Div => left.div(right),
            BinOp::Rem => left.rem(right),
            BinOp::Eq => left.eq(right),
            BinOp::Ne => left.ne(right),
            BinOp::Lt => left.lt(right),
            BinOp::Le => left.le(right),
            BinOp::Gt => left.gt(right),
            BinOp::Ge => left.ge(right),
            BinOp::And => {
                let left_bool = self.is_truthy(&left);
                if !left_bool {
                    Ok(Value::Bool(false))
                } else {
                    let right_bool = self.is_truthy(&right);
                    Ok(Value::Bool(right_bool))
                }
            }
            BinOp::Or => {
                let left_bool = self.is_truthy(&left);
                if left_bool {
                    Ok(Value::Bool(true))
                } else {
                    let right_bool = self.is_truthy(&right);
                    Ok(Value::Bool(right_bool))
                }
            }
        }
    }

    fn eval_unary(&self, op: &UnOp, val: Value) -> Result<Value, RuntimeError> {
        match op {
            UnOp::Not => Ok(Value::Bool(!self.is_truthy(&val))),
        }
    }

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::None => false,
            Value::Whole(n) => *n != 0,
            Value::Fraction(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Object { .. } => true,
            Value::List(list) => !list.is_empty(),
        }
    }

    // ==================== FUNCTION CALLS ====================

    fn call_function(&mut self, name: &str, args: Vec<Value>, _env: &mut Environment) -> Result<Value, RuntimeError> {
    match name {
        // ==================== ВСТРОЕННЫЕ ФУНКЦИИ ====================
        "read_file" => {
            logger::info("Built-in function: read_file");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("read_file expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("read_file expects string argument".to_string())),
            };
            match fs::read_to_string(path) {
                Ok(content) => Ok(Value::String(content)),
                Err(e) => {
                    logger::error(&format!("read_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot read file {}: {}", path, e)))
                }
            }
        }
        "write_file" => {
            logger::info("Built-in function: write_file");
            if args.len() != 2 {
                return Err(RuntimeError::TypeError("write_file expects 2 arguments".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("write_file first argument must be string".to_string())),
            };
            let content = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("write_file second argument must be string".to_string())),
            };
            match fs::write(path, content) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("write_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot write file {}: {}", path, e)))
                }
            }
        }
        "append_file" => {
            logger::info("Built-in function: append_file");
            if args.len() != 2 {
                return Err(RuntimeError::TypeError("append_file expects 2 arguments".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("append_file first argument must be string".to_string())),
            };
            let content = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("append_file second argument must be string".to_string())),
            };
            match fs::OpenOptions::new().append(true).create(true).open(path) {
                Ok(mut file) => {
                    use std::io::Write;
                    if let Err(e) = writeln!(file, "{}", content) {
                        logger::error(&format!("append_file failed: {}", e));
                        Err(RuntimeError::TypeError(format!("Cannot append to file {}: {}", path, e)))
                    } else {
                        Ok(Value::Bool(true))
                    }
                }
                Err(e) => {
                    logger::error(&format!("append_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot open file {} for append: {}", path, e)))
                }
            }
        }
        "delete_file" => {
            logger::info("Built-in function: delete_file");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("delete_file expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("delete_file expects string argument".to_string())),
            };
            match fs::remove_file(path) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("delete_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot delete file {}: {}", path, e)))
                }
            }
        }
        "copy_file" => {
            logger::info("Built-in function: copy_file");
            if args.len() != 2 {
                return Err(RuntimeError::TypeError("copy_file expects 2 arguments".to_string()));
            }
            let src = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("copy_file first argument must be string".to_string())),
            };
            let dst = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("copy_file second argument must be string".to_string())),
            };
            match fs::copy(src, dst) {
                Ok(_) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("copy_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot copy {} to {}: {}", src, dst, e)))
                }
            }
        }
        "rename_file" => {
            logger::info("Built-in function: rename_file");
            if args.len() != 2 {
                return Err(RuntimeError::TypeError("rename_file expects 2 arguments".to_string()));
            }
            let old = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("rename_file first argument must be string".to_string())),
            };
            let new = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("rename_file second argument must be string".to_string())),
            };
            match fs::rename(old, new) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("rename_file failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot rename {} to {}: {}", old, new, e)))
                }
            }
        }
        "file_exists" => {
            logger::info("Built-in function: file_exists");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("file_exists expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("file_exists expects string argument".to_string())),
            };
            Ok(Value::Bool(Path::new(path).exists()))
        }
        "file_size" => {
            logger::info("Built-in function: file_size");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("file_size expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("file_size expects string argument".to_string())),
            };
            match fs::metadata(path) {
                Ok(meta) => Ok(Value::Whole(meta.len() as i64)),
                Err(e) => {
                    logger::error(&format!("file_size failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot get file size: {}", e)))
                }
            }
        }
        "file_time" => {
            logger::info("Built-in function: file_time");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("file_time expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("file_time expects string argument".to_string())),
            };
            match fs::metadata(path) {
                Ok(meta) => {
                    if let Ok(modified) = meta.modified() {
                        match modified.duration_since(std::time::UNIX_EPOCH) {
                            Ok(duration) => Ok(Value::Fraction(duration.as_secs_f64())),
                            Err(_) => Err(RuntimeError::TypeError("File time is before Unix epoch".to_string())),
                        }
                    } else {
                        Err(RuntimeError::TypeError("Cannot get modification time on this platform".to_string()))
                    }
                }
                Err(e) => {
                    logger::error(&format!("file_time failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot access file {}: {}", path, e)))
                }
            }
        }
        "list_dir" => {
            logger::info("Built-in function: list_dir");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("list_dir expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("list_dir expects string argument".to_string())),
            };
            match fs::read_dir(path) {
                Ok(entries) => {
                    let mut files = Vec::new();
                    for entry in entries {
                        match entry {
                            Ok(e) => {
                                if let Some(name) = e.file_name().to_str() {
                                    files.push(Value::String(name.to_string()));
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                    Ok(Value::List(files))
                }
                Err(e) => {
                    logger::error(&format!("list_dir failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot read directory {}: {}", path, e)))
                }
            }
        }
        "create_dir" => {
            logger::info("Built-in function: create_dir");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("create_dir expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("create_dir expects string argument".to_string())),
            };
            match fs::create_dir(path) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("create_dir failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot create directory {}: {}", path, e)))
                }
            }
        }
        "remove_dir" => {
            logger::info("Built-in function: remove_dir");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("remove_dir expects 1 argument".to_string()));
            }
            let path = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("remove_dir expects string argument".to_string())),
            };
            match fs::remove_dir(path) {
                Ok(()) => Ok(Value::Bool(true)),
                Err(e) => {
                    logger::error(&format!("remove_dir failed: {}", e));
                    Err(RuntimeError::TypeError(format!("Cannot remove directory {}: {}", path, e)))
                }
            }
        }
        "len" => {
            logger::info("Built-in function: len");
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("len expects 1 argument".to_string()));
            }
            match &args[0] {
                Value::List(list) => Ok(Value::Whole(list.len() as i64)),
                Value::String(s) => Ok(Value::Whole(s.len() as i64)),
                _ => Err(RuntimeError::TypeError("len expects list or string".to_string())),
            }
        }
        "log_message" => {
            if args.len() != 2 {
                return Err(RuntimeError::TypeError("log_message expects 2 arguments".to_string()));
            }
            let level = match &args[0] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("log_message first argument must be string".to_string())),
            };
            let message = match &args[1] {
                Value::String(s) => s,
                _ => return Err(RuntimeError::TypeError("log_message second argument must be string".to_string())),
            };
            match level.as_str() {
                "info" => logger::info(message),
                "warn" => logger::warn(message),
                "error" => logger::error(message),
                "debug" => logger::debug(message),
                _ => logger::info(&format!("[{}] {}", level, message)),
            }
            Ok(Value::None)
        }
        "load_native" => {
            if args.len() != 1 {
                return Err(RuntimeError::TypeError("load_native expects 1 argument".to_string()));
            }
            let name = match &args[0] {
                Value::String(s) => s.clone(),
                _ => return Err(RuntimeError::TypeError("load_native expects string argument".to_string())),
            };
            self.load_native_module(&name)?;
            Ok(Value::None)
        }

        // ==================== ПОЛЬЗОВАТЕЛЬСКИЕ ФУНКЦИИ ====================
        _ => {
            // Сначала ищем в обычных (пользовательских) функциях
            if let Some(proc) = self.functions.get(name) {
                let proc = proc.clone();
                if proc.params.len() != args.len() {
                    return Err(RuntimeError::TypeError(format!(
                        "Function {} expects {} arguments, got {}",
                        name, proc.params.len(), args.len()
                    )));
                }
                let mut local_env = Environment::new(self.globals.clone());
                for (param, arg) in proc.params.iter().zip(args) {
                    local_env.set(param.clone(), arg, false)?;
                }
                return match self.execute_block(&proc.body, &mut local_env) {
                    Ok(()) => Ok(Value::None),
                    Err(RuntimeError::Return(val)) => Ok(val),
                    Err(e) => {
                        logger::error(&format!("Error in user function {}: {}", name, e));
                        Err(e)
                    }
                };
            }

            // Если не нашли в пользовательских, ищем в нативных модулях
            for (mod_name, module) in &self.native_modules {
                if let Some(info) = module.functions.get(name) {
                    if args.len() != info.arg_count as usize {
                        return Err(RuntimeError::TypeError(format!(
                            "Native function {} expects {} arguments, got {}",
                            name, info.arg_count, args.len()
                        )));
                    }

                    // Диспетчеризация по количеству аргументов с поддержкой возврата i64
                    match info.arg_count {
                        0 => {
                            type FnType = extern "C" fn() -> i64;
                            let func: FnType = unsafe { std::mem::transmute(info.func_ptr) };
                            let result = func();
                            return Ok(Value::Whole(result));
                        }
                        1 => {
                            // Пока поддерживаем только строковый аргумент
                            match &args[0] {
                                Value::String(s) => {
                                    let c_string = CString::new(s.as_str())
                                        .map_err(|_| RuntimeError::TypeError("String contains null byte".to_string()))?;
                                    type FnType = extern "C" fn(*const std::os::raw::c_char) -> i64;
                                    let func: FnType = unsafe { std::mem::transmute(info.func_ptr) };
                                    let result = func(c_string.as_ptr());
                                    return Ok(Value::Whole(result));
                                }
                                _ => {
                                    return Err(RuntimeError::TypeError(
                                        "Native function with one argument expects a string".to_string()
                                    ));
                                }
                            }
                        }
                        2 => {
    // Поддержка разных сигнатур: (строка, строка)
    if let (Value::String(s1), Value::String(s2)) = (&args[0], &args[1]) {
        let c_string1 = CString::new(s1.as_str())
            .map_err(|_| RuntimeError::TypeError("String contains null byte".to_string()))?;
        let c_string2 = CString::new(s2.as_str())
            .map_err(|_| RuntimeError::TypeError("String contains null byte".to_string()))?;
        
        type FnType = extern "C" fn(*const std::os::raw::c_char, *const std::os::raw::c_char) -> i64;
        let func: FnType = unsafe { std::mem::transmute(info.func_ptr) };
        let result = func(c_string1.as_ptr(), c_string2.as_ptr());
        return Ok(Value::Whole(result));
    }
    // Если переданы два целых числа (для других функций)
    else if let (Value::Whole(a), Value::Whole(b)) = (&args[0], &args[1]) {
        type FnType = extern "C" fn(i64, i64) -> i64;
        let func: FnType = unsafe { std::mem::transmute(info.func_ptr) };
        let result = func(*a, *b);
        return Ok(Value::Whole(result));
    } else {
        return Err(RuntimeError::TypeError(
            "Native function with two arguments expects either two strings or two integers".to_string()
        ));
    }
                        }
                        _ => {
                            return Err(RuntimeError::TypeError(
                                "Native functions with more arguments not yet supported".to_string()
                            ));
                        }
                    }
                }
            }

            Err(RuntimeError::UndefinedFunction(name.to_string()))
        }
    }
}

    // ==================== FIELD/INDEX ACCESS ====================

    fn get_field(&self, obj: Value, field: &str) -> Result<Value, RuntimeError> {
        match obj {
            Value::Object { fields, .. } => {
                fields.get(field).cloned().ok_or_else(|| RuntimeError::UndefinedField(field.to_string()))
            }
            _ => Err(RuntimeError::TypeError("Cannot access field on non-object".to_string())),
        }
    }

    fn get_index(&self, obj: Value, index: Value) -> Result<Value, RuntimeError> {
        match (obj, index) {
            (Value::List(list), Value::Whole(i)) => {
                let i = i as usize;
                if i < list.len() {
                    Ok(list[i].clone())
                } else {
                    Err(RuntimeError::IndexOutOfBounds)
                }
            }
            (Value::List(_), _) => Err(RuntimeError::TypeError("List index must be integer".to_string())),
            _ => Err(RuntimeError::TypeError("Cannot index non-list".to_string())),
        }
    }

    fn create_object(&self, name: &str) -> Result<Value, RuntimeError> {
        let form = self.forms.get(name).ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))?;
        let mut fields = HashMap::new();
        for (field_name, info) in &form.fields {
            let default = info.default.clone().unwrap_or(Value::None);
            fields.insert(field_name.clone(), default);
        }
        Ok(Value::Object { form_name: name.to_string(), fields })
    }

    fn assign(&mut self, target: &Expr, value: Value, env: &mut Environment) -> Result<(), RuntimeError> {
        match target {
            Expr::Variable(name) => {
                env.set(name.clone(), value, true)?;
                Ok(())
            }
            Expr::Field { object, field } => {
                let obj_val = self.eval_expr(object, env)?;
                match obj_val {
                    Value::Object { form_name, mut fields } => {
                        if fields.contains_key(field) {
                            fields.insert(field.clone(), value);
                            let new_obj = Value::Object { form_name, fields };
                            self.assign(object, new_obj, env)?;
                            Ok(())
                        } else {
                            Err(RuntimeError::UndefinedField(field.clone()))
                        }
                    }
                    _ => Err(RuntimeError::TypeError("Cannot assign to field of non-object".to_string())),
                }
            }
            Expr::Index { object, index } => {
                let idx_val = self.eval_expr(index, env)?;
                let obj_val = self.eval_expr(object, env)?;
                match (obj_val, idx_val) {
                    (Value::List(mut list), Value::Whole(i)) => {
                        let i = i as usize;
                        if i < list.len() {
                            list[i] = value;
                            let new_obj = Value::List(list);
                            self.assign(object, new_obj, env)?;
                            Ok(())
                        } else {
                            Err(RuntimeError::IndexOutOfBounds)
                        }
                    }
                    (Value::List(_), _) => Err(RuntimeError::TypeError("List index must be integer".to_string())),
                    _ => Err(RuntimeError::TypeError("Cannot index non-list".to_string())),
                }
            }
            _ => Err(RuntimeError::TypeError("Invalid assignment target".to_string())),
        }
    }
}