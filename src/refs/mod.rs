mod parser;

use crate::types::{Mapping, Value};
use anyhow::{anyhow, Result};
use nom::error::{convert_error, VerboseError};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
/// Represents a parsed Reclass reference
pub enum Token {
    /// A parsed input string which doesn't contain any Reclass references
    Literal(String),
    /// A parsed reference
    Ref(Vec<Token>),
    /// A parsed input string which is composed of one or more references, potentially with
    /// interspersed non-reference sections.
    Combined(Vec<Token>),
}

#[derive(Clone, Debug, Default)]
pub struct ResolveState {
    /// Reference paths which we've seen during reference resolution
    seen_paths: HashSet<String>,
    /// Recursion depth of the resolution (in number of calls to Token::resolve() for Token::Ref
    /// objects).
    depth: usize,
    /// Parameters key which we're currently processing.
    current_keys: Vec<String>,
}

impl ResolveState {
    /// Pushes the provided index into the last element of current_keys as `[idx]`.
    pub(crate) fn push_list_index(&mut self, idx: usize) -> std::fmt::Result {
        use std::fmt::Write;
        let mut kcount = self.current_keys.len();
        if kcount == 0 {
            self.current_keys.push(String::new());
            kcount = 1;
        }
        write!(&mut self.current_keys[kcount - 1], "[{idx}]")
    }

    /// Pushes mapping key into the `current_keys` list. If possible, the provided value is
    /// formatted with `raw_string()`. Additionally, unprocessed `String` values are pushed as-is.
    /// This function will return an error when it's called with a `Value::ValueList`.
    pub(crate) fn push_mapping_key(&mut self, key: &Value) -> Result<()> {
        let kstr = match key.raw_string() {
            Ok(s) => s,
            Err(_) => match key {
                Value::String(s) => Ok(s.clone()),
                Value::ValueList(_) => Err(anyhow!("Unable to render ValueList as key segment")),
                _ => unreachable!("raw_string() implemented for other Value variants"),
            }?,
        };
        self.current_keys.push(kstr);
        Ok(())
    }

    /// Formats paths that have been seen as a comma-separated list.
    fn seen_paths_list(&self) -> String {
        let mut paths = self
            .seen_paths
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<String>>();
        paths.sort();
        paths.join(", ")
    }

    /// Formats current key by joining the segements with dots.
    fn current_key(&self) -> String {
        self.current_keys.join(".")
    }

    /// Renders a suitable error when a reference loop is detected.
    fn render_reference_loop_error(&self) -> anyhow::Error {
        let paths = self.seen_paths_list();
        anyhow!("Detected reference loop with reference paths [{paths}].")
    }

    /// Renders a suitable error when the reference lookup depth limit is exceeded.
    fn render_recursion_depth_error(&self) -> anyhow::Error {
        let current_key = self.current_key();
        let paths = self.seen_paths_list();
        anyhow!(
            "Token resolution exceeded recursion depth of {RESOLVE_MAX_DEPTH} for \
            parameter '{current_key}'. We've seen the following reference paths: [{paths}].",
        )
    }

    /// Renders a suitable error when the reference lookup fails due to a missing key.
    fn render_missing_key_error(&self, path: &str, key: &str) -> anyhow::Error {
        let current_key = self.current_key();
        let r = format!("${{{path}}}");
        anyhow!(
            "lookup error for reference '{r}' in parameter '{current_key}': key '{key}' not found"
        )
    }

    /// Renders a lookup error with the given message
    fn render_lookup_error(&self, path: &str, key: &str, msg: &str) -> anyhow::Error {
        let current_key = self.current_key();
        let r = format!("${{{path}}}");
        anyhow!(
            "While looking up key '{key}' in reference '{r}' for parameter '{current_key}': {msg}"
        )
    }

    pub(crate) fn render_flattening_error(&self, msg: &str) -> anyhow::Error {
        let current_key = self.current_key();
        anyhow!("In {current_key}: {msg}")
    }
}

/// Maximum allowed recursion depth for Token::resolve(). We're fairly conservative with the value,
/// since it's rather unlikely that a well-formed inventory will have any references that are
/// nested deeper than 64.
const RESOLVE_MAX_DEPTH: usize = 64;

impl Token {
    /// Parses an arbitrary string into a `Token`. Returns None, if the string doesn't contain any
    /// opening reference markers.
    pub fn parse(s: &str) -> Result<Option<Self>> {
        if !s.contains("${") && !s.contains("$[") {
            // return None for strings which don't contain any references
            return Ok(None);
        }

        let token = parse_ref(s).map_err(|e| anyhow!("Error while parsing ref: {}", e.summary))?;
        Ok(Some(token))
    }

    #[cfg(test)]
    pub fn literal_from_str(l: &str) -> Self {
        Self::Literal(l.to_string())
    }

    /// Returns true if the Token is a `Token::Ref`
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(_))
    }

    /// Returns true if the Token is a `Token::Literal`
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// Renders the token into an arbitrary Value or a string. Reference values are looked up in
    /// the Mapping provided through parameter `params`.
    ///
    /// The heavy lifting is done by `Token::resolve()`.
    pub fn render(&self, params: &Mapping, state: &mut ResolveState) -> Result<Value> {
        if self.is_ref() {
            // handle value refs (i.e. refs where the full value of the key is replaced)
            // We call `interpolate()` after `resolve()` to ensure that we fully interpolate all
            // references if the result of `resolve()` is a complex Value (Mapping or Sequence).
            self.resolve(params, state)?.interpolate(params, state)
        } else {
            Ok(Value::Literal(self.resolve(params, state)?.raw_string()?))
        }
    }

    /// Resolves the Token into a [`Value`]. References are looked up in the provided `params`
    /// Mapping.
    fn resolve(&self, params: &Mapping, state: &mut ResolveState) -> Result<Value> {
        match self {
            // Literal tokens can be directly turned into `Value::Literal`
            Self::Literal(s) => Ok(Value::Literal(s.to_string())),
            Self::Combined(tokens) => {
                let res = interpolate_token_slice(tokens, params, state)?;
                // The result of `interpolate_token_slice()` for a `Token::Combined()` can't result
                // in more unresolved refs since we iterate over each segment until there's no
                // Value::String() left, so we return a Value::Literal().
                Ok(Value::Literal(res))
            }
            // For Ref tokens, we first resolve nested references in the Ref path by calling
            // `interpolate_token_slice()`. Then we split the resolved reference path into segments
            // on `:` and iteratively look up each segment in the provided `params` Mapping.
            Self::Ref(parts) => {
                // We track the number of calls to `Token::resolve()` for Token::Ref that the
                // current `state` has seen in state.depth.
                state.depth += 1;
                if state.depth > RESOLVE_MAX_DEPTH {
                    // If we've called `Token::resolve()` more than RESOLVE_MAX_DEPTH (64) times
                    // recursively, it's likely that there's still an edge case where we don't
                    // detect a reference loop with the current reference path tracking
                    // implementation. We abort at a recursion depth of 64, since it's quite
                    // unlikely that there's a legitimate case where we have a recursion depth of
                    // 64 when resolving references for a well formed inventory.
                    return Err(state.render_recursion_depth_error());
                }
                // Construct flattened ref path by resolving any potential nested references in the
                // Ref's Vec<Token>.
                let path = interpolate_token_slice(parts, params, state)?;

                if state.seen_paths.contains(&path) {
                    // we've already seen this reference, so we know there's a loop, and can abort
                    // resolution.
                    return Err(state.render_reference_loop_error());
                }
                state.seen_paths.insert(path.clone());

                // generate iterator containing flattened reference path segments
                let mut refpath_iter = path.split(':');
                // we handle the first element separately, so we can establish a local mutable
                // variable which we can update during the walk of the parameters Mapping.
                let k0 = refpath_iter.next().unwrap();
                // v is the value which we update to point to the next value as we recursively
                // descend into the params Mapping
                let mut v = params
                    .get(&k0.into())
                    .ok_or_else(|| state.render_missing_key_error(&path, k0))?;

                // newv is used to hold temporary Values generated by interpolating v
                let mut newv;

                // traversed is used to keep track of the path segments we've already processed
                let mut traversed = vec![k0.to_string()];

                // descend into the params Mapping, looking up each segment of the reference path
                // sequentially, updating `v` and `newv` as we go.
                for key in refpath_iter {
                    // For lookups into Strings and ValueLists, we locally interpolate the
                    // value into `newv` so we don't have to worry about the order in which
                    // individual references are resolved, and always do value lookups on
                    // resolved references.
                    newv = interpolate_string_or_valuelist(v, params, state)?;
                    // at this point, newv should never be a Value::String or Value::ValueList.
                    debug_assert!(!newv.is_string() && !newv.is_value_list());
                    // Do lookup in interpolated value, return error if interpolated value doesn't
                    // support lookups.
                    match newv {
                        // trivial case: v is a Mapping, we can just lookup the next value based
                        // on `key`.
                        Value::Mapping(_) => {
                            v = newv
                                .get(&key.into())
                                .ok_or_else(|| state.render_missing_key_error(&path, key))?;
                        }
                        // Sequence lookups aren't supported by Python Reclass. We may implement
                        // them in the future.
                        Value::Sequence(_) => {
                            return Err(state.render_lookup_error(
                                &path,
                                key,
                                "Sequence lookups aren't supported for Reclass references!",
                            ));
                        }
                        Value::String(_) | Value::ValueList(_) => unreachable!(
                            "We should have rendered Value::String and Value::ValueList into some other variant"
                        ),
                        // A lookup into any other Value variant is an error
                        _ => {
                            return Err(state.render_lookup_error(
                                &path,
                                key,
                                &format!(
                                    "Can't continue lookup, {} is a {}",
                                    traversed.join(":"),
                                    newv.variant()
                                ),
                            ));
                        }
                    }
                    // add current segment to list of traversed segments after lookup is completed.
                    traversed.push(key.to_string());
                }

                let mut v = v.clone();
                // Finally, we iteratively interpolate `v` while it's a `Value::String()` or
                // `Value::ValueList`. This ensures that the returned Value will never contain
                // further references. Here, we want to continue tracking the state normally.
                while v.is_string() || v.is_value_list() {
                    v = v.interpolate(params, state)?;
                }
                Ok(v)
            }
        }
    }
}

impl std::fmt::Display for Token {
    /// Returns the string representation of the Token.
    ///
    /// `format!("{}", parse_ref(<input string>))` should result in the original input string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn flatten(f: &mut std::fmt::Formatter<'_>, ts: &[Token]) -> std::fmt::Result {
            for t in ts {
                write!(f, "{t}")?;
            }
            Ok(())
        }
        match self {
            Token::Literal(s) => {
                write!(f, "{}", s.replace('\\', r"\\").replace('$', r"\$"))
            }
            Token::Ref(ts) => {
                write!(f, "${{")?;
                flatten(f, ts)?;
                write!(f, "}}")
            }
            Token::Combined(ts) => flatten(f, ts),
        }
    }
}

/// Interpolate a `Vec<Token>`. Called from `Token::resolve()` for `Token::Combined` and
/// `Token::Ref` Vecs.
fn interpolate_token_slice(
    tokens: &[Token],
    params: &Mapping,
    state: &mut ResolveState,
) -> Result<String> {
    // Iterate through each element of the Vec, and call Token::resolve() on each element.
    // Additionally, we repeatedly call `Value::interpolate()` on the resolved value for each
    // element, as long as that Value is a `Value::String`.
    let mut res = String::new();
    for t in tokens {
        // Multiple separate refs in a combined or ref token can't form loops between each other.
        // Each individual ref can still be part of a loop, so we make a fresh copy of the input
        // state before resolving each element.
        let mut st = state.clone();
        let mut v = t.resolve(params, &mut st)?;
        while v.is_string() {
            v = v.interpolate(params, &mut st)?;
        }
        res.push_str(&v.raw_string()?);
    }
    Ok(res)
}

fn interpolate_string_or_valuelist(
    v: &Value,
    params: &Mapping,
    state: &mut ResolveState,
) -> Result<Value> {
    match v {
        // For Value::String, we can simply call `interpolate()` on the value.
        Value::String(_) => v.interpolate(params, state),
        // For Value::ValueList, we interpolate each layer, and flatten the resulting layers into a
        // single Value.  We don't use `interpolate()` here, since we only want to flatten the
        // resulting ValueList here.
        Value::ValueList(l) => {
            let mut i = vec![];
            for v in l {
                // When resolving references in ValueLists, we want to track state
                // separately for each layer, since reference loops can't be
                // stretched across layers.
                let mut st = state.clone();
                let v = if v.is_string() {
                    v.interpolate(params, &mut st)?
                } else {
                    v.clone()
                };
                i.push(v);
            }
            // Finally we flatten the resulting ValueList into a single Value.
            Value::ValueList(i).flattened(state)
        }
        // Do nothing for other types
        _ => Ok(v.clone()),
    }
}

#[derive(Debug)]
/// Wraps errors generated when trying to parse a string which may contain Reclass references
pub struct ParseError<'a> {
    /// Holds a reference to the original input string
    input: &'a str,
    /// Holds a `nom::error::VerboseError`, if parsing failed with a `nom::Err::Error` or `nom::Err::Failure`
    nom_err: Option<VerboseError<&'a str>>,
    /// Holds a human-readable summary of the parse error
    summary: String,
}

impl std::fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\n\n", self.summary)?;
        if let Some(e) = &self.nom_err {
            write!(f, "{}", convert_error(self.input, e.clone()))?;
        }
        Ok(())
    }
}

/// Parses the provided input string and emits a `Token` which represents any Reclass references
/// that were found in the input string.
///
/// The function currently doesn't allow customizing the Reclass reference start and end markers,
/// or the escape character. The default Reclass reference format `${...}` and the default escape
/// character '\' are recognized by the parser.
///
/// Users should use `Token::parse()` which converts the internal `ParseError` into a format
/// suitable to be handled with `anyhow::Result`.
fn parse_ref(input: &str) -> Result<Token, ParseError> {
    use self::parser::parse_ref;
    let (uncons, token) = parse_ref(input).map_err(|e| match e {
        nom::Err::Error(e) | nom::Err::Failure(e) => ParseError {
            input,
            nom_err: Some(e),
            summary: format!("Error parsing reference '{input}'"),
        },
        nom::Err::Incomplete(needed) => ParseError {
            input,
            nom_err: None,
            summary: format!("Failed to parse input, need more data: {needed:?}"),
        },
    })?;
    // uncons can't be empty, since we use the all_consuming combinator in the nom parser, so
    // trailing data will result in a parse error.
    if !uncons.is_empty() {
        unreachable!(
            "Trailing data '{}' occurred when parsing '{}', this shouldn't happen! Parsed result: {}",
            uncons, input, token
        );
    }
    Ok(token)
}

#[cfg(test)]
mod token_tests;

#[cfg(test)]
mod parse_ref_tests;

#[cfg(test)]
mod token_resolve_parse_tests;
