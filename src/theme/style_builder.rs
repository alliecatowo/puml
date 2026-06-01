//! Phase B of #1404 — cascade resolver for `<style>` block rules.
//!
//! `StyleBuilder` accumulates [`StyleRule`] values (from parsed `<style>` blocks)
//! and resolves an effective per-property result for a given [`StyleQuery`].
//!
//! # Specificity scoring (matches `StyleParser.java`)
//!
//! | Match kind                     | Points |
//! |-------------------------------|--------|
//! | Each matched `Tag` segment    |  +100  |
//! | Each matched `Stereotype`     | +1000  |
//! | Wildcard match                |    +1  |
//! | Tie-break: later rule wins    | `source_order` |
//!
//! Merge order (lowest → highest priority): each rule is scored and then
//! properties from higher-scoring rules overwrite lower-scoring ones.
//! When two rules have equal specificity the rule with the larger `source_order`
//! (i.e. the one appearing *later* in the document) wins — matching the upstream
//! `AutomaticCounterBasic` behaviour.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::style::{PName, SName, SelectorSegment, StyleRule, StyleValue};

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

/// A query for which to resolve an effective style.
///
/// Build one per element: list the SName tags from outermost to innermost
/// (e.g. `[ClassDiagram, Class_]`) and the element's stereotype names
/// (lower-cased, without `<<`/`>>`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StyleQuery {
    /// Ancestor tag chain from diagram-root to element (outermost first).
    pub tags: Vec<SName>,
    /// Stereotype identifiers on the element (lower-cased).
    pub stereotypes: BTreeSet<String>,
    /// Mindmap/WBS depth, if applicable.
    pub depth: Option<u32>,
}

impl StyleQuery {
    /// Construct a simple tag-only query (no stereotypes, no depth).
    pub fn tags(tags: impl IntoIterator<Item = SName>) -> Self {
        Self {
            tags: tags.into_iter().collect(),
            stereotypes: BTreeSet::new(),
            depth: None,
        }
    }

    /// Add a stereotype to this query.
    pub fn with_stereotype(mut self, name: impl Into<String>) -> Self {
        self.stereotypes.insert(name.into().to_ascii_lowercase());
        self
    }
}

/// The fully-resolved style for one element after cascade.
#[derive(Debug, Clone, Default)]
pub struct EffectiveStyle {
    /// Merged properties (highest-specificity rule wins per property).
    pub properties: BTreeMap<PName, StyleValue>,
}

impl EffectiveStyle {
    /// Look up a colour property.  Returns `None` when not set by any rule.
    pub fn color(&self, name: PName) -> Option<&str> {
        self.properties.get(&name).and_then(|v| match v {
            StyleValue::Color(s) => Some(s.as_str()),
            StyleValue::Keyword(s) => Some(s.as_str()),
            StyleValue::Raw(s) => Some(s.as_str()),
            StyleValue::Number(_) => None,
        })
    }
}

// ---------------------------------------------------------------------------
// StyleBuilder
// ---------------------------------------------------------------------------

/// Accumulator + resolver for `<style>` block rules.
///
/// Call [`push`] for each [`StyleRule`] in the document (in source order) and
/// then [`lookup`] per element to obtain the merged [`EffectiveStyle`].
///
/// Results are memoized: repeated calls with the same `StyleQuery` are free
/// after the first resolution.
///
/// [`push`]: StyleBuilder::push
/// [`lookup`]: StyleBuilder::lookup
#[derive(Debug, Clone, Default)]
pub struct StyleBuilder {
    rules: Vec<StyleRule>,
    cache: BTreeMap<StyleQuery, EffectiveStyle>,
}

impl StyleBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when no rules have been pushed.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Append a rule.  Rules must be pushed in ascending `source_order`.
    pub fn push(&mut self, rule: StyleRule) {
        self.cache.clear(); // invalidate memoised results
        self.rules.push(rule);
    }

    /// Resolve the effective style for `query`.
    ///
    /// The result is memoised so subsequent calls with the same query are O(1).
    pub fn lookup(&mut self, query: &StyleQuery) -> &EffectiveStyle {
        // Avoid the borrow-checker dance: check the cache and compute only once.
        if !self.cache.contains_key(query) {
            let result = self.compute(query);
            self.cache.insert(query.clone(), result);
        }
        self.cache.get(query).expect("just inserted")
    }

    /// Resolve the effective style for `query` without memoization.
    ///
    /// Useful when the builder is behind a shared reference (`&StyleBuilder`).
    /// For hot paths where repeated queries occur, prefer [`lookup`] (which
    /// caches results) via `&mut StyleBuilder`.
    ///
    /// [`lookup`]: StyleBuilder::lookup
    pub fn resolve(&self, query: &StyleQuery) -> EffectiveStyle {
        self.compute(query)
    }

    /// Compute (without memoising) the effective style for a query.
    fn compute(&self, query: &StyleQuery) -> EffectiveStyle {
        // Collect (specificity, source_order, properties) for every matching rule.
        let mut candidates: Vec<(u32, u32, &StyleRule)> = Vec::new();

        for rule in &self.rules {
            if let Some(score) = rule_score(rule, query) {
                candidates.push((score, rule.source_order, rule));
            }
        }

        // Sort ascending so later (higher-priority) entries overwrite earlier ones.
        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0) // specificity asc
                .then(a.1.cmp(&b.1)) // source_order asc (later wins)
        });

        let mut result = EffectiveStyle::default();
        for (_, _, rule) in candidates {
            for (pname, value) in &rule.properties {
                result.properties.insert(*pname, value.clone());
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Specificity scoring helper
// ---------------------------------------------------------------------------

/// Return the specificity score for `rule` against `query`, or `None` if
/// the rule does not match the query at all.
///
/// A rule matches when every segment in its `selector_path` appears as a
/// subsequence in the query's tag list (wildcards match any tag at any
/// position).  Stereotypes on a selector segment must all appear in
/// `query.stereotypes`.
///
/// Scoring:
/// - `+100` per matched Tag segment
/// - `+1000` per matched Stereotype segment
/// - `+1` per Wildcard segment (so wildcards always lose to concrete matches)
/// - Depth pseudo: only matches if `query.depth` equals the depth value
fn rule_score(rule: &StyleRule, query: &StyleQuery) -> Option<u32> {
    // Each entry in `selector_path` is a `SelectorChain` (the segments at one
    // nesting level).  We must match each level as a subsequence of `query.tags`.
    //
    // Strategy: for each nesting level, try to consume one or more tags from the
    // query tag list.  We advance a cursor through `query.tags` greedily.
    let mut tag_cursor = 0;
    let mut score: u32 = 0;

    for chain in &rule.selector_path {
        // A SelectorChain at one nesting level may have multiple segments (when the
        // selector was something like `.Apache` inside a tag block).  We evaluate
        // each segment in the chain to see if it matches the element.
        let mut chain_matched = false;

        for segment in &chain.segments {
            match segment {
                SelectorSegment::Tag(sname) => {
                    // Advance cursor to find this tag anywhere in the remaining list.
                    let found = query.tags[tag_cursor..]
                        .iter()
                        .position(|t| t == sname)
                        .map(|offset| tag_cursor + offset);
                    if let Some(pos) = found {
                        tag_cursor = pos + 1;
                        score = score.saturating_add(100);
                        chain_matched = true;
                    } else {
                        // Required tag not found — rule does not match.
                        return None;
                    }
                }
                SelectorSegment::Stereotype(name) => {
                    let lower = name.to_ascii_lowercase();
                    if query.stereotypes.contains(&lower) {
                        score = score.saturating_add(1000);
                        chain_matched = true;
                    } else {
                        // Stereotype required but not present on element — no match.
                        return None;
                    }
                }
                SelectorSegment::Wildcard => {
                    // Wildcard: advance one tag position if available.
                    if tag_cursor < query.tags.len() {
                        tag_cursor += 1;
                        score = score.saturating_add(1);
                        chain_matched = true;
                    } else {
                        return None;
                    }
                }
                SelectorSegment::Depth(d) => {
                    // Depth pseudo-selector: match only if query carries a depth value.
                    if query.depth == Some(*d) {
                        chain_matched = true;
                    } else {
                        return None;
                    }
                }
                SelectorSegment::Unknown(_) => {
                    // Unknown selector: never matches (matches upstream null behaviour).
                    return None;
                }
            }
        }

        if !chain_matched && !chain.segments.is_empty() {
            return None;
        }
    }

    // A rule with no selector_path segments matches everything (wildcard rule).
    // Assign a minimal score so it loses to any concrete match.
    if rule.selector_path.is_empty() {
        score = score.saturating_add(1);
    }

    Some(score)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::style::{SelectorChain, StyleScheme};

    fn make_rule(
        tags: &[SName],
        stereotypes: &[&str],
        properties: &[(PName, &str)],
        source_order: u32,
    ) -> StyleRule {
        let selector_path: Vec<SelectorChain> = tags
            .iter()
            .map(|t| SelectorChain {
                segments: vec![SelectorSegment::Tag(*t)],
            })
            .chain(stereotypes.iter().map(|s| SelectorChain {
                segments: vec![SelectorSegment::Stereotype(s.to_string())],
            }))
            .collect();
        let props = properties
            .iter()
            .map(|(k, v)| (*k, StyleValue::Color(v.to_string())))
            .collect();
        StyleRule {
            selector_path,
            properties: props,
            unknown_properties: BTreeMap::new(),
            source_order,
            scheme: StyleScheme::Regular,
        }
    }

    #[test]
    fn simple_tag_match() {
        let mut builder = StyleBuilder::new();
        builder.push(make_rule(
            &[SName::ClassDiagram, SName::Class_],
            &[],
            &[(PName::BackgroundColor, "#dbeafe")],
            1,
        ));
        let query = StyleQuery::tags([SName::ClassDiagram, SName::Class_]);
        let result = builder.lookup(&query);
        assert_eq!(result.color(PName::BackgroundColor), Some("#dbeafe"));
    }

    #[test]
    fn stereotype_beats_plain() {
        let mut builder = StyleBuilder::new();
        // Plain class rule (lower priority)
        builder.push(make_rule(
            &[SName::Class_],
            &[],
            &[(PName::BackgroundColor, "#ff0000")],
            1,
        ));
        // Stereotyped rule (higher priority)
        builder.push(make_rule(
            &[SName::Class_],
            &["entity"],
            &[(PName::BackgroundColor, "#0000ff")],
            2,
        ));
        let query =
            StyleQuery::tags([SName::ClassDiagram, SName::Class_]).with_stereotype("entity");
        let result = builder.lookup(&query);
        assert_eq!(
            result.color(PName::BackgroundColor),
            Some("#0000ff"),
            "stereotyped rule must beat plain class rule"
        );
    }

    #[test]
    fn later_rule_wins_on_tie() {
        let mut builder = StyleBuilder::new();
        builder.push(make_rule(
            &[SName::Class_],
            &[],
            &[(PName::BackgroundColor, "#aaaaaa")],
            1,
        ));
        builder.push(make_rule(
            &[SName::Class_],
            &[],
            &[(PName::BackgroundColor, "#bbbbbb")],
            2,
        ));
        let query = StyleQuery::tags([SName::ClassDiagram, SName::Class_]);
        let result = builder.lookup(&query);
        assert_eq!(
            result.color(PName::BackgroundColor),
            Some("#bbbbbb"),
            "later rule must win on equal specificity"
        );
    }

    #[test]
    fn non_matching_stereotype_rule_skipped() {
        let mut builder = StyleBuilder::new();
        builder.push(make_rule(
            &[SName::Class_],
            &["service"],
            &[(PName::BackgroundColor, "#abcdef")],
            1,
        ));
        // Query with no stereotypes — the stereotype rule must not match.
        let query = StyleQuery::tags([SName::ClassDiagram, SName::Class_]);
        let result = builder.lookup(&query);
        assert!(
            result.color(PName::BackgroundColor).is_none(),
            "stereotype rule must not match element without that stereotype"
        );
    }

    #[test]
    fn empty_builder_returns_empty_style() {
        let mut builder = StyleBuilder::new();
        let query = StyleQuery::tags([SName::ClassDiagram, SName::Class_]);
        let result = builder.lookup(&query);
        assert!(result.properties.is_empty());
    }
}
