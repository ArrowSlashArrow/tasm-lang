use std::{collections::HashMap, f32::consts::E, fs, path::PathBuf};

use crate::{
    core::{
        HandlerFn,
        error::{TasmError, TasmErrorType},
        flags::{
            FlagValue::{self, ExternRefsDict},
            UnparsedDictFlagEntry,
        },
        print_errors,
        structs::{
            Instruction, Routine, SymbolPath, SymbolPathIdentifier, SymbolValue, Tasm, TasmValue,
            fits_arg_signature,
        },
    },
    instr::{INSTR_SPEC, placeholder_panic_fn},
    lexer, log,
};
use anyhow::{Result, anyhow};

use crate::Args;

pub fn compile_tasm_module(
    path: &PathBuf,
    args: &Args,
    start_using_this_group: i16,
    expect_entry_point: bool, // should always be false unless parsing from main
    silent: bool,
) -> Result<Tasm> {
    let file = match fs::read_to_string(&path) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("Couldn't read file {path:?}! {e}");
            log!(silent, "{msg}");
            return Err(anyhow!("{msg}"));
        }
    };

    let tasm = match lexer::parse_file(
        file,
        path.to_str().unwrap().to_string(),
        start_using_this_group,
        args.verbose_logs && !args.no_log,
        true,
        !expect_entry_point,
    ) {
        Ok(t) => t,
        Err(es) => {
            if !args.no_log {
                print_errors(es, &format!("Unable to compile {:?}", &path));
            }
            return Err(anyhow!("lexing failed"));
        }
    };

    Ok(tasm)
}

pub fn scan_referenced_symbols_mut<'a>(routine: &'a mut Routine) -> Vec<&'a mut TasmValue> {
    let mut symbols = vec![];
    for instr in routine.instructions.iter_mut() {
        // symbols: alias, routine
        // aliases are resolved* in lexing stage so we only care about routine idents
        let symbol_args = instr
            .args
            .iter_mut()
            .filter_map(|v| match v {
                // only deal with external symbols since they are treated in parse_modul
                TasmValue::RoutineRef(n) if n.is_external() => Some(v),
                _ => None,
            })
            .collect::<Vec<_>>();

        symbols.extend(symbol_args);
    }
    symbols
}

// tuple of (path, is in flag )
pub fn scan_referenced_symbols(
    routine: &Routine,
    ignore_local_symbols_in_args: bool,
) -> Vec<(SymbolPath, bool)> {
    let mut symbols = vec![];
    for instr in routine.instructions.iter() {
        // symbols: alias, routine
        // aliases are resolved* in lexing stage so we only care about routine idents
        let symbol_args = if ignore_local_symbols_in_args {
            instr
                .args
                .iter()
                .filter_map(|v| match v {
                    TasmValue::RoutineRef(r) if r.is_external() => Some((r.clone(), false)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        } else {
            instr
                .args
                .iter()
                .filter_map(|v| match v {
                    TasmValue::RoutineRef(r) => Some((r.clone(), false)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };

        symbols.extend(symbol_args);
        let symbol_flags = instr
            .flags
            .iter()
            .filter_map(|f| match &f.value {
                FlagValue::ExternRefsDict(e) => Some(
                    e.iter()
                        .flat_map(|(a, b)| [a, b])
                        .filter_map(|entry| match entry {
                            UnparsedDictFlagEntry::Int(_) => None,
                            UnparsedDictFlagEntry::Path(p) => Some((p.clone(), true)),
                        })
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        symbols.extend(symbol_flags);
    }
    symbols
}

pub fn parse_module(
    module_path: PathBuf,
    args: &Args,
    module_cache: &mut HashMap<PathBuf, (Tasm, bool, Vec<usize>)>,
    dependency_map: &mut HashMap<String, PathBuf>,
    start_using_this_group: &mut i16,
    has_entry_point: bool,
    silent: bool,
) -> Result<(Tasm, i16, Vec<usize>)> {
    log!(silent, "Parsing {}", module_path.to_str().unwrap());
    // lexed tasm output
    let prev_used_groups = *start_using_this_group;
    let mut module = compile_tasm_module(
        &module_path,
        args,
        prev_used_groups,
        has_entry_point,
        silent,
    )?;
    // offsetting the starting group like this allows us to parse the routines with no group collision to begin with
    // name collision is yet to be resolved
    *start_using_this_group += module.curr_group - prev_used_groups;

    // add to module cache as in-progress
    // use placeholder module here since it'll be returned at the end anyways
    // if it will cached, this entry should be overwritten with the actual module and marked as done.
    module_cache.insert(module_path.clone(), ({ module.clone() }, false, vec![]));

    log!(silent, "Got external symbols");

    let all_ext_symbols = module
        .routines
        .iter()
        .map(|routine| scan_referenced_symbols(routine, true))
        .flatten()
        .map(|v| v) // idx: replace value after done parsing (index into this list from index in all_ext_symbols)
        .collect::<Vec<_>>(); // any symbols whose index >= this do not get replaced
    // all_ext_symbols.retain(|s| s.is_external());

    if all_ext_symbols.is_empty() {
        // this would return here either if:
        // 1. it is the main module, where the main function will extract this value or
        // 2. it is a base dependency that is gonna get pushed to the module cache anyways
        return Ok((module, *start_using_this_group, vec![]));
    }

    let module_routines_clone = module.routines.clone();

    // collect mutable references here
    let mut ext_symbols = {
        module
            .routines
            .iter_mut()
            .map(|routine| scan_referenced_symbols_mut(routine))
            .flatten()
            .map(|v| v) // idx: replace value after done parsing (index into this list from index in all_ext_symbols)
            .collect::<Vec<_>>()
    };

    // ext_symbols.retain(|s| s.is_external_symbol());
    let ext_symbol_length = ext_symbols.len();
    let mut added_routines = vec![];

    let mut mut_symbols_array_idx = 0;
    // evaluate external symbols from here
    for (symbol, is_in_flag) in all_ext_symbols.iter() {
        // symbol may be either external or local
        // if local, we need to find the routine in this module and cache that routine.
        if !symbol.is_external() {
            println!("skipped an index");
            // this branch is only entered when a routine in this module is referenced in a flag
            let routine_ident = &symbol.ident;
            let routine = match module_routines_clone
                .iter()
                .find(|&r| r.ident == *routine_ident)
            {
                Some(r) => r.clone(),
                None => {
                    // TODO: error
                    return Err(anyhow!("[E00??] todo"));
                }
            };

            // also get this routine's dependencies
            let mut routine_cache = HashMap::new();
            let local_routines = &module_routines_clone;
            index_routine_deps(
                module_cache,
                &mut routine_cache,
                dependency_map,
                &routine,
                local_routines,
                // this module
                &module_path,
                &symbol.clone(),
                args,
                start_using_this_group,
                silent,
            )?;

            // aliases will be resolved in `post_processing`

            continue;
        }

        // if external, first cache the module that the symbol is located in
        let (dependency_path, dependency) = cache_module(
            &symbol,
            &module.imports[..],
            module.fname.clone(),
            &module_path,
            module_cache,
            dependency_map,
            args,
            start_using_this_group,
            silent,
        )?;
        dependency_map.insert(symbol.root.clone().unwrap(), dependency_path.clone());

        // get module from cache
        // here, it is guaranteed to be in the cache
        let (dependency_module, _is_done, dep_routine_idxes) =
            module_cache.get(&dependency_path).unwrap();

        // forward all dependencies of the parsed module to this one
        for &dep_routine_idx in dep_routine_idxes {
            // guaranteed to not fail due to being an immutable reference
            // also, we dont need to mangle the name because it's already mangled
            added_routines.push(dependency_module.routines[dep_routine_idx].clone());
        }
        // fetch routine and all its subsequent dependency routines.
        // element: (routine, is done)
        let mut routine_cache: HashMap<SymbolPathIdentifier, (Routine, bool)> = HashMap::new();
        let replace_with_value = match dependency_module
            .routines
            .iter()
            .find(|r| &r.ident == &symbol.ident)
        {
            Some(r) => {
                let external_routine = r.clone();
                // routines in the same module as that external routine
                // .clone() is necessary to use routine_cache mutably
                let local_routines = &dependency_module.routines.clone();

                index_routine_deps(
                    module_cache,
                    &mut routine_cache,
                    dependency_map,
                    &external_routine,
                    local_routines,
                    &dependency_path,
                    &symbol.clone(),
                    args,
                    start_using_this_group,
                    silent,
                )?;
                // replace existing symbol with this value
                SymbolValue::Group(routine_cache.get(&symbol.hashable()).unwrap().0.group)
            }
            None => {
                // could be referencing an alias
                match dependency_module
                    .defined_aliases
                    .iter()
                    .find(|r| r.0 == &symbol.ident)
                {
                    Some((_, alias_value)) => {
                        // since alias values cannot be cloned, we know that the alias must be defined in this module.
                        match TasmValue::to_value(&alias_value) {
                            Ok(v) => SymbolValue::TasmValue(Box::new(v)),
                            Err((etype, msg, code)) => {
                                return Err(anyhow!("[E{code:0>4}] {etype:?} {msg}"));
                            }
                        }
                    }
                    None => {
                        return Err(anyhow!(
                            "[E0035] Unable to find symbol {dependency}::{}",
                            symbol.ident
                        ));
                    }
                }
            }
        };

        // now, routine_cache has all the routines this one needs
        // mangle all names to prevent name collisions.

        if !is_in_flag {
            // this symbol is now resolved and we can turn it into a routine ident
            if let TasmValue::RoutineRef(r) = &mut ext_symbols[mut_symbols_array_idx] {
                r.assigned_value = replace_with_value;
            }
            mut_symbols_array_idx += 1;
        }
        for (dep_routine, _) in routine_cache.into_values() {
            let mangled_name = format!(
                "{}::{}",
                dependency_path.to_str().unwrap(),
                dep_routine.ident
            );
            // defer pushing to module's routines to avoid creating two simultaneous mutable references
            added_routines.push(dep_routine.ident(&mangled_name));
        }
    }

    let mut rtn_idx = module.routines.len();
    let mut deps_idxes = vec![];
    for routine in added_routines {
        deps_idxes.push(rtn_idx);
        module.routines.push(routine);
        rtn_idx += 1;
    }

    // deps_idxes tells the parent caller what other routines need to be added to its module as dependencies for routines in this module
    Ok((module, *start_using_this_group, deps_idxes))
}

pub fn cache_module(
    symbol: &SymbolPath,
    parent_module_imports: &[PathBuf],
    parent_module_fname: String,
    parent_module_path: &PathBuf,
    module_cache: &mut HashMap<PathBuf, (Tasm, bool, Vec<usize>)>,
    dependency_map: &mut HashMap<String, PathBuf>,
    args: &Args,
    start_using_this_group: &mut i16,
    silent: bool,
) -> Result<(PathBuf, String)> {
    // this is the module identifier, not its path. look this up in the current module.
    log!(silent, "called cache module with symbol {symbol:?}");
    let (dependency_path, dependency) =
        resolve_dependency_path(symbol, parent_module_imports, parent_module_path)?;

    match module_cache.get(&dependency_path) {
        Some((_mod, is_done, _deps)) => {
            if !is_done {
                return Err(anyhow!(
                    "Circular dependency detected between {} and {dependency}",
                    parent_module_fname
                ));
            }
        }
        None => {
            // add module to cache
            let module = parse_module(
                dependency_path.clone(),
                args,
                module_cache,
                dependency_map,
                start_using_this_group,
                false, // library files are not expected to have entry points
                silent,
            )?;

            // push filler entry to mark it as done
            module_cache.insert(dependency_path.clone(), (module.0, true, module.2));
        }
    }

    Ok((dependency_path, dependency))
}

/// Intended to be used for module paths specifically (i.e. the symbol should be external)
pub fn resolve_dependency_path(
    symbol: &SymbolPath,
    imports: &[PathBuf],
    parent_module_path: &PathBuf,
) -> Result<(PathBuf, String)> {
    let dependency = symbol.root.clone().unwrap();
    let dependency_path = match imports
        .iter()
        .find(|i| i.file_stem().is_some_and(|f| f == dependency.as_str()))
    {
        Some(path) => {
            let mut mpath = parent_module_path
                .parent()
                .unwrap_or(&PathBuf::new())
                .join(path);
            mpath.add_extension("tasm");
            match mpath.canonicalize() {
                Ok(m) => m,
                Err(e) => {
                    return Err(anyhow!(
                        "[E0043] Unable to resolve path for module {} in {}: {e}",
                        path.to_str().unwrap(),
                        parent_module_path.to_str().unwrap(),
                    ));
                }
            }
        }
        None => return Err(anyhow!("[E0038] Unable to find module {dependency}.")),
    };

    Ok((dependency_path, dependency))
}

// this function is for getting routine dependencies.
// actually replacing the routine path specs happens in parse_module,
// so a symbol here can refer to just a SymbolPath and also doesn't need to be mutable
pub fn index_routine_deps(
    module_cache: &mut HashMap<PathBuf, (Tasm, bool, Vec<usize>)>,
    routine_cache: &mut HashMap<SymbolPathIdentifier, (Routine, bool)>,
    dependency_map: &mut HashMap<String, PathBuf>,
    // routine that is being scanned
    routine: &Routine,
    local_routines: &[Routine],
    dependency_path: &PathBuf,
    symbol_path: &SymbolPath,
    args: &Args,
    start_using_this_group: &mut i16,
    silent: bool,
) -> Result<()> {
    // as a rust developer, using all of these .clone()s feels like driving a stake through my heart.
    // this langauge was made to be performant and memory safe, yet here i am sacrificing the former
    // in favour of the latter. this code will likely be refactored due to being too slow.
    log!(
        silent,
        "Scanning {dependency_path:?}::{} for external symbols.",
        routine.ident
    );
    // log this routine as "incomplete" in the cache
    routine_cache.insert(symbol_path.hashable(), (Routine::empty(), false));
    let symbols = scan_referenced_symbols(routine, false);
    if symbols.is_empty() {
        log!(silent, "Routine has no external symbols.");
        routine_cache.insert(symbol_path.hashable(), (routine.clone(), true));
        return Ok(());
    }

    let this_module = module_cache.get(dependency_path).unwrap();
    let fname = this_module.0.fname.clone();
    let imports = this_module.0.imports.clone();

    for (symbol, _) in symbols {
        log!(silent, "checking symbol {symbol:?}");
        if let Some(_) = routine_cache.get(&symbol.hashable()) {
            continue; // routine is confirmed to exist and we can assume that it will get parsed
        }

        if symbol.is_external() {
            // this branch assumes that symbol is a routine. it could also be an alias.
            let (routine_dependency_path, _) =
                resolve_dependency_path(&symbol, &imports, dependency_path)?;

            if let None = module_cache.get(&routine_dependency_path) {
                cache_module(
                    &symbol,
                    &imports[..],
                    fname.clone(),
                    &dependency_path,
                    module_cache,
                    dependency_map,
                    args,
                    start_using_this_group,
                    silent,
                )?;
            }

            let routine_dependency = module_cache.get(&routine_dependency_path).unwrap();
            let routines = routine_dependency.0.routines.clone();
            let aliases = routine_dependency.0.defined_aliases.clone();

            match routines.iter().find(|r| r.ident == symbol.ident) {
                Some(r) => {
                    index_routine_deps(
                        module_cache,
                        routine_cache,
                        dependency_map,
                        r,
                        &routines,
                        &routine_dependency_path,
                        &symbol,
                        args,
                        start_using_this_group,
                        silent,
                    )?;
                }
                None => {
                    if let Some(_) = aliases.iter().find(|(a, _)| **a == symbol.ident) {
                        // alias will be replaced in parse_module.
                        continue;
                    }
                    return Err(anyhow!(
                        "[E0036] Unable to find routine {}::{}",
                        symbol.root.unwrap_or("<Module>".to_string()),
                        symbol.ident
                    ));
                }
            }
        } else {
            match local_routines.iter().find(|r| r.ident == symbol.ident) {
                Some(r) => {
                    index_routine_deps(
                        module_cache,
                        routine_cache,
                        dependency_map,
                        r,
                        local_routines,
                        dependency_path,
                        &symbol,
                        args,
                        start_using_this_group,
                        silent,
                    )?;
                }
                None => {
                    log!(
                        silent,
                        "routines in this module: {:?}",
                        local_routines
                            .iter()
                            .map(|r| r.ident.clone())
                            .collect::<Vec<_>>()
                    );
                    return Err(anyhow!(
                        "[E0037] Unable to find routine {}::{}",
                        symbol.root.unwrap_or("<Module>".to_string()),
                        symbol.ident
                    ));
                }
            }
        }
    }

    routine_cache.insert(symbol_path.hashable(), (routine.clone(), true));
    Ok(())
}

pub fn post_link_processing(
    module: &mut Tasm,
    module_cache: HashMap<PathBuf, (Tasm, bool, Vec<usize>)>,
    dependency_map: HashMap<String, PathBuf>,
) -> Result<(), Vec<String>> {
    // these happen here:
    // * re-parse ExternRefDict flags with new aliases
    // * re-scan instructions with placeholder fn

    let mut errors = vec![];

    let resolve_unparsed_entry = |foreign_key: &UnparsedDictFlagEntry,
                                  errors: &mut Vec<String>|
     -> i16 {
        match foreign_key {
            UnparsedDictFlagEntry::Path(p) => {
                // this can be either a routine id or an alias
                // since an alias and routine can't have a name collision, order shouldn't matter here
                let dep_name = match &p.root {
                    Some(r) => r,
                    None => return p.get_group().unwrap(),
                };

                let dep_path = match dependency_map.get(dep_name) {
                    Some(dep) => dep,
                    None => {
                        errors.push(format!("[E0039] Unable to find dependency {dep_name}"));
                        return 0;
                    }
                };

                let symbol_name = p.ident.clone();
                let dep_module = module_cache.get(dep_path).unwrap();

                let resolve_int = |s: &str| -> Option<i16> {
                    s.parse::<i16>()
                        .ok()
                        .or_else(|| dep_module.0.routine_group_map.get(s).copied())
                };

                // (group, false) if ok, (0, true) if err
                let parse_int = |s: &str, invalid_value: &mut bool| -> i16 {
                    resolve_int(s)
                        .or_else(|| {
                            dep_module
                                .0
                                .defined_aliases
                                .get(s)
                                .and_then(|a| resolve_int(a))
                        })
                        .unwrap_or_else(|| {
                            *invalid_value = true;
                            0
                        })
                };

                match dep_module.0.routines.iter().find(|&r| r.ident == p.ident) {
                    Some(r) => {
                        // referencing a routine; get its group
                        r.group
                        // todo: also include the routine
                    }
                    None => {
                        match dep_module.0.defined_aliases.get(&symbol_name) {
                            Some(alias) => {
                                // accept alias only if it has a value that can be coercible to an int
                                let mut err = false;
                                let group = parse_int(alias, &mut err);
                                if err {
                                    errors
                                        .push(format!("[E0041] Invalid external alias dict value"));
                                    0
                                } else {
                                    group
                                }
                            }
                            None => {
                                errors.push(format!("[E0040] Could not find external symbol {dep_name}::{symbol_name}"));
                                0
                            }
                        }
                    }
                }
            }
            UnparsedDictFlagEntry::Int(i) => *i,
        }
    };

    // dedup copied routines
    module.routines.sort_by(|a, b| a.group.cmp(&b.group));
    module.routines.dedup_by(|a, b| a.group == b.group);

    for routine in &mut module.routines {
        'instrs: for instr in &mut routine.instructions {
            // replace old symbol references before searching for handler fn
            for arg in instr.args.iter_mut() {
                if let TasmValue::RoutineRef(symbol) = arg {
                    // all external symbols should have already been linked,
                    // therefore all of these symbols must have a known group

                    if !symbol.has_known_value() {
                        println!("hit weirdo branch");
                        // this only happens for **ONLY** external symbols in specific routines
                        // therefore we try to find the external symbol (which is either a routine or alias)
                        // before erroring

                        // first, find the symbol that is referenced
                        // these don't fail due to checks in index_routine_deps
                        let module = symbol.root.clone().unwrap();
                        let dep_module_path = dependency_map.get(&module).unwrap();
                        let dep_module = module_cache.get(dep_module_path).unwrap();

                        let routines = dep_module.0.routines.clone();
                        let aliases = dep_module.0.defined_aliases.clone();

                        let value = match routines.iter().find(|r| r.ident == symbol.ident) {
                            Some(r) => TasmValue::Group(r.group),
                            None => {
                                if let Some((_, alias_value)) =
                                    aliases.iter().find(|(a, _)| **a == symbol.ident)
                                {
                                    match TasmValue::to_value(&alias_value) {
                                        Ok(v) => v,
                                        Err((etype, msg, code)) => {
                                            errors.push(format!("[E{code:0>4}] {etype:?} {msg}"));
                                            continue 'instrs;
                                        }
                                    }
                                } else {
                                    errors.push(format!(
                                        "[E0043] Unable to find routine {module}::{}",
                                        symbol.ident
                                    ));
                                    continue 'instrs;
                                }
                            }
                        };
                        *arg = value;
                    } else {
                        *arg = symbol.get_value().unwrap();
                    }
                }
            }

            if instr.handler_fn as usize == placeholder_panic_fn as *const () as usize {
                // find the handler function

                match find_handler_for_instr(&instr, &module.fname, routine.ident.clone()) {
                    Ok(f) => instr.handler_fn = f,
                    Err(e) => {
                        errors.push(format!("{e}"));
                        continue;
                    }
                };
            }

            // replace all ExternRefsDict flags with Dicts since they are now linked and ready to be used in trigger constructors
            for flag in &mut instr.flags {
                if let ExternRefsDict(unparsed) = &flag.value {
                    flag.value = FlagValue::Dict(
                        unparsed
                            .iter()
                            .map(|(foreign_key, foreign_value)| {
                                (
                                    resolve_unparsed_entry(foreign_key, &mut errors),
                                    resolve_unparsed_entry(foreign_value, &mut errors),
                                )
                            })
                            .collect(),
                    );
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn find_handler_for_instr(
    instr: &Instruction,
    fname: &String,
    curr_routine: String,
) -> Result<HandlerFn, TasmError> {
    let handlers = INSTR_SPEC.get(&instr.ident).unwrap().1;
    let args = &instr.args;
    match handlers
        .iter()
        .find(|&(sig, _)| fits_arg_signature(args, sig))
        .map(|v| v.1)
    {
        Some(handler) => Ok(handler),
        None => {
            let argtypes = &args.iter().map(|a| a.get_type()).collect::<Vec<_>>();
            // otherwise, error
            Err(TasmError {
                etype: TasmErrorType::InvalidInstruction,
                file: fname.clone(),
                routine: curr_routine,
                line: instr.line_number,
                details: format!(
                    "Instruction {} has no argument handler for the argset {argtypes:?}",
                    instr.ident
                ),
                errcode: 57,
            })
        }
    }
}
