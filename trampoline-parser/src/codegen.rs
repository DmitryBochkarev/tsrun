//! Code generator for producing Rust parser code (scannerless)
//!
//! Generates:
//! - Work enum (auto-named variants)
//! - ParseResult enum
//! - Parser struct with trampoline loop
//! - Character-level matching functions

use crate::ir::{
    CharClass, CombRef, Combinator, CombinatorIndex, CompiledCapDef, CompiledChoiceDef,
    CompiledInfixOp, CompiledLookDef, CompiledLoopDef, CompiledMapDef, CompiledMemoDef,
    CompiledOptDef, CompiledPostfixOp, CompiledPrattDef, CompiledPrefixOp, CompiledRuleDef,
    CompiledSepByDef, CompiledSeqDef, CompiledSkipDef, CompiledTernaryOp, PatternInfo, PostfixOp,
};
use crate::CompiledGrammar;

/// Code generator
pub struct CodeGenerator<'a> {
    grammar: &'a CompiledGrammar,
    output: String,
    indent: usize,
    /// Indexed combinator tables (populated during indexing pass)
    index: CombinatorIndex,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(grammar: &'a CompiledGrammar) -> Self {
        Self {
            grammar,
            output: String::new(),
            indent: 0,
            index: CombinatorIndex::default(),
        }
    }

    /// Generate the complete parser code
    ///
    /// This generates a parser with ~45 fixed Work enum variants using indexed dispatch,
    /// instead of O(grammar size) variants. The implementation uses static tables for
    /// combinator definitions and table lookups for dispatch.
    pub fn generate(mut self) -> String {
        // Phase 1: Build combinator index
        self.index_combinators();

        // Phase 2: Emit code using indexed approach
        self.emit_header();
        self.emit_span();
        self.emit_parse_error();
        self.emit_parse_result_enum();
        self.emit_helpers();
        self.emit_builtin_helpers();
        self.emit_static_tables();
        self.emit_indexed_work_enum();
        self.emit_indexed_parser();
        self.output
    }

    /// Build the combinator index by traversing the grammar.
    /// This assigns IDs to each combinator and builds lookup tables.
    fn index_combinators(&mut self) {
        // First pass: register all rule names so we can resolve references
        for (i, rule) in self.grammar.rules.iter().enumerate() {
            self.index.rule_map.insert(rule.name.clone(), i as u16);
        }

        // Second pass: index each rule's combinator tree
        for rule in &self.grammar.rules {
            let entry = self.index_combinator(&rule.combinator);
            self.index.rules.push(CompiledRuleDef {
                name: rule.name.clone(),
                entry,
            });
        }
    }

    /// Recursively index a combinator, returning its CombRef
    fn index_combinator(&mut self, comb: &Combinator) -> CombRef {
        match comb {
            Combinator::Rule(name) => {
                // Look up the rule ID
                if let Some(&rule_id) = self.index.rule_map.get(name) {
                    CombRef::Rule(rule_id)
                } else {
                    // Rule not found - this should be caught by validation
                    panic!("Unknown rule reference: {}", name);
                }
            }

            Combinator::Sequence(items) => {
                let compiled_items: Vec<CombRef> =
                    items.iter().map(|c| self.index_combinator(c)).collect();
                let seq_id = self.index.sequences.len() as u16;
                self.index.sequences.push(CompiledSeqDef {
                    items: compiled_items,
                });
                CombRef::Seq(seq_id)
            }

            Combinator::Choice(alts) => {
                let compiled_alts: Vec<CombRef> =
                    alts.iter().map(|c| self.index_combinator(c)).collect();
                let choice_id = self.index.choices.len() as u16;
                self.index.choices.push(CompiledChoiceDef {
                    alts: compiled_alts,
                });
                CombRef::Choice(choice_id)
            }

            Combinator::ZeroOrMore(inner) => {
                let inner_ref = self.index_combinator(inner);
                let loop_id = self.index.zero_or_more.len() as u16;
                self.index
                    .zero_or_more
                    .push(CompiledLoopDef { item: inner_ref });
                CombRef::ZeroOrMore(loop_id)
            }

            Combinator::OneOrMore(inner) => {
                let inner_ref = self.index_combinator(inner);
                let loop_id = self.index.one_or_more.len() as u16;
                self.index
                    .one_or_more
                    .push(CompiledLoopDef { item: inner_ref });
                CombRef::OneOrMore(loop_id)
            }

            Combinator::Optional(inner) => {
                let inner_ref = self.index_combinator(inner);
                let opt_id = self.index.optionals.len() as u16;
                self.index
                    .optionals
                    .push(CompiledOptDef { inner: inner_ref });
                CombRef::Optional(opt_id)
            }

            Combinator::Literal(lit) => {
                // Intern the literal
                if let Some(&lit_id) = self.index.literal_map.get(lit) {
                    CombRef::Literal(lit_id)
                } else {
                    let lit_id = self.index.literals.len() as u16;
                    self.index.literal_map.insert(lit.clone(), lit_id);
                    self.index.literals.push(lit.clone());
                    CombRef::Literal(lit_id)
                }
            }

            Combinator::Char(c) => CombRef::Char(*c),

            Combinator::CharClass(class) => CombRef::CharClass(*class),

            Combinator::CharRange(from, to) => CombRef::CharRange(*from, *to),

            Combinator::AnyChar => CombRef::AnyChar,

            Combinator::Capture(inner) => {
                let inner_ref = self.index_combinator(inner);
                let cap_id = self.index.captures.len() as u16;
                self.index
                    .captures
                    .push(CompiledCapDef { inner: inner_ref });
                CombRef::Capture(cap_id)
            }

            Combinator::NotFollowedBy(inner) => {
                let inner_ref = self.index_combinator(inner);
                let look_id = self.index.not_followed_by.len() as u16;
                self.index
                    .not_followed_by
                    .push(CompiledLookDef { inner: inner_ref });
                CombRef::NotFollowedBy(look_id)
            }

            Combinator::FollowedBy(inner) => {
                let inner_ref = self.index_combinator(inner);
                let look_id = self.index.followed_by.len() as u16;
                self.index
                    .followed_by
                    .push(CompiledLookDef { inner: inner_ref });
                CombRef::FollowedBy(look_id)
            }

            Combinator::Skip(inner) => {
                let inner_ref = self.index_combinator(inner);
                let skip_id = self.index.skips.len() as u16;
                self.index.skips.push(CompiledSkipDef { inner: inner_ref });
                CombRef::Skip(skip_id)
            }

            Combinator::SeparatedBy {
                item,
                separator,
                trailing,
            } => {
                let item_ref = self.index_combinator(item);
                let sep_ref = self.index_combinator(separator);
                let sepby_id = self.index.separated_by.len() as u16;
                self.index.separated_by.push(CompiledSepByDef {
                    item: item_ref,
                    separator: sep_ref,
                    trailing: *trailing,
                });
                CombRef::SeparatedBy(sepby_id)
            }

            Combinator::Pratt(pratt_def) => {
                let operand = pratt_def
                    .operand
                    .as_ref()
                    .as_ref()
                    .map(|c| self.index_combinator(c));

                // Index prefix operators
                let prefix_ops: Vec<CompiledPrefixOp> = pratt_def
                    .prefix_ops
                    .iter()
                    .map(|op| {
                        let pattern = self.extract_pattern_info(&op.pattern);
                        let mapping_idx = self.intern_mapping(&op.mapping);
                        CompiledPrefixOp {
                            pattern,
                            precedence: op.precedence,
                            mapping_idx,
                        }
                    })
                    .collect();

                // Index infix operators
                let infix_ops: Vec<CompiledInfixOp> = pratt_def
                    .infix_ops
                    .iter()
                    .map(|op| {
                        let pattern = self.extract_pattern_info(&op.pattern);
                        let mapping_idx = self.intern_mapping(&op.mapping);
                        CompiledInfixOp {
                            pattern,
                            precedence: op.precedence,
                            assoc: op.assoc,
                            mapping_idx,
                        }
                    })
                    .collect();

                // Index postfix operators
                let postfix_ops: Vec<CompiledPostfixOp> = pratt_def
                    .postfix_ops
                    .iter()
                    .map(|op| self.index_postfix_op(op))
                    .collect();

                // Index ternary operator
                let ternary = pratt_def.ternary.as_ref().map(|t| {
                    let first_lit = self.extract_literal(&t.first);
                    let second_lit = self.extract_literal(&t.second);
                    let mapping_idx = self.intern_mapping(&t.mapping);
                    CompiledTernaryOp {
                        first_lit,
                        second_lit,
                        precedence: t.precedence,
                        mapping_idx,
                    }
                });

                let has_infix_with_leading =
                    infix_ops.iter().any(|op| op.pattern.leading_rule.is_some());
                let has_prefix_with_leading = prefix_ops
                    .iter()
                    .any(|op| op.pattern.leading_rule.is_some());

                let pratt_id = self.index.pratts.len() as u16;
                self.index.pratts.push(CompiledPrattDef {
                    operand,
                    prefix_ops,
                    infix_ops,
                    postfix_ops,
                    ternary,
                    has_infix_with_leading,
                    has_prefix_with_leading,
                });
                CombRef::Pratt(pratt_id)
            }

            Combinator::Mapped { inner, mapping } => {
                let inner_ref = self.index_combinator(inner);
                let mapping_idx = self.intern_mapping(mapping);
                let map_id = self.index.mapped.len() as u16;
                self.index.mapped.push(CompiledMapDef {
                    inner: inner_ref,
                    mapping_idx,
                });
                CombRef::Mapped(map_id)
            }

            Combinator::Memoize { id, inner } => {
                let inner_ref = self.index_combinator(inner);
                let memo_id = self.index.memoized.len() as u16;
                self.index.memoized.push(CompiledMemoDef {
                    memo_id: *id,
                    inner: inner_ref,
                });
                CombRef::Memoize(memo_id)
            }
        }
    }

    /// Extract pattern information from an operator combinator
    fn extract_pattern_info(&self, pattern: &Combinator) -> PatternInfo {
        match pattern {
            Combinator::Literal(lit) => PatternInfo {
                literal: lit.clone(),
                is_keyword: false,
                not_followed_by: Vec::new(),
                leading_rule: None,
            },
            Combinator::Sequence(items) => {
                // Pattern like Sequence([Rule("ws"), Literal("+")])
                // or Sequence([Literal("+"), NotFollowedBy(...)])
                let mut pattern_info = PatternInfo {
                    literal: String::new(),
                    is_keyword: false,
                    not_followed_by: Vec::new(),
                    leading_rule: None,
                };

                for item in items {
                    match item {
                        Combinator::Rule(name) => {
                            if pattern_info.literal.is_empty() {
                                // Leading rule (e.g., whitespace)
                                pattern_info.leading_rule = Some(name.clone());
                            }
                        }
                        Combinator::Literal(lit) => {
                            pattern_info.literal = lit.clone();
                        }
                        Combinator::NotFollowedBy(inner) => {
                            // Extract characters/strings that must not follow
                            if let Combinator::CharClass(CharClass::IdentCont) = inner.as_ref() {
                                pattern_info.is_keyword = true;
                            } else if let Combinator::Char(c) = inner.as_ref() {
                                pattern_info.not_followed_by.push(c.to_string());
                            } else if let Combinator::Literal(lit) = inner.as_ref() {
                                pattern_info.not_followed_by.push(lit.clone());
                            } else if let Combinator::Choice(alts) = inner.as_ref() {
                                // Handle Choice of literals like choice((lit("<"), lit("=")))
                                for alt in alts {
                                    if let Combinator::Literal(lit) = alt {
                                        pattern_info.not_followed_by.push(lit.clone());
                                    } else if let Combinator::Char(c) = alt {
                                        pattern_info.not_followed_by.push(c.to_string());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                pattern_info
            }
            _ => PatternInfo {
                literal: String::new(),
                is_keyword: false,
                not_followed_by: Vec::new(),
                leading_rule: None,
            },
        }
    }

    /// Extract a literal string from a combinator
    fn extract_literal(&self, comb: &Combinator) -> String {
        match comb {
            Combinator::Literal(lit) => lit.clone(),
            Combinator::Sequence(items) => {
                // Look for literal in sequence
                for item in items {
                    if let Combinator::Literal(lit) = item {
                        return lit.clone();
                    }
                }
                String::new()
            }
            _ => String::new(),
        }
    }

    /// Intern a mapping function string, returning its index
    fn intern_mapping(&mut self, mapping: &str) -> usize {
        // Check if we already have this mapping
        for (i, existing) in self.index.mappings.iter().enumerate() {
            if existing == mapping {
                return i;
            }
        }
        // Add new mapping
        let idx = self.index.mappings.len();
        self.index.mappings.push(mapping.to_string());
        idx
    }

    /// Index a postfix operator
    fn index_postfix_op(&mut self, op: &PostfixOp) -> CompiledPostfixOp {
        match op {
            PostfixOp::Simple {
                pattern,
                precedence,
                mapping,
            } => {
                let pattern_info = self.extract_pattern_info(pattern);
                let mapping_idx = self.intern_mapping(mapping);
                CompiledPostfixOp::Simple {
                    pattern: pattern_info,
                    precedence: *precedence,
                    mapping_idx,
                }
            }
            PostfixOp::Call {
                open,
                close,
                separator,
                arg_rule,
                precedence,
                mapping,
            } => {
                let open_lit = self.extract_literal(open);
                let close_lit = self.extract_literal(close);
                let sep_lit = self.extract_literal(separator);
                let arg_rule_id = arg_rule
                    .as_ref()
                    .and_then(|name| self.index.rule_map.get(name).copied());
                let mapping_idx = self.intern_mapping(mapping);
                CompiledPostfixOp::Call {
                    open_lit,
                    close_lit,
                    sep_lit,
                    arg_rule: arg_rule_id,
                    precedence: *precedence,
                    mapping_idx,
                }
            }
            PostfixOp::Index {
                open,
                close,
                precedence,
                mapping,
            } => {
                let open_lit = self.extract_literal(open);
                let close_lit = self.extract_literal(close);
                let mapping_idx = self.intern_mapping(mapping);
                CompiledPostfixOp::Index {
                    open_lit,
                    close_lit,
                    precedence: *precedence,
                    mapping_idx,
                }
            }
            PostfixOp::Member {
                pattern,
                precedence,
                mapping,
            } => {
                let pattern_info = self.extract_pattern_info(pattern);
                let mapping_idx = self.intern_mapping(mapping);
                CompiledPostfixOp::Member {
                    pattern: pattern_info,
                    precedence: *precedence,
                    mapping_idx,
                }
            }
            PostfixOp::Rule {
                rule_name,
                precedence,
                mapping,
            } => {
                let rule_id = self
                    .index
                    .rule_map
                    .get(rule_name)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown rule in postfix: {}", rule_name));
                let mapping_idx = self.intern_mapping(mapping);
                CompiledPostfixOp::Rule {
                    rule_id,
                    precedence: *precedence,
                    mapping_idx,
                }
            }
        }
    }

    fn emit_helpers(&mut self) {
        for helper in &self.grammar.ast_config.helper_code {
            self.output.push_str(helper);
            self.blank();
        }
    }

    fn emit_builtin_helpers(&mut self) {
        let string_type = self
            .grammar
            .ast_config
            .string_type
            .as_deref()
            .unwrap_or("String");
        // Helper function to decode unicode escapes in identifiers
        self.line("/// Decode unicode escape sequences in identifier text");
        self.line(&format!(
            "fn decode_identifier_escapes(text: &{}) -> {} {{",
            string_type, string_type
        ));
        self.indent += 1;
        // For String type, use as_str(); for custom types use as_ref()
        if string_type == "String" {
            self.line("let s: &str = text.as_str();");
        } else {
            self.line("let s: &str = text.as_ref();");
        }
        self.line("if !s.contains('\\\\') { return text.clone(); }");
        self.line("let mut result = String::with_capacity(s.len());");
        self.line("let mut chars = s.chars().peekable();");
        self.line("while let Some(c) = chars.next() {");
        self.indent += 1;
        self.line("if c == '\\\\' && chars.peek() == Some(&'u') {");
        self.indent += 1;
        self.line("chars.next();");
        self.line("if chars.peek() == Some(&'{') {");
        self.indent += 1;
        self.line("chars.next();");
        self.line("let mut hex = String::new();");
        self.line("while let Some(&h) = chars.peek() {");
        self.indent += 1;
        self.line("if h == '}' { chars.next(); break; }");
        self.line("chars.next(); hex.push(h);");
        self.indent -= 1;
        self.line("}");
        self.line("if let Ok(code) = u32::from_str_radix(&hex, 16) {");
        self.indent += 1;
        self.line("if let Some(ch) = char::from_u32(code) { result.push(ch); }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("let mut hex = String::new();");
        self.line("for _ in 0..4 { if let Some(h) = chars.next() { hex.push(h); } }");
        self.line("if let Ok(code) = u32::from_str_radix(&hex, 16) {");
        self.indent += 1;
        self.line("if let Some(ch) = char::from_u32(code) { result.push(ch); }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else { result.push(c); }");
        self.indent -= 1;
        self.line("}");
        // For String type, just return the result directly; for custom types, use From
        if string_type == "String" {
            self.line("result");
        } else {
            self.line(&format!("{}::from(result)", string_type));
        }
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_helper_methods(&mut self, string_type: &str) {
        // Current char
        self.line("fn current_char(&self) -> Option<char> {");
        self.indent += 1;
        self.line("self.input.get(self.pos..).and_then(|s| s.chars().next())");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Advance by one character and track line/column
        self.line("fn advance(&mut self) {");
        self.indent += 1;
        self.line("if let Some(c) = self.current_char() {");
        self.indent += 1;
        self.line("self.pos += c.len_utf8();");
        self.line("if c == '\\n' {");
        self.indent += 1;
        self.line("self.line += 1;");
        self.line("self.column = 1;");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.column += 1;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Match literal with line tracking
        self.line("fn match_literal(&mut self, lit: &str) -> bool {");
        self.indent += 1;
        self.line("if self.input.get(self.pos..).is_some_and(|s| s.starts_with(lit)) {");
        self.indent += 1;
        self.line("for c in lit.chars() {");
        self.indent += 1;
        self.line("self.pos += c.len_utf8();");
        self.line("if c == '\\n' {");
        self.indent += 1;
        self.line("self.line += 1;");
        self.line("self.column = 1;");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.column += 1;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("true");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("false");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Match char class with line tracking
        self.line("fn match_char_class(&mut self, class: fn(char) -> bool) -> Option<char> {");
        self.indent += 1;
        self.line("if let Some(c) = self.current_char() {");
        self.indent += 1;
        self.line("if class(c) {");
        self.indent += 1;
        self.line("self.pos += c.len_utf8();");
        self.line("if c == '\\n' {");
        self.indent += 1;
        self.line("self.line += 1;");
        self.line("self.column = 1;");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.column += 1;");
        self.indent -= 1;
        self.line("}");
        self.line("return Some(c);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("None");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Create span at current position
        self.line("fn make_span(&self, start: usize) -> Span {");
        self.indent += 1;
        self.line("Span {");
        self.indent += 1;
        self.line("start,");
        self.line("end: self.pos,");
        self.line("line: self.line,");
        self.line("column: self.column,");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Create text result
        self.line(&format!(
            "fn text_result(&self, start: usize, end: usize) -> {} {{",
            string_type
        ));
        self.indent += 1;
        if string_type == "String" {
            self.line("self.input.get(start..end).unwrap_or(\"\").to_string()");
        } else {
            // For custom string types like JsString, assume From<&str>
            self.line(&format!(
                "{}::from(self.input.get(start..end).unwrap_or(\"\"))",
                string_type
            ));
        }
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Make error
        self.line("fn make_error(&self, msg: &str) -> ParseError {");
        self.indent += 1;
        self.line("ParseError {");
        self.indent += 1;
        self.line("message: msg.to_string(),");
        self.line(
            "span: Span { start: self.pos, end: self.pos, line: self.line, column: self.column },",
        );
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Try to consume a literal (for Pratt parsing)
        self.line("fn try_consume(&mut self, s: &str) -> bool {");
        self.indent += 1;
        self.line(
            "if self.input.get(self.pos..).is_some_and(|remaining| remaining.starts_with(s)) {",
        );
        self.indent += 1;
        self.line("for c in s.chars() {");
        self.indent += 1;
        self.line("if c == '\\n' {");
        self.indent += 1;
        self.line("self.line += 1;");
        self.line("self.column = 1;");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.column += 1;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("self.pos += s.len();");
        self.line("true");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("false");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_apply_mapping_method(&mut self) {
        if self.index.mapped.is_empty() {
            // No mappings - generate a stub that just returns the input unchanged
            self.line(
                "fn apply_mapping(&self, _map_id: u16, r: ParseResult, _span: Span) -> Result<ParseResult, ParseError> {",
            );
            self.indent += 1;
            self.line("Ok(r)");
            self.indent -= 1;
            self.line("}");
            self.blank();
            return;
        }

        // Collect mapping info first to avoid borrow conflict
        let mapping_arms: Vec<(usize, String)> = self
            .index
            .mapped
            .iter()
            .enumerate()
            .map(|(map_id, mapped_def)| {
                let mapping_fn = self.index.mappings[mapped_def.mapping_idx].clone();
                (map_id, mapping_fn)
            })
            .collect();

        self.line(
            "fn apply_mapping(&self, map_id: u16, r: ParseResult, span: Span) -> Result<ParseResult, ParseError> {",
        );
        self.indent += 1;
        self.line("match map_id {");
        self.indent += 1;

        // Generate an arm for each mapping
        for (map_id, mapping_fn) in mapping_arms {
            self.line(&format!("{} => {{", map_id));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping_fn));
            self.line("mapping_fn(r, span)");
            self.indent -= 1;
            self.line("}");
        }

        // Default arm
        self.line("_ => Ok(r),");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    fn emit_header(&mut self) {
        self.line("// Auto-generated parser - DO NOT EDIT");
        self.line("//");
        self.line("// Generated by trampoline-parser (scannerless)");
        self.blank();

        // Emit user imports
        for import in &self.grammar.ast_config.imports {
            self.line(&format!("use {};", import));
        }
        if !self.grammar.ast_config.imports.is_empty() {
            self.blank();
        }
    }

    fn emit_span(&mut self) {
        if !self.grammar.ast_config.generate_span {
            return;
        }
        self.line("/// Source location span");
        self.line("#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]");
        self.line("pub struct Span {");
        self.indent += 1;
        self.line("pub start: usize,");
        self.line("pub end: usize,");
        self.line("pub line: u32,");
        self.line("pub column: u32,");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_parse_error(&mut self) {
        if !self.grammar.ast_config.generate_parse_error {
            return;
        }
        self.line("/// Parse error");
        self.line("#[derive(Debug, Clone)]");
        self.line("pub struct ParseError {");
        self.indent += 1;
        self.line("pub message: String,");
        self.line("pub span: Span,");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("impl ParseError {");
        self.indent += 1;
        self.line("pub fn new(message: String, line: u32, column: u32) -> Self {");
        self.indent += 1;
        self.line("Self { message, span: Span { start: 0, end: 0, line, column } }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("impl core::fmt::Display for ParseError {");
        self.indent += 1;
        self.line("fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {");
        self.indent += 1;
        self.line("write!(f, \"{} at {}..{}\", self.message, self.span.start, self.span.end)");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("impl core::error::Error for ParseError {}");
        self.blank();
    }

    fn emit_parse_result_enum(&mut self) {
        if !self.grammar.ast_config.generate_parse_result {
            return;
        }

        let string_type = self
            .grammar
            .ast_config
            .string_type
            .as_deref()
            .unwrap_or("String");

        self.line("/// Parse result value");
        self.line("#[derive(Debug, Clone)]");
        self.line("pub enum ParseResult {");
        self.indent += 1;
        self.line("/// No value (for skipped items)");
        self.line("None,");
        self.line("/// Captured text");
        self.line(&format!("Text({}, Span),", string_type));
        self.line("/// List of results");
        self.line("List(Vec<ParseResult>),");

        // Emit custom variants
        for variant in &self.grammar.ast_config.result_variants {
            self.line(&format!("{}({}),", variant.name, variant.rust_type));
        }

        self.indent -= 1;
        self.line("}");
        self.blank();

        // Emit helper methods
        self.line("impl ParseResult {");
        self.indent += 1;
        self.line("pub fn span(&self) -> Span {");
        self.indent += 1;
        self.line("match self {");
        self.indent += 1;
        self.line("ParseResult::None => Span::default(),");
        self.line("ParseResult::Text(_, span) => *span,");
        self.line("ParseResult::List(items) => {");
        self.indent += 1;
        self.line("if items.is_empty() { return Span::default(); }");
        self.line("let first_span = items.first().map(|i| i.span()).unwrap_or_default();");
        self.line("let last_span = items.last().map(|i| i.span()).unwrap_or_default();");
        self.line("Span { start: first_span.start, end: last_span.end, line: first_span.line, column: first_span.column }");
        self.indent -= 1;
        self.line("}");
        for variant in &self.grammar.ast_config.result_variants {
            if let Some(ref span_expr) = variant.span_expr {
                self.line(&format!(
                    "ParseResult::{}(v) => {},",
                    variant.name,
                    span_expr.replace('_', "v")
                ));
            } else {
                self.line(&format!(
                    "ParseResult::{}(_) => Span::default(),",
                    variant.name
                ));
            }
        }
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    // ========================================================================
    // Indexed code generation (reduced enum variants)
    // ========================================================================

    /// Emit the fixed Work enum with indexed variants
    fn emit_indexed_work_enum(&mut self) {
        self.line("/// Work items for the trampoline (state-indexed)");
        self.line("#[derive(Debug, Clone)]");
        self.line("enum Work {");
        self.indent += 1;

        // Rule dispatch
        self.line("/// Dispatch to a rule by ID");
        self.line("Rule { rule_id: u16, result_base: usize },");
        self.blank();

        // Sequence variants (3)
        self.line("/// Start a sequence");
        self.line("SeqStart { seq_id: u16, result_base: usize },");
        self.line("/// Continue to next step in sequence");
        self.line("SeqStep { seq_id: u16, step: u8, result_base: usize, seq_base: usize },");
        self.line("/// Complete a sequence");
        self.line("SeqComplete { seq_id: u16, result_base: usize, seq_base: usize },");
        self.blank();

        // Choice variants (2)
        self.line("/// Start a choice");
        self.line("ChoiceStart { choice_id: u16, result_base: usize },");
        self.line("/// Try next alternative in choice");
        self.line("ChoiceTry { choice_id: u16, alt: u8, result_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32, stack_base: usize },");
        self.blank();

        // ZeroOrMore variants (3)
        self.line("/// Start zero-or-more loop");
        self.line("ZeroOrMoreStart { loop_id: u16, result_base: usize },");
        self.line("/// Continue zero-or-more loop");
        self.line("ZeroOrMoreLoop { loop_id: u16, result_base: usize, loop_base: usize, iter_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.line("/// Complete zero-or-more loop");
        self.line("ZeroOrMoreComplete { loop_id: u16, result_base: usize, loop_base: usize },");
        self.blank();

        // OneOrMore variants (3)
        self.line("/// Start one-or-more loop");
        self.line("OneOrMoreStart { loop_id: u16, result_base: usize },");
        self.line("/// Continue one-or-more loop");
        self.line("OneOrMoreLoop { loop_id: u16, result_base: usize, loop_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32, count: usize },");
        self.line("/// Complete one-or-more loop");
        self.line("OneOrMoreComplete { loop_id: u16, result_base: usize, loop_base: usize },");
        self.blank();

        // Optional variants (2)
        self.line("/// Start optional");
        self.line("OptionalStart { opt_id: u16, result_base: usize },");
        self.line("/// Check optional result");
        self.line("OptionalCheck { opt_id: u16, result_base: usize, opt_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.blank();

        // Capture variants (2)
        self.line("/// Start capture");
        self.line("CaptureStart { cap_id: u16, result_base: usize },");
        self.line("/// Complete capture");
        self.line("CaptureComplete { cap_id: u16, result_base: usize, capture_base: usize, start_pos: usize, start_line: u32, start_column: u32 },");
        self.blank();

        // Lookahead variants (4)
        self.line("/// Start not-followed-by");
        self.line("NotFollowedByStart { look_id: u16, result_base: usize },");
        self.line("/// Check not-followed-by result");
        self.line("NotFollowedByCheck { look_id: u16, result_base: usize, look_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.line("/// Start followed-by");
        self.line("FollowedByStart { look_id: u16, result_base: usize },");
        self.line("/// Check followed-by result");
        self.line("FollowedByCheck { look_id: u16, result_base: usize, look_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.blank();

        // Skip variants (2)
        self.line("/// Start skip");
        self.line("SkipStart { skip_id: u16, result_base: usize },");
        self.line("/// Complete skip");
        self.line("SkipComplete { skip_id: u16, result_base: usize, skip_base: usize },");
        self.blank();

        // SeparatedBy variants (4)
        self.line("/// Start separated-by");
        self.line("SepByStart { sepby_id: u16, result_base: usize },");
        self.line("/// Parse separator in separated-by");
        self.line("SepBySep { sepby_id: u16, result_base: usize, list_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.line("/// Parse item in separated-by");
        self.line("SepByItem { sepby_id: u16, result_base: usize, list_base: usize, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32 },");
        self.line("/// Complete separated-by");
        self.line("SepByComplete { sepby_id: u16, result_base: usize, list_base: usize },");
        self.blank();

        // Mapped variants (2)
        self.line("/// Start mapped combinator");
        self.line("MappedStart { map_id: u16, result_base: usize },");
        self.line("/// Apply mapping function");
        self.line("MappedApply { map_id: u16, result_base: usize, map_base: usize, start_pos: usize, start_line: u32, start_column: u32 },");
        self.blank();

        // Memoize variants (2)
        self.line("/// Start memoized combinator");
        self.line("MemoStart { memo_id: u16, result_base: usize },");
        self.line("/// Complete memoized combinator");
        self.line("MemoComplete { memo_id: u16, result_base: usize, start_pos: usize, inner_base: usize },");
        self.blank();

        // Pratt parsing variants (~15)
        self.line("/// Start Pratt expression parsing");
        self.line("PrattStart { pratt_id: u16, result_base: usize },");
        self.line("/// Parse Pratt operand");
        self.line("PrattParseOperand { pratt_id: u16, result_base: usize, min_prec: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt operand");
        self.line("PrattAfterOperand { pratt_id: u16, result_base: usize, min_prec: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt prefix operator");
        self.line("PrattAfterPrefix { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt infix operator RHS");
        self.line("PrattAfterInfix { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// Check for Pratt postfix operator");
        self.line("PrattCheckPostfix { pratt_id: u16, result_base: usize, min_prec: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing simple Pratt postfix");
        self.line("PrattAfterPostfixSimple { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// Parse Pratt call argument");
        self.line("PrattPostfixCallArg { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, args_base: usize, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// Parse Pratt call separator");
        self.line("PrattPostfixCallSep { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, args_base: usize, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt call");
        self.line("PrattAfterPostfixCall { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, args_base: usize, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt index");
        self.line("PrattAfterPostfixIndex { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt member access");
        self.line("PrattAfterPostfixMember { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing Pratt rule-based postfix");
        self.line("PrattAfterPostfixRule { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing ternary first operand");
        self.line("PrattAfterTernaryFirst { pratt_id: u16, result_base: usize, min_prec: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing ternary second operand");
        self.line("PrattAfterTernarySecond { pratt_id: u16, result_base: usize, min_prec: u8, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing infix leading rule");
        self.line("PrattAfterInfixLeadingRule { pratt_id: u16, result_base: usize, min_prec: u8, op_idx: u8, next_prec: u8, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32, start_pos: usize, start_line: u32, start_column: u32 },");
        self.line("/// After parsing prefix leading rule");
        self.line("PrattAfterPrefixLeadingRule { pratt_id: u16, result_base: usize, min_prec: u8, checkpoint: usize, checkpoint_line: u32, checkpoint_column: u32, start_pos: usize, start_line: u32, start_column: u32 },");
        self.blank();

        // Terminal execution (immediate, no work item needed, but we keep for dispatch uniformity)
        self.line("/// Execute a literal match");
        self.line("Literal { lit_id: u16, result_base: usize },");
        self.line("/// Execute a char class match");
        self.line("CharClass { class: u8, result_base: usize },");
        self.line("/// Execute a char range match");
        self.line("CharRange { from: char, to: char, result_base: usize },");
        self.line("/// Execute a specific char match");
        self.line("Char { ch: char, result_base: usize },");
        self.line("/// Match any char");
        self.line("AnyChar { result_base: usize },");

        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    /// Emit static dispatch tables for indexed code generation
    fn emit_static_tables(&mut self) {
        // Pre-collect all data to avoid borrow conflicts
        let seq_lines: Vec<String> = self
            .index
            .sequences
            .iter()
            .map(|seq| {
                let items: Vec<String> = seq.items.iter().map(combref_to_code).collect();
                format!("&[{}],", items.join(", "))
            })
            .collect();

        let choice_lines: Vec<String> = self
            .index
            .choices
            .iter()
            .map(|choice| {
                let alts: Vec<String> = choice.alts.iter().map(combref_to_code).collect();
                format!("&[{}],", alts.join(", "))
            })
            .collect();

        let zero_or_more_lines: Vec<String> = self
            .index
            .zero_or_more
            .iter()
            .map(|l| format!("{},", combref_to_code(&l.item)))
            .collect();

        let one_or_more_lines: Vec<String> = self
            .index
            .one_or_more
            .iter()
            .map(|l| format!("{},", combref_to_code(&l.item)))
            .collect();

        let optional_lines: Vec<String> = self
            .index
            .optionals
            .iter()
            .map(|o| format!("{},", combref_to_code(&o.inner)))
            .collect();

        let capture_lines: Vec<String> = self
            .index
            .captures
            .iter()
            .map(|c| format!("{},", combref_to_code(&c.inner)))
            .collect();

        let nfb_lines: Vec<String> = self
            .index
            .not_followed_by
            .iter()
            .map(|l| format!("{},", combref_to_code(&l.inner)))
            .collect();

        let fb_lines: Vec<String> = self
            .index
            .followed_by
            .iter()
            .map(|l| format!("{},", combref_to_code(&l.inner)))
            .collect();

        let skip_lines: Vec<String> = self
            .index
            .skips
            .iter()
            .map(|s| format!("{},", combref_to_code(&s.inner)))
            .collect();

        let sepby_lines: Vec<String> = self
            .index
            .separated_by
            .iter()
            .map(|s| {
                format!(
                    "({}, {}, {}),",
                    combref_to_code(&s.item),
                    combref_to_code(&s.separator),
                    s.trailing
                )
            })
            .collect();

        let mapped_lines: Vec<String> = self
            .index
            .mapped
            .iter()
            .map(|m| format!("({}, {}),", combref_to_code(&m.inner), m.mapping_idx))
            .collect();

        let memo_lines: Vec<String> = self
            .index
            .memoized
            .iter()
            .map(|m| format!("({}, {}),", m.memo_id, combref_to_code(&m.inner)))
            .collect();

        let literal_lines: Vec<String> = self
            .index
            .literals
            .iter()
            .map(|l| format!("{:?},", l))
            .collect();

        let rule_lines: Vec<String> = self
            .index
            .rules
            .iter()
            .map(|r| format!("{},", combref_to_code(&r.entry)))
            .collect();

        // Now emit all the collected data
        // CombRef enum
        self.line("/// Reference to a combinator by type and index");
        self.line("#[derive(Debug, Clone, Copy)]");
        self.line("#[allow(dead_code)]");
        self.line("enum CombRef {");
        self.indent += 1;
        self.line("Rule(u16),");
        self.line("Seq(u16),");
        self.line("Choice(u16),");
        self.line("ZeroOrMore(u16),");
        self.line("OneOrMore(u16),");
        self.line("Optional(u16),");
        self.line("Literal(u16),");
        self.line("CharClass(u8),");
        self.line("CharRange(char, char),");
        self.line("Char(char),");
        self.line("AnyChar,");
        self.line("Capture(u16),");
        self.line("NotFollowedBy(u16),");
        self.line("FollowedBy(u16),");
        self.line("Skip(u16),");
        self.line("SeparatedBy(u16),");
        self.line("Pratt(u16),");
        self.line("Mapped(u16),");
        self.line("Memoize(u16),");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Sequence definitions
        self.line("/// Sequence combinator definitions");
        self.line("static SEQUENCES: &[&[CombRef]] = &[");
        self.indent += 1;
        for line in &seq_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Choice definitions
        self.line("/// Choice combinator definitions");
        self.line("static CHOICES: &[&[CombRef]] = &[");
        self.indent += 1;
        for line in &choice_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Loop definitions (ZeroOrMore)
        self.line("/// ZeroOrMore loop definitions");
        self.line("static ZERO_OR_MORE: &[CombRef] = &[");
        self.indent += 1;
        for line in &zero_or_more_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Loop definitions (OneOrMore)
        self.line("/// OneOrMore loop definitions");
        self.line("static ONE_OR_MORE: &[CombRef] = &[");
        self.indent += 1;
        for line in &one_or_more_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Optional definitions
        self.line("/// Optional combinator definitions");
        self.line("static OPTIONALS: &[CombRef] = &[");
        self.indent += 1;
        for line in &optional_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Capture definitions
        self.line("/// Capture combinator definitions");
        self.line("static CAPTURES: &[CombRef] = &[");
        self.indent += 1;
        for line in &capture_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Lookahead definitions
        self.line("/// NotFollowedBy combinator definitions");
        self.line("static NOT_FOLLOWED_BY: &[CombRef] = &[");
        self.indent += 1;
        for line in &nfb_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        self.line("/// FollowedBy combinator definitions");
        self.line("static FOLLOWED_BY: &[CombRef] = &[");
        self.indent += 1;
        for line in &fb_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Skip definitions
        self.line("/// Skip combinator definitions");
        self.line("static SKIPS: &[CombRef] = &[");
        self.indent += 1;
        for line in &skip_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // SeparatedBy definitions
        self.line("/// SeparatedBy combinator definitions (item, separator, trailing)");
        self.line("static SEPARATED_BY: &[(CombRef, CombRef, bool)] = &[");
        self.indent += 1;
        for line in &sepby_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Mapped definitions
        self.line("/// Mapped combinator definitions (inner, mapping_idx)");
        self.line("static MAPPED: &[(CombRef, usize)] = &[");
        self.indent += 1;
        for line in &mapped_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Memoize definitions
        self.line("/// Memoize combinator definitions (memo_id, inner)");
        self.line("static MEMOIZED: &[(usize, CombRef)] = &[");
        self.indent += 1;
        for line in &memo_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Literals table
        self.line("/// Literal strings table");
        self.line("static LITERALS: &[&str] = &[");
        self.indent += 1;
        for line in &literal_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Rule entry points
        self.line("/// Rule entry points");
        self.line("static RULES: &[CombRef] = &[");
        self.indent += 1;
        for line in &rule_lines {
            self.line(line);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Emit Pratt static tables
        self.emit_pratt_static_tables();
    }

    /// Emit Pratt parsing static tables
    fn emit_pratt_static_tables(&mut self) {
        if self.index.pratts.is_empty() {
            return;
        }

        // Pratt operator structs
        self.line("/// Pratt prefix operator definition");
        self.line("#[derive(Debug, Clone)]");
        self.line("struct PrattPrefixOp {");
        self.indent += 1;
        self.line("literal: &'static str,");
        self.line("precedence: u8,");
        self.line("is_keyword: bool,");
        self.line("not_followed_by: &'static [&'static str],");
        self.line("leading_rule: Option<u16>,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("/// Pratt infix operator definition");
        self.line("#[derive(Debug, Clone)]");
        self.line("struct PrattInfixOp {");
        self.indent += 1;
        self.line("literal: &'static str,");
        self.line("precedence: u8,");
        self.line("is_left_assoc: bool,");
        self.line("is_keyword: bool,");
        self.line("not_followed_by: &'static [&'static str],");
        self.line("leading_rule: Option<u16>,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("/// Pratt postfix operator kind");
        self.line("#[derive(Debug, Clone, Copy)]");
        self.line("enum PostfixKind {");
        self.indent += 1;
        self.line("Simple,");
        self.line("Call,");
        self.line("Index,");
        self.line("Member,");
        self.line("Rule,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("/// Pratt postfix operator definition");
        self.line("#[derive(Debug, Clone)]");
        self.line("struct PrattPostfixOp {");
        self.indent += 1;
        self.line("kind: PostfixKind,");
        self.line("open_lit: &'static str,");
        self.line("close_lit: &'static str,");
        self.line("sep_lit: &'static str,");
        self.line("precedence: u8,");
        self.line("not_followed_by: &'static [&'static str],");
        self.line("arg_rule: Option<u16>,");
        self.line("member_rule: Option<u16>,");
        self.line("rule_name_id: Option<u16>,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("/// Pratt ternary operator definition");
        self.line("#[derive(Debug, Clone)]");
        self.line("struct PrattTernaryOp {");
        self.indent += 1;
        self.line("first_lit: &'static str,");
        self.line("second_lit: &'static str,");
        self.line("precedence: u8,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("/// Pratt parser definition");
        self.line("#[derive(Debug, Clone)]");
        self.line("struct PrattDef {");
        self.indent += 1;
        self.line("operand: Option<CombRef>,");
        self.line("prefix_ops: &'static [PrattPrefixOp],");
        self.line("infix_ops: &'static [PrattInfixOp],");
        self.line("postfix_ops: &'static [PrattPostfixOp],");
        self.line("ternary: Option<PrattTernaryOp>,");
        self.line("has_infix_with_leading: bool,");
        self.line("has_prefix_with_leading: bool,");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Build Pratt definition data
        let pratt_defs: Vec<String> = self.build_pratt_defs();

        // Emit PRATTS static array
        self.line("/// Pratt parser definitions");
        self.line("static PRATTS: &[PrattDef] = &[");
        self.indent += 1;
        for def in &pratt_defs {
            self.line(def);
        }
        self.indent -= 1;
        self.line("];");
        self.blank();

        // Emit static arrays for each Pratt's operators
        self.emit_pratt_operator_arrays();
    }

    /// Build Pratt definition strings for the static array
    fn build_pratt_defs(&self) -> Vec<String> {
        self.index
            .pratts
            .iter()
            .enumerate()
            .map(|(pratt_idx, pratt)| {
                let operand = match &pratt.operand {
                    Some(cr) => format!("Some({})", combref_to_code(cr)),
                    None => "None".to_string(),
                };
                let ternary = match &pratt.ternary {
                    Some(t) => format!(
                        "Some(PrattTernaryOp {{ first_lit: {:?}, second_lit: {:?}, precedence: {} }})",
                        t.first_lit, t.second_lit, t.precedence
                    ),
                    None => "None".to_string(),
                };
                format!(
                    "PrattDef {{ operand: {}, prefix_ops: &PRATT_{}_PREFIX, infix_ops: &PRATT_{}_INFIX, postfix_ops: &PRATT_{}_POSTFIX, ternary: {}, has_infix_with_leading: {}, has_prefix_with_leading: {} }},",
                    operand, pratt_idx, pratt_idx, pratt_idx, ternary, pratt.has_infix_with_leading, pratt.has_prefix_with_leading
                )
            })
            .collect()
    }

    /// Emit operator arrays for each Pratt parser
    #[allow(clippy::type_complexity)]
    fn emit_pratt_operator_arrays(&mut self) {
        // Pre-collect all data to avoid borrow conflicts
        let pratt_arrays: Vec<(usize, Vec<String>, Vec<String>, Vec<String>)> = self
            .index
            .pratts
            .iter()
            .enumerate()
            .map(|(pratt_idx, pratt)| {
                // Prefix operators
                let prefix_lines: Vec<String> = pratt
                    .prefix_ops
                    .iter()
                    .map(|prefix_op| {
                        let nfb: Vec<String> = prefix_op
                            .pattern
                            .not_followed_by
                            .iter()
                            .map(|s| format!("{:?}", s))
                            .collect();
                        let leading = match &prefix_op.pattern.leading_rule {
                            Some(name) => {
                                let rule_id = self.index.rule_map.get(name).copied().unwrap_or(0);
                                format!("Some({})", rule_id)
                            }
                            None => "None".to_string(),
                        };
                        format!(
                            "PrattPrefixOp {{ literal: {:?}, precedence: {}, is_keyword: {}, not_followed_by: &[{}], leading_rule: {} }},",
                            prefix_op.pattern.literal,
                            prefix_op.precedence,
                            prefix_op.pattern.is_keyword,
                            nfb.join(", "),
                            leading
                        )
                    })
                    .collect();

                // Infix operators
                let infix_lines: Vec<String> = pratt
                    .infix_ops
                    .iter()
                    .map(|infix_op| {
                        let nfb: Vec<String> = infix_op
                            .pattern
                            .not_followed_by
                            .iter()
                            .map(|s| format!("{:?}", s))
                            .collect();
                        let leading = match &infix_op.pattern.leading_rule {
                            Some(name) => {
                                let rule_id = self.index.rule_map.get(name).copied().unwrap_or(0);
                                format!("Some({})", rule_id)
                            }
                            None => "None".to_string(),
                        };
                        format!(
                            "PrattInfixOp {{ literal: {:?}, precedence: {}, is_left_assoc: {}, is_keyword: {}, not_followed_by: &[{}], leading_rule: {} }},",
                            infix_op.pattern.literal,
                            infix_op.precedence,
                            infix_op.assoc == crate::Assoc::Left,
                            infix_op.pattern.is_keyword,
                            nfb.join(", "),
                            leading
                        )
                    })
                    .collect();

                // Postfix operators
                let postfix_lines: Vec<String> = pratt
                    .postfix_ops
                    .iter()
                    .map(|postfix_op| {
                        let (kind, open, close, sep, prec, nfb, arg_rule, member_rule, rule_name_id) =
                            match postfix_op {
                                CompiledPostfixOp::Simple { pattern, precedence, .. } => {
                                    let nfb: Vec<String> = pattern
                                        .not_followed_by
                                        .iter()
                                        .map(|s| format!("{:?}", s))
                                        .collect();
                                    (
                                        "PostfixKind::Simple",
                                        pattern.literal.clone(),
                                        String::new(),
                                        String::new(),
                                        *precedence,
                                        nfb,
                                        "None".to_string(),
                                        "None".to_string(),
                                        "None".to_string(),
                                    )
                                }
                                CompiledPostfixOp::Call {
                                    open_lit,
                                    close_lit,
                                    sep_lit,
                                    arg_rule,
                                    precedence,
                                    ..
                                } => {
                                    let ar = match arg_rule {
                                        Some(id) => format!("Some({})", id),
                                        None => "None".to_string(),
                                    };
                                    (
                                        "PostfixKind::Call",
                                        open_lit.clone(),
                                        close_lit.clone(),
                                        sep_lit.clone(),
                                        *precedence,
                                        vec![],
                                        ar,
                                        "None".to_string(),
                                        "None".to_string(),
                                    )
                                }
                                CompiledPostfixOp::Index {
                                    open_lit,
                                    close_lit,
                                    precedence,
                                    ..
                                } => (
                                    "PostfixKind::Index",
                                    open_lit.clone(),
                                    close_lit.clone(),
                                    String::new(),
                                    *precedence,
                                    vec![],
                                    "None".to_string(),
                                    "None".to_string(),
                                    "None".to_string(),
                                ),
                                CompiledPostfixOp::Member { pattern, precedence, .. } => {
                                    let nfb: Vec<String> = pattern
                                        .not_followed_by
                                        .iter()
                                        .map(|c| format!("{:?}", c.to_string()))
                                        .collect();
                                    (
                                        "PostfixKind::Member",
                                        pattern.literal.clone(),
                                        String::new(),
                                        String::new(),
                                        *precedence,
                                        nfb,
                                        "None".to_string(),
                                        "None".to_string(),
                                        "None".to_string(),
                                    )
                                }
                                CompiledPostfixOp::Rule { rule_id, precedence, .. } => (
                                    "PostfixKind::Rule",
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    *precedence,
                                    vec![],
                                    "None".to_string(),
                                    "None".to_string(),
                                    format!("Some({})", rule_id),
                                ),
                            };
                        format!(
                            "PrattPostfixOp {{ kind: {}, open_lit: {:?}, close_lit: {:?}, sep_lit: {:?}, precedence: {}, not_followed_by: &[{}], arg_rule: {}, member_rule: {}, rule_name_id: {} }},",
                            kind, open, close, sep, prec, nfb.join(", "), arg_rule, member_rule, rule_name_id
                        )
                    })
                    .collect();

                (pratt_idx, prefix_lines, infix_lines, postfix_lines)
            })
            .collect();

        // Now emit all collected data
        for (pratt_idx, prefix_lines, infix_lines, postfix_lines) in pratt_arrays {
            // Prefix operators
            self.line(&format!(
                "static PRATT_{}_PREFIX: &[PrattPrefixOp] = &[",
                pratt_idx
            ));
            self.indent += 1;
            for line in &prefix_lines {
                self.line(line);
            }
            self.indent -= 1;
            self.line("];");

            // Infix operators
            self.line(&format!(
                "static PRATT_{}_INFIX: &[PrattInfixOp] = &[",
                pratt_idx
            ));
            self.indent += 1;
            for line in &infix_lines {
                self.line(line);
            }
            self.indent -= 1;
            self.line("];");

            // Postfix operators
            self.line(&format!(
                "static PRATT_{}_POSTFIX: &[PrattPostfixOp] = &[",
                pratt_idx
            ));
            self.indent += 1;
            for line in &postfix_lines {
                self.line(line);
            }
            self.indent -= 1;
            self.line("];");
            self.blank();
        }
    }

    /// Emit the dispatch_combref helper for indexed generation
    fn emit_dispatch_combref(&mut self) {
        self.line("/// Push work item for a CombRef");
        self.line("fn dispatch_combref(&mut self, cref: CombRef, result_base: usize) {");
        self.indent += 1;
        self.line("match cref {");
        self.indent += 1;
        self.line(
            "CombRef::Rule(id) => self.work_stack.push(Work::Rule { rule_id: id, result_base }),",
        );
        self.line(
            "CombRef::Seq(id) => self.work_stack.push(Work::SeqStart { seq_id: id, result_base }),",
        );
        self.line("CombRef::Choice(id) => self.work_stack.push(Work::ChoiceStart { choice_id: id, result_base }),");
        self.line("CombRef::ZeroOrMore(id) => self.work_stack.push(Work::ZeroOrMoreStart { loop_id: id, result_base }),");
        self.line("CombRef::OneOrMore(id) => self.work_stack.push(Work::OneOrMoreStart { loop_id: id, result_base }),");
        self.line("CombRef::Optional(id) => self.work_stack.push(Work::OptionalStart { opt_id: id, result_base }),");
        self.line("CombRef::Literal(id) => self.work_stack.push(Work::Literal { lit_id: id, result_base }),");
        self.line("CombRef::CharClass(class) => self.work_stack.push(Work::CharClass { class, result_base }),");
        self.line("CombRef::CharRange(from, to) => self.work_stack.push(Work::CharRange { from, to, result_base }),");
        self.line("CombRef::Char(ch) => self.work_stack.push(Work::Char { ch, result_base }),");
        self.line("CombRef::AnyChar => self.work_stack.push(Work::AnyChar { result_base }),");
        self.line("CombRef::Capture(id) => self.work_stack.push(Work::CaptureStart { cap_id: id, result_base }),");
        self.line("CombRef::NotFollowedBy(id) => self.work_stack.push(Work::NotFollowedByStart { look_id: id, result_base }),");
        self.line("CombRef::FollowedBy(id) => self.work_stack.push(Work::FollowedByStart { look_id: id, result_base }),");
        self.line("CombRef::Skip(id) => self.work_stack.push(Work::SkipStart { skip_id: id, result_base }),");
        self.line("CombRef::SeparatedBy(id) => self.work_stack.push(Work::SepByStart { sepby_id: id, result_base }),");
        self.line("CombRef::Pratt(id) => self.work_stack.push(Work::PrattStart { pratt_id: id, result_base }),");
        self.line("CombRef::Mapped(id) => self.work_stack.push(Work::MappedStart { map_id: id, result_base }),");
        self.line("CombRef::Memoize(id) => self.work_stack.push(Work::MemoStart { memo_id: id, result_base }),");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    /// Emit the indexed execute method
    fn emit_indexed_execute(&mut self) {
        self.line("fn execute(&mut self, work: Work) -> Result<(), ParseError> {");
        self.indent += 1;
        self.line("match work {");
        self.indent += 1;

        // Rule dispatch
        self.line("Work::Rule { rule_id, result_base } => {");
        self.indent += 1;
        self.line("if let Some(&entry) = RULES.get(rule_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(entry, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Sequence handlers
        self.emit_indexed_seq_handlers();

        // Choice handlers
        self.emit_indexed_choice_handlers();

        // ZeroOrMore handlers
        self.emit_indexed_zero_or_more_handlers();

        // OneOrMore handlers
        self.emit_indexed_one_or_more_handlers();

        // Optional handlers
        self.emit_indexed_optional_handlers();

        // Capture handlers
        self.emit_indexed_capture_handlers();

        // Lookahead handlers
        self.emit_indexed_lookahead_handlers();

        // Skip handlers
        self.emit_indexed_skip_handlers();

        // SeparatedBy handlers
        self.emit_indexed_separated_by_handlers();

        // Mapped handlers
        self.emit_indexed_mapped_handlers();

        // Memoize handlers
        self.emit_indexed_memoize_handlers();

        // Terminal handlers
        self.emit_indexed_terminal_handlers();

        // Pratt handlers (simplified - will need full implementation)
        self.emit_indexed_pratt_handlers();

        self.indent -= 1;
        self.line("}");
        self.line("Ok(())");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_seq_handlers(&mut self) {
        // SeqStart
        self.line("Work::SeqStart { seq_id, result_base } => {");
        self.indent += 1;
        self.line("if let Some(items) = SEQUENCES.get(seq_id as usize) {");
        self.indent += 1;
        self.line("let seq_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::SeqComplete { seq_id, result_base, seq_base });");
        self.line("// Push steps in reverse order");
        self.line("for i in (1..items.len()).rev() {");
        self.indent += 1;
        self.line(
            "self.work_stack.push(Work::SeqStep { seq_id, step: i as u8, result_base, seq_base });",
        );
        self.indent -= 1;
        self.line("}");
        self.line("// Dispatch first item");
        self.line("if let Some(&first) = items.first() {");
        self.indent += 1;
        self.line("self.dispatch_combref(first, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SeqStep
        self.line("Work::SeqStep { seq_id, step, result_base, seq_base } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("if let Some(items) = SEQUENCES.get(seq_id as usize) {");
        self.indent += 1;
        self.line("if let Some(&item) = items.get(step as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SeqComplete
        self.line("Work::SeqComplete { seq_id: _, result_base: _, seq_base } => {");
        self.indent += 1;
        self.line("if self.last_error.is_none() {");
        self.indent += 1;
        self.line("let results: Vec<_> = self.result_stack.drain(seq_base..).collect();");
        self.line("self.result_stack.push(ParseResult::List(results));");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.result_stack.truncate(seq_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_choice_handlers(&mut self) {
        // ChoiceStart
        self.line("Work::ChoiceStart { choice_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let stack_base = self.result_stack.len();");
        self.line("if let Some(alts) = CHOICES.get(choice_id as usize) {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::ChoiceTry { choice_id, alt: 0, result_base, checkpoint, checkpoint_line, checkpoint_column, stack_base });");
        self.line("if let Some(&first) = alts.first() {");
        self.indent += 1;
        self.line("self.dispatch_combref(first, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // ChoiceTry
        self.line("Work::ChoiceTry { choice_id, alt, result_base, checkpoint, checkpoint_line, checkpoint_column, stack_base } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("self.result_stack.truncate(stack_base);");
        self.line("if let Some(alts) = CHOICES.get(choice_id as usize) {");
        self.indent += 1;
        self.line("let next_alt = alt + 1;");
        self.line("if let Some(&next_comb) = alts.get(next_alt as usize) {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::ChoiceTry { choice_id, alt: next_alt, result_base, checkpoint, checkpoint_line, checkpoint_column, stack_base });");
        self.line("self.dispatch_combref(next_comb, result_base);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"no alternative matched\"));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("// If no error, result is already on stack");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_zero_or_more_handlers(&mut self) {
        // ZeroOrMoreStart
        self.line("Work::ZeroOrMoreStart { loop_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let loop_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::ZeroOrMoreLoop { loop_id, result_base, loop_base, iter_base: loop_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("if let Some(&item) = ZERO_OR_MORE.get(loop_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // ZeroOrMoreLoop
        self.line("Work::ZeroOrMoreLoop { loop_id, result_base, loop_base, iter_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("self.result_stack.truncate(iter_base);");
        self.line(
            "self.work_stack.push(Work::ZeroOrMoreComplete { loop_id, result_base, loop_base });",
        );
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let iter_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::ZeroOrMoreLoop { loop_id, result_base, loop_base, iter_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("if let Some(&item) = ZERO_OR_MORE.get(loop_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // ZeroOrMoreComplete
        self.line("Work::ZeroOrMoreComplete { loop_id: _, result_base: _, loop_base } => {");
        self.indent += 1;
        self.line("let results: Vec<_> = self.result_stack.drain(loop_base..).collect();");
        self.line("self.result_stack.push(ParseResult::List(results));");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_one_or_more_handlers(&mut self) {
        // OneOrMoreStart
        self.line("Work::OneOrMoreStart { loop_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let loop_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::OneOrMoreLoop { loop_id, result_base, loop_base, checkpoint, checkpoint_line, checkpoint_column, count: 0 });");
        self.line("if let Some(&item) = ONE_OR_MORE.get(loop_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // OneOrMoreLoop
        self.line("Work::OneOrMoreLoop { loop_id, result_base, loop_base, checkpoint, checkpoint_line, checkpoint_column, count } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("if count == 0 {");
        self.indent += 1;
        self.line("// First item failed - propagate error");
        self.line("return Ok(());");
        self.indent -= 1;
        self.line("}");
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line(
            "self.work_stack.push(Work::OneOrMoreComplete { loop_id, result_base, loop_base });",
        );
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("self.work_stack.push(Work::OneOrMoreLoop { loop_id, result_base, loop_base, checkpoint, checkpoint_line, checkpoint_column, count: count + 1 });");
        self.line("if let Some(&item) = ONE_OR_MORE.get(loop_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // OneOrMoreComplete
        self.line("Work::OneOrMoreComplete { loop_id: _, result_base: _, loop_base } => {");
        self.indent += 1;
        self.line("let results: Vec<_> = self.result_stack.drain(loop_base..).collect();");
        self.line("self.result_stack.push(ParseResult::List(results));");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_optional_handlers(&mut self) {
        // OptionalStart
        self.line("Work::OptionalStart { opt_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let opt_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::OptionalCheck { opt_id, result_base, opt_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("if let Some(&inner) = OPTIONALS.get(opt_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // OptionalCheck
        self.line("Work::OptionalCheck { opt_id: _, result_base: _, opt_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("self.result_stack.truncate(opt_base);");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("}");
        self.line("// If no error, result is already on stack");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_capture_handlers(&mut self) {
        // CaptureStart
        self.line("Work::CaptureStart { cap_id, result_base } => {");
        self.indent += 1;
        self.line("let start_pos = self.pos;");
        self.line("let start_line = self.line;");
        self.line("let start_column = self.column;");
        self.line("let capture_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::CaptureComplete { cap_id, result_base, capture_base, start_pos, start_line, start_column });");
        self.line("if let Some(&inner) = CAPTURES.get(cap_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // CaptureComplete
        self.line("Work::CaptureComplete { cap_id: _, result_base: _, capture_base, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_none() {");
        self.indent += 1;
        self.line("self.result_stack.truncate(capture_base);");
        self.line("let text = self.text_result(start_pos, self.pos);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.line("self.result_stack.push(ParseResult::Text(text, span));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_lookahead_handlers(&mut self) {
        // NotFollowedByStart
        self.line("Work::NotFollowedByStart { look_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let look_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::NotFollowedByCheck { look_id, result_base, look_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("if let Some(&inner) = NOT_FOLLOWED_BY.get(look_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // NotFollowedByCheck
        self.line("Work::NotFollowedByCheck { look_id: _, result_base: _, look_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("self.result_stack.truncate(look_base);");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("// Inner failed = negative lookahead succeeded");
        self.line("self.last_error = None;");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("// Inner succeeded = negative lookahead failed");
        self.line(
            "self.last_error = Some(self.make_error(\"unexpected match in not-followed-by\"));",
        );
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // FollowedByStart
        self.line("Work::FollowedByStart { look_id, result_base } => {");
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("let look_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::FollowedByCheck { look_id, result_base, look_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("if let Some(&inner) = FOLLOWED_BY.get(look_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // FollowedByCheck
        self.line("Work::FollowedByCheck { look_id: _, result_base: _, look_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("self.result_stack.truncate(look_base);");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("if self.last_error.is_none() {");
        self.indent += 1;
        self.line("// Inner succeeded = positive lookahead succeeded");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("}");
        self.line("// If error, propagate it");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_skip_handlers(&mut self) {
        // SkipStart
        self.line("Work::SkipStart { skip_id, result_base } => {");
        self.indent += 1;
        self.line("let skip_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::SkipComplete { skip_id, result_base, skip_base });");
        self.line("if let Some(&inner) = SKIPS.get(skip_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SkipComplete
        self.line("Work::SkipComplete { skip_id: _, result_base: _, skip_base } => {");
        self.indent += 1;
        self.line("if self.last_error.is_none() {");
        self.indent += 1;
        self.line("self.result_stack.truncate(skip_base);");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_separated_by_handlers(&mut self) {
        // SepByStart
        self.line("Work::SepByStart { sepby_id, result_base } => {");
        self.indent += 1;
        self.line("if let Some(&(item, _sep, _trailing)) = SEPARATED_BY.get(sepby_id as usize) {");
        self.indent += 1;
        self.line("let list_base = self.result_stack.len();");
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("self.work_stack.push(Work::SepBySep { sepby_id, result_base, list_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SepBySep
        self.line("Work::SepBySep { sepby_id, result_base, list_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line(
            "self.work_stack.push(Work::SepByComplete { sepby_id, result_base, list_base });",
        );
        self.indent -= 1;
        self.line(
            "} else if let Some(&(_item, sep, _trailing)) = SEPARATED_BY.get(sepby_id as usize) {",
        );
        self.indent += 1;
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("self.work_stack.push(Work::SepByItem { sepby_id, result_base, list_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("self.dispatch_combref(sep, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SepByItem
        self.line("Work::SepByItem { sepby_id, result_base, list_base, checkpoint, checkpoint_line, checkpoint_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.last_error = None;");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("// Separator failed - no separator result was pushed, so nothing to pop");
        self.line(
            "self.work_stack.push(Work::SepByComplete { sepby_id, result_base, list_base });",
        );
        self.indent -= 1;
        self.line(
            "} else if let Some(&(item, _sep, _trailing)) = SEPARATED_BY.get(sepby_id as usize) {",
        );
        self.indent += 1;
        self.line("// Pop separator result (we don't keep it)");
        self.line("self.result_stack.pop();");
        self.line("let checkpoint = self.pos;");
        self.line("let checkpoint_line = self.line;");
        self.line("let checkpoint_column = self.column;");
        self.line("self.work_stack.push(Work::SepBySep { sepby_id, result_base, list_base, checkpoint, checkpoint_line, checkpoint_column });");
        self.line("self.dispatch_combref(item, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // SepByComplete
        self.line("Work::SepByComplete { sepby_id: _, result_base: _, list_base } => {");
        self.indent += 1;
        self.line("let results: Vec<_> = self.result_stack.drain(list_base..).collect();");
        self.line("self.result_stack.push(ParseResult::List(results));");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_mapped_handlers(&mut self) {
        // MappedStart
        self.line("Work::MappedStart { map_id, result_base } => {");
        self.indent += 1;
        self.line("let start_pos = self.pos;");
        self.line("let start_line = self.line;");
        self.line("let start_column = self.column;");
        self.line("let map_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::MappedApply { map_id, result_base, map_base, start_pos, start_line, start_column });");
        self.line("if let Some(&(inner, _)) = MAPPED.get(map_id as usize) {");
        self.indent += 1;
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // MappedApply - apply the mapping function
        self.line("Work::MappedApply { map_id, result_base, map_base, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("// Inner combinator failed, propagate error");
        self.indent -= 1;
        self.line("} else if let Some(inner_result) = self.result_stack.get(map_base).cloned() {");
        self.indent += 1;
        self.line("// Remove inner result and apply mapping");
        self.line("self.result_stack.truncate(map_base);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.line("let mapped = self.apply_mapping(map_id, inner_result, span);");
        self.line("match mapped {");
        self.indent += 1;
        self.line("Ok(result) => self.result_stack.push(result),");
        self.line("Err(e) => self.last_error = Some(e),");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_memoize_handlers(&mut self) {
        // MemoStart
        self.line("Work::MemoStart { memo_id, result_base } => {");
        self.indent += 1;
        self.line("if let Some(&(memo_key, inner)) = MEMOIZED.get(memo_id as usize) {");
        self.indent += 1;
        self.line("let key = (memo_key, self.pos);");
        self.line("if let Some(cached) = self.memo.get(&key) {");
        self.indent += 1;
        self.line("if let Some((result, end_pos, end_line, end_column)) = cached.clone() {");
        self.indent += 1;
        self.line("self.pos = end_pos;");
        self.line("self.line = end_line;");
        self.line("self.column = end_column;");
        self.line("self.result_stack.push(result);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"memoized failure\"));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("let start_pos = self.pos;");
        self.line("let inner_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::MemoComplete { memo_id, result_base, start_pos, inner_base });");
        self.line("self.dispatch_combref(inner, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // MemoComplete
        self.line("Work::MemoComplete { memo_id, result_base: _, start_pos, inner_base } => {");
        self.indent += 1;
        self.line("if let Some(&(memo_key, _)) = MEMOIZED.get(memo_id as usize) {");
        self.indent += 1;
        self.line("let key = (memo_key, start_pos);");
        self.line("if self.last_error.is_some() {");
        self.indent += 1;
        self.line("self.memo.insert(key, None);");
        self.indent -= 1;
        self.line("} else if let Some(result) = self.result_stack.get(inner_base).cloned() {");
        self.indent += 1;
        self.line("self.memo.insert(key, Some((result, self.pos, self.line, self.column)));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_terminal_handlers(&mut self) {
        // Literal
        self.line("Work::Literal { lit_id, result_base: _ } => {");
        self.indent += 1;
        self.line("if let Some(&lit) = LITERALS.get(lit_id as usize) {");
        self.indent += 1;
        self.line("if !self.match_literal(lit) {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"expected '{}'\", lit)));");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // CharClass
        self.line("Work::CharClass { class, result_base: _ } => {");
        self.indent += 1;
        self.line("let matched = match class {");
        self.indent += 1;
        self.line("0 => self.match_char_class(|c: char| c.is_ascii_digit()),");
        self.line("1 => self.match_char_class(|c: char| c.is_ascii_hexdigit()),");
        self.line("2 => self.match_char_class(|c: char| c.is_ascii_alphabetic()),");
        self.line("3 => self.match_char_class(|c: char| c.is_ascii_alphanumeric()),");
        self.line("4 => self.match_char_class(|c: char| matches!(c, ' ' | '\\t' | '\\x0B' | '\\x0C' | '\\r' | '\\n' | '\\u{00A0}' | '\\u{FEFF}' | '\\u{2028}' | '\\u{2029}') || c.is_whitespace()),");
        self.line("5 => self.match_char_class(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$'),");
        self.line("6 => self.match_char_class(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$'),");
        self.line("_ => None,");
        self.indent -= 1;
        self.line("};");
        self.line("if matched.is_none() {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"expected character class\"));");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // CharRange
        self.line("Work::CharRange { from, to, result_base: _ } => {");
        self.indent += 1;
        self.line("if let Some(c) = self.current_char() {");
        self.indent += 1;
        self.line("if c >= from && c <= to {");
        self.indent += 1;
        self.line("self.advance();");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"expected char in range '{}'..='{}'\", from, to)));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"unexpected end of input\"));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Char
        self.line("Work::Char { ch, result_base: _ } => {");
        self.indent += 1;
        self.line("if self.current_char() == Some(ch) {");
        self.indent += 1;
        self.line("self.advance();");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"expected '{}'\", ch)));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // AnyChar
        self.line("Work::AnyChar { result_base: _ } => {");
        self.indent += 1;
        self.line("if self.current_char().is_some() {");
        self.indent += 1;
        self.line("self.advance();");
        self.line("self.result_stack.push(ParseResult::None);");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"unexpected end of input\"));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_indexed_pratt_handlers(&mut self) {
        if self.index.pratts.is_empty() {
            // No Pratt parsers - add unreachable handlers for all Pratt work variants
            self.line("// No Pratt parsers in this grammar - unreachable handlers");
            self.line("Work::PrattStart { .. } => {}");
            self.line("Work::PrattParseOperand { .. } => {}");
            self.line("Work::PrattAfterPrefix { .. } => {}");
            self.line("Work::PrattAfterPrefixLeadingRule { .. } => {}");
            self.line("Work::PrattCheckPostfix { .. } => {}");
            self.line("Work::PrattAfterPostfixSimple { .. } => {}");
            self.line("Work::PrattPostfixCallArg { .. } => {}");
            self.line("Work::PrattPostfixCallSep { .. } => {}");
            self.line("Work::PrattAfterPostfixCall { .. } => {}");
            self.line("Work::PrattAfterPostfixIndex { .. } => {}");
            self.line("Work::PrattAfterPostfixMember { .. } => {}");
            self.line("Work::PrattAfterPostfixRule { .. } => {}");
            self.line("Work::PrattAfterOperand { .. } => {}");
            self.line("Work::PrattAfterInfix { .. } => {}");
            self.line("Work::PrattAfterInfixLeadingRule { .. } => {}");
            self.line("Work::PrattAfterTernaryFirst { .. } => {}");
            self.line("Work::PrattAfterTernarySecond { .. } => {}");
            self.blank();
            return;
        }

        self.line("// === Pratt parsing handlers ===");
        self.blank();

        // PrattStart - begin Pratt parsing with min precedence 0
        self.line("Work::PrattStart { pratt_id, result_base } => {");
        self.indent += 1;
        self.line("let start_pos = self.pos;");
        self.line("let start_line = self.line;");
        self.line("let start_column = self.column;");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base, min_prec: 0, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // PrattParseOperand - try prefix operators, then parse operand
        self.emit_pratt_parse_operand_handler();

        // PrattAfterPrefix - apply prefix mapping, continue loop
        self.emit_pratt_after_prefix_handler();

        // PrattAfterPrefixLeadingRule
        self.emit_pratt_after_prefix_leading_rule_handler();

        // PrattCheckPostfix
        self.emit_pratt_check_postfix_handler();

        // PrattAfterPostfixSimple
        self.emit_pratt_after_postfix_simple_handler();

        // PrattPostfixCallArg
        self.emit_pratt_postfix_call_arg_handler();

        // PrattPostfixCallSep
        self.emit_pratt_postfix_call_sep_handler();

        // PrattAfterPostfixCall
        self.emit_pratt_after_postfix_call_handler();

        // PrattAfterPostfixIndex
        self.emit_pratt_after_postfix_index_handler();

        // PrattAfterPostfixMember
        self.emit_pratt_after_postfix_member_handler();

        // PrattAfterPostfixRule
        self.emit_pratt_after_postfix_rule_handler();

        // PrattAfterOperand - check for infix operators
        self.emit_pratt_after_operand_handler();

        // PrattAfterInfix
        self.emit_pratt_after_infix_handler();

        // PrattAfterInfixLeadingRule
        self.emit_pratt_after_infix_leading_rule_handler();

        // PrattAfterTernaryFirst
        self.emit_pratt_after_ternary_first_handler();

        // PrattAfterTernarySecond
        self.emit_pratt_after_ternary_second_handler();
    }

    fn emit_pratt_parse_operand_handler(&mut self) {
        self.line("Work::PrattParseOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let prefix_checkpoint = self.pos;");
        self.line("let prefix_checkpoint_line = self.line;");
        self.line("let prefix_checkpoint_column = self.column;");
        self.line("let mut prefix_matched = false;");
        self.blank();

        // Try prefix operators without leading rules
        self.line("// Try prefix operators without leading rules");
        self.line("for (op_idx, prefix_op) in pratt.prefix_ops.iter().enumerate() {");
        self.indent += 1;
        self.line("if prefix_matched { break; }");
        self.line("if prefix_op.leading_rule.is_some() { continue; }");
        self.line("if prefix_op.literal.is_empty() { continue; }");
        self.blank();
        self.line("// Check not_followed_by patterns");
        self.line("let mut can_match = self.input.get(self.pos..).unwrap_or(\"\").starts_with(prefix_op.literal);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in prefix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", prefix_op.literal, nfb);");
        self.line("if self.input.get(self.pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos += prefix_op.literal.len();");
        self.line("self.column += prefix_op.literal.len() as u32;");
        self.blank();
        self.line("// Keyword boundary check");
        self.line("if prefix_op.is_keyword {");
        self.indent += 1;
        self.line("let boundary_ok = self.current_char().map_or(true, |c| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'));");
        self.line("if !boundary_ok {");
        self.indent += 1;
        self.line("self.pos = prefix_checkpoint;");
        self.line("self.line = prefix_checkpoint_line;");
        self.line("self.column = prefix_checkpoint_column;");
        self.line("continue;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("prefix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPrefix { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: prefix_op.precedence, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Check for prefix operators with leading rules
        self.line("// Try prefix operators with leading rules");
        self.line("if !prefix_matched && pratt.has_prefix_with_leading {");
        self.indent += 1;
        self.line("// Find first leading rule and parse it");
        self.line(
            "if let Some(rule_id) = pratt.prefix_ops.iter().find_map(|op| op.leading_rule) {",
        );
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterPrefixLeadingRule { pratt_id, result_base, min_prec, checkpoint: prefix_checkpoint, checkpoint_line: prefix_checkpoint_line, checkpoint_column: prefix_checkpoint_column, start_pos, start_line, start_column });");
        self.line(
            "self.work_stack.push(Work::Rule { rule_id, result_base: self.result_stack.len() });",
        );
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else if !prefix_matched {");
        self.indent += 1;
        // No prefix matched - parse operand directly
        self.line("// No prefix - parse operand directly");
        self.line("if !pratt.postfix_ops.is_empty() {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.line("if let Some(operand_ref) = pratt.operand {");
        self.indent += 1;
        self.line("self.dispatch_combref(operand_ref, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_prefix_handler(&mut self) {
        self.line("Work::PrattAfterPrefix { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let operand = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.blank();

        // Generate match on (pratt_id, op_idx) to call mapping
        self.emit_prefix_mapping_dispatch();

        self.line("// Continue with postfix/infix loop");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("if !pratt.postfix_ops.is_empty() {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_prefix_mapping_dispatch(&mut self) {
        // Pre-collect mapping arms to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, prefix_op) in pratt.prefix_ops.iter().enumerate() {
                let mapping = self.index.mappings[prefix_op.mapping_idx].clone();
                arms.push((pratt_idx, op_idx, mapping));
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(operand, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(operand); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_prefix_leading_rule_handler(&mut self) {
        self.line("Work::PrattAfterPrefixLeadingRule { pratt_id, result_base, min_prec, checkpoint, checkpoint_line, checkpoint_column, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let _ = self.result_stack.pop(); // Discard leading rule result");
        self.line("self.last_error = None;");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let mut prefix_matched = false;");
        self.blank();

        self.line("// Try prefix operators with leading rules");
        self.line("for (op_idx, prefix_op) in pratt.prefix_ops.iter().enumerate() {");
        self.indent += 1;
        self.line("if prefix_matched { break; }");
        self.line("if prefix_op.leading_rule.is_none() { continue; }");
        self.line("if prefix_op.literal.is_empty() { continue; }");
        self.blank();
        self.line("let mut can_match = self.input.get(self.pos..).unwrap_or(\"\").starts_with(prefix_op.literal);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in prefix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", prefix_op.literal, nfb);");
        self.line("if self.input.get(self.pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos += prefix_op.literal.len();");
        self.line("self.column += prefix_op.literal.len() as u32;");
        self.line("if prefix_op.is_keyword {");
        self.indent += 1;
        self.line("if !self.current_char().map_or(true, |c| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')) {");
        self.indent += 1;
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("continue;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("prefix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPrefix { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: prefix_op.precedence, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("if !prefix_matched {");
        self.indent += 1;
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.line("if !pratt.postfix_ops.is_empty() {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.line("if let Some(operand_ref) = pratt.operand {");
        self.indent += 1;
        self.line("self.dispatch_combref(operand_ref, result_base);");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_check_postfix_handler(&mut self) {
        self.line("Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let postfix_checkpoint = self.pos;");
        self.line("let mut postfix_matched = false;");
        self.blank();

        self.line("for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {");
        self.indent += 1;
        self.line("if postfix_matched { break; }");
        self.line("if postfix_op.precedence < min_prec { continue; }");
        self.blank();
        self.line("match postfix_op.kind {");
        self.indent += 1;

        // Simple postfix
        self.line("PostfixKind::Simple => {");
        self.indent += 1;
        self.line("if postfix_op.open_lit.is_empty() { continue; }");
        self.line("let mut can_match = self.input.get(self.pos..).unwrap_or(\"\").starts_with(postfix_op.open_lit);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in postfix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", postfix_op.open_lit, nfb);");
        self.line("if self.input.get(self.pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos += postfix_op.open_lit.len();");
        self.line("self.column += postfix_op.open_lit.len() as u32;");
        self.line("postfix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPostfixSimple { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");

        // Call postfix
        self.line("PostfixKind::Call => {");
        self.indent += 1;
        self.line("if self.try_consume(postfix_op.open_lit) {");
        self.indent += 1;
        self.line("postfix_matched = true;");
        self.line("let args_base = self.result_stack.len();");
        self.line("self.work_stack.push(Work::PrattPostfixCallArg { pratt_id, result_base, min_prec, op_idx: op_idx as u8, args_base, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");

        // Index postfix
        self.line("PostfixKind::Index => {");
        self.indent += 1;
        self.line("if self.try_consume(postfix_op.open_lit) {");
        self.indent += 1;
        self.line("postfix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPostfixIndex { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: 0, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");

        // Member postfix
        self.line("PostfixKind::Member => {");
        self.indent += 1;
        self.line("if postfix_op.open_lit.is_empty() { continue; }");
        self.line("let mut can_match = self.input.get(self.pos..).unwrap_or(\"\").starts_with(postfix_op.open_lit);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in postfix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", postfix_op.open_lit, nfb);");
        self.line("if self.input.get(self.pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos += postfix_op.open_lit.len();");
        self.line("self.column += postfix_op.open_lit.len() as u32;");
        self.line("postfix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPostfixMember { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");

        // Rule postfix
        self.line("PostfixKind::Rule => {");
        self.indent += 1;
        self.line("if let Some(rule_id) = postfix_op.rule_name_id {");
        self.indent += 1;
        self.line("// For rule-based postfix like tagged templates, check start char");
        self.line("if self.current_char() == Some('`') {");
        self.indent += 1;
        self.line("postfix_matched = true;");
        self.line("self.work_stack.push(Work::PrattAfterPostfixRule { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.line(
            "self.work_stack.push(Work::Rule { rule_id, result_base: self.result_stack.len() });",
        );
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");

        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("if !postfix_matched {");
        self.indent += 1;
        self.line("self.pos = postfix_checkpoint;");
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_postfix_simple_handler(&mut self) {
        self.line("Work::PrattAfterPostfixSimple { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let operand = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_postfix_simple_mapping_dispatch();
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_postfix_simple_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {
                if let CompiledPostfixOp::Simple { mapping_idx, .. } = postfix_op {
                    let mapping = self.index.mappings[*mapping_idx].clone();
                    arms.push((pratt_idx, op_idx, mapping));
                }
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(operand, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(operand); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_postfix_call_arg_handler(&mut self) {
        self.line("Work::PrattPostfixCallArg { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let postfix_op = &pratt.postfix_ops[op_idx as usize];");
        self.line("if self.try_consume(postfix_op.close_lit) {");
        self.indent += 1;
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_whitespace()) { self.advance(); }");
        self.line("self.work_stack.push(Work::PrattAfterPostfixCall { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattPostfixCallSep { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column });");
        self.line("if let Some(arg_rule) = postfix_op.arg_rule {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::Rule { rule_id: arg_rule, result_base: self.result_stack.len() });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: 0, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_postfix_call_sep_handler(&mut self) {
        self.line("Work::PrattPostfixCallSep { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let postfix_op = &pratt.postfix_ops[op_idx as usize];");
        self.line("if self.try_consume(postfix_op.sep_lit) {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattPostfixCallSep { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column });");
        self.line("if let Some(arg_rule) = postfix_op.arg_rule {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::Rule { rule_id: arg_rule, result_base: self.result_stack.len() });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: 0, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else if self.try_consume(postfix_op.close_lit) {");
        self.indent += 1;
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_whitespace()) { self.advance(); }");
        self.line("self.work_stack.push(Work::PrattAfterPostfixCall { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"expected '{}' or '{}'\", postfix_op.sep_lit, postfix_op.close_lit)));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_postfix_call_handler(&mut self) {
        self.line("Work::PrattAfterPostfixCall { pratt_id, result_base, min_prec, op_idx, args_base, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let args: Vec<ParseResult> = self.result_stack.drain(args_base..).collect();");
        self.line("let callee = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_postfix_call_mapping_dispatch();
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_postfix_call_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {
                if let CompiledPostfixOp::Call { mapping_idx, .. } = postfix_op {
                    let mapping = self.index.mappings[*mapping_idx].clone();
                    arms.push((pratt_idx, op_idx, mapping));
                }
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(callee, args, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(callee); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_postfix_index_handler(&mut self) {
        self.line("Work::PrattAfterPostfixIndex { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let postfix_op = &pratt.postfix_ops[op_idx as usize];");
        self.line("if !self.try_consume(postfix_op.close_lit) {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"expected '{}'\", postfix_op.close_lit)));");
        self.line("return Ok(());");
        self.indent -= 1;
        self.line("}");
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_whitespace()) { self.advance(); }");
        self.line("let index_expr = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let obj = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_postfix_index_mapping_dispatch();
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_postfix_index_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {
                if let CompiledPostfixOp::Index { mapping_idx, .. } = postfix_op {
                    let mapping = self.index.mappings[*mapping_idx].clone();
                    arms.push((pratt_idx, op_idx, mapping));
                }
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(obj, index_expr, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(obj); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_postfix_member_handler(&mut self) {
        self.line("Work::PrattAfterPostfixMember { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let ident_start = self.pos;");
        self.line("let is_ident_start = self.current_char().map_or(false, |c| c.is_ascii_alphabetic() || c == '_' || c == '$' || c == '#');");
        self.line("let is_unicode_escape = self.input.get(self.pos..).map_or(false, |s| s.starts_with(\"\\\\u\"));");
        self.line("if is_ident_start || is_unicode_escape {");
        self.indent += 1;
        self.line("// Parse identifier");
        self.line("if is_unicode_escape {");
        self.indent += 1;
        self.line("self.advance(); self.advance(); // Skip \\u");
        self.line("if self.current_char() == Some('{') {");
        self.indent += 1;
        self.line("// \\u{...} format");
        self.line("self.advance();");
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_hexdigit()) { self.advance(); }");
        self.line("if self.current_char() == Some('}') { self.advance(); }");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("// \\uXXXX format");
        self.line("for _ in 0..4 { if self.current_char().map_or(false, |c| c.is_ascii_hexdigit()) { self.advance(); } }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("else { self.advance(); }");
        self.line("loop {");
        self.indent += 1;
        self.line("if self.current_char().map_or(false, |c| c.is_ascii_alphanumeric() || c == '_' || c == '$') { self.advance(); }");
        self.line(
            "else if self.input.get(self.pos..).map_or(false, |s| s.starts_with(\"\\\\u\")) {",
        );
        self.indent += 1;
        self.line("// Unicode escape in identifier continuation");
        self.line("self.advance(); self.advance(); // Skip \\u");
        self.line("if self.current_char() == Some('{') {");
        self.indent += 1;
        self.line("// \\u{...} format");
        self.line("self.advance();");
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_hexdigit()) { self.advance(); }");
        self.line("if self.current_char() == Some('}') { self.advance(); }");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("// \\uXXXX format");
        self.line("for _ in 0..4 { if self.current_char().map_or(false, |c| c.is_ascii_hexdigit()) { self.advance(); } }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("} else { break; }");
        self.indent -= 1;
        self.line("}");
        self.line("let prop_name = self.text_result(ident_start, self.pos);");
        self.line("while self.current_char().map_or(false, |c| c.is_ascii_whitespace()) { self.advance(); }");
        self.line("let obj = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_postfix_member_mapping_dispatch();
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(\"expected identifier\"));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_postfix_member_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {
                if let CompiledPostfixOp::Member { mapping_idx, .. } = postfix_op {
                    let mapping = self.index.mappings[*mapping_idx].clone();
                    arms.push((pratt_idx, op_idx, mapping));
                }
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(obj, prop_name, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(obj); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_postfix_rule_handler(&mut self) {
        self.line("Work::PrattAfterPostfixRule { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let rule_result = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let obj = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_postfix_rule_mapping_dispatch();
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_postfix_rule_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, postfix_op) in pratt.postfix_ops.iter().enumerate() {
                if let CompiledPostfixOp::Rule { mapping_idx, .. } = postfix_op {
                    let mapping = self.index.mappings[*mapping_idx].clone();
                    arms.push((pratt_idx, op_idx, mapping));
                }
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(obj, rule_result, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(obj); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_operand_handler(&mut self) {
        self.line("Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let infix_checkpoint = self.pos;");
        self.blank();

        // Try simple infix operators first
        self.line("// Try simple infix operators (no leading rule)");
        self.line("for (op_idx, infix_op) in pratt.infix_ops.iter().enumerate() {");
        self.indent += 1;
        self.line("if self.pos != infix_checkpoint { break; } // Already matched one");
        self.line("if infix_op.precedence < min_prec { continue; }");
        self.line("if infix_op.leading_rule.is_some() { continue; }");
        self.line("if infix_op.literal.is_empty() { continue; }");
        self.blank();
        self.line("let mut can_match = self.input.get(self.pos..).unwrap_or(\"\").starts_with(infix_op.literal);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in infix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", infix_op.literal, nfb);");
        self.line("if self.input.get(self.pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos += infix_op.literal.len();");
        self.line("self.column += infix_op.literal.len() as u32;");
        self.line("if infix_op.is_keyword {");
        self.indent += 1;
        self.line("if !self.current_char().map_or(true, |c| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')) {");
        self.indent += 1;
        self.line("self.pos = infix_checkpoint;");
        self.line("continue;");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("let next_prec = if infix_op.is_left_assoc { infix_op.precedence + 1 } else { infix_op.precedence };");
        self.line("self.work_stack.push(Work::PrattAfterInfix { pratt_id, result_base, min_prec, op_idx: op_idx as u8, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: next_prec, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Try ternary operator
        self.line("// Try ternary operator");
        self.line("if self.pos == infix_checkpoint {");
        self.indent += 1;
        self.line("if let Some(ref ternary) = pratt.ternary {");
        self.indent += 1;
        self.line("if ternary.precedence >= min_prec {");
        self.indent += 1;
        self.line("// Check for ternary but not ?. or ??");
        self.line("let rest = self.input.get(self.pos..).unwrap_or(\"\");");
        self.line("if rest.starts_with(ternary.first_lit) && !rest.starts_with(\"?.\") && !rest.starts_with(\"??\") {");
        self.indent += 1;
        self.line("self.pos += ternary.first_lit.len();");
        self.line("self.column += ternary.first_lit.len() as u32;");
        self.line("self.work_stack.push(Work::PrattAfterTernaryFirst { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: 0, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Try infix with leading rule
        self.line("// Try infix operators with leading rule");
        self.line("if self.pos == infix_checkpoint && pratt.has_infix_with_leading {");
        self.indent += 1;
        self.line("// Find max precedence of infix ops with leading rule");
        self.line("let max_prec = pratt.infix_ops.iter().filter(|op| op.leading_rule.is_some()).map(|op| op.precedence).max().unwrap_or(0);");
        self.line("if max_prec >= min_prec {");
        self.indent += 1;
        self.line("if let Some((op_idx, infix_op)) = pratt.infix_ops.iter().enumerate().find(|(_, op)| op.leading_rule.is_some()) {");
        self.indent += 1;
        self.line("let next_prec = if infix_op.is_left_assoc { infix_op.precedence + 1 } else { infix_op.precedence };");
        self.line("self.work_stack.push(Work::PrattAfterInfixLeadingRule { pratt_id, result_base, min_prec, op_idx: op_idx as u8, next_prec, checkpoint: infix_checkpoint, checkpoint_line: self.line, checkpoint_column: self.column, start_pos, start_line, start_column });");
        self.line("if let Some(rule_id) = infix_op.leading_rule {");
        self.indent += 1;
        self.line(
            "self.work_stack.push(Work::Rule { rule_id, result_base: self.result_stack.len() });",
        );
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_infix_handler(&mut self) {
        self.line("Work::PrattAfterInfix { pratt_id, result_base, min_prec, op_idx, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let right = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let left = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_infix_mapping_dispatch();
        self.line("// Continue with postfix/infix loop");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("if !pratt.postfix_ops.is_empty() {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_infix_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            for (op_idx, infix_op) in pratt.infix_ops.iter().enumerate() {
                let mapping = self.index.mappings[infix_op.mapping_idx].clone();
                arms.push((pratt_idx, op_idx, mapping));
            }
        }

        self.line("match (pratt_id, op_idx) {");
        self.indent += 1;

        for (pratt_idx, op_idx, mapping) in arms {
            self.line(&format!("({}, {}) => {{", pratt_idx, op_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(left, right, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(left); }");
        self.indent -= 1;
        self.line("}");
    }

    fn emit_pratt_after_infix_leading_rule_handler(&mut self) {
        self.line("Work::PrattAfterInfixLeadingRule { pratt_id, result_base, min_prec, op_idx: _, next_prec: _, checkpoint, checkpoint_line, checkpoint_column, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("let _ = self.result_stack.pop(); // Discard leading rule result");
        self.line("self.last_error = None;");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("let after_ws_pos = self.pos;");
        self.blank();
        self.line("// Try all infix operators with leading rules");
        self.line("for (actual_op_idx, infix_op) in pratt.infix_ops.iter().enumerate() {");
        self.indent += 1;
        self.line("if infix_op.leading_rule.is_none() { continue; }");
        self.line("if infix_op.precedence < min_prec { continue; }");
        self.line("if infix_op.literal.is_empty() { continue; }");
        self.blank();
        self.line("let mut can_match = self.input.get(after_ws_pos..).unwrap_or(\"\").starts_with(infix_op.literal);");
        self.line("if can_match {");
        self.indent += 1;
        self.line("for nfb in infix_op.not_followed_by {");
        self.indent += 1;
        self.line("let combined = format!(\"{}{}\", infix_op.literal, nfb);");
        self.line("if self.input.get(after_ws_pos..).unwrap_or(\"\").starts_with(&combined) { can_match = false; break; }");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("if can_match {");
        self.indent += 1;
        self.line("self.pos = after_ws_pos + infix_op.literal.len();");
        self.line("self.column += infix_op.literal.len() as u32;");
        self.line("if infix_op.is_keyword {");
        self.indent += 1;
        self.line("if !self.current_char().map_or(true, |c| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')) {");
        self.indent += 1;
        self.line("continue; // Not a valid keyword match, try next operator");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.line("let next_prec = if infix_op.is_left_assoc { infix_op.precedence + 1 } else { infix_op.precedence };");
        self.line("self.work_stack.push(Work::PrattAfterInfix { pratt_id, result_base, min_prec, op_idx: actual_op_idx as u8, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: next_prec, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.line("return Ok(());");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
        self.line("// No operator matched - restore checkpoint");
        self.line("self.pos = checkpoint;");
        self.line("self.line = checkpoint_line;");
        self.line("self.column = checkpoint_column;");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_ternary_first_handler(&mut self) {
        self.line("Work::PrattAfterTernaryFirst { pratt_id, result_base, min_prec, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line(
            "while self.current_char().map_or(false, |c| c.is_whitespace()) { self.advance(); }",
        );
        self.line("if let Some(ref ternary) = pratt.ternary {");
        self.indent += 1;
        self.line("if self.try_consume(ternary.second_lit) {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterTernarySecond { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.line("self.work_stack.push(Work::PrattParseOperand { pratt_id, result_base: self.result_stack.len(), min_prec: ternary.precedence, start_pos: self.pos, start_line: self.line, start_column: self.column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.last_error = Some(self.make_error(&format!(\"Expected '{}' in ternary expression\", ternary.second_lit)));");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_pratt_after_ternary_second_handler(&mut self) {
        self.line("Work::PrattAfterTernarySecond { pratt_id, result_base, min_prec, start_pos, start_line, start_column } => {");
        self.indent += 1;
        self.line("if self.last_error.is_some() { return Ok(()); }");
        self.line("let alternate = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let consequent = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let test = self.result_stack.pop().unwrap_or(ParseResult::None);");
        self.line("let span = Span { start: start_pos, end: self.pos, line: start_line, column: start_column };");
        self.emit_ternary_mapping_dispatch();
        self.line("// Continue with postfix/infix loop");
        self.line("let pratt = &PRATTS[pratt_id as usize];");
        self.line("if !pratt.postfix_ops.is_empty() {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattCheckPostfix { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("} else {");
        self.indent += 1;
        self.line("self.work_stack.push(Work::PrattAfterOperand { pratt_id, result_base, min_prec, start_pos, start_line, start_column });");
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();
    }

    fn emit_ternary_mapping_dispatch(&mut self) {
        // Pre-collect using for loops to avoid borrow conflicts
        let mut arms: Vec<(usize, String)> = Vec::new();
        for (pratt_idx, pratt) in self.index.pratts.iter().enumerate() {
            if let Some(ternary) = &pratt.ternary {
                let mapping = self.index.mappings[ternary.mapping_idx].clone();
                arms.push((pratt_idx, mapping));
            }
        }

        self.line("match pratt_id {");
        self.indent += 1;

        for (pratt_idx, mapping) in arms {
            self.line(&format!("{} => {{", pratt_idx));
            self.indent += 1;
            self.line(&format!("let mapping_fn = {};", mapping));
            self.line("match mapping_fn(test, consequent, alternate, span) {");
            self.indent += 1;
            self.line("Ok(mapped) => self.result_stack.push(mapped),");
            self.line("Err(e) => self.last_error = Some(e),");
            self.indent -= 1;
            self.line("}");
            self.indent -= 1;
            self.line("}");
        }

        self.line("_ => { self.result_stack.push(test); }");
        self.indent -= 1;
        self.line("}");
    }

    /// Emit the indexed parser struct and methods
    fn emit_indexed_parser(&mut self) {
        let string_type = self
            .grammar
            .ast_config
            .string_type
            .as_deref()
            .unwrap_or("String");

        let has_string_dict = self.grammar.ast_config.string_dict_type.is_some();
        let string_dict_type = self
            .grammar
            .ast_config
            .string_dict_type
            .as_deref()
            .unwrap_or("StringDict");

        self.line("/// Scannerless parser (indexed dispatch)");
        self.line("pub struct Parser<'a> {");
        self.indent += 1;
        self.line("input: &'a str,");
        self.line("pos: usize,");
        self.line("line: u32,");
        self.line("column: u32,");
        self.line("work_stack: Vec<Work>,");
        self.line("result_stack: Vec<ParseResult>,");
        self.line("last_error: Option<ParseError>,");
        self.line(
            "memo: hashbrown::HashMap<(usize, usize), Option<(ParseResult, usize, u32, u32)>>,",
        );
        if has_string_dict {
            self.line(&format!("string_dict: &'a mut {},", string_dict_type));
        }
        self.indent -= 1;
        self.line("}");
        self.blank();

        self.line("impl<'a> Parser<'a> {");
        self.indent += 1;

        // Constructor
        if has_string_dict {
            self.line(&format!(
                "pub fn new(input: &'a str, string_dict: &'a mut {}) -> Self {{",
                string_dict_type
            ));
        } else {
            self.line("pub fn new(input: &'a str) -> Self {");
        }
        self.indent += 1;
        self.line("Self {");
        self.indent += 1;
        self.line("input,");
        self.line("pos: 0,");
        self.line("line: 1,");
        self.line("column: 1,");
        self.line("work_stack: Vec::new(),");
        self.line("result_stack: Vec::new(),");
        self.line("last_error: None,");
        self.line("memo: hashbrown::HashMap::new(),");
        if has_string_dict {
            self.line("string_dict,");
        }
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Parse entry point for first rule
        if !self.grammar.rules.is_empty() {
            self.line("/// Parse the input");
            self.line("pub fn parse(&mut self) -> Result<ParseResult, ParseError> {");
            self.indent += 1;
            self.line("let result_base = self.result_stack.len();");
            // Dispatch to first rule using indexed approach
            self.line("self.work_stack.push(Work::Rule { rule_id: 0, result_base });");
            self.line("self.run()?;");
            self.line("self.result_stack.pop().ok_or_else(|| ParseError {");
            self.indent += 1;
            self.line("message: \"No result\".to_string(),");
            self.line(
                "span: Span { start: 0, end: self.pos, line: self.line, column: self.column },",
            );
            self.indent -= 1;
            self.line("})");
            self.indent -= 1;
            self.line("}");
            self.blank();
        }

        // Helper methods
        self.emit_helper_methods(string_type);

        // Apply mapping method (for AST transformations)
        self.emit_apply_mapping_method();

        // Trampoline loop
        self.line("fn run(&mut self) -> Result<(), ParseError> {");
        self.indent += 1;
        self.line("while let Some(work) = self.work_stack.pop() {");
        self.indent += 1;
        self.line("self.execute(work)?;");
        self.indent -= 1;
        self.line("}");
        self.line("if let Some(err) = self.last_error.take() {");
        self.indent += 1;
        self.line("return Err(err);");
        self.indent -= 1;
        self.line("}");
        self.line("Ok(())");
        self.indent -= 1;
        self.line("}");
        self.blank();

        // Dispatch helper
        self.emit_dispatch_combref();

        // Execute method
        self.emit_indexed_execute();

        self.indent -= 1;
        self.line("}");
    }
}

/// Convert a CombRef to its generated code representation
fn combref_to_code(cref: &CombRef) -> String {
    match cref {
        CombRef::Rule(id) => format!("CombRef::Rule({})", id),
        CombRef::Seq(id) => format!("CombRef::Seq({})", id),
        CombRef::Choice(id) => format!("CombRef::Choice({})", id),
        CombRef::ZeroOrMore(id) => format!("CombRef::ZeroOrMore({})", id),
        CombRef::OneOrMore(id) => format!("CombRef::OneOrMore({})", id),
        CombRef::Optional(id) => format!("CombRef::Optional({})", id),
        CombRef::Literal(id) => format!("CombRef::Literal({})", id),
        CombRef::CharClass(class) => format!("CombRef::CharClass({})", *class as u8),
        CombRef::CharRange(from, to) => format!("CombRef::CharRange({:?}, {:?})", from, to),
        CombRef::Char(c) => format!("CombRef::Char({:?})", c),
        CombRef::AnyChar => "CombRef::AnyChar".to_string(),
        CombRef::Capture(id) => format!("CombRef::Capture({})", id),
        CombRef::NotFollowedBy(id) => format!("CombRef::NotFollowedBy({})", id),
        CombRef::FollowedBy(id) => format!("CombRef::FollowedBy({})", id),
        CombRef::Skip(id) => format!("CombRef::Skip({})", id),
        CombRef::SeparatedBy(id) => format!("CombRef::SeparatedBy({})", id),
        CombRef::Pratt(id) => format!("CombRef::Pratt({})", id),
        CombRef::Mapped(id) => format!("CombRef::Mapped({})", id),
        CombRef::Memoize(id) => format!("CombRef::Memoize({})", id),
    }
}

#[cfg(test)]
mod tests {
    use crate::Grammar;

    #[test]
    fn test_simple_grammar_generates() {
        let grammar = Grammar::new().rule("digit", |r| r.digit()).build();

        let code = grammar.generate();
        assert!(code.contains("pub struct Parser"));
        assert!(code.contains("enum Work"));
    }

    #[test]
    fn test_indexed_grammar_generates() {
        let grammar = Grammar::new()
            .rule("number", |r| r.capture(r.one_or_more(r.digit())))
            .rule("ws", |r| r.zero_or_more(r.lit(" ")))
            .build();

        let code = grammar.generate();

        // Should have the fixed Work enum with indexed variants
        assert!(code.contains("pub struct Parser"));
        assert!(code.contains("enum Work"));
        assert!(code.contains("enum CombRef"));

        // Should have static dispatch tables
        assert!(code.contains("static SEQUENCES:"));
        assert!(code.contains("static CAPTURES:"));
        assert!(code.contains("static LITERALS:"));
        assert!(code.contains("static RULES:"));

        // Should have indexed Work variants (not path-based ones)
        assert!(code.contains("SeqStart { seq_id:"));
        assert!(code.contains("CaptureStart { cap_id:"));

        // Should NOT have recursive path-based variants like NumberStart or WsStart
        // (The indexed approach doesn't generate rule-name-prefixed variants)
        assert!(!code.contains("NumberStart {"));
        assert!(!code.contains("WsStart {"));
    }

    #[test]
    fn test_indexed_variant_count() {
        // Verify the indexed approach produces a bounded number of Work variants
        let grammar = Grammar::new()
            .rule("expr", |r| {
                r.choice((
                    r.sequence((r.parse("number"), r.lit("+"), r.parse("expr"))),
                    r.sequence((r.parse("number"), r.lit("-"), r.parse("expr"))),
                    r.parse("number"),
                ))
            })
            .rule("number", |r| r.capture(r.one_or_more(r.digit())))
            .build();

        let code = grammar.generate();

        // Count Work variants by extracting the enum section
        fn count_work_variants(code: &str) -> usize {
            let start = code.find("enum Work {").unwrap_or(0);
            let end_search = &code[start..];
            let mut count = 0;
            let mut in_enum = false;
            for line in end_search.lines() {
                if line.contains("enum Work {") {
                    in_enum = true;
                    continue;
                }
                if in_enum && line.trim() == "}" {
                    break;
                }
                if in_enum && line.contains(" { ") && line.trim().ends_with("},") {
                    count += 1;
                }
            }
            count
        }

        let variants = count_work_variants(&code);

        // The indexed approach should have a fixed ~50-55 variants regardless of grammar size
        assert!(
            variants <= 60,
            "Should have ~50 fixed variants, got {}",
            variants
        );
    }
}
