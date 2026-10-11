//! Server state: open documents, imported modules and symbol resolution.

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use lsp_types::Url;
use tasmc::core::{
    consts::INIT_ROUTINE,
    error::TasmError,
    flags::resolve_flag_int,
    structs::{Tasm, TasmValue},
};

use crate::index::{AliasDef, DocIndex, safe_to_value};

pub struct Document {
    pub text: String,
}

/// An imported module, as seen from the file importing it.
pub struct Module {
    pub path: PathBuf,
    pub index: DocIndex,
    /// Lexer errors of the module; non-empty means the compiler would fail with E0044.
    pub errors: Vec<TasmError>,
}

pub struct World {
    pub docs: HashMap<Url, Document>,
    pub stdlib: PathBuf,
    pub require_entry_point: bool,
    /// Closed documents whose diagnostics still need to be cleared.
    pub cleared: Vec<Url>,
    /// Modules read from disk, keyed by canonical path, with the mtime they were read at.
    cache: RefCell<HashMap<PathBuf, CachedModule>>,
}

type CachedModule = (Option<SystemTime>, Rc<Module>);

/// Result of looking up `module::ident` from a file.
pub enum External {
    /// No `IMPORT`/`IMPORTSTD` defines this module identifier.
    NotImported,
    /// The module is imported but its file can't be read.
    Unreadable(PathBuf),
    Routine {
        path: PathBuf,
        line: usize,
    },
    Alias {
        path: PathBuf,
        alias: AliasDef,
    },
    /// The module exists but defines no routine or alias with that name.
    Missing,
}

/// Where a resolved argument came from.
pub enum Origin {
    Literal,
    LocalAlias(AliasDef),
    Routine {
        name: String,
        line: usize,
    },
    ExternalRoutine {
        module: String,
        ident: String,
        path: PathBuf,
        line: usize,
    },
    ExternalAlias {
        module: String,
        alias: AliasDef,
        path: PathBuf,
    },
}

pub struct Resolved {
    /// The value the compiler would end up with. Routines become (placeholder) groups.
    pub value: TasmValue,
    pub origin: Origin,
}

/// An error tied to a TASM error code.
pub type CodedError = (i32, String);

impl World {
    pub fn new(stdlib: PathBuf, require_entry_point: bool) -> Self {
        Self {
            docs: HashMap::new(),
            stdlib,
            require_entry_point,
            cleared: vec![],
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn index_for(&self, uri: &Url, text: &str) -> DocIndex {
        DocIndex::build(text, uri.to_file_path().ok().as_deref(), &self.stdlib)
    }

    /// Loads an imported module, preferring the editor's (possibly unsaved) contents.
    pub fn module(&self, path: &Path) -> Option<Rc<Module>> {
        let canonical = path.canonicalize().ok()?;
        for (uri, doc) in &self.docs {
            if let Ok(p) = uri.to_file_path()
                && p.canonicalize().ok().as_ref() == Some(&canonical)
            {
                return Some(Rc::new(self.build_module(canonical, &doc.text)));
            }
        }

        let mtime = fs::metadata(&canonical).and_then(|m| m.modified()).ok();
        if let Some((cached_mtime, module)) = self.cache.borrow().get(&canonical)
            && *cached_mtime == mtime
        {
            return Some(module.clone());
        }
        let text = fs::read_to_string(&canonical).ok()?;
        let module = Rc::new(self.build_module(canonical.clone(), &text));
        self.cache
            .borrow_mut()
            .insert(canonical, (mtime, module.clone()));
        Some(module)
    }

    fn build_module(&self, path: PathBuf, text: &str) -> Module {
        let index = DocIndex::build(text, Some(&path), &self.stdlib);
        let errors = match lex(text, &path.to_string_lossy(), false) {
            Ok(tasm) => tasm.errors,
            Err(e) => vec![e],
        };
        Module {
            path,
            index,
            errors,
        }
    }

    pub fn lookup(&self, index: &DocIndex, module: &str, ident: &str) -> External {
        let Some(import) = index.import(module) else {
            return External::NotImported;
        };
        let Some(path) = &import.path else {
            return External::Unreadable(PathBuf::from(&import.raw));
        };
        let Some(m) = self.module(path) else {
            return External::Unreadable(path.clone());
        };
        if let Some(r) = m.index.routine(ident)
            && r.name != INIT_ROUTINE
        {
            return External::Routine {
                path: m.path.clone(),
                line: r.line,
            };
        }
        if let Some(a) = m.index.alias(ident) {
            return External::Alias {
                path: m.path.clone(),
                alias: a.clone(),
            };
        }
        External::Missing
    }

    /// Follows the imports that are actually used (the linker only loads those) looking for a
    /// cycle. Returns the chain of module identifiers, e.g. `["a", "b", "a"]`.
    pub fn import_cycle(&self, file: &Path, index: &DocIndex, module: &str) -> Option<Vec<String>> {
        fn visit(
            world: &World,
            index: &DocIndex,
            stack: &mut Vec<(PathBuf, String)>,
            seen: &mut Vec<PathBuf>,
        ) -> Option<Vec<String>> {
            for import in &index.imports {
                if !index.referenced_modules.contains(&import.module) {
                    continue;
                }
                let Some(m) = import.path.as_ref().and_then(|p| world.module(p)) else {
                    continue;
                };
                if let Some(pos) = stack.iter().position(|(p, _)| *p == m.path) {
                    let mut chain: Vec<String> =
                        stack[pos + 1..].iter().map(|(_, n)| n.clone()).collect();
                    chain.insert(0, stack[pos].1.clone());
                    chain.push(import.module.clone());
                    return Some(chain);
                }
                if seen.contains(&m.path) {
                    continue;
                }
                seen.push(m.path.clone());
                stack.push((m.path.clone(), import.module.clone()));
                let found = visit(world, &m.index, stack, seen);
                stack.pop();
                if found.is_some() {
                    return found;
                }
            }
            None
        }

        let import = index.import(module)?;
        if !index.referenced_modules.contains(&import.module) {
            return None;
        }
        let this = file.canonicalize().ok()?;
        let stem = this.file_stem()?.to_string_lossy().into_owned();
        let m = self.module(import.path.as_ref()?)?;
        let mut stack = vec![(this, stem), (m.path.clone(), import.module.clone())];
        let mut seen = vec![];
        visit(self, &m.index, &mut stack, &mut seen)
    }

    /// Resolves an instruction argument the way the lexer and linker would:
    /// local alias substitution, value parsing, then routine and `module::symbol` lookup.
    pub fn resolve(&self, index: &DocIndex, raw: &str) -> Result<Resolved, CodedError> {
        let raw = raw.trim();
        let alias = index.alias(raw).cloned();
        let text = alias.as_ref().map(|a| a.value.as_str()).unwrap_or(raw);
        let value = safe_to_value(text)?;
        let local_origin = |o: Origin| match &alias {
            Some(a) => Origin::LocalAlias(a.clone()),
            None => o,
        };

        match value {
            TasmValue::String(s) if index.routine(&s).is_some() => {
                if s == INIT_ROUTINE {
                    return Err((33, "Cannot spawn init routine.".into()));
                }
                let line = index.routine(&s).unwrap().line;
                Ok(Resolved {
                    value: TasmValue::Group(1),
                    origin: local_origin(Origin::Routine { name: s, line }),
                })
            }
            TasmValue::RoutineRef(r) => {
                let module = r.root.clone().unwrap_or_default();
                match self.lookup(index, &module, &r.ident) {
                    External::Routine { path, line } => Ok(Resolved {
                        value: TasmValue::Group(1),
                        origin: local_origin(Origin::ExternalRoutine {
                            module,
                            ident: r.ident,
                            path,
                            line,
                        }),
                    }),
                    External::Alias { path, alias: ext } => {
                        // the linker parses external alias values without further resolution
                        let value = safe_to_value(&ext.value)?;
                        Ok(Resolved {
                            value,
                            origin: local_origin(Origin::ExternalAlias {
                                module,
                                alias: ext,
                                path,
                            }),
                        })
                    }
                    other => Err(external_error(other, &module, &r.ident)),
                }
            }
            value => Ok(Resolved {
                value,
                origin: local_origin(Origin::Literal),
            }),
        }
    }

    /// Resolves a dict flag entry (`remap: { key = value }`) to an integer, like
    /// `FlagValue::try_from` plus `post_link_processing` do.
    pub fn resolve_dict_entry(&self, index: &DocIndex, raw: &str) -> Result<i16, CodedError> {
        let raw = raw.trim();
        if let Some((module, ident)) = raw.split_once("::") {
            return match self.lookup(index, module, ident) {
                External::Routine { .. } => Ok(1),
                External::Alias { path, alias } => resolve_flag_int(&alias.value)
                    .or_else(|| {
                        self.module(&path)
                            .and_then(|m| m.index.routine(&alias.value).map(|_| 1))
                    })
                    .ok_or((41, "Invalid external alias dict value".into())),
                External::NotImported => Err((39, format!("Unable to find dependency {module}"))),
                External::Unreadable(p) => Err((
                    43,
                    format!(
                        "Unable to resolve path for module {}",
                        crate::text::display_path(&p)
                    ),
                )),
                External::Missing => Err((
                    40,
                    format!("Could not find external symbol {module}::{ident}"),
                )),
            };
        }
        if index.routine(raw).is_some() {
            return Ok(1);
        }
        resolve_flag_int(raw)
            .or_else(|| index.alias(raw).and_then(|a| resolve_flag_int(&a.value)))
            .ok_or((32, format!("{raw} is not an integer, ID, alias or routine")))
    }
}

pub fn external_error(e: External, module: &str, ident: &str) -> CodedError {
    match e {
        External::NotImported => (
            38,
            format!("Unable to find module {module}. Import it in `_init` first."),
        ),
        External::Unreadable(p) => (
            43,
            format!(
                "Unable to resolve path for module {}",
                crate::text::display_path(&p)
            ),
        ),
        External::Missing => (35, format!("Unable to find symbol {module}::{ident}")),
        External::Routine { .. } | External::Alias { .. } => (-1, String::new()),
    }
}

/// Runs the compiler's lexer on `text` and returns its state including all errors
/// (`lexer::parse_file` discards the state when there are errors).
pub fn lex(text: &str, fname: &str, require_entry_point: bool) -> Result<Tasm, TasmError> {
    let text = text.to_string();
    let fname = fname.to_string();
    catch_unwind(AssertUnwindSafe(move || {
        let mut tasm = Tasm {
            lines: text
                .replace('\t', " ")
                .lines()
                .map(|l| l.split(';').next().unwrap().trim_end().to_string())
                .collect(),
            fname,
            ..Default::default()
        };
        tasm.parse(1, !require_entry_point);
        tasm
    }))
    .map_err(|_| TasmError {
        etype: tasmc::core::error::TasmErrorType::InvalidInstruction,
        file: String::new(),
        routine: String::new(),
        line: 0,
        details: "The compiler crashed while parsing this file.".into(),
        errcode: -1,
    })
}
