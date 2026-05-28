use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use std::time::Duration;

mod builtins;
mod control;
mod includes;
mod macros;

use crate::source::SourceMap;
pub(crate) use control::preprocess;
pub(crate) use control::preprocess_with_map;

const MAX_INCLUDE_DEPTH: usize = 32;
const MAX_PREPROC_WHILE_ITERATIONS: usize = 10_000;
const MAX_PREPROC_CALL_DEPTH: usize = 32;
const MAX_PREPROC_MACRO_EXPANSION_BYTES: usize = 64 * 1024;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
const URL_INCLUDE_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
const URL_INCLUDE_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeTarget {
    path: PathBuf,
    tag: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    pub include_root: Option<PathBuf>,
    /// When true, `!include https://...`, `!includeurl`, and `file://` URL
    /// targets fetch or read content. Defaults to false to avoid surprise IO.
    pub allow_url_includes: bool,
    /// Variables pre-injected before preprocessing begins (e.g. from CLI `-D`
    /// flags). These behave like `!set VAR = VALUE` declarations at the top of
    /// the source and are accessible via `$VAR` in the diagram.
    pub inject_vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreprocessResult {
    pub source: String,
    pub source_map: SourceMap,
}

#[derive(Debug, Clone)]
struct ConditionalFrame {
    parent_active: bool,
    branch_taken: bool,
    current_active: bool,
    seen_else: bool,
}

#[derive(Debug, Clone)]
enum PreprocessDirective {
    Define(String),
    Undef(String),
    Include(String),
    IncludeOnce(String),
    IncludeMany(String),
    IncludeSub(String),
    IncludeUrl(String),
    Import(String),
    If(String),
    IfDef {
        name: String,
        negated: bool,
    },
    ElseIf(String),
    Else,
    EndIf,
    While(String),
    EndWhile,
    Foreach(String),
    EndFor,
    Break,
    Continue,
    Function,
    EndFunction,
    Procedure,
    EndProcedure,
    Assert(String),
    Log(String),
    DumpMemory(String),
    DynamicInvocation(String),
    JsonPreproc(String),
    DefineLong(String),
    EndDefineLong,
    Passthrough(String),
    Unsupported(String),
    NoOp,
    ProcedureCall {
        name: String,
        args: String,
    },
    VariableAssign {
        name: String,
        value: String,
        conditional: bool,
        scope: PreprocVariableScope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreprocCallableKind {
    Function,
    Procedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreprocVariableScope {
    Default,
    Local,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreprocLoopSignal {
    Break,
    Continue,
}

#[derive(Debug, Clone)]
struct PreprocParam {
    name: String,
    default: Option<String>,
}

#[derive(Debug, Clone)]
struct PreprocCallable {
    kind: PreprocCallableKind,
    params: Vec<PreprocParam>,
    body: Vec<String>,
}

#[derive(Debug, Clone)]
struct PreprocMacro {
    params: Vec<PreprocParam>,
    body: String,
}

#[derive(Debug, Clone, Default)]
struct PreprocState {
    defines: BTreeMap<String, String>,
    macros: BTreeMap<String, PreprocMacro>,
    vars: BTreeMap<String, String>,
    callables: BTreeMap<String, PreprocCallable>,
    // Counters used by the deterministic builtins `%false_then_true` /
    // `%true_then_false`. PlantUML semantics use a per-callsite latch — we
    // key by the argument value so identical sources produce identical
    // AST/render bytes. Interior mutability lets us update from
    // `expand_function_invocations` which only borrows `&PreprocState`.
    false_then_true_counts: RefCell<BTreeMap<String, u64>>,
    true_then_false_counts: RefCell<BTreeMap<String, u64>>,
    global_assigns: RefCell<BTreeSet<String>>,
    loop_depth: usize,
    loop_signal: Option<PreprocLoopSignal>,
}
